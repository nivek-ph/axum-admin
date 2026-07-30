use axum::{
    Json,
    extract::{Path, State},
};

use super::dto::{
    RoleData, RoleListData, RoleMenuIdsData, RoleMenuRequest, RolePermissionRequest,
    RolePermissionsData, RoleRequest, RoleResponse,
};
use crate::{ApiResponse, AppResult, EmptyData, state::AppState};

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
    let role = RoleResponse::from(state.roles.create(payload.into()).await?);

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
    let role = RoleResponse::from(state.roles.update(id, payload.into()).await?);

    Ok(Json(ApiResponse::ok(RoleData { role })))
}

#[utoipa::path(
    delete,
    path = "/roles/{id}",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role deleted", body = ApiResponse<EmptyData>))
)]
pub async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
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
    let access = state.roles.menu_access(id).await?;

    Ok(Json(ApiResponse::ok(RoleMenuIdsData {
        menu_ids: access.menu_ids,
        effective_menu_ids: access.effective_menu_ids,
        protected: access.protected,
    })))
}

#[utoipa::path(
    put,
    path = "/roles/{id}/menus",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RoleMenuRequest,
    responses((status = 200, description = "Role menus saved", body = ApiResponse<EmptyData>))
)]
pub async fn set_role_menus(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleMenuRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.roles.set_menu_ids(id, payload.menu_ids).await?;

    Ok(Json(ApiResponse::new("OK", "saved", None)))
}

#[utoipa::path(
    get,
    path = "/roles/{id}/permissions",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role permissions", body = ApiResponse<RolePermissionsData>))
)]
pub async fn get_role_permissions(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<RolePermissionsData>>> {
    let policy = state.roles.permissions(id).await?;
    let catalog = state.roles.permission_catalog(id).await?;
    Ok(Json(ApiResponse::ok((policy, catalog).into())))
}

#[utoipa::path(
    put,
    path = "/roles/{id}/permissions",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RolePermissionRequest,
    responses((status = 200, description = "Role permissions saved", body = ApiResponse<EmptyData>))
)]
pub async fn set_role_permissions(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RolePermissionRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.roles.set_permissions(id, payload.permissions).await?;
    Ok(Json(ApiResponse::new("OK", "saved", None)))
}
