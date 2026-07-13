use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, put},
};
use serde_json::Value;

use super::dto::{RoleDeptPayload, RoleMenuPayload, RolePayload, RoleResponse, RoleUsersPayload};
use super::error::map_error;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_roles).post(create_role))
        .route("/{id}", put(update_role).delete(delete_role))
        .route("/{id}/menus", get(get_role_menus).put(set_role_menus))
        .route("/{id}/depts", get(get_role_depts).put(set_role_depts))
        .route("/{id}/users", get(get_role_users).put(set_role_users))
}

pub async fn get_roles(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let list = state
        .roles
        .list()
        .await
        .map_err(map_error)?
        .into_iter()
        .map(RoleResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

pub async fn create_role(
    State(state): State<AppState>,
    Json(payload): Json<RolePayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let role = RoleResponse::from(
        state
            .roles
            .create(payload.into())
            .await
            .map_err(map_error)?,
    );

    Ok(Json(ApiResponse::ok(serde_json::json!({ "role": role }))))
}

pub async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RolePayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let role = RoleResponse::from(
        state
            .roles
            .update(id, payload.into())
            .await
            .map_err(map_error)?,
    );

    Ok(Json(ApiResponse::ok(serde_json::json!({ "role": role }))))
}

pub async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.roles.delete(id).await.map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn get_role_menus(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let menu_ids = state.roles.menu_ids(id).await.map_err(map_error)?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "menuIds": menu_ids,
    }))))
}

pub async fn set_role_menus(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleMenuPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .roles
        .set_menu_ids(id, payload.menu_ids)
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("saved")))
}

pub async fn get_role_depts(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let dept_ids = state.roles.dept_ids(id).await.map_err(map_error)?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "deptIds": dept_ids,
    }))))
}

pub async fn set_role_depts(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleDeptPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .roles
        .set_dept_ids(id, payload.dept_ids)
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("saved")))
}

pub async fn get_role_users(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let user_ids = state.roles.user_ids(id).await.map_err(map_error)?;

    Ok(Json(ApiResponse::ok(serde_json::json!(user_ids))))
}

pub async fn set_role_users(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RoleUsersPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .roles
        .set_user_ids(id, payload.user_ids)
        .await
        .map_err(map_error)?;

    Ok(Json(ApiResponse::ok_message("saved")))
}
