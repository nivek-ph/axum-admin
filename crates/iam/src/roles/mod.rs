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

#[derive(Debug, Clone)]
pub struct RoleAccessView {
    pub permissions: Vec<String>,
    pub tree: Vec<crate::menus::MenuView>,
    pub protected: bool,
}

#[derive(Debug, Clone)]
pub struct RolePayload {
    pub code: String,
    pub name: String,
    pub status: Option<String>,
    pub sort: Option<i32>,
}
