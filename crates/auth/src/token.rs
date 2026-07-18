use redis::{AsyncCommands, aio::MultiplexedConnection};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::jwt::{Claims, JwtService};

const SESSION_KEY_PREFIX: &str = "auth:login-session:";
const SESSION_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;
const TOKEN_PART_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TokenIssueError {
    #[error("token signing failed")]
    Signing(#[source] jsonwebtoken::errors::Error),
    #[error("token session store is unavailable")]
    StoreUnavailable,
    #[error("token session store operation failed")]
    Store(#[from] redis::RedisError),
}

#[derive(Debug, thiserror::Error)]
pub enum TokenSessionError {
    #[error("token expired")]
    Expired(#[source] jsonwebtoken::errors::Error),
    #[error("token invalid")]
    Invalid(#[source] jsonwebtoken::errors::Error),
    #[error("token session invalid")]
    SessionInvalid,
    #[error("token session store is unavailable")]
    StoreUnavailable,
    #[error("token session store operation failed")]
    Store(#[from] redis::RedisError),
}

#[derive(Debug, thiserror::Error)]
pub enum TokenRevokeError {
    #[error("token invalid")]
    Invalid(#[source] jsonwebtoken::errors::Error),
    #[error("token session store is unavailable")]
    StoreUnavailable,
    #[error("token session store operation failed")]
    Store(#[from] redis::RedisError),
}

#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("refresh token invalid")]
    Invalid,
    #[error("login session invalid")]
    SessionInvalid,
    #[error("token signing failed")]
    Signing(#[source] jsonwebtoken::errors::Error),
    #[error("token session store is unavailable")]
    StoreUnavailable,
    #[error("token session store operation failed")]
    Store(#[from] redis::RedisError),
}

#[derive(Debug, Clone)]
pub struct RefreshGrant {
    session_id: String,
    user_id: i64,
    refresh_hash: String,
}

impl RefreshGrant {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn user_id(&self) -> i64 {
        self.user_id
    }
}

#[derive(Clone)]
pub struct TokenService {
    jwt_service: JwtService,
    redis_connection: Option<MultiplexedConnection>,
}

impl TokenService {
    pub fn new(jwt_secret: &str, redis_connection: MultiplexedConnection) -> Self {
        Self {
            jwt_service: JwtService::new(jwt_secret),
            redis_connection: Some(redis_connection),
        }
    }

    pub fn without_session_store(jwt_secret: &str) -> Self {
        Self {
            jwt_service: JwtService::new(jwt_secret),
            redis_connection: None,
        }
    }

    pub async fn create_session(
        &self,
        user_id: i64,
        username: &str,
    ) -> Result<TokenPair, TokenIssueError> {
        let session_id = random_token_part();
        let refresh_secret = random_token_part();
        let access_token = self
            .jwt_service
            .issue_token(user_id, username, &session_id)
            .map_err(TokenIssueError::Signing)?;
        let mut redis = self
            .redis_connection
            .clone()
            .ok_or(TokenIssueError::StoreUnavailable)?;
        let _: () = redis::pipe()
            .atomic()
            .hset(session_key(&session_id), "user_id", user_id)
            .hset(
                session_key(&session_id),
                "refresh_hash",
                hash_refresh_secret(&refresh_secret),
            )
            .expire(session_key(&session_id), SESSION_TTL_SECONDS as i64)
            .query_async(&mut redis)
            .await?;
        Ok(TokenPair {
            access_token,
            refresh_token: format!("{session_id}.{refresh_secret}"),
        })
    }

    pub async fn decode_active(&self, token: &str) -> Result<Claims, TokenSessionError> {
        let claims = self.jwt_service.decode_token(token).map_err(|error| {
            if matches!(
                error.kind(),
                jsonwebtoken::errors::ErrorKind::ExpiredSignature
            ) {
                TokenSessionError::Expired(error)
            } else {
                TokenSessionError::Invalid(error)
            }
        })?;
        let mut redis = self
            .redis_connection
            .clone()
            .ok_or(TokenSessionError::StoreUnavailable)?;
        let session_user_id: Option<i64> = redis.hget(session_key(&claims.sid), "user_id").await?;
        if session_user_id != Some(claims.user_id) {
            return Err(TokenSessionError::SessionInvalid);
        }
        Ok(claims)
    }

    pub async fn inspect_refresh(&self, refresh_token: &str) -> Result<RefreshGrant, RefreshError> {
        let (session_id, secret) =
            parse_refresh_token(refresh_token).ok_or(RefreshError::Invalid)?;
        let mut redis = self
            .redis_connection
            .clone()
            .ok_or(RefreshError::StoreUnavailable)?;
        let (user_id, stored_hash): (Option<i64>, Option<String>) = redis::cmd("HMGET")
            .arg(session_key(session_id))
            .arg("user_id")
            .arg("refresh_hash")
            .query_async(&mut redis)
            .await?;
        let (Some(user_id), Some(stored_hash)) = (user_id, stored_hash) else {
            return Err(RefreshError::SessionInvalid);
        };
        let refresh_hash = hash_refresh_secret(secret);
        if stored_hash != refresh_hash {
            return Err(RefreshError::Invalid);
        }
        Ok(RefreshGrant {
            session_id: session_id.to_string(),
            user_id,
            refresh_hash,
        })
    }

    pub async fn rotate_refresh(
        &self,
        grant: RefreshGrant,
        username: &str,
    ) -> Result<TokenPair, RefreshError> {
        let refresh_secret = random_token_part();
        let next_hash = hash_refresh_secret(&refresh_secret);
        let access_token = self
            .jwt_service
            .issue_token(grant.user_id, username, &grant.session_id)
            .map_err(RefreshError::Signing)?;
        let mut redis = self
            .redis_connection
            .clone()
            .ok_or(RefreshError::StoreUnavailable)?;
        let result: i64 = redis::Script::new(
            r#"
            if redis.call('EXISTS', KEYS[1]) == 0 then
                return 0
            end
            if redis.call('HGET', KEYS[1], 'user_id') ~= ARGV[3] then
                return 0
            end
            if redis.call('HGET', KEYS[1], 'refresh_hash') ~= ARGV[1] then
                return -1
            end
            redis.call('HSET', KEYS[1], 'refresh_hash', ARGV[2])
            return 1
            "#,
        )
        .key(session_key(&grant.session_id))
        .arg(&grant.refresh_hash)
        .arg(&next_hash)
        .arg(grant.user_id)
        .invoke_async(&mut redis)
        .await?;
        match result {
            1 => Ok(TokenPair {
                access_token,
                refresh_token: format!("{}.{}", grant.session_id, refresh_secret),
            }),
            -1 => Err(RefreshError::Invalid),
            _ => Err(RefreshError::SessionInvalid),
        }
    }

    pub async fn revoke(&self, token: &str) -> Result<(), TokenRevokeError> {
        let claims = self
            .jwt_service
            .decode_token(token)
            .map_err(TokenRevokeError::Invalid)?;
        let mut redis = self
            .redis_connection
            .clone()
            .ok_or(TokenRevokeError::StoreUnavailable)?;
        let _: usize = redis.del(session_key(&claims.sid)).await?;
        Ok(())
    }
}

#[inline]
fn session_key(session_id: &str) -> String {
    format!("{SESSION_KEY_PREFIX}{session_id}")
}

fn random_token_part() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_refresh_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

fn parse_refresh_token(token: &str) -> Option<(&str, &str)> {
    let (session_id, secret) = token.split_once('.')?;
    let valid_part = |part: &str| {
        part.len() == TOKEN_PART_HEX_LENGTH && part.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    (valid_part(session_id) && valid_part(secret)).then_some((session_id, secret))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use jsonwebtoken::{EncodingKey, Header, encode};

    use super::*;

    #[tokio::test]
    async fn issuing_without_session_store_fails_closed() {
        let error = TokenService::without_session_store("test-secret")
            .create_session(1, "admin")
            .await
            .expect_err("issuing without a session store should fail");

        assert!(matches!(error, TokenIssueError::StoreUnavailable));
    }

    #[tokio::test]
    async fn login_sessions_are_independent_and_revocable() {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let redis = redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open");
        let tokens = TokenService::new("test-secret", redis);

        let first = tokens
            .create_session(1, "admin")
            .await
            .expect("first login session should be issued");
        let second = tokens
            .create_session(1, "admin")
            .await
            .expect("second login session should be issued");
        let first_claims = tokens
            .decode_active(&first.access_token)
            .await
            .expect("first login session should be active");
        let second_claims = tokens
            .decode_active(&second.access_token)
            .await
            .expect("second login session should be active");
        assert_ne!(first_claims.sid, second_claims.sid);

        tokens
            .revoke(&first.access_token)
            .await
            .expect("first login session should be revoked");
        let error = tokens
            .decode_active(&first.access_token)
            .await
            .expect_err("revoked login session should be rejected");
        assert!(matches!(error, TokenSessionError::SessionInvalid));
        tokens
            .decode_active(&second.access_token)
            .await
            .expect("second login session should remain active");

        tokens
            .revoke(&second.access_token)
            .await
            .expect("second login session should be cleaned up");
    }

    #[tokio::test]
    async fn login_session_issues_a_token_pair_with_absolute_expiration() {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let redis = redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open");
        let mut inspection = redis.clone();
        let tokens = TokenService::new("test-secret", redis);

        let pair = tokens
            .create_session(1, "admin")
            .await
            .expect("login session should be issued");
        let claims = tokens
            .decode_active(&pair.access_token)
            .await
            .expect("access token should reference an active session");
        let (sid, secret) = parse_refresh_token(&pair.refresh_token)
            .expect("issued refresh token should be well formed");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_secs() as usize;
        let ttl: i64 = inspection
            .ttl(session_key(sid))
            .await
            .expect("session TTL should be readable");
        let stored_hash: String = inspection
            .hget(session_key(sid), "refresh_hash")
            .await
            .expect("refresh hash should be stored");

        assert_eq!(claims.sid, sid);
        assert!((14 * 60..=15 * 60).contains(&(claims.exp - now)));
        assert!((7 * 24 * 60 * 60 - 1..=7 * 24 * 60 * 60).contains(&ttl));
        assert_ne!(stored_hash, secret);

        tokens
            .revoke(&pair.access_token)
            .await
            .expect("login session should be cleaned up");
    }

    #[tokio::test]
    async fn concurrent_refresh_rotation_has_one_winner_without_extending_ttl() {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let redis = redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open");
        let mut inspection = redis.clone();
        let tokens = TokenService::new("test-secret", redis);
        let original = tokens
            .create_session(1, "admin")
            .await
            .expect("login session should be issued");
        let first_grant = tokens
            .inspect_refresh(&original.refresh_token)
            .await
            .expect("refresh token should be accepted");
        let second_grant = tokens
            .inspect_refresh(&original.refresh_token)
            .await
            .expect("refresh token should be accepted before rotation");
        let ttl_before: i64 = inspection
            .ttl(session_key(first_grant.session_id()))
            .await
            .expect("session TTL should be readable");

        let (first, second) = tokio::join!(
            tokens.rotate_refresh(first_grant, "admin"),
            tokens.rotate_refresh(second_grant, "admin")
        );
        let (winner, loser) = match (first, second) {
            (Ok(pair), Err(error)) | (Err(error), Ok(pair)) => (pair, error),
            result => panic!("exactly one refresh should win: {result:?}"),
        };
        assert!(matches!(loser, RefreshError::Invalid));
        let old_error = tokens
            .inspect_refresh(&original.refresh_token)
            .await
            .expect_err("rotated refresh token should be rejected");
        assert!(matches!(old_error, RefreshError::Invalid));
        let new_grant = tokens
            .inspect_refresh(&winner.refresh_token)
            .await
            .expect("new refresh token should be accepted");
        let ttl_after: i64 = inspection
            .ttl(session_key(new_grant.session_id()))
            .await
            .expect("session TTL should remain readable");

        assert!(ttl_after <= ttl_before);
        tokens
            .revoke(&winner.access_token)
            .await
            .expect("login session should be cleaned up");
    }

    #[tokio::test]
    async fn refresh_errors_distinguish_invalid_missing_and_unavailable_sessions() {
        let unavailable = TokenService::without_session_store("test-secret");
        assert!(matches!(
            unavailable
                .inspect_refresh("malformed")
                .await
                .expect_err("malformed token should fail"),
            RefreshError::Invalid
        ));
        let valid_shape = format!("{}.{}", "a".repeat(64), "b".repeat(64));
        assert!(matches!(
            unavailable
                .inspect_refresh(&valid_shape)
                .await
                .expect_err("missing store should fail closed"),
            RefreshError::StoreUnavailable
        ));

        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let redis = redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open");
        let tokens = TokenService::new("test-secret", redis);
        let pair = tokens
            .create_session(1, "admin")
            .await
            .expect("login session should be issued");
        let mut mismatched = pair.refresh_token.clone();
        let replacement = if mismatched.ends_with('0') { '1' } else { '0' };
        mismatched.pop();
        mismatched.push(replacement);
        assert!(matches!(
            tokens
                .inspect_refresh(&mismatched)
                .await
                .expect_err("mismatched secret should fail"),
            RefreshError::Invalid
        ));
        tokens
            .revoke(&pair.access_token)
            .await
            .expect("login session should be removed");
        assert!(matches!(
            tokens
                .inspect_refresh(&pair.refresh_token)
                .await
                .expect_err("missing session should fail"),
            RefreshError::SessionInvalid
        ));
    }

    #[tokio::test]
    async fn expired_access_token_is_classified_separately() {
        let token = encode(
            &Header::default(),
            &Claims {
                user_id: 1,
                username: "admin".to_string(),
                sid: "session-id".to_string(),
                exp: 1,
            },
            &EncodingKey::from_secret(b"test-secret"),
        )
        .expect("expired token should encode");
        let error = TokenService::without_session_store("test-secret")
            .decode_active(&token)
            .await
            .expect_err("expired token should be rejected before session lookup");

        assert!(matches!(error, TokenSessionError::Expired(_)));
    }
}
