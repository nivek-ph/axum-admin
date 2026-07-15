use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LoginLogResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub username: String,
    pub ip: String,
    pub status: bool,
    #[serde(rename = "errorMessage")]
    pub error_message: String,
    pub agent: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

impl From<audit::login_logs::LoginLogView> for LoginLogResponse {
    fn from(v: audit::login_logs::LoginLogView) -> Self {
        Self {
            id: v.id,
            username: v.username,
            ip: v.ip,
            status: v.status,
            error_message: v.error_message,
            agent: v.agent,
            created_at: v.created_at,
        }
    }
}
