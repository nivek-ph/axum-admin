mod error;
mod service;

pub use error::{AccountError, RefreshIdentityError};
use serde::Deserialize;
pub use service::Accounts;
use sqlx::FromRow;
use utoipa::IntoParams;

use crate::roles::RoleSummary;

#[derive(Debug, Clone, FromRow)]
struct UserRecord {
    id: i64,
    uuid: String,
    username: String,
    password_hash: String,
    nick_name: String,
    header_img: String,
    home_route: String,
    enable: bool,
    phone: Option<String>,
    email: Option<String>,
    origin_setting: Option<serde_json::Value>,
    dept_id: Option<i64>,
    dept_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserInfoView {
    pub id: i64,
    pub uuid: String,
    pub user_name: String,
    pub nick_name: String,
    pub header_img: String,
    pub home_route: String,
    pub enable: i32,
    pub phone: String,
    pub email: String,
    pub origin_setting: Option<serde_json::Value>,
    pub dept_id: Option<i64>,
    pub dept_name: String,
    pub roles: Vec<RoleSummary>,
    pub role_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveRoleSource {
    pub id: i64,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePermissionSource {
    pub permission: String,
    pub roles: Vec<EffectiveRoleSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAccessView {
    pub assigned_roles: Vec<RoleSummary>,
    pub effective_permissions: Vec<EffectivePermissionSource>,
}

#[derive(Debug, Clone)]
pub struct LoginAccount {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub enable: bool,
}

#[derive(Debug, Clone)]
pub struct RefreshIdentity {
    pub username: String,
}

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

#[derive(Debug, Clone)]
pub struct SetSelfInfoRequest {
    pub nick_name: Option<String>,
    pub header_img: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetSelfSettingRequest {
    pub origin_setting: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct UserListQuery {
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
