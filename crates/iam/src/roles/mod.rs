mod error;
mod service;

pub use error::RoleError;
pub use service::RoleService;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct RoleSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub sort: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMenuAccess {
    pub menu_ids: Vec<i64>,
    pub effective_menu_ids: Vec<i64>,
    pub protected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleOperationPermissionSelection {
    pub permissions: Vec<String>,
    pub protected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleOperationPermissionsWithCatalog {
    pub permissions: Vec<String>,
    pub catalog: Vec<OperationPermissionCatalogItem>,
    pub protected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPermissionCatalogItem {
    pub permission: String,
    pub title: String,
    pub menu_type: String,
    pub status: String,
    pub effectively_enabled: bool,
    pub owning_page_id: i64,
    pub owning_page_title: String,
    pub page_visible: bool,
}

#[derive(Debug, Clone)]
pub struct RolePayload {
    pub code: String,
    pub name: String,
    pub status: Option<String>,
    pub sort: Option<i32>,
}
