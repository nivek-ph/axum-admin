use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct LoginLogSearch {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub username: Option<String>,
    pub status: Option<bool>,
}
impl From<LoginLogSearch> for audit::login_logs::LoginLogSearch {
    fn from(v: LoginLogSearch) -> Self {
        Self {
            page: v.page,
            page_size: v.page_size,
            username: v.username,
            status: v.status,
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct IdRequest {
    #[serde(rename = "id")]
    pub id: i64,
}
#[derive(Debug, Deserialize)]
pub struct IdsRequest {
    pub ids: Vec<i64>,
}
