use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::Value;

use super::dto::{
    DictionaryDetailData, DictionaryDetailRequest, DictionaryDetailResponse, DictionaryDetailValue,
    DictionaryExportValue, DictionaryImportData, DictionaryListRequest, DictionaryNodeData,
    DictionaryRequest, DictionaryResponse, DictionaryTreeData, DictionaryWithDetailsResponse,
    EmptyDictionary, ImportDictionaryRequest,
};
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    post,
    path = "/dictionaries",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    request_body = DictionaryRequest,
    responses((status = 200, description = "Dictionary created", body = ApiResponse<Value>))
)]
pub async fn create_sys_dictionary(
    State(state): State<AppState>,
    Json(payload): Json<DictionaryRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.dictionaries.create(payload.into()).await?;

    Ok(Json(ApiResponse::ok_message("created")))
}

#[utoipa::path(
    put,
    path = "/dictionaries/{id}",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Dictionary ID")),
    request_body = DictionaryRequest,
    responses((status = 200, description = "Dictionary updated", body = ApiResponse<Value>))
)]
pub async fn update_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<DictionaryRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.dictionaries.update(id, payload.into()).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{id}",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Dictionary ID")),
    responses((status = 200, description = "Dictionary detail", body = ApiResponse<DictionaryDetailData>))
)]
pub async fn find_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<DictionaryDetailData>>> {
    let item = state
        .dictionaries
        .find(Some(id), None)
        .await?
        .map(DictionaryWithDetailsResponse::from);
    let dictionary = match item {
        Some(item) => DictionaryDetailValue::Dictionary(item),
        None => DictionaryDetailValue::Empty(EmptyDictionary {}),
    };
    Ok(Json(ApiResponse::ok(DictionaryDetailData { dictionary })))
}

#[utoipa::path(
    get,
    path = "/dictionaries",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(DictionaryListRequest),
    responses((status = 200, description = "Dictionary list", body = ApiResponse<Vec<DictionaryResponse>>))
)]
pub async fn get_sys_dictionary_list(
    State(state): State<AppState>,
    Query(payload): Query<DictionaryListRequest>,
) -> AppResult<Json<ApiResponse<Vec<DictionaryResponse>>>> {
    let list = state
        .dictionaries
        .list(payload)
        .await?
        .into_iter()
        .map(DictionaryResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(list)))
}

#[utoipa::path(
    delete,
    path = "/dictionaries/{id}",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Dictionary ID")),
    responses((status = 200, description = "Dictionary deleted", body = ApiResponse<Value>))
)]
pub async fn delete_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.dictionaries.delete(id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{id}/export",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Dictionary ID")),
    responses((status = 200, description = "Dictionary export", body = ApiResponse<DictionaryExportValue>))
)]
pub async fn export_sys_dictionary_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<DictionaryExportValue>>> {
    let data = state.dictionaries.export(id).await?;
    let data = match data {
        Some(data) => DictionaryExportValue::Dictionary(data.into()),
        None => DictionaryExportValue::Empty(EmptyDictionary {}),
    };
    Ok(Json(ApiResponse::ok(data)))
}

#[utoipa::path(
    post,
    path = "/dictionaries/import",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    request_body = ImportDictionaryRequest,
    responses((status = 200, description = "Dictionary imported", body = ApiResponse<DictionaryImportData>))
)]
pub async fn import_sys_dictionary(
    State(state): State<AppState>,
    Json(payload): Json<ImportDictionaryRequest>,
) -> AppResult<Json<ApiResponse<DictionaryImportData>>> {
    let input = payload.into_input().map_err(anyhow::Error::from)?;
    state.dictionaries.import(input).await?;

    Ok(Json(ApiResponse::new("OK", "imported", None)))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{id}/tree",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Dictionary ID")),
    responses((status = 200, description = "Dictionary tree", body = ApiResponse<DictionaryTreeData>))
)]
pub async fn get_dictionary_tree(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<DictionaryTreeData>>> {
    let list = state
        .dictionaries
        .tree_by_dictionary(id)
        .await?
        .into_iter()
        .map(DictionaryDetailResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(DictionaryTreeData { list })))
}

