use admin_httpz::ApiResponse;
use axum::Json;
use serde_json::{Value, json};

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "system",
    responses(
        (status = 200, description = "Health check response", body = crate::docs::HealthResponse)
    )
)]
pub async fn health() -> Json<ApiResponse<Value>> {
    Json(ApiResponse::ok(json!({ "alive": true })))
}
