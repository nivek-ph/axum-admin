use axum::{
    Json,
    extract::{Path, Query, State},
};

use super::dto::{
    StorageListData, StorageListRequest, StorageRequest, StorageResponse, StorageStatusRequest,
};
use crate::{ApiResponse, AppResult, EmptyData, state::AppState};

#[utoipa::path(
    get,
    path = "/storages",
    tag = "storages",
    security(("bearer_auth" = [])),
    params(StorageListRequest),
    responses((status = 200, description = "Storages", body = ApiResponse<StorageListData>))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<StorageListRequest>,
) -> AppResult<Json<ApiResponse<StorageListData>>> {
    let list = state
        .storages
        .list(query.into())
        .await?
        .into_iter()
        .map(StorageResponse::from)
        .collect();
    Ok(Json(ApiResponse::ok(StorageListData { list })))
}

#[utoipa::path(
    get,
    path = "/storages/{id}",
    tag = "storages",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Storage ID")),
    responses((status = 200, description = "Storage", body = ApiResponse<StorageResponse>))
)]
pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<StorageResponse>>> {
    let item = state.storages.find(id).await?;
    Ok(Json(ApiResponse::ok(item.into())))
}

#[utoipa::path(
    post,
    path = "/storages",
    tag = "storages",
    security(("bearer_auth" = [])),
    request_body = StorageRequest,
    responses((status = 200, description = "Storage created", body = ApiResponse<StorageResponse>))
)]
pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<StorageRequest>,
) -> AppResult<Json<ApiResponse<StorageResponse>>> {
    let payload: file_storage::storages::StorageInput = payload.into();
    let item = state.storages.create(payload).await?;
    Ok(Json(ApiResponse::new("OK", "created", Some(item.into()))))
}

#[utoipa::path(
    put,
    path = "/storages/{id}",
    tag = "storages",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Storage ID")),
    request_body = StorageRequest,
    responses((status = 200, description = "Storage updated", body = ApiResponse<StorageResponse>))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<StorageRequest>,
) -> AppResult<Json<ApiResponse<StorageResponse>>> {
    let payload: file_storage::storages::StorageInput = payload.into();
    let item = state.storages.update(id, payload).await?;
    Ok(Json(ApiResponse::new("OK", "updated", Some(item.into()))))
}

#[utoipa::path(
    patch,
    path = "/storages/{id}/status",
    tag = "storages",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Storage ID")),
    request_body = StorageStatusRequest,
    responses((status = 200, description = "Storage status updated", body = ApiResponse<EmptyData>))
)]
pub async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<StorageStatusRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.storages.set_enabled(id, payload.enabled).await?;
    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/storages/{id}/default",
    tag = "storages",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Storage ID")),
    responses((status = 200, description = "Default storage updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_default(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.storages.set_default(id).await?;
    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    delete,
    path = "/storages/{id}",
    tag = "storages",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Storage ID")),
    responses((status = 200, description = "Storage deleted", body = ApiResponse<EmptyData>))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.storages.delete(id).await?;
    Ok(Json(ApiResponse::new("OK", "deleted", None)))
}
