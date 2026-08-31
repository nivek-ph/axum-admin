use std::collections::BTreeSet;

use super::{
    AccessCatalog, CatalogError,
    source::{AccessBinding, AccessNode, MenuType},
};

#[test]
fn menu_type_uses_lowercase_database_values() {
    assert_eq!(MenuType::Directory.to_string(), "directory");
    assert_eq!("page".parse::<MenuType>(), Ok(MenuType::Page));
}

fn catalog(bindings: Vec<AccessBinding>) -> Result<AccessCatalog, CatalogError> {
    let menu_ids = bindings
        .iter()
        .map(|binding| binding.menu_id)
        .collect::<BTreeSet<_>>();
    let mut nodes = vec![AccessNode {
        id: -1,
        parent_id: None,
        title: "Test directory".to_string(),
        menu_type: MenuType::Directory,
        status: "enabled".to_string(),
        permission: None,
    }];
    nodes.extend(menu_ids.into_iter().map(|id| AccessNode {
        id,
        parent_id: Some(-1),
        title: format!("Menu {id}"),
        menu_type: MenuType::Page,
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

    assert_eq!(
        catalog.required_permission("get", "/api/users/batch"),
        Ok("test:menu:11")
    );
    assert_eq!(
        catalog.required_permission("GET", "/api/users/42"),
        Ok("test:menu:10")
    );
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
        binding(10, "GET", "/api/roles/{id}/access"),
        binding(11, "GET", "/api/{area}/{id}/access"),
    ])
    .expect("matchit should accept overlapping routes");

    assert_eq!(
        catalog.required_permission("GET", "/api/roles/2/access"),
        Ok("test:menu:10")
    );
    assert_eq!(
        catalog.required_permission("GET", "/api/acme/2/access"),
        Ok("test:menu:11")
    );
}

#[test]
fn keeps_non_overlapping_dynamic_route_patterns_distinct() {
    let catalog = catalog(vec![
        binding(10, "GET", "/api/{area}/users/{id}"),
        binding(11, "GET", "/api/roles/admin/{id}"),
    ])
    .expect("non-overlapping catalog routes should be valid");

    assert_eq!(
        catalog.required_permission("GET", "/api/acme/users/42"),
        Ok("test:menu:10")
    );
    assert_eq!(
        catalog.required_permission("GET", "/api/roles/admin/42"),
        Ok("test:menu:11")
    );
}

#[test]
fn follows_matchit_template_syntax() {
    let catalog = catalog(vec![
        binding(10, "GET", "/api/files/{*rest}"),
        binding(11, "GET", "/api/files/{{name}}"),
        binding(12, "GET", "/api/files/file-{id}"),
    ])
    .expect("matchit template syntax should be accepted");

    assert_eq!(
        catalog.required_permission("GET", "/api/files/a/b"),
        Ok("test:menu:10")
    );
    assert_eq!(
        catalog.required_permission("GET", "/api/files/{name}"),
        Ok("test:menu:11")
    );
    assert_eq!(
        catalog.required_permission("GET", "/api/files/file-42"),
        Ok("test:menu:12")
    );
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
    let catalog = catalog(vec![binding(10, "GET", "/api/users")]).expect("catalog should be valid");

    assert_eq!(
        catalog.required_permission("POST", "/api/users"),
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
                menu_type: MenuType::Directory,
                status: "disabled".to_string(),
                permission: None,
            },
            AccessNode {
                id: 2,
                parent_id: Some(1),
                title: "Users".to_string(),
                menu_type: MenuType::Page,
                status: "enabled".to_string(),
                permission: Some("system:user:list".to_string()),
            },
        ],
        vec![binding(2, "GET", "/api/users")],
    )
    .expect("disabled nodes are valid catalog entries");

    assert_eq!(
        catalog.required_permission("GET", "/api/users"),
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
            menu_type: MenuType::Action,
            status: "enabled".to_string(),
            permission: Some("system:user:create".to_string()),
        }],
        vec![],
    );

    assert!(matches!(result, Err(CatalogError::InvalidTree)));
}

#[test]
fn role_access_action_selection_adds_its_page_without_adding_siblings() {
    let catalog = AccessCatalog::from_parts(
        vec![
            AccessNode {
                id: 1,
                parent_id: None,
                title: "Directory".to_string(),
                menu_type: MenuType::Directory,
                status: "enabled".to_string(),
                permission: None,
            },
            AccessNode {
                id: 2,
                parent_id: Some(1),
                title: "Users".to_string(),
                menu_type: MenuType::Page,
                status: "enabled".to_string(),
                permission: Some("users:list".to_string()),
            },
            AccessNode {
                id: 3,
                parent_id: Some(2),
                title: "Create user".to_string(),
                menu_type: MenuType::Action,
                status: "enabled".to_string(),
                permission: Some("users:create".to_string()),
            },
            AccessNode {
                id: 4,
                parent_id: Some(2),
                title: "Delete user".to_string(),
                menu_type: MenuType::Action,
                status: "enabled".to_string(),
                permission: Some("users:delete".to_string()),
            },
        ],
        vec![],
    )
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
    let catalog = AccessCatalog::from_parts(
        vec![
            AccessNode {
                id: 1,
                parent_id: None,
                title: "Directory".to_string(),
                menu_type: MenuType::Directory,
                status: "enabled".to_string(),
                permission: None,
            },
            AccessNode {
                id: 2,
                parent_id: Some(1),
                title: "Users".to_string(),
                menu_type: MenuType::Page,
                status: "enabled".to_string(),
                permission: Some("users:list".to_string()),
            },
            AccessNode {
                id: 3,
                parent_id: Some(2),
                title: "Create user".to_string(),
                menu_type: MenuType::Action,
                status: "enabled".to_string(),
                permission: Some("users:create".to_string()),
            },
        ],
        vec![],
    )
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
    let result = AccessCatalog::from_parts(
        vec![
            AccessNode {
                id: 1,
                parent_id: Some(2),
                title: "First".to_string(),
                menu_type: MenuType::Directory,
                status: "enabled".to_string(),
                permission: None,
            },
            AccessNode {
                id: 2,
                parent_id: Some(1),
                title: "Second".to_string(),
                menu_type: MenuType::Directory,
                status: "enabled".to_string(),
                permission: None,
            },
        ],
        vec![],
    );

    assert!(matches!(result, Err(CatalogError::InvalidTree)));
}
