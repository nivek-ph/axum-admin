use audit::login_logs::CreateLoginLog;
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
        record_login(&state.login_logs, &input, false, &error.to_string(), None).await;
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
                &state.login_logs,
                &input,
                false,
                "captcha operation failed",
                None,
            )
            .await;
            return Err(LoginError::Captcha(error));
        }
    };
    if !captcha_valid {
        record_login(
            &state.login_logs,
            &input,
            false,
            "captcha is invalid or expired",
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
            record_login(&state.login_logs, &input, false, &error.to_string(), None).await;
            return Err(LoginError::Identity(error));
        }
    };

    let token = match state.tokens.issue(identity.id, &identity.username) {
        Ok(token) => token,
        Err(error) => {
            record_login(
                &state.login_logs,
                &input,
                false,
                "token operation failed",
                Some(identity.user.id),
            )
            .await;
            return Err(LoginError::Token(error));
        }
    };

    record_login(
        &state.login_logs,
        &input,
        true,
        "login succeeded",
        Some(identity.user.id),
    )
    .await;

    Ok(LoginResponse {
        user: UserResponse::from(identity.user),
        token,
    })
}

async fn record_login(
    audit: &audit::login_logs::LoginLogService,
    input: &LoginInput,
    status: bool,
    message: &str,
    user_id: Option<i64>,
) {
    let _ = audit
        .record(CreateLoginLog {
            username: input.username.clone(),
            ip: input.ip.clone(),
            status,
            error_message: message.to_string(),
            agent: input.agent.clone(),
            user_id,
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
}
