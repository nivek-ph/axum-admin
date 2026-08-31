use std::collections::{BTreeSet, HashMap, HashSet};

use super::{AccessBinding, AccessNode, CatalogError, MenuType, routes::RouteBinding};

#[derive(Debug, Clone)]
pub(super) struct MenuIndex {
    nodes: HashMap<i64, AccessNode>,
    enabled_menu_ids: HashSet<i64>,
    enabled_permissions: HashSet<String>,
}

impl MenuIndex {
    pub(super) fn from_nodes(nodes: Vec<AccessNode>) -> Result<Self, CatalogError> {
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

        Ok(Self {
            nodes: node_map,
            enabled_menu_ids,
            enabled_permissions,
        })
    }

    pub(super) fn active_route_bindings(
        &self,
        bindings: Vec<AccessBinding>,
    ) -> Result<Vec<RouteBinding>, CatalogError> {
        bindings
            .into_iter()
            .filter_map(|binding| {
                let node = self.nodes.get(&binding.menu_id)?;
                if node.menu_type == MenuType::Directory {
                    return Some(Err(CatalogError::InvalidBinding));
                }
                self.enabled_menu_ids.contains(&binding.menu_id).then(|| {
                    Ok(RouteBinding {
                        method: binding.method,
                        path: binding.path,
                        permission: node
                            .permission
                            .clone()
                            .ok_or(CatalogError::InvalidBinding)?,
                    })
                })
            })
            .collect()
    }

    pub(super) fn enabled_permissions(&self) -> &HashSet<String> {
        &self.enabled_permissions
    }

    pub(super) fn normalize_role_access(
        &self,
        permissions: impl IntoIterator<Item = String>,
    ) -> Result<BTreeSet<String>, CatalogError> {
        let by_permission = self
            .nodes
            .values()
            .filter_map(|node| {
                node.permission
                    .as_ref()
                    .map(|permission| (permission, node))
            })
            .collect::<HashMap<_, _>>();
        let mut normalized = BTreeSet::new();
        for permission in permissions {
            if permission.is_empty() || permission == "*" {
                return Err(CatalogError::InvalidTree);
            }
            let node = by_permission
                .get(&permission)
                .ok_or(CatalogError::InvalidTree)?;
            if !self.enabled_menu_ids.contains(&node.id)
                || !matches!(node.menu_type, MenuType::Page | MenuType::Action)
            {
                return Err(CatalogError::InvalidTree);
            }
            normalized.insert(permission);
            if node.menu_type == MenuType::Action {
                let page = node
                    .parent_id
                    .and_then(|id| self.nodes.get(&id))
                    .ok_or(CatalogError::InvalidTree)?;
                normalized.insert(page.permission.clone().ok_or(CatalogError::InvalidTree)?);
            }
        }
        Ok(normalized)
    }

    pub(super) fn navigation_menu_ids(&self, permissions: &BTreeSet<String>) -> BTreeSet<i64> {
        let mut menu_ids = BTreeSet::new();
        for node in self.nodes.values() {
            if node.menu_type != MenuType::Page
                || !self.enabled_menu_ids.contains(&node.id)
                || !node
                    .permission
                    .as_ref()
                    .is_some_and(|permission| permissions.contains(permission))
            {
                continue;
            }
            menu_ids.insert(node.id);
            let mut parent_id = node.parent_id;
            while let Some(id) = parent_id {
                let Some(parent) = self.nodes.get(&id) else {
                    break;
                };
                if self.enabled_menu_ids.contains(&id) {
                    menu_ids.insert(id);
                }
                parent_id = parent.parent_id;
            }
        }
        menu_ids
    }
}

fn validate_node(node: &AccessNode, nodes: &HashMap<i64, AccessNode>) -> Result<(), CatalogError> {
    let valid_status = matches!(node.status.as_str(), "enabled" | "disabled");
    let valid_permission = match node.menu_type {
        MenuType::Directory => node.permission.is_none(),
        MenuType::Page | MenuType::Action => node
            .permission
            .as_ref()
            .is_some_and(|value| !value.is_empty()),
    };
    if !valid_status || !valid_permission {
        return Err(CatalogError::InvalidTree);
    }

    match (node.menu_type, node.parent_id) {
        (MenuType::Action, Some(parent_id))
            if nodes.get(&parent_id).map(|parent| parent.menu_type) != Some(MenuType::Page) =>
        {
            return Err(CatalogError::InvalidTree);
        }
        (MenuType::Action, None) => return Err(CatalogError::InvalidTree),
        (MenuType::Page, Some(parent_id))
            if nodes.get(&parent_id).map(|parent| parent.menu_type)
                != Some(MenuType::Directory) =>
        {
            return Err(CatalogError::InvalidTree);
        }
        (MenuType::Directory, Some(parent_id))
            if nodes.get(&parent_id).map(|parent| parent.menu_type)
                != Some(MenuType::Directory) =>
        {
            return Err(CatalogError::InvalidTree);
        }
        _ => {}
    }

    let mut ancestors = HashSet::new();
    let mut parent_id = node.parent_id;
    while let Some(parent) = parent_id {
        if !ancestors.insert(parent) || parent == node.id {
            return Err(CatalogError::InvalidTree);
        }
        parent_id = nodes.get(&parent).and_then(|item| item.parent_id);
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
