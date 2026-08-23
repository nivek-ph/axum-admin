use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use sqlx::PgPool;

use super::{ApiBinding, MenuError, MenuMeta, MenuView, model::MenuRecord};
use crate::{access::AccessCatalog, authorization::Authorization};

#[derive(Clone)]
pub struct MenuService {
    pool: PgPool,
    authorization: Authorization,
    access_catalog: Arc<AccessCatalog>,
}

impl MenuService {
    pub(crate) fn from_catalog(
        pool: PgPool,
        authorization: Authorization,
        access_catalog: Arc<AccessCatalog>,
    ) -> Self {
        Self {
            pool,
            authorization,
            access_catalog,
        }
    }

    pub async fn current(&self, user_id: i64) -> Result<(Vec<MenuView>, Vec<String>), MenuError> {
        let active_role_ids = self.authorization.active_user_role_ids(user_id).await?;
        let effective_permissions = self
            .authorization
            .effective_permissions_for(user_id, &active_role_ids)
            .await?;
        let menu_ids = self
            .access_catalog
            .navigation_menu_ids(&effective_permissions);
        let enabled_permissions = self.access_catalog.enabled_permissions();
        let permissions = effective_permissions
            .into_iter()
            .filter(|permission| enabled_permissions.contains(permission))
            .collect::<Vec<_>>();
        let allowed = menu_ids.into_iter().collect::<HashSet<_>>();
        let records = load_records(&self.pool).await?;
        Ok((build_tree(records, Some(&allowed), false), permissions))
    }

    pub async fn tree(&self) -> Result<Vec<MenuView>, MenuError> {
        Ok(build_tree(load_records(&self.pool).await?, None, true))
    }
}

async fn load_records(pool: &PgPool) -> Result<Vec<MenuRecord>, sqlx::Error> {
    let mut records = sqlx::query_as::<_, MenuRecord>(
        r#"
        SELECT id, COALESCE(parent_id, 0) AS parent_id, path, name, hidden, component,
               sort, active_name, keep_alive, default_menu, title, icon, close_tab,
               transition_type, parameters, menu_btn, menu_type, status, permission
        FROM sys_menus
        ORDER BY sort, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let bindings = sqlx::query_as::<_, ApiBinding>(
        "SELECT menu_id, method, path_pattern FROM sys_menu_apis ORDER BY method, path_pattern",
    )
    .fetch_all(pool)
    .await?;
    let mut by_menu: HashMap<i64, Vec<ApiBinding>> = HashMap::new();
    for binding in bindings {
        by_menu.entry(binding.menu_id).or_default().push(binding);
    }
    for record in &mut records {
        record.api_bindings = by_menu.remove(&record.id).unwrap_or_default();
    }
    Ok(records)
}

fn build_tree(
    records: Vec<MenuRecord>,
    allowed: Option<&HashSet<i64>>,
    include_actions: bool,
) -> Vec<MenuView> {
    let mut children: HashMap<i64, Vec<MenuRecord>> = HashMap::new();
    for record in records {
        if (allowed.is_none() || record.status == "enabled")
            && allowed.is_none_or(|ids| ids.contains(&record.id))
            && (include_actions || record.menu_type != "action")
        {
            children.entry(record.parent_id).or_default().push(record);
        }
    }
    build_children(0, &mut children)
}

fn build_children(parent_id: i64, records: &mut HashMap<i64, Vec<MenuRecord>>) -> Vec<MenuView> {
    records
        .remove(&parent_id)
        .unwrap_or_default()
        .into_iter()
        .map(|record| {
            let id = record.id;
            MenuView {
                id,
                parent_id: record.parent_id,
                path: record.path,
                name: record.name,
                hidden: record.hidden,
                component: record.component,
                sort: record.sort,
                meta: MenuMeta {
                    active_name: record.active_name,
                    keep_alive: record.keep_alive,
                    default_menu: record.default_menu,
                    title: record.title,
                    icon: record.icon,
                    close_tab: record.close_tab,
                    transition_type: record.transition_type,
                },
                parameters: serde_json::from_value(record.parameters).unwrap_or_default(),
                menu_btn: serde_json::from_value(record.menu_btn).unwrap_or_default(),
                menu_type: record.menu_type,
                status: record.status,
                permission: record.permission,
                api_bindings: record.api_bindings,
                children: build_children(id, records),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: i64, parent_id: i64, menu_type: &str, status: &str) -> MenuRecord {
        MenuRecord {
            id,
            parent_id,
            path: String::new(),
            name: format!("n{id}"),
            hidden: false,
            component: String::new(),
            sort: id as i32,
            active_name: String::new(),
            keep_alive: false,
            default_menu: false,
            title: format!("N{id}"),
            icon: String::new(),
            close_tab: false,
            transition_type: String::new(),
            parameters: serde_json::json!([]),
            menu_btn: serde_json::json!([]),
            menu_type: menu_type.into(),
            status: status.into(),
            permission: None,
            api_bindings: Vec::new(),
        }
    }

    #[test]
    fn current_tree_excludes_actions_and_unassigned_nodes() {
        let allowed = HashSet::from([1, 2, 3]);
        let tree = build_tree(
            vec![
                record(1, 0, "directory", "enabled"),
                record(2, 1, "page", "enabled"),
                record(3, 2, "action", "enabled"),
                record(4, 1, "page", "enabled"),
            ],
            Some(&allowed),
            false,
        );
        assert_eq!(tree[0].children.len(), 1);
        assert!(tree[0].children[0].children.is_empty());
    }

    #[test]
    fn management_tree_includes_disabled_catalog_nodes() {
        let tree = build_tree(
            vec![
                record(1, 0, "directory", "enabled"),
                record(2, 1, "page", "disabled"),
                record(3, 2, "action", "disabled"),
            ],
            None,
            true,
        );

        assert_eq!(tree[0].children[0].status, "disabled");
        assert_eq!(tree[0].children[0].children[0].status, "disabled");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn current_navigation_uses_effective_page_permissions(pool: PgPool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img,
                enable, dept_id
            )
            values (9001, 'menu-user', 'menu-user', 'hash', 'Menu User', '', true, 1)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "insert into sys_roles (id, name, code, status) values (9001, 'Menu Role', 'menu-role', 'enabled')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values
                ('g', 'user:9001', 'role:9001', '', '', '', ''),
                ('p', 'role:9001', 'system:user:list', '', '', '', '')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let authorization = crate::authorization::Authorization::load(pool.clone())
            .await
            .unwrap();
        let catalog = Arc::new(AccessCatalog::load(&pool).await.unwrap());
        let menus = MenuService::from_catalog(pool, authorization, catalog);

        let (tree, permissions) = menus.current(9001).await.unwrap();

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, 10);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].id, 11);
        assert_eq!(permissions, vec!["system:user:list"]);
    }
}
