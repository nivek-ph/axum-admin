use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};

use super::dto::*;
use crate::{ApiResponse, AppResult, extractors::current_user::CurrentUser, state::AppState};

#[utoipa::path(
    get,
    path = "/users/me",
    tag = "user",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user info", body = ApiResponse<UserInfoData>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_user_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ApiResponse<UserInfoData>>> {
    let user = UserResponse::from(state.users.info(user.id).await?);
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

    let (list, total) = state
        .users
        .list_with_scope(payload, user.data_scope.clone())
        .await?;

    let list = list.into_iter().map(UserResponse::from).collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(UserListData {
        list,
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
    responses((status = 200, description = "User registered", body = ApiResponse<UserMutationData>))
)]
pub async fn admin_register(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<RegisterUserRequest>,
) -> AppResult<Json<ApiResponse<UserMutationData>>> {
    state.users.register_as(user.id, payload).await?;

    Ok(Json(ApiResponse::new("OK", "registered", None)))
}

#[utoipa::path(
    put,
    path = "/users/me/password",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses((status = 200, description = "Password changed", body = ApiResponse<UserMutationData>))
)]
pub async fn change_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<Json<ApiResponse<UserMutationData>>> {
    state.users.change_password(user.id, payload).await?;

    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/{id}",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses((status = 200, description = "User updated", body = ApiResponse<UserMutationData>))
)]
pub async fn set_user_info_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<UserMutationData>>> {
    state.users.update(user.id, id, payload.into()).await?;

    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/me",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = UpdateSelfRequest,
    responses((status = 200, description = "Current user updated", body = ApiResponse<UserMutationData>))
)]
pub async fn set_self_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<UpdateSelfRequest>,
) -> AppResult<Json<ApiResponse<UserMutationData>>> {
    state.users.set_self_info(user.id, payload).await?;

    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/me/settings",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = UpdateSelfSettingsRequest,
    responses((status = 200, description = "User settings updated", body = ApiResponse<UserMutationData>))
)]
pub async fn set_self_setting(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<UpdateSelfSettingsRequest>,
) -> AppResult<Json<ApiResponse<UserMutationData>>> {
    state.users.set_self_setting(user.id, payload).await?;

    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    responses((status = 200, description = "User deleted", body = ApiResponse<UserMutationData>))
)]
pub async fn delete_user_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<UserMutationData>>> {
    state.users.delete(user.id, id).await?;

    Ok(Json(ApiResponse::new("OK", "deleted", None)))
}

#[utoipa::path(
    post,
    path = "/users/{id}/password/reset",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = ResetPasswordRequest,
    responses((status = 200, description = "Password reset", body = ApiResponse<UserMutationData>))
)]
pub async fn reset_password_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<UserMutationData>>> {
    state
        .users
        .reset_password_as(user.id, id, payload.into())
        .await?;

    Ok(Json(ApiResponse::new("OK", "password reset", None)))
}

#[utoipa::path(
    put,
    path = "/users/{id}/roles",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = SetUserRolesRequest,
    responses((status = 200, description = "User roles updated", body = ApiResponse<UserMutationData>))
)]
pub async fn set_user_roles_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Extension(audit_context): Extension<audit::AuditContext>,
    Path(id): Path<i64>,
    Json(payload): Json<SetUserRolesRequest>,
) -> AppResult<Json<ApiResponse<UserMutationData>>> {
    state
        .users
        .set_roles_as(user.id, id, payload, audit_context)
        .await?;

    Ok(Json(ApiResponse::new("OK", "roles updated", None)))
}
