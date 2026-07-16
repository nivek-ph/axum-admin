use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::Value;

use super::dto::{
    EmptyParameter, IdsRequest, ParamResponse, ParameterByKeyData, ParameterByKeyRequest,
    ParameterDetailData, ParameterListData, ParameterListRequest, ParameterRequest,
};
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    post,
    path = "/params",
    tag = "parameter",
    security(("bearer_auth" = [])),
    request_body = ParameterRequest,
    responses((status = 200, description = "Parameter created", body = ApiResponse<Value>))
)]
pub async fn create_sys_params(
    State(state): State<AppState>,
    Json(payload): Json<ParameterRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.parameters.create(payload).await?;

    Ok(Json(ApiResponse::ok_message("created")))
}

#[utoipa::path(
    put,
    path = "/params/{id}",
    tag = "parameter",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Parameter ID")),
    request_body = ParameterRequest,
    responses((status = 200, description = "Parameter updated", body = ApiResponse<Value>))
)]
pub async fn update_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<ParameterRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.parameters.update(id, payload).await?;

    Ok(Json(ApiResponse::ok_message("updated")))
}

#[utoipa::path(
    get,
    path = "/params/{id}",
    tag = "parameter",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Parameter ID")),
    responses((status = 200, description = "Parameter detail", body = ApiResponse<ParameterDetailData>))
)]
pub async fn find_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ParameterDetailData>>> {
    let item = state.parameters.find(id).await?.map(ParamResponse::from);
    let data = match item {
        Some(item) => ParameterDetailData::Parameter(item),
        None => ParameterDetailData::Empty(EmptyParameter {}),
    };
    Ok(Json(ApiResponse::ok(data)))
}

#[utoipa::path(
    get,
    path = "/params",
    tag = "parameter",
    security(("bearer_auth" = [])),
    params(ParameterListRequest),
    responses((status = 200, description = "Parameter list", body = ApiResponse<ParameterListData>))
)]
pub async fn get_sys_params_list(
    State(state): State<AppState>,
    Query(payload): Query<ParameterListRequest>,
) -> AppResult<Json<ApiResponse<ParameterListData>>> {
    let (list, total, page, page_size) = state.parameters.list(payload).await?;

    let list = list
        .into_iter()
        .map(ParamResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(ParameterListData {
        list,
        total,
        page,
        page_size,
    })))
}

#[utoipa::path(
    delete,
    path = "/params/{id}",
    tag = "parameter",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Parameter ID")),
    responses((status = 200, description = "Parameter deleted", body = ApiResponse<Value>))
)]
pub async fn delete_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.parameters.delete(id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

#[utoipa::path(
    delete,
    path = "/params/batch",
    tag = "parameter",
    security(("bearer_auth" = [])),
    params(IdsRequest),
    responses((status = 200, description = "Parameters deleted", body = ApiResponse<Value>))
)]
pub async fn delete_sys_params_by_ids(
    State(state): State<AppState>,
    Query(payload): Query<IdsRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.parameters.delete_many(payload.ids).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

#[utoipa::path(
    get,
    path = "/params/by-key",
    tag = "parameter",
    security(("bearer_auth" = [])),
    params(("key" = String, Query, description = "Parameter key")),
    responses((status = 200, description = "Parameter value", body = ApiResponse<ParameterByKeyData>))
)]
pub async fn get_sys_param(
    State(state): State<AppState>,
    Query(payload): Query<ParameterByKeyRequest>,
) -> AppResult<Json<ApiResponse<ParameterByKeyData>>> {
    let key = payload.key.unwrap_or_default();
    let item = state
        .parameters
        .by_key(&key)
        .await?
        .map(ParamResponse::from);
    Ok(Json(ApiResponse::ok(ParameterByKeyData {
        parameter: item,
    })))
}
