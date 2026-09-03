use std::collections::BTreeSet;

use super::{AccessCatalog, AccessNode, CatalogError, MenuType};

#[test]
fn menu_type_uses_lowercase_database_values() {
    assert_eq!(MenuType::Directory.to_string(), "directory");
    assert_eq!("page".parse::<MenuType>(), Ok(MenuType::Page));
}

#[test]
fn disabled_ancestors_remove_descendant_permissions() {
    let catalog = AccessCatalog::from_nodes(vec![
        AccessNode {
            id: 1,
            parent_id: None,
            menu_type: MenuType::Directory,
            status: "disabled".to_string(),
            permission: None,
        },
        AccessNode {
            id: 2,
            parent_id: Some(1),
            menu_type: MenuType::Page,
            status: "enabled".to_string(),
            permission: Some("system:user:list".to_string()),
        },
    ])
    .expect("disabled nodes are valid catalog entries");

    assert!(catalog.enabled_permissions().is_empty());
}

#[test]
fn actions_must_have_page_parents() {
    let result = AccessCatalog::from_nodes(vec![AccessNode {
        id: 2,
        parent_id: None,
        menu_type: MenuType::Action,
        status: "enabled".to_string(),
        permission: Some("system:user:create".to_string()),
    }]);

    assert!(matches!(result, Err(CatalogError::InvalidTree)));
}

#[test]
fn role_access_action_selection_adds_its_page_without_adding_siblings() {
    let catalog = AccessCatalog::from_nodes(vec![
        AccessNode {
            id: 1,
            parent_id: None,
            menu_type: MenuType::Directory,
            status: "enabled".to_string(),
            permission: None,
        },
        AccessNode {
            id: 2,
            parent_id: Some(1),
            menu_type: MenuType::Page,
            status: "enabled".to_string(),
            permission: Some("users:list".to_string()),
        },
        AccessNode {
            id: 3,
            parent_id: Some(2),
            menu_type: MenuType::Action,
            status: "enabled".to_string(),
            permission: Some("users:create".to_string()),
        },
        AccessNode {
            id: 4,
            parent_id: Some(2),
            menu_type: MenuType::Action,
            status: "enabled".to_string(),
            permission: Some("users:delete".to_string()),
        },
    ])
    .unwrap();

    assert_eq!(
        catalog
            .normalize_role_access(["users:create".to_string()])
            .unwrap(),
        BTreeSet::from(["users:create".to_string(), "users:list".to_string()])
    );
}

#[test]
fn navigation_is_derived_only_from_selected_page_permissions() {
    let catalog = AccessCatalog::from_nodes(vec![
        AccessNode {
            id: 1,
            parent_id: None,
            menu_type: MenuType::Directory,
            status: "enabled".to_string(),
            permission: None,
        },
        AccessNode {
            id: 2,
            parent_id: Some(1),
            menu_type: MenuType::Page,
            status: "enabled".to_string(),
            permission: Some("users:list".to_string()),
        },
        AccessNode {
            id: 3,
            parent_id: Some(2),
            menu_type: MenuType::Action,
            status: "enabled".to_string(),
            permission: Some("users:create".to_string()),
        },
    ])
    .unwrap();

    assert!(
        catalog
            .navigation_menu_ids(&BTreeSet::from(["users:create".to_string()]))
            .is_empty()
    );
    assert_eq!(
        catalog.navigation_menu_ids(&BTreeSet::from(["users:list".to_string()])),
        BTreeSet::from([1, 2])
    );
}

#[test]
fn rejects_directory_cycles() {
    let result = AccessCatalog::from_nodes(vec![
        AccessNode {
            id: 1,
            parent_id: Some(2),
            menu_type: MenuType::Directory,
            status: "enabled".to_string(),
            permission: None,
        },
        AccessNode {
            id: 2,
            parent_id: Some(1),
            menu_type: MenuType::Directory,
            status: "enabled".to_string(),
            permission: None,
        },
    ]);

    assert!(matches!(result, Err(CatalogError::InvalidTree)));
}
