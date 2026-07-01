use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get},
};
use serde_json::Value;

use crate::state::AppState;

pub fn login_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_login_log_list).delete(delete_login_log_by_ids))
        .route(
            "/{id}",
            get(find_login_log_by_id).delete(delete_login_log_by_id),
        )
}

pub fn operation_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_operation_log_list).delete(delete_operation_log_by_ids),
        )
        .route("/{id}", delete(delete_operation_log_by_id))
}

pub async fn get_login_log_list(
    State(state): State<AppState>,
    Query(payload): Query<system::logs::LoginLogSearch>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);
    let page_size = payload.page_size.max(1);
    let (list, total) = system::logs::get_login_log_list(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}

pub async fn find_login_log(
    State(state): State<AppState>,
    Query(payload): Query<system::logs::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (list, _) = system::logs::get_login_log_list(
        &state.pool,
        system::logs::LoginLogSearch {
            page: 1,
            page_size: 9999,
            username: None,
            status: None,
        },
    )
    .await?;
    let item = list.into_iter().find(|log| log.id == payload.id);
    Ok(Json(ApiResponse::ok(match item {
        Some(log) => serde_json::json!(log),
        None => serde_json::json!({}),
    })))
}

pub async fn find_login_log_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (list, _) = system::logs::get_login_log_list(
        &state.pool,
        system::logs::LoginLogSearch {
            page: 1,
            page_size: 9999,
            username: None,
            status: None,
        },
    )
    .await?;
    let item = list.into_iter().find(|log| log.id == id);
    Ok(Json(ApiResponse::ok(match item {
        Some(log) => serde_json::json!(log),
        None => serde_json::json!({}),
    })))
}

pub async fn delete_login_log(
    State(state): State<AppState>,
    Json(payload): Json<system::logs::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::logs::delete_login_log(&state.pool, payload.id).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_login_log_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::logs::delete_login_log(&state.pool, id).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_login_log_by_ids(
    State(state): State<AppState>,
    Json(payload): Json<system::logs::IdsRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::logs::delete_login_logs(&state.pool, payload.ids).await?;

    Ok(Json(ApiResponse::ok_message("batch deleted")))
}

pub async fn get_operation_log_list(
    State(state): State<AppState>,
    Query(payload): Query<system::logs::OperationLogSearch>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let page = payload.page.max(1);
    let page_size = payload.page_size.max(1);
    let (list, total) = system::logs::get_operation_log_list(&state.pool, payload).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": list,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}

pub async fn delete_operation_log(
    State(state): State<AppState>,
    Json(payload): Json<system::logs::IdRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::logs::delete_operation_log(&state.pool, payload.id).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_operation_log_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::logs::delete_operation_log(&state.pool, id).await?;

    Ok(Json(ApiResponse::ok_message("deleted")))
}

pub async fn delete_operation_log_by_ids(
    State(state): State<AppState>,
    Json(payload): Json<system::logs::IdsRequest>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::logs::delete_operation_logs(&state.pool, payload.ids).await?;

    Ok(Json(ApiResponse::ok_message("batch deleted")))
}
