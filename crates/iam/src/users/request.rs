use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RegisterRequest {
    #[serde(rename = "username")]
    pub user_name: String,
    #[serde(rename = "password")]
    pub password: String,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    #[serde(rename = "headerImg")]
    pub header_img: Option<String>,
    #[serde(rename = "roleIds")]
    pub role_ids: Option<Vec<i64>>,
    #[serde(rename = "deptId", alias = "dept_id")]
    pub dept_id: Option<i64>,
    pub enable: Option<i32>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    #[serde(rename = "id")]
    pub id: i64,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    #[serde(rename = "headerImg")]
    pub header_img: String,
    pub enable: i32,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "deptId", alias = "dept_id")]
    pub dept_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub password: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    #[serde(rename = "id")]
    pub id: i64,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetUserRolesRequest {
    #[serde(rename = "roleIds", alias = "role_ids")]
    pub role_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetUserListRequest {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub username: Option<String>,
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "orderKey")]
    pub order_key: Option<String>,
    pub desc: Option<bool>,
}
