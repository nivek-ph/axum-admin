use audit::{
    AuditAction, AuditActor, AuditEvent, AuditReason, AuditResource, AuditResult, AuditService,
    AuditSource,
};
use auth::{captcha::CaptchaError, token::TokenIssueError};
use axum::{Json, extract::State, http::HeaderMap};
use iam::users;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{ApiResponse, AppResult, routes::users::dto::UserResponse, state::AppState};

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
    pub token: String,
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
    #[error("{0}")]
    Identity(#[source] users::AuthenticateError),
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
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    let result = execute_login(
        &state,
        LoginInput {
            username: payload.username,
            password: payload.password,
            captcha: payload.captcha,
            captcha_id: payload.captcha_id,
            ip: header_value(&headers, "x-forwarded-for"),
            agent: header_value(&headers, "user-agent"),
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

    let identity = match state
        .users
        .authenticate(users::LoginRequest {
            username: input.username.clone(),
            password: input.password.clone(),
        })
        .await
    {
        Ok(identity) => identity,
        Err(error) => {
            let (result, reason) = match &error {
                users::AuthenticateError::InvalidCredentials => {
                    (AuditResult::Denied, AuditReason::InvalidCredentials)
                }
                users::AuthenticateError::Disabled => {
                    (AuditResult::Denied, AuditReason::UserDisabled)
                }
                users::AuthenticateError::Credential(_) | users::AuthenticateError::Database(_) => {
                    (AuditResult::Failed, AuditReason::InternalError)
                }
            };
            record_login(&state.audits, &input, result, Some(reason), None).await;
            return Err(LoginError::Identity(error));
        }
    };

    let token = match state.tokens.issue(identity.id, &identity.username) {
        Ok(token) => token,
        Err(error) => {
            record_login(
                &state.audits,
                &input,
                AuditResult::Failed,
                Some(AuditReason::TokenIssueFailed),
                Some(identity.user.id),
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
        Some(identity.user.id),
    )
    .await;

    Ok(LoginResponse {
        user: UserResponse::from(identity.user),
        token,
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

fn header_value(headers: &HeaderMap, key: &str) -> String {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_captcha_is_rejected() {
        let error = LoginInput {
            username: "admin".to_string(),
            password: "secret".to_string(),
            captcha: String::new(),
            captcha_id: String::new(),
            ip: String::new(),
            agent: String::new(),
        }
        .validate()
        .expect_err("missing captcha should fail");

        assert!(matches!(error, LoginError::CaptchaRequired));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn missing_captcha_records_a_denied_login_event(pool: sqlx::PgPool) {
        let state = crate::state::test_state(pool.clone());
        let error = execute_login(
            &state,
            LoginInput {
                username: "admin".to_string(),
                password: "must-not-be-recorded".to_string(),
                captcha: String::new(),
                captcha_id: String::new(),
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
}
