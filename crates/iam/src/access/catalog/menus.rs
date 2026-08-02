use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::OnceLock,
};

use super::{AccessBinding, AccessNode, CatalogError, PermissionCatalogEntry};

#[derive(Debug, Clone)]
pub(super) struct MenuIndex {
    nodes: HashMap<i64, AccessNode>,
    enabled_menu_ids: HashSet<i64>,
    enabled_permissions: OnceLock<HashSet<String>>,
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

        Ok(Self {
            nodes: node_map,
            enabled_menu_ids,
            enabled_permissions: OnceLock::new(),
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
                if node.menu_type == "directory" || node.permission.is_none() {
                    return Some(Err(CatalogError::InvalidBinding));
                }
                self.enabled_menu_ids
                    .contains(&binding.menu_id)
                    .then_some(Ok(binding))
            })
            .collect()
    }

    pub(super) fn enabled_permissions(&self) -> &HashSet<String> {
        self.enabled_permissions.get_or_init(|| {
            self.nodes
                .values()
                .filter(|node| self.enabled_menu_ids.contains(&node.id))
                .filter_map(|node| node.permission.clone())
                .collect()
        })
    }

    pub(super) fn permission_catalog(
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

    pub(super) fn page_entry_permissions(
        &self,
        configured_menu_ids: &HashSet<i64>,
    ) -> BTreeSet<String> {
        configured_menu_ids
            .iter()
            .filter_map(|menu_id| self.nodes.get(menu_id))
            .filter(|node| node.menu_type == "page")
            .filter_map(|node| node.permission.clone())
            .collect()
    }

    pub(super) fn is_action_permission(&self, permission: &str) -> bool {
        self.nodes.values().any(|node| {
            node.menu_type == "action" && node.permission.as_deref() == Some(permission)
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
            if node.menu_type == "action" {
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
