use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};
use serde_json::Value;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_category_list).post(add_category))
        .route("/{id}", delete(delete_category_by_id))
}

pub async fn get_category_list(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = file_storage::category::list(&state.pool).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!(list))))
}

pub async fn add_category(
    State(state): State<AppState>,
    Json(payload): Json<file_storage::category::CategoryPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    file_storage::category::upsert(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("operation succeeded")))
}

pub async fn delete_category(
    State(state): State<AppState>,
    Json(payload): Json<file_storage::category::DeleteCategoryPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    file_storage::category::delete(&state.pool, payload.id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_category_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    file_storage::category::delete(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}
