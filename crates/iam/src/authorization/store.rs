use std::collections::{BTreeSet, HashMap};

use sqlx::{PgConnection, PgPool};

use super::{AuthorizationError, PolicyAdministrationError};
use crate::access::ResolvedDataScope;

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

    pub(super) async fn validate_policy_invariants(&self) -> Result<(), AuthorizationError> {
        let invalid = sqlx::query_scalar::<_, bool>(
            r#"
            select exists(
                select 1
                from casbin_rule
                where not (
                    (
                        ptype = 'p'
                        and v0 ~ '^(user|role):[1-9][0-9]*$'
                        and v1 <> ''
                        and v2 = '' and v3 = '' and v4 = '' and v5 = ''
                    )
                    or (
                        ptype = 'g'
                        and v0 ~ '^user:[1-9][0-9]*$'
                        and v1 ~ '^role:[1-9][0-9]*$'
                        and v2 = '' and v3 = '' and v4 = '' and v5 = ''
                    )
                )
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        if invalid {
            return Err(AuthorizationError::Configuration(
                "persisted policy shape is invalid".to_string(),
            ));
        }
        let system_roles =
            sqlx::query_scalar::<_, i64>("select count(*) from sys_roles where is_system")
                .fetch_one(&self.pool)
                .await?;
        if system_roles != 1 {
            return Err(AuthorizationError::Configuration(
                "exactly one system role is required".to_string(),
            ));
        }
        let invalid_reference = sqlx::query_scalar::<_, bool>(
            r#"
            select exists(
                select 1
                from casbin_rule policy
                where
                    (
                        policy.ptype = 'p'
                        and policy.v0 like 'role:%'
                        and not exists (
                            select 1 from sys_roles role
                            where role.id = split_part(policy.v0, ':', 2)::bigint
                        )
                    )
                    or (
                        policy.ptype = 'p'
                        and policy.v0 like 'user:%'
                        and not exists (
                            select 1 from sys_users account
                            where account.id = split_part(policy.v0, ':', 2)::bigint
                        )
                    )
                    or (
                        policy.ptype = 'p'
                        and policy.v1 = '*'
                        and (
                            policy.v0 not like 'role:%'
                            or not exists (
                                select 1 from sys_roles role
                                where role.id = split_part(policy.v0, ':', 2)::bigint
                                  and role.is_system
                            )
                        )
                    )
                    or (
                        policy.ptype = 'p'
                        and policy.v1 <> '*'
                        and not exists (
                            select 1 from sys_menus menu
                            where menu.permission = policy.v1
                        )
                    )
                    or (
                        policy.ptype = 'g'
                        and (
                            not exists (
                                select 1 from sys_users account
                                where account.id = split_part(policy.v0, ':', 2)::bigint
                            )
                            or not exists (
                                select 1 from sys_roles role
                                where role.id = split_part(policy.v1, ':', 2)::bigint
                            )
                        )
                    )
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        if invalid_reference {
            return Err(AuthorizationError::Configuration(
                "persisted policy references an invalid subject".to_string(),
            ));
        }
        let system_wildcards = sqlx::query_scalar::<_, i64>(
            r#"
            select count(*)
            from casbin_rule policy
            join sys_roles role
              on policy.v0 = 'role:' || role.id::text
             and role.is_system
            where policy.ptype = 'p' and policy.v1 = '*'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        if system_wildcards != 1 {
            return Err(AuthorizationError::Configuration(
                "the system role must have exactly one wildcard policy".to_string(),
            ));
        }
        Ok(())
    }

    pub(super) async fn role_is_system(&self, role_id: i64) -> Result<Option<bool>, sqlx::Error> {
        sqlx::query_scalar("select is_system from sys_roles where id = $1")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub(super) async fn known_permissions(
        &self,
        permissions: &BTreeSet<String>,
    ) -> Result<bool, sqlx::Error> {
        let concrete = permissions.iter().cloned().collect::<Vec<_>>();
        let count = sqlx::query_scalar::<_, i64>(
            "select count(*) from sys_menus where permission = any($1)",
        )
        .bind(&concrete)
        .fetch_one(&self.pool)
        .await?;
        Ok(count == concrete.len() as i64)
    }

    pub(super) async fn enabled_permissions(&self) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            with recursive menu_tree as (
                select id, status = 'enabled' as enabled
                from sys_menus
                where parent_id is null

                union all

                select child.id, parent.enabled and child.status = 'enabled'
                from sys_menus child
                join menu_tree parent on child.parent_id = parent.id
            )
            select distinct menu.permission
            from menu_tree
            join sys_menus menu on menu.id = menu_tree.id
            where menu_tree.enabled and menu.permission is not null
            order by menu.permission
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn user_exists(&self, user_id: i64) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("select exists(select 1 from sys_users where id = $1)")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
    }

    pub(super) async fn role_exists(&self, role_id: i64) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("select exists(select 1 from sys_roles where id = $1)")
            .bind(role_id)
            .fetch_one(&self.pool)
            .await
    }

    pub(super) async fn role_permissions(&self, role_id: i64) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            select v1
            from casbin_rule
            where ptype = 'p' and v0 = $1
            order by v1
            "#,
        )
        .bind(role_subject(role_id))
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn user_role_ids(&self, user_id: i64) -> Result<Vec<i64>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            select split_part(v1, ':', 2)::bigint
            from casbin_rule
            where ptype = 'g' and v0 = $1 and v1 like 'role:%'
            order by split_part(v1, ':', 2)::bigint
            "#,
        )
        .bind(user_subject(user_id))
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn active_user_role_ids_for(
        &self,
        user_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<i64>>, sqlx::Error> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query_as::<_, (i64, i64)>(
            r#"
            select split_part(membership.v0, ':', 2)::bigint as user_id,
                   split_part(membership.v1, ':', 2)::bigint as role_id
            from casbin_rule membership
            join sys_roles role
              on membership.v1 = 'role:' || role.id::text
             and role.status = 'enabled'
            where membership.ptype = 'g'
              and membership.v0 like 'user:%'
              and split_part(membership.v0, ':', 2)::bigint = any($1)
            order by user_id, role_id
            "#,
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await?;
        let mut memberships = HashMap::<i64, Vec<i64>>::new();
        for (user_id, role_id) in rows {
            memberships.entry(user_id).or_default().push(role_id);
        }
        Ok(memberships)
    }

    pub(super) async fn effective_permissions_for(
        &self,
        user_id: i64,
        active_role_ids: &[i64],
    ) -> Result<BTreeSet<String>, sqlx::Error> {
        let subjects = std::iter::once(user_subject(user_id))
            .chain(active_role_ids.iter().copied().map(role_subject))
            .collect::<Vec<_>>();
        Ok(sqlx::query_scalar::<_, String>(
            r#"
            select distinct policy.v1
            from casbin_rule policy
            where policy.ptype = 'p'
              and policy.v0 = any($1)
            order by policy.v1
            "#,
        )
        .bind(subjects)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect())
    }

    pub(super) async fn role_user_ids(&self, role_id: i64) -> Result<Vec<i64>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            select split_part(v0, ':', 2)::bigint
            from casbin_rule
            where ptype = 'g' and v1 = $1 and v0 like 'user:%'
            order by split_part(v0, ':', 2)::bigint
            "#,
        )
        .bind(role_subject(role_id))
        .fetch_all(&self.pool)
        .await
    }
}

