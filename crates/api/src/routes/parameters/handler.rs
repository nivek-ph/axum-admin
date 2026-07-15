use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get},
};
use serde_json::Value;

use super::dto::{IdsRequest, ParamListQuery, ParamPayload, ParamResponse};
use super::error::map_error;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_sys_params_list).post(create_sys_params))
        .route("/by-key", get(get_sys_param))
        .route("/batch", delete(delete_sys_params_by_ids))
        .route(
            "/{id}",
            get(find_sys_params_by_id)
                .put(update_sys_params_by_id)
                .delete(delete_sys_params_by_id),
        )
}

pub async fn create_sys_params(
    State(state): State<AppState>,
    Json(payload): Json<ParamPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .parameters
        .create(payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn update_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<ParamPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    state
        .parameters
        .update(payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn find_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = state
        .parameters
        .find(id)
        .await
        .map_err(map_error)?
        .map(ParamResponse::from);
    Ok(Json(ApiResponse::ok(
        item.map(|value| serde_json::json!(value))
            .unwrap_or_else(|| serde_json::json!({})),
    )))
}

pub async fn get_sys_params_list(
    State(state): State<AppState>,
    Query(payload): Query<ParamListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (list, total, page, page_size) = state
        .parameters
        .list(payload.into())
        .await
        .map_err(map_error)?;
    let list = list
        .into_iter()
        .map(ParamResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size
    }))))
}

pub async fn delete_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state.parameters.delete(id).await.map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_sys_params_by_ids(
    State(state): State<AppState>,
    Query(payload): Query<IdsRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    state
        .parameters
        .delete_many(payload.ids)
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn get_sys_param(
    State(state): State<AppState>,
    Query(payload): Query<ParamListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let key = payload.key.unwrap_or_default();
    let item = state
        .parameters
        .by_key(&key)
        .await
        .map_err(map_error)?
        .map(ParamResponse::from);
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "sysParam": item
    }))))
}
