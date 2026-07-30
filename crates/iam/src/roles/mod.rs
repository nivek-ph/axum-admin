mod service;

pub use service::RoleService;
pub(crate) use service::find;
use sqlx::FromRow;

use crate::{access::CatalogError, authorization::AuthorizationError};

#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("role not found")]
    NotFound,
    #[error("system role cannot be deleted")]
    Immutable,
    #[error(transparent)]
    InvalidMenuAssignment(#[from] CatalogError),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
}

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

#[derive(Debug, Clone)]
pub struct RolePayload {
    pub code: String,
    pub name: String,
    pub status: Option<String>,
    pub sort: Option<i32>,
    pub data_scope: Option<String>,
}
