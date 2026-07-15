use axum::{
    Json,
    extract::{Path, State},
};
use iam::roles::{RoleDeptPayload, RoleMenuPayload, RolePayload, RoleUsersPayload};
use serde_json::Value;

use super::dto::RoleResponse;
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    get,
    path = "/roles",
    tag = "role",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Role list", body = ApiResponse<Value>))
)]
pub async fn get_roles(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let list = state
        .roles
        .list()
        .await?
        .into_iter()
        .map(RoleResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

#[utoipa::path(
    post,
    path = "/roles",
    tag = "role",
    security(("bearer_auth" = [])),
    request_body = RolePayload,
    responses((status = 200, description = "Role created", body = ApiResponse<Value>))
)]
pub async fn create_role(
    State(state): State<AppState>,
    Json(payload): Json<RolePayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let role = RoleResponse::from(state.roles.create(payload).await?);

    Ok(Json(ApiResponse::ok(serde_json::json!({ "role": role }))))
}

#[utoipa::path(
    put,
    path = "/roles/{id}",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RolePayload,
    responses((status = 200, description = "Role updated", body = ApiResponse<Value>))
)]
pub async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RolePayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let role = RoleResponse::from(state.roles.update(id, payload).await?);

    Ok(Json(ApiResponse::ok(serde_json::json!({ "role": role }))))
}

#[utoipa::path(
    delete,
    path = "/roles/{id}",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role deleted", body = ApiResponse<Value>))
)]
pub async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.roles.delete(id).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

#[utoipa::path(
    get,
    path = "/roles/{id}/menus",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role menu IDs", body = ApiResponse<Value>))
)]
pub async fn get_role_menus(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let menu_ids = state.roles.menu_ids(id).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "menuIds": menu_ids,
    }))))
}

#[utoipa::path(
    put,
    path = "/roles/{id}/menus",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RoleMenuPayload,
    responses((status = 200, description = "Role menus saved", body = ApiResponse<Value>))
)]
pub async fn set_role_menus(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleMenuPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.roles.set_menu_ids(id, payload.menu_ids).await?;

    Ok(Json(ApiResponse::ok_message("saved")))
}

#[utoipa::path(
    get,
    path = "/roles/{id}/depts",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role department IDs", body = ApiResponse<Value>))
)]
pub async fn get_role_depts(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let dept_ids = state.roles.dept_ids(id).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "deptIds": dept_ids,
    }))))
}

#[utoipa::path(
    put,
    path = "/roles/{id}/depts",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RoleDeptPayload,
    responses((status = 200, description = "Role departments saved", body = ApiResponse<Value>))
)]
pub async fn set_role_depts(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleDeptPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.roles.set_dept_ids(id, payload.dept_ids).await?;

    Ok(Json(ApiResponse::ok_message("saved")))
}

#[utoipa::path(
    get,
    path = "/roles/{id}/users",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    responses((status = 200, description = "Role user IDs", body = ApiResponse<Value>))
)]
pub async fn get_role_users(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let user_ids = state.roles.user_ids(id).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!(user_ids))))
}

#[utoipa::path(
    put,
    path = "/roles/{id}/users",
    tag = "role",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Role ID")),
    request_body = RoleUsersPayload,
    responses((status = 200, description = "Role users saved", body = ApiResponse<Value>))
)]
pub async fn set_role_users(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleUsersPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.roles.set_user_ids(id, payload.user_ids).await?;

    Ok(Json(ApiResponse::ok_message("saved")))
}
