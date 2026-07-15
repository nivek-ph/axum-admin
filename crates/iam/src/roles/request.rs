use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RolePayload {
    pub code: String,
    pub name: String,
    pub status: Option<String>,
    pub sort: Option<i32>,
    #[serde(alias = "dataScope")]
    pub data_scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RoleMenuPayload {
    #[serde(rename = "menuIds", alias = "menu_ids")]
    pub menu_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RoleDeptPayload {
    #[serde(rename = "deptIds", alias = "dept_ids")]
    pub dept_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RoleUsersPayload {
    #[serde(rename = "userIds", alias = "user_ids")]
    pub user_ids: Vec<i64>,
}
