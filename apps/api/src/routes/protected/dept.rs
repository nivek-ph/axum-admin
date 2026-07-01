use admin_httpz::{ApiResponse, AppResult};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde_json::Value;

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
    let tree = system::depts::tree(&state.pool).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": tree
    }))))
}

pub async fn find_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let item = system::depts::find(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "dept": item.map(|dept| serde_json::json!(dept)).unwrap_or_else(|| serde_json::json!({}))
    }))))
}

pub async fn create_dept(
    State(state): State<AppState>,
    Json(payload): Json<system::depts::CreateDeptPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::depts::create(&state.pool, payload).await?;
    Ok(Json(ApiResponse::ok_message("created")))
}

pub async fn update_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<system::depts::UpdateDeptPayload>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::depts::update(&state.pool, id, payload).await?;
    Ok(Json(ApiResponse::ok_message("updated")))
}

pub async fn delete_dept_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    system::depts::delete(&state.pool, id).await?;
    Ok(Json(ApiResponse::ok_message("deleted")))
}
