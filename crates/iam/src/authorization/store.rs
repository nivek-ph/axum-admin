use std::collections::{BTreeMap, BTreeSet, HashMap};

use sqlx::{PgConnection, PgPool};

use super::{AuthorizationError, EffectivePermissionGrant, EffectiveRoleGrant};

const SUPER_ADMIN_CODE: &str = "super_admin";

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
        let invalid_shape = sqlx::query_scalar::<_, bool>(
            r#"
            select exists(
                select 1
                from casbin_rule
                where not (
                    (
                        ptype = 'p'
                        and v0 ~ '^(user|role):[1-9][0-9]*$'
                        and v1 <> '' and v1 <> '*'
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
        if invalid_shape {
            return Err(AuthorizationError::Configuration(
                "persisted policy shape is invalid".to_string(),
            ));
        }

        let protected_roles = sqlx::query_scalar::<_, i64>(
            "select count(*) from sys_roles where code = 'super_admin' and status = 'enabled'",
        )
        .fetch_one(&self.pool)
        .await?;
        if protected_roles != 1 {
            return Err(AuthorizationError::Configuration(
                "one enabled super_admin role is required".to_string(),
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
                            where policy.v0 = 'role:' || role.id::text
                        )
                    )
                    or (
                        policy.ptype = 'p'
                        and policy.v0 like 'user:%'
                        and not exists (
                            select 1 from sys_users account
                            where policy.v0 = 'user:' || account.id::text
                        )
                    )
                    or (
                        policy.ptype = 'p'
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
                                where policy.v0 = 'user:' || account.id::text
                            )
                            or not exists (
                                select 1 from sys_roles role
                                where policy.v1 = 'role:' || role.id::text
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
                "persisted policy references an invalid subject or Permission".to_string(),
            ));
        }
        Ok(())
    }

    pub(super) async fn known_permissions(
        &self,
        permissions: &BTreeSet<String>,
    ) -> Result<bool, sqlx::Error> {
        known_permissions_with(&self.pool, permissions).await
    }

    pub(super) async fn user_exists(&self, user_id: i64) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("select exists(select 1 from sys_users where id = $1)")
            .bind(user_id)
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

    pub(super) async fn user_permissions(&self, user_id: i64) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            select v1
            from casbin_rule
            where ptype = 'p' and v0 = $1
            order by v1
            "#,
        )
        .bind(user_subject(user_id))
        .fetch_all(&self.pool)
        .await
    }

    pub(super) async fn user_role_ids(&self, user_id: i64) -> Result<Vec<i64>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            select role.id
            from casbin_rule membership
            join sys_roles role on membership.v1 = 'role:' || role.id::text
            where membership.ptype = 'g' and membership.v0 = $1
            order by role.id
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
            select account.id, role.id
            from sys_users account
            join casbin_rule membership
              on membership.ptype = 'g'
             and membership.v0 = 'user:' || account.id::text
            join sys_roles role
              on membership.v1 = 'role:' || role.id::text
             and role.status = 'enabled'
            where account.id = any($1)
            order by account.id, role.id
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
            where policy.ptype = 'p' and policy.v0 = any($1)
            order by policy.v1
            "#,
        )
        .bind(subjects)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect())
    }

    pub(super) async fn effective_permission_grants(
        &self,
        user_id: i64,
        active_role_ids: &[i64],
    ) -> Result<Vec<EffectivePermissionGrant>, sqlx::Error> {
        let user = user_subject(user_id);
        let rows =
            sqlx::query_as::<_, (String, Option<i64>, Option<String>, Option<String>, bool)>(
                r#"
                select policy.v1,
                       role.id,
                       role.code,
                       role.name,
                       policy.v0 = $1 as direct
                from casbin_rule policy
                left join sys_roles role on policy.v0 = 'role:' || role.id::text
                where policy.ptype = 'p'
                  and (policy.v0 = $1 or role.id = any($2))
                order by policy.v1, direct desc, role.sort, role.id
                "#,
            )
            .bind(user)
            .bind(active_role_ids)
            .fetch_all(&self.pool)
            .await?;
        let mut by_permission =
            BTreeMap::<String, (bool, BTreeMap<i64, EffectiveRoleGrant>)>::new();
        for (permission, role_id, role_code, role_name, direct) in rows {
            let entry = by_permission.entry(permission).or_default();
            entry.0 |= direct;
            if let (Some(id), Some(code), Some(name)) = (role_id, role_code, role_name) {
                entry.1.insert(id, EffectiveRoleGrant { id, code, name });
            }
        }
        Ok(by_permission
            .into_iter()
            .map(|(permission, (direct, roles))| EffectivePermissionGrant {
                permission,
                direct,
                roles: roles.into_values().collect(),
            })
            .collect())
    }
}

