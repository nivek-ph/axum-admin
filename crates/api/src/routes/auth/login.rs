use audit::{
    AuditAction, AuditActor, AuditEvent, AuditReason, AuditResource, AuditResult, AuditService,
    AuditSource,
};
use auth::{captcha::CaptchaError, password::PasswordError, token::TokenIssueError};
use axum::{Extension, Json, extract::State};
use iam::accounts;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use utoipa::ToSchema;

use crate::{
    ApiResponse, AppResult,
    extractors::{client_ip::ClientIp, user_agent::UserAgent},
    request_id::request_id_text,
    routes::users::dto::UserResponse,
    state::AppState,
};

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub captcha: String,
    #[serde(rename = "captchaId")]
    pub captcha_id: String,
}

struct LoginInput {
    username: String,
    password: String,
    captcha: String,
    captcha_id: String,
    req_id: String,
    ip: String,
    agent: String,
}

impl LoginInput {
    fn validate(&self) -> Result<(), LoginError> {
        if self.captcha.trim().is_empty() || self.captcha_id.trim().is_empty() {
            return Err(LoginError::CaptchaRequired);
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    pub user: UserResponse,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum LoginError {
    #[error("captcha is required")]
    CaptchaRequired,
    #[error("captcha is invalid or expired")]
    CaptchaInvalid,
    #[error("captcha operation failed")]
    Captcha(#[source] CaptchaError),
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("user is disabled")]
    Disabled,
    #[error("password operation failed")]
    Password(#[source] PasswordError),
    #[error("account operation failed")]
    Account(#[source] accounts::AccountError),
    #[error("token operation failed")]
    Token(#[source] TokenIssueError),
}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login success", body = ApiResponse<LoginResponse>),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Extension(request_id): Extension<RequestId>,
    UserAgent(agent): UserAgent,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    let result = execute_login(
        &state,
        LoginInput {
            username: payload.username,
            password: payload.password,
            captcha: payload.captcha,
            captcha_id: payload.captcha_id,
            req_id: request_id_text(&request_id),
            ip,
            agent,
        },
    )
    .await?;
    Ok(Json(ApiResponse::ok(result)))
}

async fn execute_login(state: &AppState, input: LoginInput) -> Result<LoginResponse, LoginError> {
    if let Err(error) = input.validate() {
        record_login(
            &state.audits,
            &input,
            AuditResult::Denied,
            Some(AuditReason::CaptchaRequired),
            None,
        )
        .await;
        return Err(error);
    }

    let captcha_valid = match state
        .captcha
        .verify(&input.captcha_id, &input.captcha)
        .await
    {
        Ok(valid) => valid,
        Err(error) => {
            record_login(
                &state.audits,
                &input,
                AuditResult::Failed,
                Some(AuditReason::CaptchaFailed),
                None,
            )
            .await;
            return Err(LoginError::Captcha(error));
        }
    };
    if !captcha_valid {
        record_login(
            &state.audits,
            &input,
            AuditResult::Denied,
            Some(AuditReason::CaptchaInvalid),
            None,
        )
        .await;
        return Err(LoginError::CaptchaInvalid);
    }

    let account = match state.accounts.login_account(&input.username).await {
        Ok(Some(account)) => account,
        Ok(None) => {
            record_login(
                &state.audits,
                &input,
                AuditResult::Denied,
                Some(AuditReason::InvalidCredentials),
                None,
            )
            .await;
            return Err(LoginError::InvalidCredentials);
        }
        Err(error) => {
            record_login(
                &state.audits,
                &input,
                AuditResult::Failed,
                Some(AuditReason::InternalError),
                None,
            )
            .await;
            return Err(LoginError::Account(error));
        }
    };
    if !account.enable {
        record_login(
            &state.audits,
            &input,
            AuditResult::Denied,
            Some(AuditReason::UserDisabled),
            None,
        )
        .await;
        return Err(LoginError::Disabled);
    }
    let password_valid = match state
        .passwords
        .verify_password(&input.password, &account.password_hash)
    {
        Ok(password_valid) => password_valid,
        Err(error) => {
            record_login(
                &state.audits,
                &input,
                AuditResult::Failed,
                Some(AuditReason::InternalError),
                None,
            )
            .await;
            return Err(LoginError::Password(error));
        }
    };
    if !password_valid {
        record_login(
            &state.audits,
            &input,
            AuditResult::Denied,
            Some(AuditReason::InvalidCredentials),
            None,
        )
        .await;
        return Err(LoginError::InvalidCredentials);
    }
    let user = match state.accounts.info(account.id).await {
        Ok(user) => user,
        Err(error) => {
            record_login(
                &state.audits,
                &input,
                AuditResult::Failed,
                Some(AuditReason::InternalError),
                None,
            )
            .await;
            return Err(LoginError::Account(error));
        }
    };

    let pair = match state
        .tokens
        .create_session(account.id, &account.username)
        .await
    {
        Ok(pair) => pair,
        Err(error) => {
            record_login(
                &state.audits,
                &input,
                AuditResult::Failed,
                Some(AuditReason::TokenIssueFailed),
                Some(user.id),
            )
            .await;
            return Err(LoginError::Token(error));
        }
    };

    record_login(
        &state.audits,
        &input,
        AuditResult::Succeeded,
        None,
        Some(user.id),
    )
    .await;

    Ok(LoginResponse {
        user: UserResponse::from(user),
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
    })
}

async fn record_login(
    audit: &AuditService,
    input: &LoginInput,
    result: AuditResult,
    reason_code: Option<AuditReason>,
    user_id: Option<i64>,
) {
    audit
        .record_best_effort(AuditEvent {
            req_id: input.req_id.clone(),
            actor: AuditActor {
                id: user_id,
                label: input.username.clone(),
            },
            action: AuditAction::Login,
            resource: AuditResource::Account(input.username.clone()),
            result,
            reason_code,
            source: AuditSource {
                ip: input.ip.clone(),
                user_agent: input.agent.clone(),
            },
            changes: Vec::new(),
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn redis_connection() -> redis::aio::MultiplexedConnection {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open")
    }

    async fn seed_captcha(redis: &mut redis::aio::MultiplexedConnection, id: &str, answer: &str) {
        redis::cmd("SETEX")
            .arg(format!("auth:captcha:{id}"))
            .arg(300)
            .arg(answer)
            .query_async::<()>(redis)
            .await
            .expect("captcha should be seeded");
    }

    fn login_input(username: &str, password: &str, captcha_id: &str) -> LoginInput {
        LoginInput {
            username: username.to_string(),
            password: password.to_string(),
            captcha: "ABCD".to_string(),
            captcha_id: captcha_id.to_string(),
            req_id: format!("req-{captcha_id}"),
            ip: "127.0.0.1".to_string(),
            agent: "login-test".to_string(),
        }
    }

    #[test]
    fn missing_captcha_is_rejected() {
        let error = LoginInput {
            username: "admin".to_string(),
            password: "secret".to_string(),
            captcha: String::new(),
            captcha_id: String::new(),
            req_id: "req-login-validation-1".to_string(),
            ip: String::new(),
            agent: String::new(),
        }
        .validate()
        .expect_err("missing captcha should fail");

        assert!(matches!(error, LoginError::CaptchaRequired));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn missing_captcha_records_a_denied_login_event(pool: sqlx::PgPool) {
        let state = crate::state::tests::test_state(pool.clone()).await;
        let error = execute_login(
            &state,
            LoginInput {
                username: "admin".to_string(),
                password: "must-not-be-recorded".to_string(),
                captcha: String::new(),
                captcha_id: String::new(),
                req_id: "req-login-denied-1".to_string(),
                ip: "127.0.0.1".to_string(),
                agent: "login-test".to_string(),
            },
        )
        .await
        .expect_err("missing captcha should fail");
        assert!(matches!(error, LoginError::CaptchaRequired));

        let event: (String, String, String, String) = sqlx::query_as(
            r#"
            select action, result, reason_code, changes::text
            from sys_audit_events
            where actor_label = 'admin'
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(event.0, "auth.login");
        assert_eq!(event.1, "denied");
        assert_eq!(event.2, "captcha_required");
        assert_eq!(event.3, "[]");

        let payload = sqlx::query_scalar::<_, String>(
            "select jsonb_agg(to_jsonb(e))::text from sys_audit_events e",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!payload.contains("must-not-be-recorded"));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn successful_login_records_the_expected_audit_classification(pool: sqlx::PgPool) {
        record_login(
            &AuditService::new(pool.clone()),
            &LoginInput {
                username: "admin".to_string(),
                password: "must-not-be-recorded".to_string(),
                captcha: "must-not-be-recorded".to_string(),
                captcha_id: "must-not-be-recorded".to_string(),
                req_id: "req-login-success-1".to_string(),
                ip: "127.0.0.1".to_string(),
                agent: "login-test".to_string(),
            },
            AuditResult::Succeeded,
            None,
            Some(1),
        )
        .await;

        let event: (String, String, Option<String>, String) = sqlx::query_as(
            r#"
            select action, result, reason_code, jsonb_agg(to_jsonb(e)) over ()::text
            from sys_audit_events e
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(event.0, "auth.login");
        assert_eq!(event.1, "succeeded");
        assert_eq!(event.2, None);
        assert!(!event.3.contains("must-not-be-recorded"));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn missing_account_and_wrong_password_share_the_non_enumerating_login_result(
        pool: sqlx::PgPool,
    ) {
        let password_hash = auth::password::PasswordService::new()
            .hash_password("correct-password")
            .unwrap();
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id
            )
            values (711, 'login-non-enumeration', 'known-user', $1, 'Known User', '',
                    'dashboard', true, 1)
            "#,
        )
        .bind(password_hash)
        .execute(&pool)
        .await
        .unwrap();
        let mut redis = redis_connection().await;
        seed_captcha(&mut redis, "missing-account", "ABCD").await;
        seed_captcha(&mut redis, "wrong-password", "ABCD").await;
        let mut state = crate::state::tests::test_state(pool.clone()).await;
        state.captcha = auth::captcha::CaptchaService::new(redis);

        let missing = execute_login(
            &state,
            login_input("missing-user", "must-not-be-recorded", "missing-account"),
        )
        .await
        .expect_err("missing account should be rejected");
        let wrong = execute_login(
            &state,
            login_input("known-user", "must-not-be-recorded", "wrong-password"),
        )
        .await
        .expect_err("wrong password should be rejected");

        assert!(matches!(missing, LoginError::InvalidCredentials));
        assert!(matches!(wrong, LoginError::InvalidCredentials));
        let events = sqlx::query_as::<_, (String, String)>(
            "select result, reason_code from sys_audit_events order by req_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            events,
            vec![
                ("denied".to_string(), "invalid_credentials".to_string()),
                ("denied".to_string(), "invalid_credentials".to_string()),
            ]
        );
        let audit_payload = sqlx::query_scalar::<_, String>(
            "select jsonb_agg(to_jsonb(e))::text from sys_audit_events e",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!audit_payload.contains("must-not-be-recorded"));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn account_lookup_failure_keeps_failed_login_audit_anonymous(pool: sqlx::PgPool) {
        let password_hash = auth::password::PasswordService::new()
            .hash_password("correct-password")
            .unwrap();
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id
            )
            values (712, 'login-identity-failure', 'identity-failure', $1, 'Identity Failure', '',
                    'dashboard', true, 1)
            "#,
        )
        .bind(password_hash)
        .execute(&pool)
        .await
        .unwrap();
        let mut redis = redis_connection().await;
        seed_captcha(&mut redis, "identity-failure", "ABCD").await;
        let mut state = crate::state::tests::test_state(pool.clone()).await;
        state.captcha = auth::captcha::CaptchaService::new(redis);
        sqlx::query("drop table sys_users cascade")
            .execute(&pool)
            .await
            .unwrap();

        execute_login(
            &state,
            login_input("identity-failure", "correct-password", "identity-failure"),
        )
        .await
        .expect_err("account lookup should fail when account storage is unavailable");

        let event = sqlx::query_as::<_, (Option<i64>, String, String)>(
            "select actor_id, result, reason_code from sys_audit_events",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            event,
            (None, "failed".to_string(), "internal_error".to_string())
        );
    }
}
