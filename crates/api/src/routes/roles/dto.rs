use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::menus::dto::MenuResponse;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RoleRequest {
    pub code: String,
    pub name: String,
    pub status: Option<String>,
    pub sort: Option<i32>,
}

impl From<RoleRequest> for iam::roles::RolePayload {
    fn from(value: RoleRequest) -> Self {
        Self {
            code: value.code,
            name: value.name,
            status: value.status,
            sort: value.sort,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RoleAccessRequest {
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub sort: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleListData {
    pub list: Vec<RoleResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleData {
    pub role: RoleResponse,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleAccessData {
    pub permissions: Vec<String>,
    pub tree: Vec<MenuResponse>,
    pub protected: bool,
}

impl From<iam::roles::RoleSummary> for RoleResponse {
    fn from(value: iam::roles::RoleSummary) -> Self {
        Self {
            id: value.id,
            code: value.code,
            name: value.name,
            status: value.status,
            sort: value.sort,
        }
    }
}

impl From<iam::roles::RoleAccessView> for RoleAccessData {
    fn from(value: iam::roles::RoleAccessView) -> Self {
        Self {
            permissions: value.permissions,
            tree: value.tree.into_iter().map(Into::into).collect(),
            protected: value.protected,
        }
    }
}
