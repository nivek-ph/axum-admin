use std::collections::BTreeSet;

use sqlx::PgPool;

use super::{RoleError, RolePayload, RoleSummary};
use crate::access::AccessService;

#[derive(Clone)]
pub struct RoleService {
    pool: PgPool,
    access: AccessService,
}

impl RoleService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            access: AccessService::new(pool.clone()),
            pool,
        }
    }
    pub fn with_access(pool: PgPool, access: AccessService) -> Self {
        Self { pool, access }
    }
    pub async fn list(&self) -> Result<Vec<RoleSummary>, RoleError> {
        Ok(list(&self.pool).await?)
    }
    pub async fn create(&self, p: RolePayload) -> Result<RoleSummary, RoleError> {
        let role = create(&self.pool, p).await?;
        self.changed().await?;
        Ok(role)
    }
    pub async fn update(&self, id: i64, p: RolePayload) -> Result<RoleSummary, RoleError> {
        let role = update(&self.pool, id, p).await?;
        self.changed().await?;
        Ok(role)
    }
    pub async fn delete(&self, id: i64) -> Result<(), RoleError> {
        delete(&self.pool, id).await?;
        self.changed().await
    }
    pub async fn menu_ids(&self, id: i64) -> Result<Vec<i64>, RoleError> {
        ids(&self.pool, id, "sys_role_menus", "menu_id").await
    }
    pub async fn set_menu_ids(&self, id: i64, values: Vec<i64>) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        let values = normalize(values);
        self.access
            .validate_menu_assignment(&values.iter().copied().collect())?;
        replace(&self.pool, id, "sys_role_menus", "menu_id", values).await?;
        self.changed().await
    }
    pub async fn dept_ids(&self, id: i64) -> Result<Vec<i64>, RoleError> {
        ids(&self.pool, id, "sys_role_depts", "dept_id").await
    }
    pub async fn set_dept_ids(&self, id: i64, v: Vec<i64>) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        replace(&self.pool, id, "sys_role_depts", "dept_id", normalize(v)).await?;
        self.changed().await
    }
    pub async fn user_ids(&self, id: i64) -> Result<Vec<i64>, RoleError> {
        ids(&self.pool, id, "sys_user_roles", "user_id").await
    }
    pub async fn set_user_ids(&self, id: i64, v: Vec<i64>) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        replace(&self.pool, id, "sys_user_roles", "user_id", normalize(v)).await?;
        self.changed().await
    }
    async fn changed(&self) -> Result<(), RoleError> {
        self.access.invalidate().await?;
        Ok(())
    }
}

pub(crate) async fn list(pool: &PgPool) -> Result<Vec<RoleSummary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id,code,name,status,sort,data_scope,is_system FROM sys_roles ORDER BY sort,id",
    )
    .fetch_all(pool)
    .await
}

pub(crate) async fn find(pool: &PgPool, id: i64) -> Result<Option<RoleSummary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id,code,name,status,sort,data_scope,is_system FROM sys_roles WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
async fn create(pool: &PgPool, p: RolePayload) -> Result<RoleSummary, RoleError> {
    Ok(sqlx::query_as("INSERT INTO sys_roles(code,name,status,sort,data_scope) VALUES($1,$2,$3,$4,$5) RETURNING id,code,name,status,sort,data_scope,is_system").bind(p.code).bind(p.name).bind(p.status.unwrap_or_else(||"enabled".into())).bind(p.sort.unwrap_or(0)).bind(p.data_scope.unwrap_or_else(||"self".into())).fetch_one(pool).await?)
}

async fn update(pool: &PgPool, id: i64, p: RolePayload) -> Result<RoleSummary, RoleError> {
    let current = find(pool, id).await?.ok_or(RoleError::NotFound)?;
    if current.is_system {
        return Err(RoleError::Immutable);
    }
    sqlx::query_as("UPDATE sys_roles SET name=$1,status=COALESCE($2,status),sort=COALESCE($3,sort),data_scope=COALESCE($4,data_scope),updated_at=now() WHERE id=$5 RETURNING id,code,name,status,sort,data_scope,is_system").bind(p.name).bind(p.status).bind(p.sort).bind(p.data_scope).bind(id).fetch_one(pool).await.map_err(Into::into)
}

async fn delete(pool: &PgPool, id: i64) -> Result<(), RoleError> {
    ensure_mutable(pool, id).await?;
    let used: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sys_user_roles WHERE role_id=$1)")
            .bind(id)
            .fetch_one(pool)
            .await?;
    if used {
        return Err(RoleError::InUse);
    }
    sqlx::query("DELETE FROM sys_roles WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
async fn ensure_mutable(pool: &PgPool, id: i64) -> Result<(), RoleError> {
    let r = find(pool, id).await?.ok_or(RoleError::NotFound)?;
    if r.is_system {
        Err(RoleError::Immutable)
    } else {
        Ok(())
    }
}

async fn ids(
    pool: &PgPool,
    role_id: i64,
    table: &str,
    column: &str,
) -> Result<Vec<i64>, RoleError> {
    if find(pool, role_id).await?.is_none() {
        return Err(RoleError::NotFound);
    }
    let sql = format!("SELECT {column} FROM {table} WHERE role_id=$1 ORDER BY {column}");
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(role_id)
        .fetch_all(pool)
        .await?)
}

async fn replace(
    pool: &PgPool,
    role_id: i64,
    table: &str,
    column: &str,
    values: Vec<i64>,
) -> Result<(), RoleError> {
    let mut tx = pool.begin().await?;
    let del = format!("DELETE FROM {table} WHERE role_id=$1");
    sqlx::query(sqlx::AssertSqlSafe(del))
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    for value in values {
        let sql = if column == "user_id" {
            format!("INSERT INTO {table}(user_id,role_id) VALUES($2,$1) ON CONFLICT DO NOTHING")
        } else {
            format!("INSERT INTO {table}(role_id,{column}) VALUES($1,$2) ON CONFLICT DO NOTHING")
        };
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(role_id)
            .bind(value)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn normalize(v: Vec<i64>) -> Vec<i64> {
    v.into_iter()
        .filter(|v| *v > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_ids() {
        assert_eq!(normalize(vec![3, 1, 3, 0]), vec![1, 3]);
    }
}
