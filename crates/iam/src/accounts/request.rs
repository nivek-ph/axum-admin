use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone)]
pub struct CreateAccountInput {
    pub user_name: String,
    pub password_hash: String,
    pub nick_name: String,
    pub header_img: Option<String>,
    pub role_ids: Option<Vec<i64>>,
    pub dept_id: Option<i64>,
    pub enable: Option<i32>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateUserInput {
    pub nick_name: String,
    pub header_img: String,
    pub enable: i32,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub dept_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetSelfInfoRequest {
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    #[serde(rename = "headerImg")]
    pub header_img: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetSelfSettingRequest {
    #[serde(flatten)]
    pub origin_setting: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct DeleteUserRequest {
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetUserListRequest {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub keyword: Option<String>,
    pub username: Option<String>,
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "orderKey")]
    pub order_key: Option<String>,
    pub desc: Option<bool>,
}
