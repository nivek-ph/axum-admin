use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    jwt::{Claims, JwtService},
    password::AuthError,
};
use captcha_rs::CaptchaBuilder;
use redis::{AsyncCommands, aio::MultiplexedConnection};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const REDIS_HASH_KEY: &str = "auth:revoked-tokens";
const CAPTCHA_KEY_PREFIX: &str = "auth:captcha:";
const CAPTCHA_TTL_SECONDS: u64 = 300;

pub struct CaptchaChallenge {
    pub id: String,
    pub image: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthSessionError {
    #[error("{0}")]
    Auth(#[from] AuthError),
    #[error("token revoked")]
    Revoked,
    #[error("redis revoke store is unavailable")]
    RevocationStoreUnavailable,
    #[error("{0}")]
    Redis(#[from] redis::RedisError),
    #[error("captcha image rendering failed")]
    CaptchaRenderFailed,
}

#[derive(Clone)]
pub struct AuthSessionService {
    jwt_service: JwtService,
    redis_connection: Option<MultiplexedConnection>,
}

impl AuthSessionService {
    pub fn new(jwt_secret: &str, redis_connection: MultiplexedConnection) -> Self {
        Self {
            jwt_service: JwtService::new(jwt_secret),
            redis_connection: Some(redis_connection),
        }
    }

    pub fn without_revocation_store(jwt_secret: &str) -> Self {
        Self {
            jwt_service: JwtService::new(jwt_secret),
            redis_connection: None,
        }
    }

    pub fn issue_token(&self, user_id: i64, username: &str) -> Result<String, AuthSessionError> {
        Ok(self.jwt_service.issue_token(user_id, username)?)
    }

    pub async fn create_captcha(&self) -> Result<CaptchaChallenge, AuthSessionError> {
        let id = Uuid::new_v4().to_string();

        let captcha = CaptchaBuilder::new()
            .length(4)
            .width(220)
            .height(64)
            .dark_mode(false)
            .complexity(2)
            .compression(85)
            .drop_shadow(true)
            .interference_lines(1)
            .interference_ellipses(0)
            .distortion(2)
            .build();
        let image = captcha.to_base64();
        let code = captcha.text;

        // TODO: remove it after captcha is fixed
        if image == "data:image/jpeg;base64," {
            return Err(AuthSessionError::CaptchaRenderFailed);
        }

        let key = format!("{CAPTCHA_KEY_PREFIX}{id}");
        let mut redis = self.redis_connection()?;
        let _: () = redis.set_ex(key, &code, CAPTCHA_TTL_SECONDS).await?;
        Ok(CaptchaChallenge { id, image })
    }

    pub async fn verify_captcha(&self, id: &str, answer: &str) -> Result<bool, AuthSessionError> {
        let key = format!("{CAPTCHA_KEY_PREFIX}{id}");
        let mut redis = self.redis_connection()?;
        let expected: Option<String> = redis::cmd("GETDEL")
            .arg(key)
            .query_async(&mut redis)
            .await?;
        Ok(expected.is_some_and(|value| value.eq_ignore_ascii_case(answer.trim())))
    }

    pub async fn decode_active_token(&self, token: &str) -> Result<Claims, AuthSessionError> {
        let claims = self.decode_token(token)?;
        if self.is_token_revoked(token).await? {
            return Err(AuthSessionError::Revoked);
        }
        Ok(claims)
    }

    pub async fn revoke_token(&self, token: &str) -> Result<(), AuthSessionError> {
        let claims = self.decode_token(token)?;
        let now_epoch = current_epoch_seconds();
        let Some(ttl) = redis_ttl_seconds(claims.exp, now_epoch) else {
            return Ok(());
        };

        let mut redis = self.redis_connection()?;
        let field = redis_hash_field_for_token(token);
        set_revoked_token_field(&mut redis, &field, redis_revoke_value(now_epoch), ttl).await?;

        Ok(())
    }

    pub async fn is_token_revoked(&self, token: &str) -> Result<bool, AuthSessionError> {
        let mut redis = self.redis_connection()?;
        let revoked: bool = redis
            .hexists(REDIS_HASH_KEY, redis_hash_field_for_token(token))
            .await?;
        Ok(revoked)
    }

    fn redis_connection(&self) -> Result<MultiplexedConnection, AuthSessionError> {
        self.redis_connection
            .clone()
            .ok_or(AuthSessionError::RevocationStoreUnavailable)
    }

    fn decode_token(&self, token: &str) -> Result<Claims, AuthError> {
        self.jwt_service.decode_token(token)
    }
}

fn current_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

fn token_hash_for_storage(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn redis_hash_field_for_token(token: &str) -> String {
    token_hash_for_storage(token)
}

fn redis_ttl_seconds(exp: usize, now_epoch: i64) -> Option<u64> {
    let remaining = exp as i64 - now_epoch;
    if remaining <= 0 {
        return None;
    }
    Some((remaining + 1) as u64)
}

fn redis_revoke_value(now_epoch: i64) -> String {
    now_epoch.to_string()
}

async fn set_revoked_token_field(
    redis: &mut MultiplexedConnection,
    field: &str,
    value: String,
    ttl: u64,
) -> Result<(), redis::RedisError> {
    let _: (usize, Vec<i64>) = redis::pipe()
        .hset(REDIS_HASH_KEY, field, value)
        .cmd("HEXPIRE")
        .arg(REDIS_HASH_KEY)
        .arg(ttl)
        .arg("FIELDS")
        .arg(1)
        .arg(field)
        .query_async(redis)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_token_for_revocation_storage() {
        let first = redis_hash_field_for_token("token-one");
        let second = redis_hash_field_for_token("token-two");

        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
        assert!(first.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn ttl_exceeds_remaining_token_lifetime() {
        assert_eq!(redis_ttl_seconds(101, 100), Some(2));
        assert_eq!(redis_ttl_seconds(100, 100), None);
        assert_eq!(redis_ttl_seconds(99, 100), None);
    }

    #[test]
    fn revocation_value_is_current_epoch_second() {
        assert_eq!(redis_revoke_value(12345), "12345");
    }
}
