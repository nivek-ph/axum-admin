use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use super::{ApiBinding, MenuError, MenuMeta, MenuRecord, MenuView};
use crate::access::AccessService;

#[derive(Clone)]
pub struct MenuService {
    pool: PgPool,
    access: AccessService,
}

impl MenuService {
    pub fn new(pool: PgPool, access: AccessService) -> Self {
        Self { pool, access }
    }

    pub async fn current(&self, user_id: i64) -> Result<(Vec<MenuView>, Vec<String>), MenuError> {
        let snapshot = self.access.snapshot(user_id).await?;
        let allowed = snapshot.menu_ids.iter().copied().collect::<HashSet<_>>();
        let records = load_records(&self.pool).await?;
        Ok((
            build_tree(records, Some(&allowed), false),
            snapshot.permissions.into_iter().collect(),
        ))
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
        if record.status == "enabled"
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
}
