use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::ApiResponse;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HealthData {
    pub alive: bool,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses(
        (status = 200, description = "Health check response", body = ApiResponse<HealthData>)
    )
)]
pub async fn health() -> Json<ApiResponse<HealthData>> {
    Json(ApiResponse::ok(HealthData { alive: true }))
}
