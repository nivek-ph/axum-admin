use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use file_storage::files::{FileEditPayload, FileListQuery, FileUpload, ImportUrlPayload};
use serde::Deserialize;
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use super::dto::FileResponse;
use crate::{ApiResponse, AppResult, mappings::MULTIPLE_FILES_NOT_SUPPORTED, state::AppState};

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct UploadMetadataQuery {
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub category: String,
}

#[derive(Debug, ToSchema)]
pub struct UploadFileRequest {
    #[schema(value_type = String, format = Binary)]
    #[schema(example = "example.png")]
    #[allow(dead_code)]
    pub file: Vec<u8>,
}

#[utoipa::path(
    get,
    path = "/files",
    tag = "file",
    security(("bearer_auth" = [])),
    params(FileListQuery),
    responses((status = 200, description = "File list", body = ApiResponse<Value>))
)]
pub async fn get_file_list_by_query(
    State(state): State<AppState>,
    Query(payload): Query<FileListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (list, total, page, page_size) = state.files.list(payload).await?;
    let list = list.into_iter().map(FileResponse::from).collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size
    }))))
}

#[utoipa::path(
    delete,
    path = "/files/{id}",
    tag = "file",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "File ID")),
    responses((status = 200, description = "File deleted", body = ApiResponse<Value>))
)]
pub async fn delete_file_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.files.delete(id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

#[utoipa::path(
    patch,
    path = "/files/{id}/name",
    tag = "file",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "File ID")),
    request_body = FileEditPayload,
    responses((status = 200, description = "File renamed", body = ApiResponse<Value>))
)]
pub async fn edit_file_name_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<FileEditPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    state.files.edit_name(payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    post,
    path = "/files/import-url",
    tag = "file",
    security(("bearer_auth" = [])),
    request_body = ImportUrlPayload,
    responses((status = 200, description = "URL imported", body = ApiResponse<Value>))
)]
pub async fn import_url(
    State(state): State<AppState>,
    Json(payload): Json<ImportUrlPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.files.import_url(payload).await?;
    Ok(Json(ApiResponse::ok_message("imported")))
}

#[utoipa::path(
    post,
    path = "/files/upload",
    tag = "file",
    security(("bearer_auth" = [])),
    params(UploadMetadataQuery),
    request_body(content = inline(UploadFileRequest), content_type = "multipart/form-data"),
    responses((status = 200, description = "File uploaded", body = ApiResponse<Value>))
)]
pub async fn upload_file(
    State(state): State<AppState>,
    Query(query): Query<UploadMetadataQuery>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<Value>>> {
    let mut pending_upload: Option<FileUpload> = None;

    loop {
        let Some(mut field) = (match multipart.next_field().await {
            Ok(field) => field,
            Err(error) => {
                if let Some(upload) = pending_upload.take() {
                    upload.abort().await?;
                }
                return Err(error.into());
            }
        }) else {
            break;
        };
        let file_name = field.file_name().map(|v| v.to_string());

        if let Some(file_name) = file_name {
            if let Some(upload) = pending_upload.take() {
                upload.abort().await?;
                return Err(MULTIPLE_FILES_NOT_SUPPORTED.into());
            }
            let mut upload = state
                .files
                .begin_upload(&file_name, &query.tag, &query.category)
                .await?;
            loop {
                let chunk = match field.chunk().await {
                    Ok(chunk) => chunk,
                    Err(error) => {
                        upload.abort().await?;
                        return Err(error.into());
                    }
                };
                let Some(chunk) = chunk else {
                    break;
                };
                if let Err(error) = upload.write_chunk(&chunk).await {
                    upload.abort().await?;
                    return Err(error.into());
                }
            }
            pending_upload = Some(upload);
        }
    }

    let uploaded = match pending_upload {
        Some(upload) => Some(FileResponse::from(upload.finish().await?)),
        None => None,
    };
    let file_url = uploaded.as_ref().map(|file| file.url.clone());

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "file": uploaded,
        "url": file_url
    }))))
}
