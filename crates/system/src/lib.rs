pub mod api_registry;
pub mod authority;
pub mod data_scope;
pub mod depts;
pub mod dictionary;
pub mod errors;
pub mod logs;
pub mod menu;
pub mod params;
pub mod permission_apis;
pub mod permissions;
pub mod roles;
pub mod users;

#[cfg(test)]
mod tests {
    #[test]
    fn default_menu_payload_contains_dashboard_entry() {
        let menus = crate::menu::default_menus();

        assert!(!menus.is_empty());
        assert_eq!(menus[0].name, "dashboard");
        assert_eq!(menus[0].component, "view/dashboard/index.vue");
        assert_eq!(menus[0].meta.title, "Dashboard");
    }

    #[test]
    fn default_menu_payload_contains_core_admin_entries() {
        let menu_names = crate::menu::default_menus()
            .into_iter()
            .map(|menu| menu.name)
            .collect::<Vec<_>>();

        for name in [
            "users",
            "roles",
            "departments",
            "permissions",
            "api-permissions",
            "menus",
            "apis",
        ] {
            assert!(menu_names.contains(&name.to_string()));
        }
    }

    #[test]
    fn default_menus_include_button_permission_nodes() {
        let menus = crate::menu::default_menus();
        let users = menus
            .iter()
            .find(|menu| menu.name == "users")
            .expect("users menu");
        let action_permissions = users
            .children
            .iter()
            .filter_map(|child| child.permission.as_deref())
            .collect::<Vec<_>>();

        assert!(action_permissions.contains(&"system:user:list"));
        assert!(action_permissions.contains(&"system:user:create"));
        assert!(action_permissions.contains(&"system:user:update"));
        assert!(action_permissions.contains(&"system:user:delete"));
        assert!(action_permissions.contains(&"system:user:reset-password"));
    }

    #[test]
    fn default_menus_include_page_permission_nodes() {
        let menus = crate::menu::default_menus();
        let dashboard = menus
            .iter()
            .find(|menu| menu.name == "dashboard")
            .expect("dashboard menu");
        let users = menus
            .iter()
            .find(|menu| menu.name == "users")
            .expect("users menu");

        assert_eq!(
            dashboard.permission.as_deref(),
            Some("system:dashboard:page")
        );
        assert_eq!(users.permission.as_deref(), Some("system:users:page"));
    }

    #[test]
    fn default_authorities_contains_super_admin() {
        let authorities = crate::authority::default_authorities();

        assert_eq!(authorities.len(), 1);
        assert_eq!(authorities[0].authority_id, 1);
        assert_eq!(authorities[0].authority_name, "Super Admin");
    }
}
