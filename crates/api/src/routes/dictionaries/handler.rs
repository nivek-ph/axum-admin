use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde_json::Value;

use crate::state::AppState;

use super::dto::{
    DictionaryDetailPayload, DictionaryDetailResponse, DictionaryListQuery, DictionaryPayload,
    DictionaryResponse, DictionaryWithDetailsResponse, ImportDictionaryPayload,
};
use super::error::map_error;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_sys_dictionary_list).post(create_sys_dictionary),
        )
        .route("/import", post(import_sys_dictionary))
        .route(
            "/by-type/{dictionary_type}/tree",
            get(get_dictionary_tree_by_type),
        )
        .route(
            "/{id}",
            get(find_sys_dictionary_by_id)
                .put(update_sys_dictionary_by_id)
                .delete(delete_sys_dictionary_by_id),
        )
        .route("/{id}/export", get(export_sys_dictionary_by_id))
        .route(
            "/{id}/tree",
            get(get_dictionary_tree).post(create_dictionary_tree_node),
        )
        .route(
            "/{id}/tree/{node_id}",
            get(find_dictionary_tree_node)
                .put(update_dictionary_tree_node)
                .delete(delete_dictionary_tree_node),
        )
        .route(
            "/{id}/tree/{node_id}/children",
            get(get_dictionary_tree_node_children),
        )
        .route(
            "/{id}/tree/{node_id}/path",
            get(get_dictionary_tree_node_path),
        )
}

pub async fn create_sys_dictionary(
    State(state): State<AppState>,
    Json(payload): Json<DictionaryPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .create(payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn update_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<DictionaryPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    state
        .dictionaries
        .update(payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn find_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = state
        .dictionaries
        .find(Some(id), None)
        .await
        .map_err(map_error)?
        .map(DictionaryWithDetailsResponse::from);
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "resysDictionary": item.map(|value| serde_json::json!(value)).unwrap_or_else(|| serde_json::json!({}))
    }))))
}

pub async fn get_sys_dictionary_list(
    State(state): State<AppState>,
    Query(payload): Query<DictionaryListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = state
        .dictionaries
        .list(payload.into())
        .await
        .map_err(map_error)?
        .into_iter()
        .map(DictionaryResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!(list))))
}

pub async fn delete_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.dictionaries.delete(id).await.map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn export_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let data = state.dictionaries.export(id).await.map_err(map_error)?;
    Ok(Json(ApiResponse::ok(
        data.unwrap_or_else(|| serde_json::json!({})),
    )))
}

pub async fn import_sys_dictionary(
    State(state): State<AppState>,
    Json(payload): Json<ImportDictionaryPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .import(payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("imported")))
}

pub async fn get_dictionary_tree(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = state
        .dictionaries
        .tree_by_dictionary(id)
        .await
        .map_err(map_error)?
        .into_iter()
        .map(DictionaryDetailResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list
    }))))
}

pub async fn create_dictionary_tree_node(
    State(state): State<AppState>,
    Path(dictionary_id): Path<i64>,
    Json(payload): Json<DictionaryDetailPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .create_detail(dictionary_id, payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn find_dictionary_tree_node(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = DictionaryDetailResponse::from(
        state
            .dictionaries
            .find_detail(dictionary_id, node_id)
            .await
            .map_err(map_error)?,
    );
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "reSysDictionaryDetail": item
    }))))
}

pub async fn update_dictionary_tree_node(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
    Json(payload): Json<DictionaryDetailPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .update_detail(dictionary_id, node_id, payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn delete_dictionary_tree_node(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .delete_detail(dictionary_id, node_id)
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn get_dictionary_tree_by_type(
    State(state): State<AppState>,
    Path(dictionary_type): Path<String>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = state
        .dictionaries
        .tree_by_type(&dictionary_type)
        .await
        .map_err(map_error)?
        .into_iter()
        .map(DictionaryDetailResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

pub async fn get_dictionary_tree_node_children(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .find_detail(dictionary_id, node_id)
        .await
        .map_err(map_error)?;
    let list = state
        .dictionaries
        .details_by_parent(dictionary_id, node_id)
        .await
        .map_err(map_error)?
        .into_iter()
        .map(DictionaryDetailResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}

pub async fn get_dictionary_tree_node_path(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let list = state
        .dictionaries
        .detail_path(dictionary_id, node_id)
        .await
        .map_err(map_error)?
        .into_iter()
        .map(DictionaryDetailResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({ "list": list }))))
}