#[utoipa::path(
    post,
    path = "/dictionaries/{id}/tree",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Dictionary ID")),
    request_body = DictionaryDetailRequest,
    responses((status = 200, description = "Dictionary node created", body = ApiResponse<Value>))
)]
pub async fn create_dictionary_tree_node(
    State(state): State<AppState>,
    Path(dictionary_id): Path<i64>,
    Json(payload): Json<DictionaryDetailRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .create_detail(dictionary_id, payload.into())
        .await?;
    Ok(Json(ApiResponse::ok_message("created")))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{id}/tree/{node_id}",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(
        ("id" = i64, Path, description = "Dictionary ID"),
        ("node_id" = i64, Path, description = "Node ID")
    ),
    responses((status = 200, description = "Dictionary node", body = ApiResponse<DictionaryNodeData>))
)]
pub async fn find_dictionary_tree_node(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<DictionaryNodeData>>> {
    let item = DictionaryDetailResponse::from(
        state
            .dictionaries
            .find_detail(dictionary_id, node_id)
            .await?,
    );
    Ok(Json(ApiResponse::ok(DictionaryNodeData { detail: item })))
}

#[utoipa::path(
    put,
    path = "/dictionaries/{id}/tree/{node_id}",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(
        ("id" = i64, Path, description = "Dictionary ID"),
        ("node_id" = i64, Path, description = "Node ID")
    ),
    request_body = DictionaryDetailRequest,
    responses((status = 200, description = "Dictionary node updated", body = ApiResponse<Value>))
)]
pub async fn update_dictionary_tree_node(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
    Json(payload): Json<DictionaryDetailRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .update_detail(dictionary_id, node_id, payload.into())
        .await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    delete,
    path = "/dictionaries/{id}/tree/{node_id}",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(
        ("id" = i64, Path, description = "Dictionary ID"),
        ("node_id" = i64, Path, description = "Node ID")
    ),
    responses((status = 200, description = "Dictionary node deleted", body = ApiResponse<Value>))
)]
pub async fn delete_dictionary_tree_node(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .dictionaries
        .delete_detail(dictionary_id, node_id)
        .await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

#[utoipa::path(
    get,
    path = "/dictionaries/by-type/{dictionary_type}/tree",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(("dictionary_type" = String, Path, description = "Dictionary type")),
    responses((status = 200, description = "Dictionary tree by type", body = ApiResponse<DictionaryTreeData>))
)]
pub async fn get_dictionary_tree_by_type(
    State(state): State<AppState>,
    Path(dictionary_type): Path<String>,
) -> AppResult<Json<ApiResponse<DictionaryTreeData>>> {
    let list = state
        .dictionaries
        .tree_by_type(&dictionary_type)
        .await?
        .into_iter()
        .map(DictionaryDetailResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(DictionaryTreeData { list })))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{id}/tree/{node_id}/children",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(
        ("id" = i64, Path, description = "Dictionary ID"),
        ("node_id" = i64, Path, description = "Node ID")
    ),
    responses((status = 200, description = "Dictionary node children", body = ApiResponse<DictionaryTreeData>))
)]
pub async fn get_dictionary_tree_node_children(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<DictionaryTreeData>>> {
    state
        .dictionaries
        .find_detail(dictionary_id, node_id)
        .await?;
    let list = state
        .dictionaries
        .details_by_parent(dictionary_id, node_id)
        .await?
        .into_iter()
        .map(DictionaryDetailResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(DictionaryTreeData { list })))
}

#[utoipa::path(
    get,
    path = "/dictionaries/{id}/tree/{node_id}/path",
    tag = "dictionary",
    security(("bearer_auth" = [])),
    params(
        ("id" = i64, Path, description = "Dictionary ID"),
        ("node_id" = i64, Path, description = "Node ID")
    ),
    responses((status = 200, description = "Dictionary node path", body = ApiResponse<DictionaryTreeData>))
)]
pub async fn get_dictionary_tree_node_path(
    State(state): State<AppState>,
    Path((dictionary_id, node_id)): Path<(i64, i64)>,
) -> AppResult<Json<ApiResponse<DictionaryTreeData>>> {
    let list = state
        .dictionaries
        .detail_path(dictionary_id, node_id)
        .await?
        .into_iter()
        .map(DictionaryDetailResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(DictionaryTreeData { list })))
}
