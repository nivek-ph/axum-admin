use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub sort: i32,
    pub data_scope: String,
    pub is_system: bool,
}
impl From<iam::roles::RoleSummary> for RoleResponse {
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

#[derive(Debug, Deserialize)]
pub struct RolePayload {
    pub code: String,
    pub name: String,
    pub status: Option<String>,
    pub sort: Option<i32>,
    #[serde(alias = "dataScope")]
    pub data_scope: Option<String>,
}
impl From<RolePayload> for iam::roles::RolePayload {
    fn from(v: RolePayload) -> Self {
        Self {
            code: v.code,
            name: v.name,
            status: v.status,
            sort: v.sort,
            data_scope: v.data_scope,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RoleMenuPayload {
    #[serde(rename = "menuIds", alias = "menu_ids")]
    pub menu_ids: Vec<i64>,
}
#[derive(Debug, Deserialize)]
pub struct RoleDeptPayload {
    #[serde(rename = "deptIds", alias = "dept_ids")]
    pub dept_ids: Vec<i64>,
}
#[derive(Debug, Deserialize)]
pub struct RoleUsersPayload {
    #[serde(rename = "userIds", alias = "user_ids")]
    pub user_ids: Vec<i64>,
}