pub(super) async fn role_is_system_for_update(
    connection: &mut PgConnection,
    role_id: i64,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar("select is_system from sys_roles where id = $1 for update")
        .bind(role_id)
        .fetch_optional(connection)
        .await
}

pub(super) async fn known_permissions_in(
    connection: &mut PgConnection,
    permissions: &BTreeSet<String>,
) -> Result<bool, sqlx::Error> {
    let concrete = permissions.iter().cloned().collect::<Vec<_>>();
    let count =
        sqlx::query_scalar::<_, i64>("select count(*) from sys_menus where permission = any($1)")
            .bind(&concrete)
            .fetch_one(connection)
            .await?;
    Ok(count == concrete.len() as i64)
}

pub(super) async fn users_exist_in(
    connection: &mut PgConnection,
    user_ids: &[i64],
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>("select count(*) from sys_users where id = any($1)")
        .bind(user_ids)
        .fetch_one(connection)
        .await?;
    Ok(count == user_ids.len() as i64)
}

pub(super) async fn ensure_user_in_scope(
    connection: &mut PgConnection,
    user_id: i64,
    data_scope: &ResolvedDataScope,
) -> Result<(), PolicyAdministrationError> {
    let visible = match data_scope {
        ResolvedDataScope::All => lock_visible_user(connection, user_id, None).await?,
        ResolvedDataScope::Owner(owner_id) if *owner_id == user_id => {
            lock_visible_user(connection, user_id, None).await?
        }
        ResolvedDataScope::Owner(_) => false,
        ResolvedDataScope::DeptIds(dept_ids) if dept_ids.is_empty() => false,
        ResolvedDataScope::DeptIds(dept_ids) => {
            lock_visible_user(connection, user_id, Some(dept_ids)).await?
        }
    };
    if visible {
        Ok(())
    } else {
        Err(PolicyAdministrationError::UserNotFound)
    }
}

