use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfoData {
    #[serde(rename = "userInfo")]
    pub user_info: UserResponse,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_info_data_keeps_transport_shape() {
        let value = serde_json::to_value(UserInfoData {
            user_info: UserResponse {
                id: 1,
                uuid: "uuid".to_string(),
                user_name: "admin".to_string(),
                nick_name: "Admin".to_string(),
                header_img: String::new(),
                home_route: "Dashboard".to_string(),
                enable: 1,
                phone: String::new(),
                email: String::new(),
                origin_setting: None,
                dept_id: None,
                dept_name: String::new(),
                roles: Vec::new(),
                role_ids: Vec::new(),
            },
        })
        .expect("user info data should serialize");

        assert_eq!(value["userInfo"]["userName"], "admin");
        assert_eq!(value["userInfo"]["homeRoute"], "Dashboard");
        assert!(value.get("user_info").is_none());
    }
}
