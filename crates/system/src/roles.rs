use crate::errors;
use admin_httpz::AppError;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::collections::BTreeSet;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoleSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub sort: i32,
    pub data_scope: String,
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoleAssignment {
    pub user_id: i64,
    pub role_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RolePayload {
    pub code: String,
    pub name: String,
    pub status: Option<String>,
    pub sort: Option<i32>,
    #[serde(alias = "dataScope")]
    pub data_scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RolePermissionPayload {
    #[serde(rename = "permissionIds", alias = "permission_ids")]
    pub permission_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleDeptPayload {
    #[serde(rename = "deptIds", alias = "dept_ids")]
    pub dept_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleUsersPayload {
    #[serde(rename = "userIds", alias = "user_ids")]
    pub user_ids: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("role not found")]
    NotFound,
    #[error("system role cannot be deleted")]
    Immutable,
}

impl From<RoleError> for AppError {
    fn from(error: RoleError) -> Self {
        match error {
            RoleError::Database(error) => errors::roles::ROLE_DB_FAILED
                .into_error()
                .with_source(error),
            RoleError::NotFound => errors::roles::ROLE_NOT_FOUND.into(),
            RoleError::Immutable => errors::roles::ROLE_IMMUTABLE.into(),
        }
    }
}

pub async fn list(pool: &PgPool) -> Result<Vec<RoleSummary>, sqlx::Error> {
    sqlx::query_as::<_, RoleSummary>(
        r#"
        select id, code, name, status, sort, data_scope, is_system
        from sys_roles
        order by sort, id
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn ensure_builtin_roles(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
        values
            (1, 'super_admin', 'Super Admin', 'enabled', 0, 'all', true),
            (2, 'dev', 'Dev', 'enabled', 10, 'self', false),
            (3, 'ops', 'Ops', 'enabled', 20, 'all', false)
        on conflict (id) do update
        set code = excluded.code,
            name = excluded.name,
            status = excluded.status,
            sort = excluded.sort,
            data_scope = excluded.data_scope,
            is_system = excluded.is_system,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "select setval(pg_get_serial_sequence('sys_roles', 'id'), (select max(id) from sys_roles))",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn ensure_builtin_role_permissions(pool: &PgPool) -> Result<(), sqlx::Error> {
    let dev_permissions = [
        "system:dashboard:page",
        "system:users:page",
        "system:profile:page",
        "system:user:list",
        "system:user:update",
    ];
    let ops_permissions = [
        "system:dashboard:page",
        "system:params:page",
        "system:dictionaries:page",
        "system:files:page",
        "system:login-logs:page",
        "system:operation-logs:page",
        "system:profile:page",
        "system:system-config:page",
        "system:system-state:page",
        "system:param:list",
        "system:param:create",
        "system:param:get",
        "system:param:update",
        "system:param:delete",
        "system:param:get-by-key",
        "system:param:batch-delete",
        "system:dictionary:list",
        "system:dictionary:create",
        "system:dictionary:get",
        "system:dictionary:update",
        "system:dictionary:delete",
        "system:dictionary:import",
        "system:dictionary:export",
        "system:dictionary:details-tree",
        "system:dictionary-detail:create",
        "system:dictionary-detail:tree-by-type",
        "system:dictionary-detail:by-parent",
        "system:dictionary-detail:get",
        "system:dictionary-detail:update",
        "system:dictionary-detail:delete",
        "system:dictionary-detail:path",
        "system:file:list",
        "system:file:import-url",
        "system:file:upload",
        "system:file:delete",
        "system:file:rename",
        "system:login-log:list",
        "system:login-log:batch-delete",
        "system:login-log:get",
        "system:login-log:delete",
        "system:operation-log:list",
        "system:operation-log:batch-delete",
        "system:operation-log:delete",
        "system:config:get",
        "system:config:update",
        "system:state:get",
        "system:state:reload",
    ];

    sqlx::query(
        r#"
        insert into sys_role_permissions (role_id, permission_id)
        select 1, id
        from sys_permissions
        on conflict do nothing
        "#,
    )
    .execute(pool)
    .await?;

    ensure_role_permissions(pool, 2, &dev_permissions).await?;
    ensure_role_permissions(pool, 3, &ops_permissions).await?;

    Ok(())
}

async fn ensure_role_permissions(
    pool: &PgPool,
    role_id: i64,
    permission_codes: &[&str],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into sys_role_permissions (role_id, permission_id)
        select $1, id
        from sys_permissions
        where code = any($2)
        on conflict do nothing
        "#,
    )
    .bind(role_id)
    .bind(permission_codes)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find(pool: &PgPool, id: i64) -> Result<Option<RoleSummary>, sqlx::Error> {
    sqlx::query_as::<_, RoleSummary>(
        r#"
        select id, code, name, status, sort, data_scope, is_system
        from sys_roles
        where id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn create(pool: &PgPool, payload: RolePayload) -> Result<RoleSummary, RoleError> {
    let role = sqlx::query_as::<_, RoleSummary>(
        r#"
        insert into sys_roles (code, name, status, sort, data_scope)
        values ($1, $2, $3, $4, $5)
        returning id, code, name, status, sort, data_scope, is_system
        "#,
    )
    .bind(payload.code)
    .bind(payload.name)
    .bind(payload.status.unwrap_or_else(|| "enabled".to_string()))
    .bind(payload.sort.unwrap_or(0))
    .bind(payload.data_scope.unwrap_or_else(|| "all".to_string()))
    .fetch_one(pool)
    .await?;

    Ok(role)
}

pub async fn update(
    pool: &PgPool,
    id: i64,
    payload: RolePayload,
) -> Result<RoleSummary, RoleError> {
    let role = sqlx::query_as::<_, RoleSummary>(
        r#"
        update sys_roles
        set code = $1,
            name = $2,
            status = coalesce($3, status),
            sort = coalesce($4, sort),
            data_scope = coalesce($5, data_scope),
            updated_at = now()
        where id = $6
        returning id, code, name, status, sort, data_scope, is_system
        "#,
    )
    .bind(payload.code)
    .bind(payload.name)
    .bind(payload.status)
    .bind(payload.sort)
    .bind(payload.data_scope)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    role.ok_or(RoleError::NotFound)
}

pub async fn delete(pool: &PgPool, id: i64) -> Result<(), RoleError> {
    let role = find(pool, id).await?.ok_or(RoleError::NotFound)?;
    if role.is_system {
        return Err(RoleError::Immutable);
    }

    sqlx::query("delete from sys_roles where id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn permission_ids(pool: &PgPool, role_id: i64) -> Result<Vec<i64>, RoleError> {
    ensure_exists(pool, role_id).await?;
    let ids = sqlx::query_scalar(
        r#"
        select permission_id
        from sys_role_permissions
        where role_id = $1
        order by permission_id
        "#,
    )
    .bind(role_id)
    .fetch_all(pool)
    .await?;

    Ok(ids)
}

pub async fn set_permission_ids(
    pool: &PgPool,
    role_id: i64,
    permission_ids: Vec<i64>,
) -> Result<(), RoleError> {
    ensure_exists(pool, role_id).await?;

    let mut tx = pool.begin().await?;
    sqlx::query("delete from sys_role_permissions where role_id = $1")
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    for permission_id in permission_ids {
        sqlx::query(
            r#"
            insert into sys_role_permissions (role_id, permission_id)
            values ($1, $2)
            on conflict do nothing
            "#,
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn dept_ids(pool: &PgPool, role_id: i64) -> Result<Vec<i64>, RoleError> {
    ensure_exists(pool, role_id).await?;
    let ids = sqlx::query_scalar(
        r#"
        select dept_id
        from sys_role_depts
        where role_id = $1
        order by dept_id
        "#,
    )
    .bind(role_id)
    .fetch_all(pool)
    .await?;

    Ok(ids)
}

pub async fn set_dept_ids(
    pool: &PgPool,
    role_id: i64,
    dept_ids: Vec<i64>,
) -> Result<(), RoleError> {
    ensure_exists(pool, role_id).await?;

    let mut tx = pool.begin().await?;
    sqlx::query("delete from sys_role_depts where role_id = $1")
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    for dept_id in dept_ids {
        sqlx::query(
            r#"
            insert into sys_role_depts (role_id, dept_id)
            values ($1, $2)
            on conflict do nothing
            "#,
        )
        .bind(role_id)
        .bind(dept_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn user_ids(pool: &PgPool, role_id: i64) -> Result<Vec<i64>, RoleError> {
    ensure_exists(pool, role_id).await?;
    let ids = sqlx::query_scalar(
        r#"
        select user_id
        from sys_user_roles
        where role_id = $1
        order by user_id
        "#,
    )
    .bind(role_id)
    .fetch_all(pool)
    .await?;

    Ok(ids)
}

pub async fn set_user_ids(
    pool: &PgPool,
    role_id: i64,
    user_ids: Vec<i64>,
) -> Result<(), RoleError> {
    ensure_exists(pool, role_id).await?;
    let normalized = normalize_user_ids(user_ids);

    let mut tx = pool.begin().await?;
    sqlx::query("delete from sys_user_roles where role_id = $1")
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    for user_id in normalized {
        sqlx::query(
            r#"
            insert into sys_user_roles (user_id, role_id)
            values ($1, $2)
            on conflict do nothing
            "#,
        )
        .bind(user_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn user_has_role_code(
    pool: &PgPool,
    user_id: i64,
    role_code: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        select exists(
            select 1
            from sys_user_roles ur
            join sys_roles r on r.id = ur.role_id
            where ur.user_id = $1
              and r.code = $2
              and r.status = 'enabled'
        )
        "#,
    )
    .bind(user_id)
    .bind(role_code)
    .fetch_one(pool)
    .await
}

async fn ensure_exists(pool: &PgPool, role_id: i64) -> Result<(), RoleError> {
    find(pool, role_id)
        .await?
        .map(|_| ())
        .ok_or(RoleError::NotFound)
}

fn normalize_user_ids(user_ids: Vec<i64>) -> Vec<i64> {
    user_ids
        .into_iter()
        .filter(|id| *id > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_assignment_holds_multiple_roles() {
        let assignment = RoleAssignment {
            user_id: 7,
            role_ids: vec![1, 2],
        };

        assert_eq!(assignment.user_id, 7);
        assert_eq!(assignment.role_ids, vec![1, 2]);
    }

    #[test]
    fn normalize_user_ids_deduplicates_and_sorts_members() {
        assert_eq!(normalize_user_ids(vec![9, 3, 9, 1]), vec![1, 3, 9]);
    }
}
