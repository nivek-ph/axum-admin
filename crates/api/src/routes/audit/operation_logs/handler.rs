use audit::operation_logs::OperationLogSearch;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::{Value, json};

use super::dto::OperationLogResponse;
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    get,
    path = "/operation-logs/{id}",
    tag = "audit",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Operation log ID")),
    responses((status = 200, description = "Operation log detail", body = ApiResponse<Value>))
)]
pub async fn find_operation_log_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = state
        .operation_logs
        .find(id)
        .await?
        .map(OperationLogResponse::from);
    Ok(Json(ApiResponse::ok(json!(item))))
}

#[utoipa::path(
    get,
    path = "/operation-logs",
    tag = "audit",
    security(("bearer_auth" = [])),
    params(OperationLogSearch),
    responses((status = 200, description = "Operation log list", body = ApiResponse<Value>))
)]
pub async fn get_operation_log_list(
    State(state): State<AppState>,
    Query(payload): Query<OperationLogSearch>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);

    let page_size = payload.page_size.max(1);
    let (list, total) = state.operation_logs.list(payload).await?;
    let list = list
        .into_iter()
        .map(OperationLogResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}
