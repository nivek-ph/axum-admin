use admin_httpz::{ApiResponse, AppResult, OptionAppExt};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde_json::Value;

use crate::errors::request as errors;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_sys_dictionary_detail))
        .route("/tree-by-type", get(get_dictionary_tree_list_by_type))
        .route("/by-parent", get(get_dictionary_details_by_parent))
        .route(
            "/{id}",
            get(find_sys_dictionary_detail_by_id)
                .put(update_sys_dictionary_detail_by_id)
                .delete(delete_sys_dictionary_detail_by_id),
        )
        .route("/{id}/path", get(get_dictionary_path_by_id))
}

pub async fn create_sys_dictionary_detail(
    State(state): State<AppState>,
    Json(payload): Json<system::dictionary::SysDictionaryDetail>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::dictionary::create_detail(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn update_sys_dictionary_detail(
    State(state): State<AppState>,
    Json(payload): Json<system::dictionary::SysDictionaryDetail>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::dictionary::update_detail(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn update_sys_dictionary_detail_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<system::dictionary::SysDictionaryDetail>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    system::dictionary::update_detail(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn find_sys_dictionary_detail(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let id = payload.id.ok_or_spec(errors::ID_REQUIRED)?;
    let item = system::dictionary::find_detail(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "reSysDictionaryDetail": item
    }))))
}

pub async fn find_sys_dictionary_detail_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = system::dictionary::find_detail(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "reSysDictionaryDetail": item
    }))))
}

pub async fn get_dictionary_tree_list(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::DictionaryTreeQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list =
        system::dictionary::tree_by_dictionary(&state.pool, payload.sys_dictionary_id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list
    }))))
}

pub async fn get_dictionary_tree_list_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = system::dictionary::tree_by_dictionary(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list
    }))))
}

pub async fn get_sys_dictionary_detail_list(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::DictionaryTreeQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list =
        system::dictionary::tree_by_dictionary(&state.pool, payload.sys_dictionary_id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

pub async fn get_dictionary_tree_list_by_type(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::DictionaryTypeQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = system::dictionary::tree_by_type(&state.pool, &payload.dict_type).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

pub async fn get_dictionary_details_by_parent(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::DictionaryParentQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = system::dictionary::details_by_parent(&state.pool, payload.parent_id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

pub async fn get_dictionary_path(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let id = payload.id.ok_or_spec(errors::ID_REQUIRED)?;
    let list = system::dictionary::detail_path(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

pub async fn get_dictionary_path_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = system::dictionary::detail_path(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

pub async fn delete_sys_dictionary_detail(
    State(state): State<AppState>,
    Json(payload): Json<system::dictionary::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let id = payload.id.ok_or_spec(errors::ID_REQUIRED)?;
    system::dictionary::delete_detail(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_sys_dictionary_detail_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::dictionary::delete_detail(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}
