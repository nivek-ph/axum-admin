use axum::{
    Json,
    extract::{Path, Query, State},
};

use super::dto::{
    AuditAnalysisRequest, AuditAnalysisResponse, AuditEventListData, AuditEventListRequest,
    AuditEventResponse,
};
use crate::{ApiResponse, AppResult, state::AppState};

#[utoipa::path(
    get,
    path = "/audit/events",
    tag = "audit",
    security(("bearer_auth" = [])),
    params(AuditEventListRequest),
    responses((status = 200, description = "Audit event list", body = ApiResponse<AuditEventListData>))
)]
pub async fn get_audit_events(
    State(state): State<AppState>,
    Query(query): Query<AuditEventListRequest>,
) -> AppResult<Json<ApiResponse<AuditEventListData>>> {
    let (events, total, page, page_size) = state.audits.list(query).await?;
    let events = events
        .into_iter()
        .map(AuditEventResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(AuditEventListData {
        list: events,
        total,
        page,
        page_size,
    })))
}

#[utoipa::path(
    get,
    path = "/audit/events/{id}",
    tag = "audit",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "Audit event ID")),
    responses((status = 200, description = "Audit event detail", body = ApiResponse<Option<AuditEventResponse>>))
)]
pub async fn find_audit_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Option<AuditEventResponse>>>> {
    let event = state.audits.find(id).await?.map(AuditEventResponse::from);
    Ok(Json(ApiResponse::ok(event)))
}

#[utoipa::path(
    post,
    path = "/audit/events/analyze",
    tag = "audit",
    security(("bearer_auth" = [])),
    request_body = AuditAnalysisRequest,
    responses((status = 200, description = "AI-assisted audit analysis", body = ApiResponse<AuditAnalysisResponse>))
)]
pub async fn analyze_audit_events(
    State(state): State<AppState>,
    Json(request): Json<AuditAnalysisRequest>,
) -> AppResult<Json<ApiResponse<AuditAnalysisResponse>>> {
    let (events, _, _, _) = state.audits.list(request.into()).await?;
    let analysis = state.audit_analyzer.analyze(&events).await?;

    Ok(Json(ApiResponse::ok(analysis.into())))
}
