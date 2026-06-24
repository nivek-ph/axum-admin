pub mod api_registry;
pub mod authority;
pub mod dictionary;
pub mod errors;
pub mod logs;
pub mod menu;
pub mod params;
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

        for name in ["users", "roles", "menus", "apis"] {
            assert!(menu_names.contains(&name.to_string()));
        }
    }

    #[test]
    fn default_authorities_contains_super_admin() {
        let authorities = crate::authority::default_authorities();

        assert_eq!(authorities.len(), 1);
        assert_eq!(authorities[0].authority_id, 888);
        assert_eq!(authorities[0].authority_name, "Super Admin");
    }
}
