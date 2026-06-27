use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::Value;

use crate::extractors::current_user::CurrentUser;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/users/me",
    tag = "user",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user info", body = crate::docs::UserInfoResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_user_info(CurrentUser(user): CurrentUser) -> Json<ApiResponse<Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "userInfo": user.user,
    })))
}

pub async fn get_user_list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<system::users::GetUserListRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);
    let page_size = payload.page_size.max(1);
    let (list, total) = system::users::get_user_list(&state.pool, payload, Some(user.id)).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}

pub async fn get_user_list_by_query(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(payload): Query<system::users::GetUserListRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);
    let page_size = payload.page_size.max(1);
    let (list, total) = system::users::get_user_list(&state.pool, payload, Some(user.id)).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}

pub async fn admin_register(
    State(state): State<AppState>,
    Json(payload): Json<system::users::RegisterRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::register_user(&state.pool, &state.password_service, payload).await?;

    Ok(Json(ApiResponse::ok_message("registered")))
}

pub async fn change_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<system::users::ChangePasswordRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::change_password(&state.pool, &state.password_service, user.id, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn set_user_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<system::users::UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::ensure_user_in_scope(&state.pool, user.id, payload.id).await?;
    system::users::update_user(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn set_user_info_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(mut payload): Json<system::users::UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    system::users::ensure_user_in_scope(&state.pool, user.id, id).await?;
    system::users::update_user(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn set_self_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<system::users::SetSelfInfoRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::set_self_info(&state.pool, user.id, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn set_self_setting(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<system::users::SetSelfSettingRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::set_self_setting(&state.pool, user.id, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn delete_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<system::users::DeleteUserRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::ensure_user_in_scope(&state.pool, user.id, payload.id).await?;
    system::users::delete_user(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_user_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::ensure_user_in_scope(&state.pool, user.id, id).await?;
    system::users::delete_user(&state.pool, system::users::DeleteUserRequest { id }).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn reset_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<system::users::ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::ensure_user_in_scope(&state.pool, user.id, payload.id).await?;
    system::users::reset_password(&state.pool, &state.password_service, payload).await?;

    Ok(Json(ApiResponse::ok_message("password reset")))
}

pub async fn reset_password_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(mut payload): Json<system::users::ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    system::users::ensure_user_in_scope(&state.pool, user.id, id).await?;
    system::users::reset_password(&state.pool, &state.password_service, payload).await?;

    Ok(Json(ApiResponse::ok_message("password reset")))
}

pub async fn set_user_roles_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<system::users::SetUserRolesRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::users::ensure_user_in_scope(&state.pool, user.id, id).await?;
    system::users::set_user_roles(&state.pool, id, payload).await?;

    Ok(Json(ApiResponse::ok_message("roles updated")))
}
