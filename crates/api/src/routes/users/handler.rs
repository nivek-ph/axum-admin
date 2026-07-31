use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};

use super::dto::*;
use crate::{
    ApiResponse, AppResult, EmptyData, extractors::current_user::CurrentUser, state::AppState,
};

#[utoipa::path(
    get,
    path = "/users/me",
    tag = "user",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Current user info", body = ApiResponse<UserInfoData>))
)]
pub async fn get_user_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ApiResponse<UserInfoData>>> {
    let user = UserResponse::from(state.accounts.info(user.id).await?);
    Ok(Json(ApiResponse::ok(UserInfoData { user_info: user })))
}

#[utoipa::path(
    get,
    path = "/users",
    tag = "user",
    security(("bearer_auth" = [])),
    params(UserListRequest),
    responses((status = 200, description = "User list", body = ApiResponse<UserListData>))
)]
pub async fn get_user_list_by_query(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(payload): Query<UserListRequest>,
) -> AppResult<Json<ApiResponse<UserListData>>> {
    let page = payload.page.max(1);
    let page_size = payload.page_size.max(1);
    let (list, total) = state.accounts.list(user.id, payload.into()).await?;
    Ok(Json(ApiResponse::ok(UserListData {
        list: list.into_iter().map(UserResponse::from).collect(),
        total,
        page,
        page_size,
    })))
}

#[utoipa::path(
    post,
    path = "/users",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = RegisterUserRequest,
    responses((status = 200, description = "User registered", body = ApiResponse<EmptyData>))
)]
pub async fn admin_register(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<RegisterUserRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    let password_hash = state.passwords.hash_password(&payload.password)?;
    state
        .accounts
        .create(user.id, payload.into_account_input(password_hash))
        .await?;
    Ok(Json(ApiResponse::new("OK", "registered", None)))
}

#[utoipa::path(
    put,
    path = "/users/me/password",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses((status = 200, description = "Password changed", body = ApiResponse<EmptyData>))
)]
pub async fn change_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    let current_hash = state.accounts.password_hash(user.id).await?;
    if !state
        .passwords
        .verify_password(&payload.password, &current_hash)?
    {
        return Err(crate::mappings::INVALID_PASSWORD.into());
    }
    let password_hash = state.passwords.hash_password(&payload.new_password)?;
    revoke_sessions_and_persist_password(
        &state,
        iam::accounts::PreparedPasswordUpdate::new(user.id, password_hash),
    )
    .await?;
    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/{id}",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses((status = 200, description = "User updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_user_info_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.accounts.update(user.id, id, payload.into()).await?;
    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/me",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = UpdateSelfRequest,
    responses((status = 200, description = "Current user updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_self_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<UpdateSelfRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state
        .accounts
        .set_self_info(user.id, payload.into())
        .await?;
    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/me/settings",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = UpdateSelfSettingsRequest,
    responses((status = 200, description = "User settings updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_self_setting(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<UpdateSelfSettingsRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state
        .accounts
        .set_self_setting(user.id, payload.into())
        .await?;
    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    responses((status = 200, description = "User deleted", body = ApiResponse<EmptyData>))
)]
pub async fn delete_user_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.accounts.delete(user.id, id).await?;
    Ok(Json(ApiResponse::new("OK", "deleted", None)))
}

#[utoipa::path(
    post,
    path = "/users/{id}/password/reset",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = ResetPasswordRequest,
    responses((status = 200, description = "Password reset", body = ApiResponse<EmptyData>))
)]
pub async fn reset_password_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.accounts.validate_password_reset(user.id, id).await?;
    let password_hash = state.passwords.hash_password(&payload.password)?;
    revoke_sessions_and_persist_password(
        &state,
        iam::accounts::PreparedPasswordUpdate::new(id, password_hash),
    )
    .await?;
    Ok(Json(ApiResponse::new("OK", "password reset", None)))
}

async fn revoke_sessions_and_persist_password(
    state: &AppState,
    prepared: iam::accounts::PreparedPasswordUpdate,
) -> AppResult<()> {
    state
        .tokens
        .revoke_user_sessions(prepared.user_id())
        .await?;
    state.accounts.persist_password_update(prepared).await?;
    Ok(())
}

#[utoipa::path(
    put,
    path = "/users/{id}/roles",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = SetUserRolesRequest,
    responses((status = 200, description = "User roles updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_user_roles_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Extension(audit_context): Extension<audit::AuditContext>,
    Path(id): Path<i64>,
    Json(payload): Json<SetUserRolesRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state
        .accounts
        .replace_roles(user.id, id, payload.role_ids, audit_context)
        .await?;
    Ok(Json(ApiResponse::new("OK", "roles updated", None)))
}

#[utoipa::path(
    get,
    path = "/users/{id}/permissions",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    responses((status = 200, description = "User access", body = ApiResponse<UserAccessData>))
)]
pub async fn get_user_access(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<UserAccessData>>> {
    Ok(Json(ApiResponse::ok(
        state.accounts.access(user.id, id).await?.into(),
    )))
}

#[utoipa::path(
    put,
    path = "/users/{id}/permissions",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = SetUserPermissionsRequest,
    responses((status = 200, description = "User permissions updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_user_permissions(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Extension(audit_context): Extension<audit::AuditContext>,
    Path(id): Path<i64>,
    Json(payload): Json<SetUserPermissionsRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state
        .accounts
        .replace_direct_permissions(user.id, id, payload.permissions, audit_context)
        .await?;
    Ok(Json(ApiResponse::new(
        "OK",
        "direct permissions updated",
        None,
    )))
}
