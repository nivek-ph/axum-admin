use audit::AuditQuery;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::Value;

use super::dto::AuditEventResponse;
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    get,
    path = "/audit/events",
    tag = "audit",
    security(("bearer_auth" = [])),
    params(AuditQuery),
    responses((status = 200, description = "Audit event list", body = ApiResponse<Value>))
)]
pub async fn get_audit_events(
    State(state): State<AppState>,
    Query(query): Query<AuditQuery>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (events, total, page, page_size) = state.audits.list(query).await?;
    let events = events
        .into_iter()
        .map(AuditEventResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "list": events,
        "total": total,
        "page": page,
        "pageSize": page_size,
    }))))
}

#[utoipa::path(
    get,
    path = "/audit/events/{id}",
    tag = "audit",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Audit event ID")),
    responses((status = 200, description = "Audit event detail", body = ApiResponse<Value>))
)]
pub async fn find_audit_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let event = state.audits.find(id).await?.map(AuditEventResponse::from);
    Ok(Json(ApiResponse::ok(serde_json::json!(event))))
}
