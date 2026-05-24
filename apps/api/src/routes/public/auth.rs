use admin_httpz::{ApiResponse, AppResult};
use axum::{Json, extract::State, http::HeaderMap};
use serde_json::Value;
use system::users::LoginError;

use crate::auth::errors;
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
    let login_result = match state
        .auth_session_service
        .login(&state.pool, &state.password_service, payload)
        .await
    {
        Ok(result) => {
            let _ = system::logs::create_login_log(
                &state.pool,
                system::logs::CreateLoginLog {
                    username,
                    ip: header_value(&headers, "x-forwarded-for"),
                    status: true,
                    error_message: "登录成功".to_string(),
                    agent: header_value(&headers, "user-agent"),
                    user_id: Some(result.user.id),
                },
            )
            .await;
            result
        }
        Err(error) => {
            let error_message = error.to_string();
            let app_error = match &error {
                LoginError::InvalidCredentials => system::errors::users::INVALID_CREDENTIALS.into(),
                LoginError::Disabled => system::errors::users::USER_DISABLED.into(),
                LoginError::UserNotFound => errors::SESSION_INVALID.into(),
                LoginError::UserAlreadyExists => system::errors::users::USER_ALREADY_EXISTS.into(),
                LoginError::InvalidPassword => system::errors::users::INVALID_PASSWORD.into(),
                LoginError::Auth(_) | LoginError::Database(_) => errors::LOGIN_OPERATION_FAILED
                    .into_error()
                    .with_source(error),
            };
            let _ = system::logs::create_login_log(
                &state.pool,
                system::logs::CreateLoginLog {
                    username,
                    ip: header_value(&headers, "x-forwarded-for"),
                    status: false,
                    error_message,
                    agent: header_value(&headers, "user-agent"),
                    user_id: None,
                },
            )
            .await;
            return Err(app_error);
        }
    };

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "user": login_result.user,
        "token": login_result.token,
    }))))
}

fn header_value(headers: &HeaderMap, key: &str) -> String {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}
