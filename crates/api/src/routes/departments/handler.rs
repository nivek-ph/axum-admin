use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::Value;

use super::dto::{DeptNodeResponse, DeptPayload, DeptResponse};
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    get,
    path = "/depts",
    tag = "department",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Department tree", body = ApiResponse<Value>))
)]
pub async fn get_dept_tree(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let tree = state
        .departments
        .tree()
        .await?
        .into_iter()
        .map(DeptNodeResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": tree
    }))))
}

#[utoipa::path(
    get,
    path = "/depts/{id}",
    tag = "department",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Department ID")),
    responses((status = 200, description = "Department detail", body = ApiResponse<Value>))
)]
pub async fn find_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = state.departments.find(id).await?.map(DeptResponse::from);
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "dept": item.map(|dept| serde_json::json!(dept)).unwrap_or_else(|| serde_json::json!({}))
    }))))
}

#[utoipa::path(
    post,
    path = "/depts",
    tag = "department",
    security(("bearer_auth" = [])),
    request_body = DeptPayload,
    responses((status = 200, description = "Department created", body = ApiResponse<Value>))
)]
pub async fn create_dept(
    State(state): State<AppState>,
    Json(payload): Json<DeptPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;
    state.departments.create(payload.into()).await?;
    Ok(Json(ApiResponse::ok_message("created")))
}

#[utoipa::path(
    put,
    path = "/depts/{id}",
    tag = "department",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Department ID")),
    request_body = DeptPayload,
    responses((status = 200, description = "Department updated", body = ApiResponse<Value>))
)]
pub async fn update_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<DeptPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;
    state.departments.update(id, payload.into()).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    delete,
    path = "/depts/{id}",
    tag = "department",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Department ID")),
    responses((status = 200, description = "Department deleted", body = ApiResponse<Value>))
)]
pub async fn delete_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;
    state.departments.delete(id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

async fn invalidate_access(state: &AppState) -> AppResult<()> {
    state
        .access
        .invalidate()
        .await
        .map_err(crate::mappings::access_invalidation_error)
}
