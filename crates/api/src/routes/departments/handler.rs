use axum::{
    Json,
    extract::{Path, State},
};

use super::dto::{
    DeptDetail, DeptDetailData, DeptMutationData, DeptNodeResponse, DeptPayload, DeptResponse,
    DeptTreeData, EmptyDept,
};
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    get,
    path = "/depts",
    tag = "department",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Department tree", body = ApiResponse<DeptTreeData>))
)]
pub async fn get_dept_tree(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<DeptTreeData>>> {
    let tree = state
        .departments
        .tree()
        .await?
        .into_iter()
        .map(DeptNodeResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(DeptTreeData { list: tree })))
}

#[utoipa::path(
    get,
    path = "/depts/{id}",
    tag = "department",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Department ID")),
    responses((status = 200, description = "Department detail", body = ApiResponse<DeptDetailData>))
)]
pub async fn find_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<DeptDetailData>>> {
    let dept = match state.departments.find(id).await? {
        Some(dept) => DeptDetail::Dept(DeptResponse::from(dept)),
        None => DeptDetail::Empty(EmptyDept {}),
    };
    Ok(Json(ApiResponse::ok(DeptDetailData { dept })))
}

#[utoipa::path(
    post,
    path = "/depts",
    tag = "department",
    security(("bearer_auth" = [])),
    request_body = DeptPayload,
    responses((status = 200, description = "Department created", body = ApiResponse<DeptMutationData>))
)]
pub async fn create_dept(
    State(state): State<AppState>,
    Json(payload): Json<DeptPayload>,
) -> AppResult<Json<ApiResponse<DeptMutationData>>> {
    state.departments.create(payload.into()).await?;
    Ok(Json(ApiResponse::new("OK", "created", None)))
}

#[utoipa::path(
    put,
    path = "/depts/{id}",
    tag = "department",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Department ID")),
    request_body = DeptPayload,
    responses((status = 200, description = "Department updated", body = ApiResponse<DeptMutationData>))
)]
pub async fn update_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<DeptPayload>,
) -> AppResult<Json<ApiResponse<DeptMutationData>>> {
    state.departments.update(id, payload.into()).await?;
    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    delete,
    path = "/depts/{id}",
    tag = "department",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Department ID")),
    responses((status = 200, description = "Department deleted", body = ApiResponse<DeptMutationData>))
)]
pub async fn delete_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<DeptMutationData>>> {
    state.departments.delete(id).await?;
    Ok(Json(ApiResponse::new("OK", "deleted", None)))
}
