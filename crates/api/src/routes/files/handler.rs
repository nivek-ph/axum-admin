use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use file_storage::files::FileUpload;

use super::dto::{
    FileListData, FileListRequest, FileResponse, ImportFileUrlRequest, RenameFileRequest,
    UploadFileData, UploadFileRequest, UploadMetadataRequest,
};
use crate::{
    ApiResponse, AppResult, EmptyData, mappings::MULTIPLE_FILES_NOT_SUPPORTED, state::AppState,
};

#[utoipa::path(
    get,
    path = "/files",
    tag = "file",
    security(("bearer_auth" = [])),
    params(FileListRequest),
    responses((status = 200, description = "File list", body = ApiResponse<FileListData>))
)]
pub async fn get_file_list_by_query(
    State(state): State<AppState>,
    Query(payload): Query<FileListRequest>,
) -> AppResult<Json<ApiResponse<FileListData>>> {
    let (list, total, page, page_size) = state.files.list(payload).await?;
    let list = list
        .into_iter()
        .map(|file| FileResponse::from_stored(&state.public_base_url, file))
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(FileListData {
        list,
        total,
        page,
        page_size,
    })))
}

#[utoipa::path(
    delete,
    path = "/files/{id}",
    tag = "file",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "File ID")),
    responses((status = 200, description = "File deleted", body = ApiResponse<EmptyData>))
)]
pub async fn delete_file_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.files.delete(id).await?;
    Ok(Json(ApiResponse::new("OK", "deleted", None)))
}

#[utoipa::path(
    patch,
    path = "/files/{id}/name",
    tag = "file",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "File ID")),
    request_body = RenameFileRequest,
    responses((status = 200, description = "File renamed", body = ApiResponse<EmptyData>))
)]
pub async fn edit_file_name_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RenameFileRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.files.edit_name(payload.into_input(id)).await?;
    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    post,
    path = "/files/import-url",
    tag = "file",
    security(("bearer_auth" = [])),
    request_body = ImportFileUrlRequest,
    responses((status = 200, description = "URL imported", body = ApiResponse<EmptyData>))
)]
pub async fn import_url(
    State(state): State<AppState>,
    Json(payload): Json<ImportFileUrlRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.files.import_url(payload.into()).await?;
    Ok(Json(ApiResponse::new("OK", "imported", None)))
}

#[utoipa::path(
    post,
    path = "/files/upload",
    tag = "file",
    security(("bearer_auth" = [])),
    params(UploadMetadataRequest),
    request_body(content = inline(UploadFileRequest), content_type = "multipart/form-data"),
    responses((status = 200, description = "File uploaded", body = ApiResponse<UploadFileData>))
)]
pub async fn upload_file(
    State(state): State<AppState>,
    Query(query): Query<UploadMetadataRequest>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<UploadFileData>>> {
    let mut pending_upload: Option<FileUpload> = None;

    loop {
        let Some(mut field) = (match multipart.next_field().await {
            Ok(field) => field,
            Err(error) => {
                if let Some(upload) = pending_upload.take() {
                    abort_upload(upload, "multipart read failed").await;
                }
                return Err(error.into());
            }
        }) else {
            break;
        };
        let file_name = field.file_name().map(|v| v.to_string());

        if let Some(file_name) = file_name {
            if let Some(upload) = pending_upload.take() {
                abort_upload(upload, "multiple files received").await;
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
                        abort_upload(upload, "file chunk read failed").await;
                        return Err(error.into());
                    }
                };
                let Some(chunk) = chunk else {
                    break;
                };
                if let Err(error) = upload.write_chunk(&chunk).await {
                    abort_upload(upload, "file chunk write failed").await;
                    return Err(error.into());
                }
            }
            pending_upload = Some(upload);
        }
    }

    let uploaded = match pending_upload {
        Some(upload) => Some(FileResponse::from_stored(
            &state.public_base_url,
            upload.finish().await?,
        )),
        None => None,
    };
    let file_url = uploaded.as_ref().map(|file| file.url.clone());

    Ok(Json(ApiResponse::ok(UploadFileData {
        file: uploaded,
        url: file_url,
    })))
}

async fn abort_upload(upload: FileUpload, reason: &'static str) {
    if let Err(error) = upload.abort().await {
        tracing::error!(%error, reason, "failed to clean up upload");
    }
}
