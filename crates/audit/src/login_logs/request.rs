use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Clone)]
pub struct CreateLoginLog {
    pub username: String,
    pub ip: String,
    pub status: bool,
    pub error_message: String,
    pub agent: String,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LoginLogSearch {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub username: Option<String>,
    pub status: Option<bool>,
}
