use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use strum::EnumProperty;
use utoipa::IntoParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
pub enum AuditAction {
    #[strum(serialize = "auth.login")]
    Login,
    #[strum(serialize = "auth.access_denied")]
    AccessDenied,
    #[strum(serialize = "user.assign_roles")]
    AssignUserRoles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditActor {
    pub id: Option<i64>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumProperty)]
pub enum AuditResource {
    #[strum(props(resource_type = "account"))]
    Account(String),
    #[strum(props(resource_type = "user"))]
    User(i64),
    #[strum(props(resource_type = "route"))]
    Route(String),
}

impl AuditResource {
    pub fn resource_type(&self) -> String {
        self.get_str("resource_type")
            .expect("every audit resource should declare its resource type")
            .to_string()
    }

    pub fn resource_id(&self) -> String {
        match self {
            Self::Account(value) | Self::Route(value) => value.clone(),
            Self::User(id) => id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
pub enum AuditResult {
    #[strum(serialize = "succeeded")]
    Succeeded,
    #[strum(serialize = "denied")]
    Denied,
    #[strum(serialize = "failed")]
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
pub enum AuditReason {
    #[strum(serialize = "captcha_required")]
    CaptchaRequired,
    #[strum(serialize = "captcha_invalid")]
    CaptchaInvalid,
    #[strum(serialize = "captcha_failed")]
    CaptchaFailed,
    #[strum(serialize = "invalid_credentials")]
    InvalidCredentials,
    #[strum(serialize = "user_disabled")]
    UserDisabled,
    #[strum(serialize = "token_issue_failed")]
    TokenIssueFailed,
    #[strum(serialize = "permission_denied")]
    PermissionDenied,
    #[strum(serialize = "invalid_role_assignment")]
    InvalidRoleAssignment,
    #[strum(serialize = "internal_error")]
    InternalError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSource {
    pub ip: String,
    pub user_agent: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditContext {
    pub actor: AuditActor,
    pub source: AuditSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum AuditValue {
    Ids(Vec<i64>),
    Text(String),
    Masked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldChange {
    pub field: String,
    pub before: AuditValue,
    pub after: AuditValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub actor: AuditActor,
    pub action: AuditAction,
    pub resource: AuditResource,
    pub result: AuditResult,
    pub reason_code: Option<AuditReason>,
    pub source: AuditSource,
    pub changes: Vec<FieldChange>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AuditQuery {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub actor: Option<String>,
    pub action: Option<String>,
    #[serde(rename = "resourceType")]
    pub resource_type: Option<String>,
    #[serde(rename = "resourceId")]
    pub resource_id: Option<String>,
    pub result: Option<String>,
    #[serde(rename = "startedAt")]
    pub started_at: Option<String>,
    #[serde(rename = "endedAt")]
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AuditEventView {
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