async fn known_permissions_with<'e, E>(
    executor: E,
    permissions: &BTreeSet<String>,
) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let concrete = permissions.iter().cloned().collect::<Vec<_>>();
    let count =
        sqlx::query_scalar::<_, i64>("select count(*) from sys_menus where permission = any($1)")
            .bind(&concrete)
            .fetch_one(executor)
            .await?;
    Ok(count == concrete.len() as i64)
}

pub(super) async fn known_permissions_in(
    connection: &mut PgConnection,
    permissions: &BTreeSet<String>,
) -> Result<bool, sqlx::Error> {
    known_permissions_with(connection, permissions).await
}

pub(super) async fn protected_role_for_update(
    connection: &mut PgConnection,
    role_id: i64,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar("select code = 'super_admin' from sys_roles where id = $1 for update")
        .bind(role_id)
        .fetch_optional(connection)
        .await
}

pub(super) async fn roles_are_assignable_in(
    connection: &mut PgConnection,
    role_ids: &[i64],
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>(
        "select count(*) from sys_roles where id = any($1) and status = 'enabled'",
    )
    .bind(role_ids)
    .fetch_one(connection)
    .await?;
    Ok(count == role_ids.len() as i64)
}

pub(super) async fn roles_exist_in(
    connection: &mut PgConnection,
    role_ids: &[i64],
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>("select count(*) from sys_roles where id = any($1)")
        .bind(role_ids)
        .fetch_one(connection)
        .await?;
    Ok(count == role_ids.len() as i64)
}

pub(super) async fn user_exists_in(
    connection: &mut PgConnection,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("select exists(select 1 from sys_users where id = $1)")
        .bind(user_id)
        .fetch_one(connection)
        .await
}

pub(super) async fn is_active_super_admin_in(
    connection: &mut PgConnection,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        select exists(
            select 1
            from sys_users account
            join casbin_rule membership
              on membership.ptype = 'g'
             and membership.v0 = 'user:' || account.id::text
            join sys_roles role
              on membership.v1 = 'role:' || role.id::text
            where account.id = $1
              and account.enable
              and role.code = 'super_admin'
              and role.status = 'enabled'
        )
        "#,
    )
    .bind(user_id)
    .fetch_one(connection)
    .await
}

pub(super) async fn active_super_admin_count_in(
    connection: &mut PgConnection,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        select count(distinct account.id)
        from sys_users account
        join casbin_rule membership
          on membership.ptype = 'g'
         and membership.v0 = 'user:' || account.id::text
        join sys_roles role
          on membership.v1 = 'role:' || role.id::text
        where account.enable
          and role.code = 'super_admin'
          and role.status = 'enabled'
        "#,
    )
    .fetch_one(connection)
    .await
}

pub(super) async fn super_admin_role_id_in(
    connection: &mut PgConnection,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("select id from sys_roles where code = $1 and status = 'enabled' for update")
        .bind(SUPER_ADMIN_CODE)
        .fetch_one(connection)
        .await
}

pub(super) async fn replace_role_permissions_in(
    connection: &mut PgConnection,
    role_id: i64,
    permissions: BTreeSet<String>,
) -> Result<(), sqlx::Error> {
    lock_role(connection, role_id).await?;
    replace_permissions_for_subject(connection, &role_subject(role_id), permissions).await
}

pub(super) async fn replace_user_permissions_in(
    connection: &mut PgConnection,
    user_id: i64,
    permissions: BTreeSet<String>,
) -> Result<Vec<String>, sqlx::Error> {
    lock_user(connection, user_id).await?;
    let subject = user_subject(user_id);
    let previous =
        sqlx::query_scalar("select v1 from casbin_rule where ptype = 'p' and v0 = $1 order by v1")
            .bind(&subject)
            .fetch_all(&mut *connection)
            .await?;
    replace_permissions_for_subject(connection, &subject, permissions).await?;
    Ok(previous)
}

async fn replace_permissions_for_subject(
    connection: &mut PgConnection,
    subject: &str,
    permissions: BTreeSet<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query("delete from casbin_rule where ptype = 'p' and v0 = $1")
        .bind(subject)
        .execute(&mut *connection)
        .await?;
    for permission in permissions {
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('p', $1, $2, '', '', '', '')",
        )
        .bind(subject)
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
        select role.id
        from casbin_rule membership
        join sys_roles role on membership.v1 = 'role:' || role.id::text
        where membership.ptype = 'g' and membership.v0 = $1
        order by role.id
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
    if role_ids.is_empty() {
        return Ok(());
    }
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
