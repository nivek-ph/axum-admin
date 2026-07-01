use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get},
};
use serde_json::Value;
use system::errors;

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
    Json(payload): Json<system::params::SysParam>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::params::create(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn update_sys_params(
    State(state): State<AppState>,
    Json(payload): Json<system::params::SysParam>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::params::update(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn update_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut payload): Json<system::params::SysParam>,
) -> AppResult<Json<ApiResponse<Value>>> {
    payload.id = id;
    system::params::update(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn find_sys_params(
    State(state): State<AppState>,
    Query(payload): Query<system::params::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = system::params::find(&state.pool, payload.id).await?;
    Ok(Json(ApiResponse::ok(
        item.map(serde_json::to_value)
            .transpose()
            .map_err(|e| {
                errors::params::PARAM_JSON_ENCODE_FAILED
                    .into_error()
                    .with_source(e)
            })?
            .unwrap_or_else(|| serde_json::json!({})),
    )))
}

pub async fn find_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = system::params::find(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(
        item.map(serde_json::to_value)
            .transpose()
            .map_err(|e| {
                errors::params::PARAM_JSON_ENCODE_FAILED
                    .into_error()
                    .with_source(e)
            })?
            .unwrap_or_else(|| serde_json::json!({})),
    )))
}

pub async fn get_sys_params_list(
    State(state): State<AppState>,
    Query(payload): Query<system::params::ParamListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (list, total, page, page_size) = system::params::list(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size
    }))))
}

pub async fn delete_sys_params(
    State(state): State<AppState>,
    Query(payload): Query<system::params::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::params::delete(&state.pool, payload.id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_sys_params_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::params::delete(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_sys_params_by_ids(
    State(state): State<AppState>,
    Query(payload): Query<system::params::IdsRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::params::delete_many(&state.pool, payload.ids).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn get_sys_param(
    State(state): State<AppState>,
    Query(payload): Query<system::params::ParamListQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let key = payload.key.unwrap_or_default();
    let item = system::params::get_by_key(&state.pool, &key).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "sysParam": item
    }))))
}
