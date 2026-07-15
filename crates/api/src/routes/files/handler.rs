use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use serde_json::Value;

use super::dto::{FileEditPayload, FileListQuery, FileResponse, ImportUrlPayload};
use super::error::map_error;
use crate::errors::request as errors;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_file_list_by_query))
        .route("/import-url", post(import_url))
        .route(
            "/upload",
            post(upload_file).layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        )
        .route("/{id}", delete(delete_file_by_id))
        .route("/{id}/name", patch(edit_file_name_by_id))
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadMetadataQuery {
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub category: String,
}

pub async fn get_file_list_by_query(
    State(state): State<AppState>,
    Query(payload): Query<FileListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (list, total, page, page_size) =
        state.files.list(payload.into()).await.map_err(map_error)?;
    let list = list.into_iter().map(FileResponse::from).collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size
    }))))
}

pub async fn delete_file_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.files.delete(id).await.map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn edit_file_name_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<FileEditPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    state
        .files
        .edit_name(payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn import_url(
    State(state): State<AppState>,
    Json(payload): Json<ImportUrlPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .files
        .import_url(payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("imported")))
}

pub async fn upload_file(
    State(state): State<AppState>,
    Query(query): Query<UploadMetadataQuery>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<Value>>> {
    let mut uploaded = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(errors::multipart_field_error)?
    {
        let file_name = field
            .file_name()
            .map(ToString::to_string)
            .unwrap_or_else(|| "upload.bin".to_string());
        let bytes = field.bytes().await.map_err(errors::multipart_field_error)?;
        uploaded = Some(FileResponse::from(
            state
                .files
                .upload(&file_name, &query.tag, &query.category, bytes.as_ref())
                .await
                .map_err(map_error)?,
        ));
    }

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "file": uploaded,
        "url": uploaded.as_ref().map(|file| file.url.clone()).unwrap_or_default()
    }))))
}
