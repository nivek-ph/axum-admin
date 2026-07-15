use admin_httpz::{ApiResponse, AppResult};
use axum::{Json, Router, extract::State, routing::get};
use serde_json::Value;

use super::{dto::MenuPayload, error::map_error};
use crate::{extractors::current_user::CurrentUser, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/current", get(get_menu))
        .route("/tree", get(get_base_menu_tree))
}

#[utoipa::path(get, path = "/api/menus/current", tag = "menu", security(("bearer_auth" = [])))]
pub async fn get_menu(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (menus, permissions) = state.menus.current(user.id).await.map_err(map_error)?;
    let menus = menus.into_iter().map(MenuPayload::from).collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(
        serde_json::json!({ "menus": menus, "permissions": permissions }),
    )))
}

pub async fn get_base_menu_tree(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let menus = state
        .menus
        .tree()
        .await
        .map_err(map_error)?
        .into_iter()
        .map(MenuPayload::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({ "menus": menus }))))
}
