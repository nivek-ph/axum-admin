use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde_json::Value;

use super::dto::{DeptNodeResponse, DeptPayload, DeptResponse};
use super::error::map_error;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_dept_tree).post(create_dept))
        .route(
            "/{id}",
            get(find_dept_by_id)
                .put(update_dept_by_id)
                .delete(delete_dept_by_id),
        )
}

pub async fn get_dept_tree(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let tree = state
        .departments
        .tree()
        .await
        .map_err(map_error)?
        .into_iter()
        .map(DeptNodeResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": tree
    }))))
}

pub async fn find_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = state
        .departments
        .find(id)
        .await
        .map_err(map_error)?
        .map(DeptResponse::from);
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "dept": item.map(|dept| serde_json::json!(dept)).unwrap_or_else(|| serde_json::json!({}))
    }))))
}

pub async fn create_dept(
    State(state): State<AppState>,
    Json(payload): Json<DeptPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;
    state
        .departments
        .create(payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn update_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<DeptPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;
    state
        .departments
        .update(id, payload.into())
        .await
        .map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn delete_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    invalidate_access(&state).await?;
    state.departments.delete(id).await.map_err(map_error)?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}

async fn invalidate_access(state: &AppState) -> AppResult<()> {
    state.access.invalidate().await.map_err(|source| {
        crate::errors::INTERNAL_SERVER_ERROR
            .into_error()
            .with_source(source)
    })
}
