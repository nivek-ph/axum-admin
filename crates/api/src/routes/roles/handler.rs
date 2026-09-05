use axum::{
    Extension, Json,
    extract::{Path, State},
};

use super::dto::{
    RoleAccessData, RoleAccessRequest, RoleData, RoleListData, RoleRequest, RoleResponse,
};
use crate::{
    ApiResponse, AppResult, EmptyData, extractors::current_user::CurrentUser, state::AppState,
};

#[utoipa::path(get, path = "/roles", tag = "role", security(("bearer_auth" = [])))]
pub async fn get_roles(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ApiResponse<RoleListData>>> {
    let list = state
        .roles
        .list(user.id)
        .await?
        .into_iter()
        .map(RoleResponse::from)
        .collect();
    Ok(Json(ApiResponse::ok(RoleListData { list })))
}

#[utoipa::path(post, path = "/roles", tag = "role", security(("bearer_auth" = [])), request_body = RoleRequest)]
pub async fn create_role(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Extension(audit_context): Extension<audit::AuditContext>,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<ApiResponse<RoleData>>> {
    let role = state.roles.create(user.id, payload, audit_context).await?;
    Ok(Json(ApiResponse::ok(RoleData { role: role.into() })))
}

#[utoipa::path(put, path = "/roles/{id}", tag = "role", security(("bearer_auth" = [])), request_body = RoleRequest)]
pub async fn update_role(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Extension(audit_context): Extension<audit::AuditContext>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<ApiResponse<RoleData>>> {
    let role = state
        .roles
        .update(user.id, id, payload, audit_context)
        .await?;
    Ok(Json(ApiResponse::ok(RoleData { role: role.into() })))
}

#[utoipa::path(delete, path = "/roles/{id}", tag = "role", security(("bearer_auth" = [])))]
pub async fn delete_role(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Extension(audit_context): Extension<audit::AuditContext>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.roles.delete(user.id, id, audit_context).await?;
    Ok(Json(ApiResponse::new("OK", "deleted", None)))
}

#[utoipa::path(get, path = "/roles/{id}/access", tag = "role", security(("bearer_auth" = [])))]
pub async fn get_role_access(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<RoleAccessData>>> {
    Ok(Json(ApiResponse::ok(
        state.roles.access(user.id, id).await?.into(),
    )))
}

#[utoipa::path(put, path = "/roles/{id}/access", tag = "role", security(("bearer_auth" = [])), request_body = RoleAccessRequest)]
pub async fn set_role_access(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Extension(audit_context): Extension<audit::AuditContext>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleAccessRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state
        .roles
        .replace_access(user.id, id, payload.permissions, audit_context)
        .await?;
    Ok(Json(ApiResponse::new("OK", "saved", None)))
}