async fn lock_visible_user(
    connection: &mut PgConnection,
    user_id: i64,
    dept_ids: Option<&[i64]>,
) -> Result<bool, sqlx::Error> {
    let user = if let Some(dept_ids) = dept_ids {
        sqlx::query_scalar::<_, i64>(
            "select id from sys_users where id = $1 and dept_id = any($2) for update",
        )
        .bind(user_id)
        .bind(dept_ids)
        .fetch_optional(&mut *connection)
        .await?
    } else {
        sqlx::query_scalar::<_, i64>("select id from sys_users where id = $1 for update")
            .bind(user_id)
            .fetch_optional(&mut *connection)
            .await?
    };
    Ok(user.is_some())
}

pub(super) async fn roles_are_assignable_in(
    connection: &mut PgConnection,
    actor_user_id: i64,
    role_ids: &[i64],
) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query_as::<_, (i64, bool)>(
        "select id, is_system from sys_roles where id = any($1) and status = 'enabled' order by id for update",
    )
    .bind(role_ids)
    .fetch_all(&mut *connection)
    .await?;
    if rows.len() != role_ids.len() {
        return Ok(false);
    }
    if !rows.iter().any(|(_, is_system)| *is_system) {
        return Ok(true);
    }
    sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
            select 1
            from casbin_rule membership
            join sys_roles role
              on membership.v1 = 'role:' || role.id::text
             and role.status = 'enabled'
             and role.is_system
            where membership.ptype = 'g'
              and membership.v0 = $1
        )
        "#,
    )
    .bind(user_subject(actor_user_id))
    .fetch_one(connection)
    .await
}

pub(super) async fn replace_role_permissions_in(
    connection: &mut PgConnection,
    role_id: i64,
    permissions: BTreeSet<String>,
) -> Result<(), sqlx::Error> {
    lock_role(connection, role_id).await?;
    let subject = role_subject(role_id);
    sqlx::query("delete from casbin_rule where ptype = 'p' and v0 = $1")
        .bind(&subject)
        .execute(&mut *connection)
        .await?;
    for permission in permissions {
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('p', $1, $2, '', '', '', '')",
        )
        .bind(&subject)
        .bind(permission)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

pub(super) async fn replace_user_roles_in(
    connection: &mut PgConnection,
    user_id: i64,
    role_ids: BTreeSet<i64>,
) -> Result<Vec<i64>, sqlx::Error> {
    lock_roles(connection, &role_ids).await?;
    lock_user(connection, user_id).await?;
    let user = user_subject(user_id);
    let previous = sqlx::query_scalar(
        r#"
        select split_part(v1, ':', 2)::bigint
        from casbin_rule
        where ptype = 'g' and v0 = $1
        order by split_part(v1, ':', 2)::bigint
        "#,
    )
    .bind(&user)
    .fetch_all(&mut *connection)
    .await?;
    sqlx::query("delete from casbin_rule where ptype = 'g' and v0 = $1")
        .bind(&user)
        .execute(&mut *connection)
        .await?;
    for role_id in role_ids {
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', $1, $2, '', '', '', '')",
        )
        .bind(&user)
        .bind(role_subject(role_id))
        .execute(&mut *connection)
        .await?;
    }
    Ok(previous)
}

