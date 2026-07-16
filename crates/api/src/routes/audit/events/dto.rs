use serde::Serialize;

#[derive(Debug, Serialize)]
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
    pub changes: serde_json::Value,
    pub created_at: String,
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
