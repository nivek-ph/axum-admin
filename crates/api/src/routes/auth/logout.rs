use axum::{Json, extract::State, http::HeaderMap};
use serde_json::Value;

use crate::{
    ApiResponse, AppResult, mappings::LOGIN_REQUIRED, middleware::auth::extract_bearer_token,
    state::AppState,
};

#[utoipa::path(
    post,
    path = "/auth/logout",
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
    let token = extract_bearer_token(&headers).ok_or(LOGIN_REQUIRED)?;
    state.tokens.revoke(token).await?;

    Ok(Json(ApiResponse::ok_message("signed out")))
}
