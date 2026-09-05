use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
};

use super::dto::{
    FileListData, FileListRequest, FileResponse, ImportFileUrlRequest, RenameFileRequest,
    StartUploadRequest, UploadFileData, UploadSessionData,
};
use crate::{ApiResponse, AppResult, EmptyData, state::AppState};

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
    let (list, total, page, page_size) = state.files.list(payload.into()).await?;
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
    path = "/files/uploads",
    tag = "file",
    security(("bearer_auth" = [])),
    request_body = StartUploadRequest,
    responses((status = 200, description = "Upload started", body = ApiResponse<UploadSessionData>))
)]
pub async fn start_upload(
    State(state): State<AppState>,
    Json(payload): Json<StartUploadRequest>,
) -> AppResult<Json<ApiResponse<UploadSessionData>>> {
    let session = state.files.start_upload(payload.into()).await?;
    Ok(Json(ApiResponse::ok(UploadSessionData::from_session(
        session,
    ))))
}

#[utoipa::path(
    get,
    path = "/files/uploads/{id}",
    tag = "file",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Upload session ID")),
    responses((status = 200, description = "Upload status", body = ApiResponse<UploadSessionData>))
)]
pub async fn upload_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ApiResponse<UploadSessionData>>> {
    let session = state.files.upload_status(&id).await?;
    Ok(Json(ApiResponse::ok(UploadSessionData::from_session(
        session,
    ))))
}

#[utoipa::path(
    patch,
    path = "/files/uploads/{id}",
    tag = "file",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Upload session ID")),
    request_body(content = Vec<u8>, content_type = "application/octet-stream"),
    responses((status = 200, description = "Chunk accepted", body = ApiResponse<UploadSessionData>))
)]
pub async fn upload_chunk(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<ApiResponse<UploadSessionData>>> {
    let offset = headers
        .get("upload-offset")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .ok_or(file_storage::files::FileError::OffsetMismatch)?;
    let session = state.files.write_upload_chunk(&id, offset, &body).await?;
    Ok(Json(ApiResponse::ok(UploadSessionData::from_session(
        session,
    ))))
}

#[utoipa::path(
    post,
    path = "/files/uploads/{id}/complete",
    tag = "file",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Upload session ID")),
    responses((status = 200, description = "Upload completed", body = ApiResponse<UploadFileData>))
)]
pub async fn complete_upload(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ApiResponse<UploadFileData>>> {
    let file = FileResponse::from_stored(
        &state.public_base_url,
        state.files.complete_upload(&id).await?,
    );
    Ok(Json(ApiResponse::ok(UploadFileData {
        url: Some(file.url.clone()),
        file: Some(file),
    })))
}
