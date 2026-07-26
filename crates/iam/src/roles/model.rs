use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct RoleSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub sort: i32,
    pub data_scope: String,
    pub is_system: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleAssignment {
    pub user_id: i64,
    pub role_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMenuAccess {
    pub menu_ids: Vec<i64>,
    pub effective_menu_ids: Vec<i64>,
    pub system_managed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionCatalogItem {
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
pub struct RolePermissions {
    pub permissions: Vec<String>,
    pub catalog: Vec<PermissionCatalogItem>,
    pub system_managed: bool,
}
