use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use serde_json::Value;

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
pub struct UploadClassQuery {
    #[serde(rename = "classId")]
    pub class_id: Option<i64>,
}

pub async fn get_file_list(
    State(state): State<AppState>,
    Json(payload): Json<file_storage::files::FileListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (list, total, page, page_size) = file_storage::files::list(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size
    }))))
}

pub async fn get_file_list_by_query(
    State(state): State<AppState>,
    Query(payload): Query<file_storage::files::FileListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (list, total, page, page_size) = file_storage::files::list(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size
    }))))
}

pub async fn delete_file(
    State(state): State<AppState>,
    Json(payload): Json<file_storage::files::FileDeletePayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    file_storage::files::delete_file(&state.pool, payload.id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_file_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    file_storage::files::delete_file(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn edit_file_name(
    State(state): State<AppState>,
    Json(payload): Json<file_storage::files::FileEditPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    file_storage::files::edit_name(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn edit_file_name_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<file_storage::files::FileEditPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    file_storage::files::edit_name(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn import_url(
    State(state): State<AppState>,
    Json(payload): Json<file_storage::files::ImportUrlPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    file_storage::files::import_url(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("imported")))
}

pub async fn upload_file(
    State(state): State<AppState>,
    Query(query): Query<UploadClassQuery>,
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
        uploaded = Some(
            file_storage::files::store_uploaded_bytes(
                &state.pool,
                "./uploads",
                &file_name,
                query.class_id.unwrap_or(0),
                bytes.as_ref(),
            )
            .await?,
        );
    }

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "file": uploaded,
        "url": uploaded.as_ref().map(|file| file.url.clone()).unwrap_or_default()
    }))))
}
