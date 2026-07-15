use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct UserRoleResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub sort: i32,
    pub data_scope: String,
    pub is_system: bool,
}
impl From<iam::roles::RoleSummary> for UserRoleResponse {
    fn from(v: iam::roles::RoleSummary) -> Self {
        Self {
            id: v.id,
            code: v.code,
            name: v.name,
            status: v.status,
            sort: v.sort,
            data_scope: v.data_scope,
            is_system: v.is_system,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub uuid: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    #[serde(rename = "headerImg")]
    pub header_img: String,
    #[serde(rename = "homeRoute")]
    pub home_route: String,
    pub enable: i32,
    pub phone: String,
    pub email: String,
    #[serde(rename = "originSetting")]
    pub origin_setting: Option<serde_json::Value>,
    #[serde(rename = "deptId")]
    pub dept_id: Option<i64>,
    #[serde(rename = "deptName")]
    pub dept_name: String,
    pub roles: Vec<UserRoleResponse>,
    #[serde(rename = "roleIds")]
    pub role_ids: Vec<i64>,
}
impl From<iam::users::UserInfoView> for UserResponse {
    fn from(v: iam::users::UserInfoView) -> Self {
        Self {
            id: v.id,
            uuid: v.uuid,
            user_name: v.user_name,
            nick_name: v.nick_name,
            header_img: v.header_img,
            home_route: v.home_route,
            enable: v.enable,
            phone: v.phone,
            email: v.email,
            origin_setting: v.origin_setting,
            dept_id: v.dept_id,
            dept_name: v.dept_name,
            roles: v.roles.into_iter().map(Into::into).collect(),
            role_ids: v.role_ids,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "passWord")]
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

impl From<RegisterRequest> for iam::users::RegisterRequest {
    fn from(value: RegisterRequest) -> Self {
        Self {
            user_name: value.user_name,
            password: value.password,
            nick_name: value.nick_name,
            header_img: value.header_img,
            role_ids: value.role_ids,
            dept_id: value.dept_id,
            enable: value.enable,
            phone: value.phone,
            email: value.email,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

impl From<UpdateUserRequest> for iam::users::UpdateUserRequest {
    fn from(value: UpdateUserRequest) -> Self {
        Self {
            id: value.id,
            nick_name: value.nick_name,
            header_img: value.header_img,
            enable: value.enable,
            phone: value.phone,
            email: value.email,
            dept_id: value.dept_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    pub password: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
}

impl From<ChangePasswordRequest> for iam::users::ChangePasswordRequest {
    fn from(value: ChangePasswordRequest) -> Self {
        Self {
            password: value.password,
            new_password: value.new_password,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSelfInfoRequest {
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    #[serde(rename = "headerImg")]
    pub header_img: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

impl From<SetSelfInfoRequest> for iam::users::SetSelfInfoRequest {
    fn from(value: SetSelfInfoRequest) -> Self {
        Self {
            nick_name: value.nick_name,
            header_img: value.header_img,
            phone: value.phone,
            email: value.email,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSelfSettingRequest {
    #[serde(flatten)]
    pub origin_setting: serde_json::Value,
}

impl From<SetSelfSettingRequest> for iam::users::SetSelfSettingRequest {
    fn from(value: SetSelfSettingRequest) -> Self {
        Self {
            origin_setting: value.origin_setting,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResetPasswordRequest {
    #[serde(rename = "id")]
    pub id: i64,
    pub password: String,
}

impl From<ResetPasswordRequest> for iam::users::ResetPasswordRequest {
    fn from(value: ResetPasswordRequest) -> Self {
        Self {
            id: value.id,
            password: value.password,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetUserRolesRequest {
    #[serde(rename = "roleIds", alias = "role_ids")]
    pub role_ids: Vec<i64>,
}

impl From<SetUserRolesRequest> for iam::users::SetUserRolesRequest {
    fn from(value: SetUserRolesRequest) -> Self {
        Self {
            role_ids: value.role_ids,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

impl From<GetUserListRequest> for iam::users::GetUserListRequest {
    fn from(value: GetUserListRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            username: value.username,
            nick_name: value.nick_name,
            phone: value.phone,
            email: value.email,
            order_key: value.order_key,
            desc: value.desc,
        }
    }
}
