use std::collections::{BTreeSet, HashMap, HashSet};

use sqlx::{FromRow, PgPool};

use crate::IamInitError;

#[derive(Debug, FromRow)]
struct CatalogNodeRow {
    id: i64,
    parent_id: Option<i64>,
    title: String,
    menu_type: String,
    status: String,
    permission: Option<String>,
}

#[derive(Debug, FromRow)]
struct CatalogBindingRow {
    menu_id: i64,
    method: String,
    path_pattern: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessBinding {
    pub menu_id: i64,
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CatalogError {
    #[error("access catalog contains conflicting route bindings")]
    ConflictingBinding,
    #[error("request route is not bound to an access node")]
    Unbound,
    #[error("request route matches multiple access nodes")]
    Ambiguous,
    #[error("access catalog contains an invalid route binding")]
    InvalidBinding,
    #[error("access catalog contains an invalid menu tree")]
    InvalidTree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RouteBinding {
    menu_id: i64,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessCatalog {
    exact: HashMap<(String, String), i64>,
    dynamic: HashMap<String, Vec<RouteBinding>>,
    nodes: HashMap<i64, AccessNode>,
    enabled_menu_ids: HashSet<i64>,
    enabled_permissions: HashSet<String>,
    permissions_by_menu_id: HashMap<i64, String>,
    parents: HashMap<i64, Option<i64>>,
}

impl AccessCatalog {
    pub(crate) async fn load(pool: &PgPool) -> Result<Self, IamInitError> {
        let nodes = sqlx::query_as::<_, CatalogNodeRow>(
            "select id, parent_id, title, menu_type, status, permission from sys_menus order by id",
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| AccessNode {
            id: row.id,
            parent_id: row.parent_id,
            title: row.title,
            menu_type: row.menu_type,
            status: row.status,
            permission: row.permission,
        })
        .collect();
        let bindings = sqlx::query_as::<_, CatalogBindingRow>(
            "select menu_id, method, path_pattern from sys_menu_apis order by method, path_pattern",
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| AccessBinding {
            menu_id: row.menu_id,
            method: row.method,
            path: row.path_pattern,
        })
        .collect();
        Ok(Self::from_parts(nodes, bindings)?)
    }

    pub fn from_parts(
        nodes: Vec<AccessNode>,
        bindings: Vec<AccessBinding>,
    ) -> Result<Self, CatalogError> {
        let node_map = nodes
            .into_iter()
            .map(|node| (node.id, node))
            .collect::<HashMap<_, _>>();
        let mut permissions = HashSet::new();
        for node in node_map.values() {
            validate_node(node, &node_map)?;
            if let Some(permission) = node.permission.as_ref()
                && !permissions.insert(permission.clone())
            {
                return Err(CatalogError::InvalidTree);
            }
        }

        let enabled_menu_ids = node_map
            .values()
            .filter(|node| node_is_effectively_enabled(node.id, &node_map))
            .map(|node| node.id)
            .collect::<HashSet<_>>();
        let enabled_permissions = node_map
            .values()
            .filter(|node| enabled_menu_ids.contains(&node.id))
            .filter_map(|node| node.permission.clone())
            .collect::<HashSet<_>>();
        let permissions_by_menu_id = node_map
            .values()
            .filter(|node| enabled_menu_ids.contains(&node.id))
            .filter_map(|node| {
                node.permission
                    .as_ref()
                    .map(|permission| (node.id, permission.clone()))
            })
            .collect();
        let active_bindings = bindings
            .into_iter()
            .filter_map(|binding| {
                let node = node_map.get(&binding.menu_id)?;
                if node.menu_type == "directory" || node.permission.is_none() {
                    return Some(Err(CatalogError::InvalidBinding));
                }
                enabled_menu_ids
                    .contains(&binding.menu_id)
                    .then_some(Ok(binding))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let parents = node_map
            .values()
            .map(|node| (node.id, node.parent_id))
            .collect();
        Self::build(
            active_bindings,
            enabled_menu_ids,
            enabled_permissions,
            permissions_by_menu_id,
            parents,
            node_map,
        )
    }

    fn build(
        bindings: Vec<AccessBinding>,
        enabled_menu_ids: HashSet<i64>,
        enabled_permissions: HashSet<String>,
        permissions_by_menu_id: HashMap<i64, String>,
        parents: HashMap<i64, Option<i64>>,
        nodes: HashMap<i64, AccessNode>,
    ) -> Result<Self, CatalogError> {
        let mut exact = HashMap::new();
        let mut dynamic = HashMap::<String, Vec<RouteBinding>>::new();
        let mut route_shapes = HashSet::new();

        for binding in bindings {
            let method = normalize_method(&binding.method)?;
            let path = normalize_path(&binding.path)?;
            let shape = route_shape(&path);
            if !route_shapes.insert((method.clone(), shape)) {
                return Err(CatalogError::ConflictingBinding);
            }

            if path_is_dynamic(&path) {
                dynamic.entry(method).or_default().push(RouteBinding {
                    menu_id: binding.menu_id,
                    path,
                });
            } else if exact.insert((method, path), binding.menu_id).is_some() {
                return Err(CatalogError::ConflictingBinding);
            }
        }

        Ok(Self {
            exact,
            dynamic,
            nodes,
            enabled_menu_ids,
            enabled_permissions,
            permissions_by_menu_id,
            parents,
        })
    }

    pub fn resolve(&self, method: &str, path: &str) -> Result<i64, CatalogError> {
        let method = normalize_method(method)?;
        let path = normalize_path(path)?;
        if let Some(menu_id) = self.exact.get(&(method.clone(), path.clone())) {
            return Ok(*menu_id);
        }

        let matches = self
            .dynamic
            .get(&method)
            .into_iter()
            .flatten()
            .filter(|binding| path_pattern_matches(&binding.path, &path))
            .map(|binding| binding.menu_id)
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [] => Err(CatalogError::Unbound),
            [menu_id] => Ok(*menu_id),
            _ => Err(CatalogError::Ambiguous),
        }
    }

    pub fn enabled_permissions(&self) -> &HashSet<String> {
        &self.enabled_permissions
    }

    pub fn permission_catalog(
        &self,
        visible_page_ids: &BTreeSet<i64>,
        role_enabled: bool,
    ) -> Vec<PermissionCatalogEntry> {
        let mut entries = self
            .nodes
            .values()
            .filter(|node| node.menu_type == "page" || node.menu_type == "action")
            .filter_map(|node| {
                let permission = node.permission.clone()?;
                let page = if node.menu_type == "page" {
                    node
                } else {
                    self.nodes.get(&node.parent_id?)?
                };
                Some((
                    page.id,
                    node.menu_type == "action",
                    node.id,
                    PermissionCatalogEntry {
                        permission,
                        title: node.title.clone(),
                        menu_type: node.menu_type.clone(),
                        status: node.status.clone(),
                        effectively_enabled: self.enabled_menu_ids.contains(&node.id),
                        owning_page_id: page.id,
                        owning_page_title: page.title.clone(),
                        page_visible: role_enabled && visible_page_ids.contains(&page.id),
                    },
                ))
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|(page_id, is_action, node_id, _)| (*page_id, *is_action, *node_id));
        entries.into_iter().map(|(_, _, _, entry)| entry).collect()
    }

    pub fn permission_for_menu(&self, menu_id: i64) -> Result<&str, CatalogError> {
        self.permissions_by_menu_id
            .get(&menu_id)
            .map(String::as_str)
            .ok_or(CatalogError::InvalidBinding)
    }

    pub fn validate_assignment(&self, menu_ids: &HashSet<i64>) -> Result<(), CatalogError> {
        for menu_id in menu_ids {
            let node = self.nodes.get(menu_id).ok_or(CatalogError::InvalidTree)?;
            if node.menu_type == "action" {
                return Err(CatalogError::InvalidTree);
            }
            let mut parent_id = self.parents.get(menu_id).copied().flatten();
            while let Some(parent) = parent_id {
                if !menu_ids.contains(&parent) {
                    return Err(CatalogError::InvalidTree);
                }
                parent_id = self.parents.get(&parent).copied().flatten();
            }
        }
        Ok(())
    }

    pub fn effective_page_access(
        &self,
        configured_menu_ids: &HashSet<i64>,
        role_enabled: bool,
    ) -> BTreeSet<i64> {
        if !role_enabled {
            return BTreeSet::new();
        }
        let mut effective = BTreeSet::new();
        for menu_id in configured_menu_ids {
            let Some(node) = self.nodes.get(menu_id) else {
                continue;
            };
            if node.menu_type != "page" || !self.enabled_menu_ids.contains(menu_id) {
                continue;
            }
            effective.insert(*menu_id);
            let mut parent_id = node.parent_id;
            while let Some(parent) = parent_id {
                if configured_menu_ids.contains(&parent) && self.enabled_menu_ids.contains(&parent)
                {
                    effective.insert(parent);
                }
                parent_id = self.nodes.get(&parent).and_then(|item| item.parent_id);
            }
        }
        effective
    }

    pub fn system_page_access(&self) -> BTreeSet<i64> {
        let configured = self
            .nodes
            .values()
            .filter(|node| node.menu_type != "action")
            .map(|node| node.id)
            .collect::<HashSet<_>>();
        self.effective_page_access(&configured, true)
    }
}

fn validate_node(node: &AccessNode, nodes: &HashMap<i64, AccessNode>) -> Result<(), CatalogError> {
    let valid_type = matches!(node.menu_type.as_str(), "directory" | "page" | "action");
    let valid_status = matches!(node.status.as_str(), "enabled" | "disabled");
    let valid_permission = match node.menu_type.as_str() {
        "directory" => node.permission.is_none(),
        "page" | "action" => node
            .permission
            .as_ref()
            .is_some_and(|value| !value.is_empty()),
        _ => false,
    };
    if !valid_type || !valid_status || !valid_permission {
        return Err(CatalogError::InvalidTree);
    }

    match (node.menu_type.as_str(), node.parent_id) {
        ("action", Some(parent_id))
            if nodes
                .get(&parent_id)
                .map(|parent| parent.menu_type.as_str())
                != Some("page") =>
        {
            return Err(CatalogError::InvalidTree);
        }
        ("action", None) => return Err(CatalogError::InvalidTree),
        ("page", Some(parent_id))
            if nodes
                .get(&parent_id)
                .map(|parent| parent.menu_type.as_str())
                != Some("directory") =>
        {
            return Err(CatalogError::InvalidTree);
        }
        ("directory", Some(parent_id))
            if nodes
                .get(&parent_id)
                .map(|parent| parent.menu_type.as_str())
                != Some("directory") =>
        {
            return Err(CatalogError::InvalidTree);
        }
        _ => {}
    }
    Ok(())
}

fn node_is_effectively_enabled(node_id: i64, nodes: &HashMap<i64, AccessNode>) -> bool {
    let mut current = nodes.get(&node_id);
    let mut visited = HashSet::new();
    while let Some(node) = current {
        if node.status != "enabled" || !visited.insert(node.id) {
            return false;
        }
        current = node.parent_id.and_then(|parent_id| nodes.get(&parent_id));
        if node.parent_id.is_some() && current.is_none() {
            return false;
        }
    }
    true
}

fn normalize_method(method: &str) -> Result<String, CatalogError> {
    let method = method.trim().to_ascii_uppercase();
    if method.is_empty() || !method.chars().all(|ch| ch.is_ascii_uppercase()) {
        return Err(CatalogError::InvalidBinding);
    }
    Ok(method)
}

fn normalize_path(path: &str) -> Result<String, CatalogError> {
    let path = path.trim();
    if !path.starts_with("/api") || path.contains('?') || path.contains('#') {
        return Err(CatalogError::InvalidBinding);
    }
    let normalized = path.trim_end_matches('/');
    Ok(if normalized.is_empty() {
        "/api".to_string()
    } else {
        normalized.to_string()
    })
}

fn is_dynamic_segment(segment: &str) -> bool {
    segment.len() > 2 && segment.starts_with('{') && segment.ends_with('}')
}

fn path_is_dynamic(path: &str) -> bool {
    path.trim_matches('/').split('/').any(is_dynamic_segment)
}

fn route_shape(path: &str) -> String {
    path.trim_matches('/')
        .split('/')
        .map(|segment| {
            if is_dynamic_segment(segment) {
                "{}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn path_pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern_parts = pattern.trim_matches('/').split('/').collect::<Vec<_>>();
    let path_parts = path.trim_matches('/').split('/').collect::<Vec<_>>();
    pattern_parts.len() == path_parts.len()
        && pattern_parts
            .iter()
            .zip(path_parts)
            .all(|(left, right)| is_dynamic_segment(left) || *left == right)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::{AccessBinding, AccessCatalog, AccessNode, CatalogError};

    fn catalog(bindings: Vec<AccessBinding>) -> Result<AccessCatalog, CatalogError> {
        AccessCatalog::build(
            bindings,
            HashSet::new(),
            HashSet::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
        )
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
    fn rejects_duplicate_dynamic_route_shapes() {
        let result = catalog(vec![
            binding(10, "GET", "/api/users/{id}"),
            binding(11, "GET", "/api/users/{user_id}"),
        ]);

        assert_eq!(result, Err(CatalogError::ConflictingBinding));
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
        assert!(catalog.enabled_menu_ids.is_empty());
        assert!(catalog.enabled_permissions().is_empty());
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

        assert_eq!(result, Err(CatalogError::InvalidTree));
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
