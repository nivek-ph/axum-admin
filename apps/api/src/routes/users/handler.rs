use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
};
use serde_json::Value;

use super::dto::*;
use super::error::map_error;
use crate::extractors::current_user::CurrentUser;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_user_info).put(set_self_info))
        .route("/me/password", put(change_password))
        .route("/me/settings", put(set_self_setting))
        .route("/", get(get_user_list_by_query).post(admin_register))
        .route("/{id}", put(set_user_info_by_id).delete(delete_user_by_id))
        .route("/{id}/password/reset", post(reset_password_by_id))
        .route("/{id}/roles", put(set_user_roles_by_id))
}

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
pub async fn get_user_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ApiResponse<Value>>> {
    let user = UserResponse::from(state.users.info(user.id).await.map_err(map_error)?);
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "userInfo": user,
    }))))
}

pub async fn get_user_list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<GetUserListRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);
    let page_size = payload.page_size.max(1);
    let (list, total) = state
        .users
        .list_with_scope(payload.into(), user.data_scope.clone())
        .await
        .map_err(map_error)?;
    let list = list.into_iter().map(UserResponse::from).collect::<Vec<_>>();

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
    Query(payload): Query<GetUserListRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);
    let page_size = payload.page_size.max(1);
    let (list, total) = state
        .users
        .list_with_scope(payload.into(), user.data_scope.clone())
        .await
        .map_err(map_error)?;
    let list = list.into_iter().map(UserResponse::from).collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}

pub async fn admin_register(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_authorization(&state).await?;
    state
        .users
        .register_as(user.id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("registered")))
}

pub async fn change_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .users
        .change_password(user.id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn set_user_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let target_user_id = payload.id;
    invalidate_authorization(&state).await?;
    state
        .users
        .update_as(user.id, target_user_id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn set_user_info_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_authorization(&state).await?;
    state
        .users
        .update_as(user.id, id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn set_self_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<SetSelfInfoRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .users
        .set_self_info(user.id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn set_self_setting(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<SetSelfSettingRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .users
        .set_self_setting(user.id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn delete_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<DeleteUserRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_authorization(&state).await?;
    state
        .users
        .delete_as(user.id, payload.id)
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_user_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_authorization(&state).await?;
    state
        .users
        .delete_as(user.id, id)
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn reset_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let target_user_id = payload.id;
    state
        .users
        .reset_password_as(user.id, target_user_id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("password reset")))
}

pub async fn reset_password_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .users
        .reset_password_as(user.id, id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("password reset")))
}

pub async fn set_user_roles_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<SetUserRolesRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_authorization(&state).await?;
    state
        .users
        .set_roles_as(user.id, id, payload.into())
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("roles updated")))
}

async fn invalidate_authorization(state: &AppState) -> AppResult<()> {
    state.authorization.invalidate().await.map_err(|source| {
        crate::errors::INTERNAL_SERVER_ERROR
            .into_error()
            .with_source(source)
    })
}
