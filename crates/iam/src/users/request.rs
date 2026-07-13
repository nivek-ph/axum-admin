#[derive(Debug, Clone)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct RegisterRequest {
    pub user_name: String,
    pub password: String,
    pub nick_name: String,
    pub header_img: Option<String>,
    pub role_ids: Option<Vec<i64>>,
    pub dept_id: Option<i64>,
    pub enable: Option<i32>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateUserRequest {
    pub id: i64,
    pub nick_name: String,
    pub header_img: String,
    pub enable: i32,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub dept_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ChangePasswordRequest {
    pub password: String,
    pub new_password: String,
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
pub struct DeleteUserRequest {
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct ResetPasswordRequest {
    pub id: i64,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct SetUserRolesRequest {
    pub role_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct GetUserListRequest {
    pub page: i64,
    pub page_size: i64,
    pub username: Option<String>,
    pub nick_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub order_key: Option<String>,
    pub desc: Option<bool>,
}
