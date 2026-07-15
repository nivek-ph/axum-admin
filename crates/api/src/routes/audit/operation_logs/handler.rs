use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde_json::Value;

use super::dto::{OperationLogResponse, OperationLogSearch};
use super::error::map_error;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_operation_log_list))
        .route("/{id}", get(find_operation_log_by_id))
}

pub async fn find_operation_log_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = state
        .operation_logs
        .find(id)
        .await
        .map_err(map_error)?
        .map(OperationLogResponse::from);
    Ok(Json(ApiResponse::ok(match item {
        Some(log) => serde_json::json!(log),
        None => serde_json::json!({}),
    })))
}

pub async fn get_operation_log_list(
    State(state): State<AppState>,
    Query(payload): Query<OperationLogSearch>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);
    let page_size = payload.page_size.max(1);
    let (list, total) = state
        .operation_logs
        .list(payload.into())
        .await
        .map_err(map_error)?;
    let list = list
        .into_iter()
        .map(OperationLogResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}
