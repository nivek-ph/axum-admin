use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde_json::Value;

use crate::{extractors::current_user::CurrentUser, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/current", get(get_menu))
        .route("/", get(get_menu_list).post(add_base_menu))
        .route("/tree", get(get_base_menu_tree))
        .route(
            "/{id}",
            get(get_base_menu_by_path_id)
                .put(update_base_menu_by_id)
                .delete(delete_base_menu_by_id),
        )
        .route(
            "/{id}/roles",
            get(get_menu_roles_by_id).put(set_menu_roles_by_id),
        )
}

#[utoipa::path(
    get,
    path = "/api/menus/current",
    tag = "menu",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Dynamic menus", body = crate::docs::MenuResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_menu(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ApiResponse<Value>>> {
    let menus =
        system::menu::get_menu_tree_for_user(&state.pool, user.id, user.authority_id).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "menus": menus,
    }))))
}

pub async fn get_menu_list(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let menus = system::menu::get_menu_list(&state.pool).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!(menus))))
}

pub async fn get_base_menu_tree(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let menus = system::menu::get_base_menu_tree(&state.pool).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "menus": menus,
    }))))
}

pub async fn add_base_menu(
    State(state): State<AppState>,
    Json(payload): Json<system::menu::MenuView>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::menu::add_base_menu(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn update_base_menu(
    State(state): State<AppState>,
    Json(payload): Json<system::menu::MenuView>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::menu::update_base_menu(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn update_base_menu_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<system::menu::MenuView>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    system::menu::update_base_menu(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn delete_base_menu(
    State(state): State<AppState>,
    Json(payload): Json<system::menu::MenuIdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::menu::delete_base_menu(&state.pool, payload.id).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_base_menu_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::menu::delete_base_menu(&state.pool, id).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn get_base_menu_by_id(
    State(state): State<AppState>,
    Json(payload): Json<system::menu::MenuIdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let menu = system::menu::get_base_menu_by_id(&state.pool, payload.id).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "menu": menu,
    }))))
}

pub async fn get_base_menu_by_path_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let menu = system::menu::get_base_menu_by_id(&state.pool, id).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "menu": menu,
    }))))
}

pub async fn get_menu_roles(
    State(state): State<AppState>,
    Query(payload): Query<system::menu::MenuIdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = system::menu::get_menu_roles(&state.pool, payload.id).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!(data))))
}

pub async fn get_menu_roles_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = system::menu::get_menu_roles(&state.pool, id).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!(data))))
}

pub async fn set_menu_roles(
    State(state): State<AppState>,
    Json(payload): Json<system::menu::SetMenuRolesRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::menu::set_menu_roles(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok_message("assigned")))
}

pub async fn set_menu_roles_by_id(
    State(state): State<AppState>,
    Path(menu_id): Path<i64>,
    Json(mut payload): Json<system::menu::SetMenuRolesRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.menu_id = menu_id;
    system::menu::set_menu_roles(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok_message("assigned")))
}
