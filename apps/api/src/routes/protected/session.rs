use admin_httpz::{ApiResponse, AppResult, OptionAppExt};
use axum::{Json, extract::State, http::HeaderMap};
use serde_json::Value;

use crate::{errors::auth as errors, middleware::auth::extract_bearer_token, state::AppState};

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Logout success"),
        (status = 401, description = "Invalid or revoked session")
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ApiResponse<Value>>> {
    let token = extract_bearer_token(&headers).ok_or_spec(errors::LOGIN_REQUIRED)?;
    state.auth_session_service.revoke_token(token).await?;

    Ok(Json(ApiResponse::ok_message("退出登录成功")))
}