pub(super) async fn replace_role_users_in(
    connection: &mut PgConnection,
    role_id: i64,
    user_ids: &[i64],
) -> Result<(), sqlx::Error> {
    lock_role(connection, role_id).await?;
    lock_users(connection, user_ids).await?;
    let role = role_subject(role_id);
    sqlx::query("delete from casbin_rule where ptype = 'g' and v1 = $1")
        .bind(&role)
        .execute(&mut *connection)
        .await?;
    for user_id in user_ids {
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', $1, $2, '', '', '', '')",
        )
        .bind(user_subject(*user_id))
        .bind(&role)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

pub(super) async fn remove_user_in(
    connection: &mut PgConnection,
    user_id: i64,
) -> Result<(), sqlx::Error> {
    lock_user(connection, user_id).await?;
    let subject = user_subject(user_id);
    sqlx::query(
        "delete from casbin_rule where (ptype = 'p' and v0 = $1) or (ptype = 'g' and v0 = $1)",
    )
    .bind(subject)
    .execute(connection)
    .await?;
    Ok(())
}

pub(super) async fn remove_role_in(
    connection: &mut PgConnection,
    role_id: i64,
) -> Result<(), sqlx::Error> {
    lock_role(connection, role_id).await?;
    let subject = role_subject(role_id);
    sqlx::query(
        "delete from casbin_rule where (ptype = 'p' and v0 = $1) or (ptype = 'g' and v1 = $1)",
    )
    .bind(subject)
    .execute(connection)
    .await?;
    Ok(())
}

pub(super) fn normalize_ids(ids: &[i64]) -> Vec<i64> {
    ids.iter()
        .copied()
        .filter(|id| *id > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn user_subject(user_id: i64) -> String {
    format!("user:{user_id}")
}

pub(super) fn role_subject(role_id: i64) -> String {
    format!("role:{role_id}")
}

pub(super) async fn lock_policy_table(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("lock table casbin_rule in share row exclusive mode")
        .execute(connection)
        .await?;
    Ok(())
}

async fn lock_role(connection: &mut PgConnection, role_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("select id from sys_roles where id = $1 for update")
        .bind(role_id)
        .fetch_one(connection)
        .await?;
    Ok(())
}

async fn lock_roles(
    connection: &mut PgConnection,
    role_ids: &BTreeSet<i64>,
) -> Result<(), sqlx::Error> {
    let role_ids = role_ids.iter().copied().collect::<Vec<_>>();
    let locked = sqlx::query_scalar::<_, i64>(
        "select id from sys_roles where id = any($1) order by id for update",
    )
    .bind(&role_ids)
    .fetch_all(connection)
    .await?;
    if locked.len() == role_ids.len() {
        Ok(())
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

async fn lock_user(connection: &mut PgConnection, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("select id from sys_users where id = $1 for update")
        .bind(user_id)
        .fetch_one(connection)
        .await?;
    Ok(())
}

async fn lock_users(connection: &mut PgConnection, user_ids: &[i64]) -> Result<(), sqlx::Error> {
    let locked = sqlx::query_scalar::<_, i64>(
        "select id from sys_users where id = any($1) order by id for update",
    )
    .bind(user_ids)
    .fetch_all(connection)
    .await?;
    if locked.len() == user_ids.len() {
        Ok(())
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}
