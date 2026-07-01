use admin_httpz::{ApiResponse, AppResult, OptionAppExt};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde_json::Value;

use crate::errors::request as errors;
use crate::state::AppState;

use super::dictionary_detail;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_sys_dictionary_list).post(create_sys_dictionary),
        )
        .route("/import", post(import_sys_dictionary))
        .route(
            "/{id}",
            get(find_sys_dictionary_by_id)
                .put(update_sys_dictionary_by_id)
                .delete(delete_sys_dictionary_by_id),
        )
        .route("/{id}/export", get(export_sys_dictionary_by_id))
        .route(
            "/{id}/details/tree",
            get(dictionary_detail::get_dictionary_tree_list_by_id),
        )
}

pub async fn create_sys_dictionary(
    State(state): State<AppState>,
    Json(payload): Json<system::dictionary::SysDictionary>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::dictionary::create(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn update_sys_dictionary(
    State(state): State<AppState>,
    Json(payload): Json<system::dictionary::SysDictionary>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::dictionary::update(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn update_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<system::dictionary::SysDictionary>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    system::dictionary::update(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn find_sys_dictionary(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item =
        system::dictionary::find_by_query(&state.pool, payload.id, payload.dict_type).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "resysDictionary": item.unwrap_or_else(|| serde_json::json!({}))
    }))))
}

pub async fn find_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = system::dictionary::find_by_query(&state.pool, Some(id), None).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "resysDictionary": item.unwrap_or_else(|| serde_json::json!({}))
    }))))
}

pub async fn get_sys_dictionary_list(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::DictionaryListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = system::dictionary::list(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!(list))))
}

pub async fn delete_sys_dictionary(
    State(state): State<AppState>,
    Json(payload): Json<system::dictionary::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let id = payload.id.ok_or_spec(errors::ID_REQUIRED)?;
    system::dictionary::delete(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::dictionary::delete(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn export_sys_dictionary(
    State(state): State<AppState>,
    Query(payload): Query<system::dictionary::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let id = payload.id.ok_or_spec(errors::ID_REQUIRED)?;
    let data = system::dictionary::export_dictionary(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(
        data.unwrap_or_else(|| serde_json::json!({})),
    )))
}

pub async fn export_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = system::dictionary::export_dictionary(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(
        data.unwrap_or_else(|| serde_json::json!({})),
    )))
}

pub async fn import_sys_dictionary(
    State(state): State<AppState>,
    Json(payload): Json<system::dictionary::ImportDictionaryPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::dictionary::import_dictionary(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("imported")))
}
