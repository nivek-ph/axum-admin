use sqlx::FromRow;

use crate::{access::ResolvedDataScope, roles::RoleSummary};

#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    pub id: i64,
    pub uuid: String,
    pub username: String,
    pub password_hash: String,
    pub nick_name: String,
    pub header_img: String,
    pub home_route: String,
    pub enable: bool,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub origin_setting: Option<serde_json::Value>,
    pub dept_id: Option<i64>,
    pub dept_name: Option<String>,
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

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: i64,
    pub data_scope: ResolvedDataScope,
}

#[derive(Debug, Clone)]
pub struct LoginIdentity {
    pub id: i64,
    pub username: String,
    pub user: UserInfoView,
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

    pub(crate) fn new(user_id: i64, password_hash: String) -> Self {
        Self {
            user_id,
            password_hash,
        }
    }

    pub(crate) fn into_parts(self) -> (i64, String) {
        (self.user_id, self.password_hash)
    }
}
