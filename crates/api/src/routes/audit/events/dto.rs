use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type AuditEventListRequest = audit::AuditQuery;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditAnalysisRequest {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub result: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
}

impl From<AuditAnalysisRequest> for audit::AuditQuery {
    fn from(value: AuditAnalysisRequest) -> Self {
        Self {
            page: 1,
            page_size: 50,
            actor: value.actor,
            action: value.action,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            result: value.result,
            started_at: value.started_at,
            ended_at: value.ended_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventResponse {
    pub id: i64,
    pub actor_id: Option<i64>,
    pub actor_label: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub result: String,
    pub reason_code: Option<String>,
    pub source_ip: String,
    pub user_agent: String,
    /// Audit producers own this event-specific JSON object, so its fields are intentionally open.
    pub changes: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventListData {
    pub list: Vec<AuditEventResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

impl From<audit::AuditEventView> for AuditEventResponse {
    fn from(value: audit::AuditEventView) -> Self {
        Self {
            id: value.id,
            actor_id: value.actor_id,
            actor_label: value.actor_label,
            action: value.action,
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            result: value.result,
            reason_code: value.reason_code,
            source_ip: value.source_ip,
            user_agent: value.user_agent,
            changes: value.changes,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditAnalysisResponse {
    pub summary: String,
    pub risk_level: audit::AuditRiskLevel,
    pub findings: Vec<audit::AuditFinding>,
}

impl From<audit::AuditAnalysis> for AuditAnalysisResponse {
    fn from(value: audit::AuditAnalysis) -> Self {
        Self {
            summary: value.summary,
            risk_level: value.risk_level,
            findings: value.findings,
        }
    }
}
