use admin_httpz::{ApiResponse, AppResult};
use axum::{Json, extract::State, http::HeaderMap};
use serde_json::Value;

use crate::errors::auth::{
    CAPTCHA_INVALID, CAPTCHA_OPERATION_FAILED, CAPTCHA_REQUIRED, LOGIN_OPERATION_FAILED,
};
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = system::users::LoginRequest,
    responses(
        (status = 200, description = "Login success", body = crate::docs::LoginResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<system::users::LoginRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let username = payload.username.clone();

    if payload.captcha.trim().is_empty() || payload.captcha_id.trim().is_empty() {
        record_failed_login(&state, &headers, username, CAPTCHA_REQUIRED.message).await;
        return Err(CAPTCHA_REQUIRED.into());
    }
    let captcha_valid = match state
        .auth_session_service
        .verify_captcha(&payload.captcha_id, &payload.captcha)
        .await
    {
        Ok(valid) => valid,
        Err(error) => {
            record_failed_login(&state, &headers, username, CAPTCHA_OPERATION_FAILED.message).await;
            return Err(CAPTCHA_OPERATION_FAILED.into_error().with_source(error));
        }
    };
    if !captcha_valid {
        record_failed_login(&state, &headers, username, CAPTCHA_INVALID.message).await;
        return Err(CAPTCHA_INVALID.into());
    }

    let login_result = match state
        .auth_session_service
        .login(&state.pool, &state.password_service, payload)
        .await
        .map_err(|error| {
            let error_message = error.to_string();
            let app_error = match error {
                system::users::LoginError::InvalidCredentials
                | system::users::LoginError::UserNotFound => {
                    system::errors::users::INVALID_CREDENTIALS.into()
                }
                system::users::LoginError::Disabled => system::errors::users::USER_DISABLED.into(),
                system::users::LoginError::UserAlreadyExists => {
                    system::errors::users::USER_ALREADY_EXISTS.into()
                }
                system::users::LoginError::InvalidPassword => {
                    system::errors::users::INVALID_PASSWORD.into()
                }
                system::users::LoginError::Auth(_) | system::users::LoginError::Database(_) => {
                    LOGIN_OPERATION_FAILED.into_error().with_source(error)
                }
            };
            (app_error, error_message)
        }) {
        Ok(result) => {
            let _ = system::logs::create_login_log(
                &state.pool,
                system::logs::CreateLoginLog {
                    username,
                    ip: header_value(&headers, "x-forwarded-for"),
                    status: true,
                    error_message: "login succeeded".to_string(),
                    agent: header_value(&headers, "user-agent"),
                    user_id: Some(result.user.id),
                },
            )
            .await;
            result
        }
        Err((app_error, error_message)) => {
            record_failed_login(&state, &headers, username, &error_message).await;
            return Err(app_error);
        }
    };

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "user": login_result.user,
        "token": login_result.token,
    }))))
}

async fn record_failed_login(
    state: &AppState,
    headers: &HeaderMap,
    username: String,
    error_message: &str,
) {
    let _ = system::logs::create_login_log(
        &state.pool,
        system::logs::CreateLoginLog {
            username,
            ip: header_value(headers, "x-forwarded-for"),
            status: false,
            error_message: error_message.to_string(),
            agent: header_value(headers, "user-agent"),
            user_id: None,
        },
    )
    .await;
}

fn header_value(headers: &HeaderMap, key: &str) -> String {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}
