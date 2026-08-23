use std::collections::{BTreeSet, HashMap, HashSet};

use sqlx::PgPool;

use super::AuthorizationError;

pub(super) const SUPER_ADMIN_CODE: &str = "super_admin";

#[derive(Debug, Clone)]
pub(super) struct RoleFact {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug)]
pub(super) struct PolicyFacts {
    pub users: HashSet<i64>,
    pub roles: HashMap<i64, RoleFact>,
    pub enabled_permissions: HashSet<String>,
    pub action_pages: HashMap<String, String>,
    pub super_admin_role_id: i64,
}

#[derive(Clone)]
pub(super) struct PolicyStore {
    pool: PgPool,
}

impl PolicyStore {
    pub(super) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub(super) async fn policy_facts(&self) -> Result<PolicyFacts, AuthorizationError> {
        let users = sqlx::query_scalar::<_, i64>("select id from sys_users")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect();
        let role_rows = sqlx::query_as::<_, (i64, String, String, String)>(
            "select id, code, name, status from sys_roles",
        )
        .fetch_all(&self.pool)
        .await?;
        let roles = role_rows
            .into_iter()
            .map(|(id, code, name, status)| {
                (
                    id,
                    RoleFact {
                        id,
                        code,
                        name,
                        status,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        let protected = roles
            .values()
            .filter(|role| role.code == SUPER_ADMIN_CODE && role.status == "enabled")
            .map(|role| role.id)
            .collect::<Vec<_>>();
        if protected.len() != 1 {
            return Err(AuthorizationError::Configuration(
                "one enabled super_admin role is required".to_string(),
            ));
        }
        let enabled_permissions = sqlx::query_scalar::<_, String>(
            r#"
            with recursive enabled_nodes as (
                select id, parent_id, permission, menu_type, status,
                       status = 'enabled' as effectively_enabled
                from sys_menus
                where parent_id is null
                union all
                select child.id, child.parent_id, child.permission, child.menu_type, child.status,
                       parent.effectively_enabled and child.status = 'enabled'
                from sys_menus child
                join enabled_nodes parent on child.parent_id = parent.id
            )
            select permission
            from enabled_nodes
            where effectively_enabled
              and menu_type in ('page', 'action')
              and permission is not null
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect();
        let action_pages = sqlx::query_as::<_, (String, String)>(
            r#"
            select action.permission, page.permission
            from sys_menus action
            join sys_menus page on page.id = action.parent_id
            where action.menu_type = 'action'
              and page.menu_type = 'page'
              and action.permission is not null
              and page.permission is not null
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect();
        Ok(PolicyFacts {
            users,
            roles,
            enabled_permissions,
            action_pages,
            super_admin_role_id: protected[0],
        })
    }

    pub(super) async fn user_exists(&self, user_id: i64) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("select exists(select 1 from sys_users where id = $1)")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
    }

    pub(super) async fn role(&self, role_id: i64) -> Result<Option<RoleFact>, sqlx::Error> {
        Ok(sqlx::query_as::<_, (i64, String, String, String)>(
            "select id, code, name, status from sys_roles where id = $1",
        )
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|(id, code, name, status)| RoleFact {
            id,
            code,
            name,
            status,
        }))
    }

    pub(super) async fn roles(
        &self,
        role_ids: &BTreeSet<i64>,
    ) -> Result<HashMap<i64, RoleFact>, sqlx::Error> {
        if role_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let ids = role_ids.iter().copied().collect::<Vec<_>>();
        Ok(sqlx::query_as::<_, (i64, String, String, String)>(
            "select id, code, name, status from sys_roles where id = any($1)",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(id, code, name, status)| {
            (
                id,
                RoleFact {
                    id,
                    code,
                    name,
                    status,
                },
            )
        })
        .collect())
    }
}

pub(super) fn normalize_ids(ids: &[i64]) -> BTreeSet<i64> {
    ids.iter().copied().filter(|id| *id > 0).collect()
}

pub(super) fn user_subject(user_id: i64) -> String {
    format!("user:{user_id}")
}

pub(super) fn role_subject(role_id: i64) -> String {
    format!("role:{role_id}")
}

pub(super) fn parse_user_subject(subject: &str) -> Option<i64> {
    parse_subject(subject, "user:")
}

pub(super) fn parse_role_subject(subject: &str) -> Option<i64> {
    parse_subject(subject, "role:")
}

fn parse_subject(subject: &str, prefix: &str) -> Option<i64> {
    let value = subject.strip_prefix(prefix)?;
    if value.starts_with('0') {
        return None;
    }
    value.parse::<i64>().ok().filter(|id| *id > 0)
}
