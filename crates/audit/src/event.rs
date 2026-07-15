use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::IntoParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    Login,
    AccessDenied,
    AssignUserRoles,
}

impl AuditAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Login => "auth.login",
            Self::AccessDenied => "auth.access_denied",
            Self::AssignUserRoles => "user.assign_roles",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditActor {
    pub id: Option<i64>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditResource {
    Account(String),
    User(i64),
    Route(String),
}

impl AuditResource {
    pub const fn resource_type(&self) -> &'static str {
        match self {
            Self::Account(_) => "account",
            Self::User(_) => "user",
            Self::Route(_) => "route",
        }
    }

    pub fn resource_id(&self) -> String {
        match self {
            Self::Account(value) | Self::Route(value) => value.clone(),
            Self::User(id) => id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditResult {
    Succeeded,
    Denied,
    Failed,
}

impl AuditResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditReason {
    CaptchaRequired,
    CaptchaInvalid,
    CaptchaFailed,
    InvalidCredentials,
    UserDisabled,
    TokenIssueFailed,
    PermissionDenied,
    InvalidRoleAssignment,
    InternalError,
}

impl AuditReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CaptchaRequired => "captcha_required",
            Self::CaptchaInvalid => "captcha_invalid",
            Self::CaptchaFailed => "captcha_failed",
            Self::InvalidCredentials => "invalid_credentials",
            Self::UserDisabled => "user_disabled",
            Self::TokenIssueFailed => "token_issue_failed",
            Self::PermissionDenied => "permission_denied",
            Self::InvalidRoleAssignment => "invalid_role_assignment",
            Self::InternalError => "internal_error",
        }
    }
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
