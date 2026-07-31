use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
pub struct RoleMenuRequest {
    #[serde(rename = "menuIds", alias = "menu_ids")]
    pub menu_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RolePermissionRequest {
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
pub struct RoleMenuIdsData {
    pub menu_ids: Vec<i64>,
    pub effective_menu_ids: Vec<i64>,
    pub protected: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RolePermissionsData {
    pub permissions: Vec<String>,
    pub catalog: Vec<OperationPermissionCatalogItem>,
    pub protected: bool,
}

impl From<iam::roles::RoleSummary> for RoleResponse {
    fn from(v: iam::roles::RoleSummary) -> Self {
        Self {
            id: v.id,
            code: v.code,
            name: v.name,
            status: v.status,
            sort: v.sort,
        }
    }
}

impl From<iam::roles::RoleOperationPermissionsWithCatalog> for RolePermissionsData {
    fn from(role_permissions: iam::roles::RoleOperationPermissionsWithCatalog) -> Self {
        Self {
            permissions: role_permissions.permissions,
            catalog: role_permissions
                .catalog
                .into_iter()
                .map(|item| OperationPermissionCatalogItem {
                    permission: item.permission,
                    title: item.title,
                    menu_type: item.menu_type,
                    status: item.status,
                    effectively_enabled: item.effectively_enabled,
                    owning_page_id: item.owning_page_id,
                    owning_page_title: item.owning_page_title,
                    page_visible: item.page_visible,
                })
                .collect(),
            protected: role_permissions.protected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_ids_keep_transport_shape() {
        let menus = serde_json::to_value(RoleMenuIdsData {
            menu_ids: vec![1, 2],
            effective_menu_ids: vec![1],
            protected: false,
        })
        .expect("menu IDs should serialize");
        assert_eq!(menus["menuIds"], serde_json::json!([1, 2]));
        assert_eq!(menus["effectiveMenuIds"], serde_json::json!([1]));
        assert!(menus.get("menu_ids").is_none());
    }
}
