use audit::login_logs::LoginLogSearch;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::Value;

use super::dto::LoginLogResponse;
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    get,
    path = "/login-logs",
    tag = "audit",
    security(("bearer_auth" = [])),
    params(LoginLogSearch),
    responses((status = 200, description = "Login log list", body = ApiResponse<Value>))
)]
pub async fn get_login_log_list(
    State(state): State<AppState>,
    Query(payload): Query<LoginLogSearch>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);

    let page_size = payload.page_size.max(1);

    let (list, total) = state.login_logs.list(payload).await?;

    let list = list
        .into_iter()
        .map(LoginLogResponse::from)
        .collect::<Vec<_>>();

    let data = serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    });

    Ok(Json(ApiResponse::ok(data)))
}

#[utoipa::path(
    get,
    path = "/login-logs/{id}",
    tag = "audit",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Login log ID")),
    responses((status = 200, description = "Login log detail", body = ApiResponse<Value>))
)]
pub async fn find_login_log_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = state.login_logs.find(id).await?.map(LoginLogResponse::from);

    Ok(Json(ApiResponse::ok(serde_json::json!(item))))
}
