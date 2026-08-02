mod menus;
mod routes;

use std::collections::{BTreeSet, HashSet};

use menus::MenuIndex;
use routes::RouteIndex;
use sqlx::{FromRow, PgPool};

use super::CatalogError;
use crate::IamInitError;

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct AccessNode {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
    pub menu_type: String,
    pub status: String,
    pub permission: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionCatalogEntry {
    pub permission: String,
    pub title: String,
    pub menu_type: String,
    pub status: String,
    pub effectively_enabled: bool,
    pub owning_page_id: i64,
    pub owning_page_title: String,
    pub page_visible: bool,
}

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct AccessBinding {
    pub menu_id: i64,
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct AccessCatalog {
    menus: MenuIndex,
    routes: RouteIndex,
}

impl AccessCatalog {
    pub(crate) async fn load(pool: &PgPool) -> Result<Self, IamInitError> {
        let nodes = sqlx::query_as::<_, AccessNode>(
            "select id, parent_id, title, menu_type, status, permission from sys_menus order by id",
        )
        .fetch_all(pool)
        .await?;
        let bindings = sqlx::query_as::<_, AccessBinding>(
            "select menu_id, method, path_pattern as path from sys_menu_apis order by method, path_pattern",
        )
        .fetch_all(pool)
        .await?;
        Ok(Self::from_parts(nodes, bindings)?)
    }

    pub fn from_parts(
        nodes: Vec<AccessNode>,
        bindings: Vec<AccessBinding>,
    ) -> Result<Self, CatalogError> {
        let menus = MenuIndex::from_nodes(nodes)?;
        let active_bindings = menus.active_bindings(bindings)?;
        let routes = RouteIndex::from_bindings(active_bindings)?;
        Ok(Self { menus, routes })
    }

    pub fn resolve(&self, method: &str, path: &str) -> Result<i64, CatalogError> {
        self.routes.resolve(method, path)
    }

    pub fn enabled_permissions(&self) -> &HashSet<String> {
        self.menus.enabled_permissions()
    }

    pub fn permission_catalog(
        &self,
        visible_page_ids: &BTreeSet<i64>,
        role_enabled: bool,
    ) -> Vec<PermissionCatalogEntry> {
        self.menus
            .permission_catalog(visible_page_ids, role_enabled)
    }

    pub(crate) fn page_entry_permissions(
        &self,
        configured_menu_ids: &HashSet<i64>,
    ) -> BTreeSet<String> {
        self.menus.page_entry_permissions(configured_menu_ids)
    }

    pub(crate) fn is_action_permission(&self, permission: &str) -> bool {
        self.menus.is_action_permission(permission)
    }

    pub fn permission_for_menu(&self, menu_id: i64) -> Result<&str, CatalogError> {
        self.menus.permission_for_menu(menu_id)
    }

    pub fn validate_assignment(&self, menu_ids: &HashSet<i64>) -> Result<(), CatalogError> {
        self.menus.validate_assignment(menu_ids)
    }

    pub fn effective_page_access(
        &self,
        configured_menu_ids: &HashSet<i64>,
        role_enabled: bool,
    ) -> BTreeSet<i64> {
        self.menus
            .effective_page_access(configured_menu_ids, role_enabled)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use super::{AccessBinding, AccessCatalog, AccessNode, CatalogError};

    fn catalog(bindings: Vec<AccessBinding>) -> Result<AccessCatalog, CatalogError> {
        let menu_ids = bindings
            .iter()
            .map(|binding| binding.menu_id)
            .collect::<BTreeSet<_>>();
        let mut nodes = vec![AccessNode {
            id: -1,
            parent_id: None,
            title: "Test directory".to_string(),
            menu_type: "directory".to_string(),
            status: "enabled".to_string(),
            permission: None,
        }];
        nodes.extend(menu_ids.into_iter().map(|id| AccessNode {
            id,
            parent_id: Some(-1),
            title: format!("Menu {id}"),
            menu_type: "page".to_string(),
            status: "enabled".to_string(),
            permission: Some(format!("test:menu:{id}")),
        }));
        AccessCatalog::from_parts(nodes, bindings)
    }

    fn binding(menu_id: i64, method: &str, path: &str) -> AccessBinding {
        AccessBinding {
            menu_id,
            method: method.to_string(),
            path: path.to_string(),
        }
    }

    #[test]
    fn resolves_exact_routes_before_dynamic_routes() {
        let catalog = catalog(vec![
            binding(10, "GET", "/api/users/{id}"),
            binding(11, "GET", "/api/users/batch"),
        ])
        .expect("catalog should be valid");

        assert_eq!(catalog.resolve("get", "/api/users/batch"), Ok(11));
        assert_eq!(catalog.resolve("GET", "/api/users/42"), Ok(10));
    }

    #[test]
    fn rejects_matchit_conflicting_dynamic_route_shapes() {
        let result = catalog(vec![
            binding(10, "GET", "/api/users/{id}"),
            binding(11, "GET", "/api/users/{user_id}"),
        ]);

        assert!(matches!(result, Err(CatalogError::ConflictingBinding)));
    }

    #[test]
    fn follows_matchit_priority_for_overlapping_routes() {
        let catalog = catalog(vec![
            binding(10, "GET", "/api/roles/{id}/permissions"),
            binding(11, "GET", "/api/{area}/{id}/permissions"),
        ])
        .expect("matchit should accept overlapping routes");

        assert_eq!(catalog.resolve("GET", "/api/roles/2/permissions"), Ok(10));
        assert_eq!(catalog.resolve("GET", "/api/acme/2/permissions"), Ok(11));
    }

    #[test]
    fn keeps_non_overlapping_dynamic_route_patterns_distinct() {
        let catalog = catalog(vec![
            binding(10, "GET", "/api/{area}/users/{id}"),
            binding(11, "GET", "/api/roles/admin/{id}"),
        ])
        .expect("non-overlapping catalog routes should be valid");

        assert_eq!(catalog.resolve("GET", "/api/acme/users/42"), Ok(10));
        assert_eq!(catalog.resolve("GET", "/api/roles/admin/42"), Ok(11));
    }

    #[test]
    fn follows_matchit_template_syntax() {
        let catalog = catalog(vec![
            binding(10, "GET", "/api/files/{*rest}"),
            binding(11, "GET", "/api/files/{{name}}"),
            binding(12, "GET", "/api/files/file-{id}"),
        ])
        .expect("matchit template syntax should be accepted");

        assert_eq!(catalog.resolve("GET", "/api/files/a/b"), Ok(10));
        assert_eq!(catalog.resolve("GET", "/api/files/{name}"), Ok(11));
        assert_eq!(catalog.resolve("GET", "/api/files/file-42"), Ok(12));
    }

    #[test]
    fn rejects_matchit_invalid_route_syntax() {
        for path in ["/api/files/{id}-suffix", "/api/files/{*rest}/tail"] {
            assert!(
                matches!(
                    catalog(vec![binding(10, "GET", path)]),
                    Err(CatalogError::InvalidBinding)
                ),
                "invalid matchit route should be rejected: {path}"
            );
        }
    }

    #[test]
    fn rejects_unbound_routes() {
        let catalog =
            catalog(vec![binding(10, "GET", "/api/users")]).expect("catalog should be valid");

        assert_eq!(
            catalog.resolve("POST", "/api/users"),
            Err(CatalogError::Unbound)
        );
    }

    #[test]
    fn disabled_ancestors_remove_descendant_routes_and_permissions() {
        let catalog = AccessCatalog::from_parts(
            vec![
                AccessNode {
                    id: 1,
                    parent_id: None,
                    title: "Directory".to_string(),
                    menu_type: "directory".to_string(),
                    status: "disabled".to_string(),
                    permission: None,
                },
                AccessNode {
                    id: 2,
                    parent_id: Some(1),
                    title: "Users".to_string(),
                    menu_type: "page".to_string(),
                    status: "enabled".to_string(),
                    permission: Some("system:user:list".to_string()),
                },
            ],
            vec![binding(2, "GET", "/api/users")],
        )
        .expect("disabled nodes are valid catalog entries");

        assert_eq!(
            catalog.resolve("GET", "/api/users"),
            Err(CatalogError::Unbound)
        );
        assert!(catalog.enabled_permissions().is_empty());
        assert!(std::ptr::eq(
            catalog.enabled_permissions(),
            catalog.enabled_permissions()
        ));
    }

    #[test]
    fn actions_must_have_page_parents() {
        let result = AccessCatalog::from_parts(
            vec![AccessNode {
                id: 2,
                parent_id: None,
                title: "Create user".to_string(),
                menu_type: "action".to_string(),
                status: "enabled".to_string(),
                permission: Some("system:user:create".to_string()),
            }],
            vec![],
        );

        assert!(matches!(result, Err(CatalogError::InvalidTree)));
    }

    #[test]
    fn rejects_directory_cycles() {
        let result = AccessCatalog::from_parts(
            vec![
                AccessNode {
                    id: 1,
                    parent_id: Some(2),
                    title: "First".to_string(),
                    menu_type: "directory".to_string(),
                    status: "enabled".to_string(),
                    permission: None,
                },
                AccessNode {
                    id: 2,
                    parent_id: Some(1),
                    title: "Second".to_string(),
                    menu_type: "directory".to_string(),
                    status: "enabled".to_string(),
                    permission: None,
                },
            ],
            vec![],
        );

        assert!(matches!(result, Err(CatalogError::InvalidTree)));
    }

    #[test]
    fn assignments_must_include_every_ancestor() {
        let catalog = AccessCatalog::from_parts(
            vec![
                AccessNode {
                    id: 1,
                    parent_id: None,
                    title: "Directory".to_string(),
                    menu_type: "directory".to_string(),
                    status: "enabled".to_string(),
                    permission: None,
                },
                AccessNode {
                    id: 2,
                    parent_id: Some(1),
                    title: "Users".to_string(),
                    menu_type: "page".to_string(),
                    status: "enabled".to_string(),
                    permission: Some("system:user:list".to_string()),
                },
            ],
            vec![],
        )
        .expect("catalog should be valid");

        assert_eq!(
            catalog.validate_assignment(&HashSet::from([2])),
            Err(CatalogError::InvalidTree)
        );
        assert_eq!(catalog.validate_assignment(&HashSet::from([1, 2])), Ok(()));
    }

    #[test]
    fn page_assignments_preserve_disabled_nodes_but_reject_actions() {
        let catalog = AccessCatalog::from_parts(
            vec![
                AccessNode {
                    id: 1,
                    parent_id: None,
                    title: "Directory".to_string(),
                    menu_type: "directory".to_string(),
                    status: "disabled".to_string(),
                    permission: None,
                },
                AccessNode {
                    id: 2,
                    parent_id: Some(1),
                    title: "Users".to_string(),
                    menu_type: "page".to_string(),
                    status: "disabled".to_string(),
                    permission: Some("system:user:list".to_string()),
                },
                AccessNode {
                    id: 3,
                    parent_id: Some(2),
                    title: "Create user".to_string(),
                    menu_type: "action".to_string(),
                    status: "disabled".to_string(),
                    permission: Some("system:user:create".to_string()),
                },
            ],
            vec![],
        )
        .unwrap();

        assert_eq!(catalog.validate_assignment(&HashSet::from([1, 2])), Ok(()));
        assert_eq!(
            catalog.validate_assignment(&HashSet::from([1, 2, 3])),
            Err(CatalogError::InvalidTree)
        );
        assert!(
            catalog
                .effective_page_access(&HashSet::from([1, 2]), true)
                .is_empty()
        );
    }
}
