mod service;

pub use service::Accounts;
use sqlx::FromRow;

use crate::{authorization::AuthorizationError, roles::RoleSummary};

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("user not found")]
    NotFound,
    #[error("user already exists")]
    AlreadyExists,
    #[error("selected roles are invalid")]
    InvalidRoles,
    #[error("only an active super_admin may perform this operation")]
    AccessDenied,
    #[error("the final active super_admin cannot be removed")]
    LastSuperAdmin,
    #[error("selected permissions are invalid")]
    InvalidPermissions,
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
}

#[derive(Debug, thiserror::Error)]
pub enum RefreshIdentityError {
    #[error("user not found")]
    NotFound,
    #[error("user is disabled")]
    Disabled,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
}

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
    pub direct: bool,
    pub roles: Vec<EffectiveRoleSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountPermissionCatalogItem {
    pub permission: String,
    pub title: String,
    pub menu_type: String,
    pub status: String,
    pub effectively_enabled: bool,
    pub owning_page_id: i64,
    pub owning_page_title: String,
    pub page_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAccessView {
    pub role_ids: Vec<i64>,
    pub direct_permissions: Vec<String>,
    pub effective_permissions: Vec<EffectivePermissionSource>,
    pub catalog: Vec<AccountPermissionCatalogItem>,
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

#[derive(Debug)]
pub struct PreparedPasswordUpdate {
    user_id: i64,
    password_hash: String,
}

impl PreparedPasswordUpdate {
    pub fn user_id(&self) -> i64 {
        self.user_id
    }

    pub fn new(user_id: i64, password_hash: String) -> Self {
        Self {
            user_id,
            password_hash,
        }
    }

    fn into_parts(self) -> (i64, String) {
        (self.user_id, self.password_hash)
    }
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

#[derive(Debug, Clone)]
pub struct GetUserListRequest {
    pub page: i64,
    pub page_size: i64,
    pub keyword: Option<String>,
    pub username: Option<String>,
    pub nick_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub order_key: Option<String>,
    pub desc: Option<bool>,
}
