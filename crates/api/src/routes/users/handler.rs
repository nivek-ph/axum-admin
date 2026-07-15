use axum::{
    Json,
    extract::{Path, Query, State},
};
use iam::users::{
    ChangePasswordRequest, GetUserListRequest, RegisterRequest, ResetPasswordRequest,
    SetSelfInfoRequest, SetSelfSettingRequest, SetUserRolesRequest, UpdateUserRequest,
};
use serde_json::Value;

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
    params(GetUserListRequest),
    responses((status = 200, description = "User list", body = ApiResponse<Value>))
)]
pub async fn get_user_list_by_query(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(payload): Query<GetUserListRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);

    let page_size = payload.page_size.max(1);

    let (list, total) = state
        .users
        .list_with_scope(payload, user.data_scope.clone())
        .await?;

    let list = list.into_iter().map(UserResponse::from).collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}

#[utoipa::path(
    post,
    path = "/users",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = RegisterRequest,
    responses((status = 200, description = "User registered", body = ApiResponse<Value>))
)]
pub async fn admin_register(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;

    state.users.register_as(user.id, payload).await?;

    Ok(Json(ApiResponse::ok_message("registered")))
}

#[utoipa::path(
    put,
    path = "/users/me/password",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses((status = 200, description = "Password changed", body = ApiResponse<Value>))
)]
pub async fn change_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.users.change_password(user.id, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    put,
    path = "/users/{id}",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses((status = 200, description = "User updated", body = ApiResponse<Value>))
)]
pub async fn set_user_info_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;

    state.users.update(user.id, id, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    put,
    path = "/users/me",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = SetSelfInfoRequest,
    responses((status = 200, description = "Current user updated", body = ApiResponse<Value>))
)]
pub async fn set_self_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<SetSelfInfoRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.users.set_self_info(user.id, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    put,
    path = "/users/me/settings",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = SetSelfSettingRequest,
    responses((status = 200, description = "User settings updated", body = ApiResponse<Value>))
)]
pub async fn set_self_setting(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<SetSelfSettingRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.users.set_self_setting(user.id, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    responses((status = 200, description = "User deleted", body = ApiResponse<Value>))
)]
pub async fn delete_user_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;
    state.users.delete(user.id, id).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

#[utoipa::path(
    post,
    path = "/users/{id}/password/reset",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = ResetPasswordRequest,
    responses((status = 200, description = "Password reset", body = ApiResponse<Value>))
)]
pub async fn reset_password_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.users.reset_password_as(user.id, id, payload).await?;

    Ok(Json(ApiResponse::ok_message("password reset")))
}

#[utoipa::path(
    put,
    path = "/users/{id}/roles",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = SetUserRolesRequest,
    responses((status = 200, description = "User roles updated", body = ApiResponse<Value>))
)]
pub async fn set_user_roles_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<SetUserRolesRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;

    state.users.set_roles_as(user.id, id, payload).await?;

    Ok(Json(ApiResponse::ok_message("roles updated")))
}

async fn invalidate_access(state: &AppState) -> AppResult<()> {
    state
        .access
        .invalidate()
        .await
        .map_err(crate::mappings::access_invalidation_error)
}
