#[derive(Debug, Clone)]
pub struct RolePayload {
    pub code: String,
    pub name: String,
    pub status: Option<String>,
    pub sort: Option<i32>,
    pub data_scope: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoleMenuPayload {
    pub menu_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct RoleDeptPayload {
    pub dept_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct RoleUsersPayload {
    pub user_ids: Vec<i64>,
}
