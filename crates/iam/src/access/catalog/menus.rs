use std::collections::{BTreeSet, HashMap, HashSet};

use super::{AccessBinding, AccessNode, CatalogError, MenuType, PermissionCatalogEntry};

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

    pub(super) fn active_bindings(
        &self,
        bindings: Vec<AccessBinding>,
    ) -> Result<Vec<AccessBinding>, CatalogError> {
        bindings
            .into_iter()
            .filter_map(|binding| {
                let node = self.nodes.get(&binding.menu_id)?;
                if node.menu_type == MenuType::Directory || node.permission.is_none() {
                    return Some(Err(CatalogError::InvalidBinding));
                }
                self.enabled_menu_ids
                    .contains(&binding.menu_id)
                    .then_some(Ok(binding))
            })
            .collect()
    }

    pub(super) fn enabled_permissions(&self) -> &HashSet<String> {
        &self.enabled_permissions
    }

    pub(super) fn permission_catalog(
        &self,
        visible_page_ids: &BTreeSet<i64>,
        role_enabled: bool,
    ) -> Vec<PermissionCatalogEntry> {
        let mut entries = self
            .nodes
            .values()
            .filter(|node| matches!(node.menu_type, MenuType::Page | MenuType::Action))
            .filter_map(|node| {
                let permission = node.permission.clone()?;
                let page = if node.menu_type == MenuType::Page {
                    node
                } else {
                    self.nodes.get(&node.parent_id?)?
                };
                Some((
                    page.id,
                    node.menu_type == MenuType::Action,
                    node.id,
                    PermissionCatalogEntry {
                        permission,
                        title: node.title.clone(),
                        menu_type: node.menu_type.to_string(),
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

    pub(super) fn page_entry_permissions(
        &self,
        configured_menu_ids: &HashSet<i64>,
    ) -> BTreeSet<String> {
        configured_menu_ids
            .iter()
            .filter_map(|menu_id| self.nodes.get(menu_id))
            .filter(|node| node.menu_type == MenuType::Page)
            .filter_map(|node| node.permission.clone())
            .collect()
    }

    pub(super) fn is_action_permission(&self, permission: &str) -> bool {
        self.nodes.values().any(|node| {
            node.menu_type == MenuType::Action && node.permission.as_deref() == Some(permission)
        })
    }

    pub(super) fn permission_for_menu(&self, menu_id: i64) -> Result<&str, CatalogError> {
        if !self.enabled_menu_ids.contains(&menu_id) {
            return Err(CatalogError::InvalidBinding);
        }
        self.nodes
            .get(&menu_id)
            .and_then(|node| node.permission.as_deref())
            .ok_or(CatalogError::InvalidBinding)
    }

    pub(super) fn validate_assignment(&self, menu_ids: &HashSet<i64>) -> Result<(), CatalogError> {
        for menu_id in menu_ids {
            let node = self.nodes.get(menu_id).ok_or(CatalogError::InvalidTree)?;
            if node.menu_type == MenuType::Action {
                return Err(CatalogError::InvalidTree);
            }
            let mut parent_id = node.parent_id;
            while let Some(parent) = parent_id {
                if !menu_ids.contains(&parent) {
                    return Err(CatalogError::InvalidTree);
                }
                parent_id = self.nodes.get(&parent).and_then(|item| item.parent_id);
            }
        }
        Ok(())
    }

    pub(super) fn effective_page_access(
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
            if node.menu_type != MenuType::Page || !self.enabled_menu_ids.contains(menu_id) {
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
