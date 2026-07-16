use axum::{
    Json,
    extract::{Path, State},
};

use super::dto::{
    RoleData, RoleDeptIdsData, RoleDeptRequest, RoleListData, RoleMenuIdsData, RoleMenuRequest,
    RoleMutationData, RoleRequest, RoleResponse, RoleUserIdsData, RoleUsersRequest,
};
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    get,
    path = "/roles",
    tag = "role",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Role list", body = ApiResponse<RoleListData>))
)]
pub async fn get_roles(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<RoleListData>>> {
    let list = state
        .roles
        .list()
        .await?
        .into_iter()
        .map(RoleResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(RoleListData { list })))
}

#[utoipa::path(
    post,
    path = "/roles",
    tag = "role",
    security(("bearer_auth" = [])),
    request_body = RoleRequest,
    responses((status = 200, description = "Role created", body = ApiResponse<RoleData>))
)]
pub async fn create_role(
    State(state): State<AppState>,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<ApiResponse<RoleData>>> {
    let role = RoleResponse::from(state.roles.create(payload).await?);

    Ok(Json(ApiResponse::ok(RoleData { role })))
}

#[utoipa::path(
    put,
    path = "/roles/{id}",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RoleRequest,
    responses((status = 200, description = "Role updated", body = ApiResponse<RoleData>))
)]
pub async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<ApiResponse<RoleData>>> {
    let role = RoleResponse::from(state.roles.update(id, payload).await?);

    Ok(Json(ApiResponse::ok(RoleData { role })))
}

#[utoipa::path(
    delete,
    path = "/roles/{id}",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role deleted", body = ApiResponse<RoleMutationData>))
)]
pub async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<RoleMutationData>>> {
    state.roles.delete(id).await?;

    Ok(Json(ApiResponse::new("OK", "deleted", None)))
}

#[utoipa::path(
    get,
    path = "/roles/{id}/menus",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role menu IDs", body = ApiResponse<RoleMenuIdsData>))
)]
pub async fn get_role_menus(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<RoleMenuIdsData>>> {
    let menu_ids = state.roles.menu_ids(id).await?;

    Ok(Json(ApiResponse::ok(RoleMenuIdsData { menu_ids })))
}

#[utoipa::path(
    put,
    path = "/roles/{id}/menus",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RoleMenuRequest,
    responses((status = 200, description = "Role menus saved", body = ApiResponse<RoleMutationData>))
)]
pub async fn set_role_menus(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleMenuRequest>,
) -> AppResult<Json<ApiResponse<RoleMutationData>>> {
    state.roles.set_menu_ids(id, payload.menu_ids).await?;

    Ok(Json(ApiResponse::new("OK", "saved", None)))
}

#[utoipa::path(
    get,
    path = "/roles/{id}/depts",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role department IDs", body = ApiResponse<RoleDeptIdsData>))
)]
pub async fn get_role_depts(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<RoleDeptIdsData>>> {
    let dept_ids = state.roles.dept_ids(id).await?;

    Ok(Json(ApiResponse::ok(RoleDeptIdsData { dept_ids })))
}

#[utoipa::path(
    put,
    path = "/roles/{id}/depts",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RoleDeptRequest,
    responses((status = 200, description = "Role departments saved", body = ApiResponse<RoleMutationData>))
)]
pub async fn set_role_depts(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleDeptRequest>,
) -> AppResult<Json<ApiResponse<RoleMutationData>>> {
    state.roles.set_dept_ids(id, payload.dept_ids).await?;

    Ok(Json(ApiResponse::new("OK", "saved", None)))
}

#[utoipa::path(
    get,
    path = "/roles/{id}/users",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role user IDs", body = ApiResponse<RoleUserIdsData>))
)]
pub async fn get_role_users(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<RoleUserIdsData>>> {
    let user_ids = state.roles.user_ids(id).await?;

    Ok(Json(ApiResponse::ok(RoleUserIdsData(user_ids))))
}

#[utoipa::path(
    put,
    path = "/roles/{id}/users",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RoleUsersRequest,
    responses((status = 200, description = "Role users saved", body = ApiResponse<RoleMutationData>))
)]
pub async fn set_role_users(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleUsersRequest>,
) -> AppResult<Json<ApiResponse<RoleMutationData>>> {
    state.roles.set_user_ids(id, payload.user_ids).await?;

    Ok(Json(ApiResponse::new("OK", "saved", None)))
}
