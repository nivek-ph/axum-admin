use std::{collections::BTreeSet, sync::Arc};

use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use super::{
    AccessEvaluationError, AccessInitError, AccessPropagationError,
    catalog::{AccessBinding, AccessCatalog, AccessNode, CatalogError},
};
use crate::access::scope::ResolvedDataScope;

const SUPER_ADMIN_ROLE_CODE: &str = "super_admin";
const AUTHZ_VERSION_KEY: &str = "ava:authz:version";
const AUTHZ_USER_KEY_PREFIX: &str = "ava:authz:user:";
const AUTHZ_SNAPSHOT_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessSnapshot {
    pub version: i64,
    pub user_id: i64,
    pub role_codes: BTreeSet<String>,
    pub menu_ids: BTreeSet<i64>,
    pub permissions: BTreeSet<String>,
    pub data_scope: ResolvedDataScope,
}

impl AccessSnapshot {
    pub fn is_super_admin(&self) -> bool {
        self.role_codes.contains(SUPER_ADMIN_ROLE_CODE)
    }

    pub fn allows_menu(&self, menu_id: i64) -> bool {
        self.menu_ids.contains(&menu_id)
    }
}

#[derive(Debug, FromRow)]
struct CatalogNodeRow {
    id: i64,
    parent_id: Option<i64>,
    menu_type: String,
    status: String,
    permission: Option<String>,
}

#[derive(Debug, FromRow)]
struct CatalogBindingRow {
    menu_id: i64,
    method: String,
    path_pattern: String,
}

#[derive(Debug, FromRow)]
struct GrantedMenuRow {
    id: i64,
    permission: Option<String>,
}

#[derive(Clone)]
pub struct AccessService {
    pool: PgPool,
    catalog: Arc<AccessCatalog>,
    redis: Option<MultiplexedConnection>,
}

impl AccessService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            catalog: Arc::new(AccessCatalog::new(Vec::new()).expect("empty catalog is valid")),
            redis: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_catalog(pool: PgPool, catalog: AccessCatalog) -> Self {
        Self {
            pool,
            catalog: Arc::new(catalog),
            redis: None,
        }
    }

    pub async fn load(
        pool: PgPool,
        mut redis: MultiplexedConnection,
    ) -> Result<Self, AccessInitError> {
        let catalog = Arc::new(load_catalog(&pool).await?);
        let _: bool = redis.set_nx(AUTHZ_VERSION_KEY, 1_i64).await?;
        increment_access_version(&mut redis).await?;
        let service = Self {
            pool,
            catalog,
            redis: Some(redis),
        };
        Ok(service)
    }

    pub async fn snapshot(&self, user_id: i64) -> Result<AccessSnapshot, AccessEvaluationError> {
        let Some(mut redis) = self.redis.clone() else {
            return self.load_snapshot_from_db(user_id, 0).await;
        };
        let cache_key = format!("{AUTHZ_USER_KEY_PREFIX}{user_id}");
        let (version, cached): (Option<i64>, Option<String>) = redis::cmd("MGET")
            .arg(AUTHZ_VERSION_KEY)
            .arg(&cache_key)
            .query_async(&mut redis)
            .await?;
        let version = version.unwrap_or(1);
        if let Some(cached) = cached {
            let snapshot: AccessSnapshot = serde_json::from_str(&cached)?;
            if snapshot.version == version {
                return Ok(snapshot);
            }
        }

        let snapshot = self.load_snapshot_from_db(user_id, version).await?;
        let payload = serde_json::to_string(&snapshot)?;
        let _: () = redis
            .set_ex(cache_key, payload, AUTHZ_SNAPSHOT_TTL_SECONDS)
            .await?;
        Ok(snapshot)
    }

    pub async fn bump_version(&self) -> Result<(), AccessPropagationError> {
        let Some(mut redis) = self.redis.clone() else {
            return Ok(());
        };
        increment_access_version(&mut redis).await?;
        Ok(())
    }

    pub fn required_menu(&self, method: &str, path: &str) -> Result<i64, AccessEvaluationError> {
        Ok(self.catalog.resolve(method, path)?)
    }

    pub(crate) fn validate_menu_assignment(
        &self,
        menu_ids: &BTreeSet<i64>,
    ) -> Result<(), CatalogError> {
        self.catalog
            .validate_assignment(&menu_ids.iter().copied().collect())
    }

    async fn load_snapshot_from_db(
        &self,
        user_id: i64,
        version: i64,
    ) -> Result<AccessSnapshot, AccessEvaluationError> {
        let enabled = sqlx::query_scalar::<_, bool>("SELECT enable FROM sys_users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AccessEvaluationError::UserNotFound)?;
        if !enabled {
            return Err(AccessEvaluationError::UserDisabled);
        }

        let role_codes = sqlx::query_scalar::<_, String>(
            r#"
            SELECT r.code
            FROM sys_user_roles ur
            JOIN sys_roles r ON r.id = ur.role_id
            WHERE ur.user_id = $1 AND r.status = 'enabled'
            ORDER BY r.code
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();

        let (menu_ids, permissions) = if role_codes.contains(SUPER_ADMIN_ROLE_CODE) {
            (
                self.catalog.enabled_menu_ids().iter().copied().collect(),
                self.catalog.enabled_permissions().iter().cloned().collect(),
            )
        } else {
            let rows = sqlx::query_as::<_, GrantedMenuRow>(
                r#"
                WITH RECURSIVE enabled_menus AS (
                    SELECT id, parent_id, permission
                    FROM sys_menus
                    WHERE parent_id IS NULL AND status = 'enabled'
                    UNION ALL
                    SELECT child.id, child.parent_id, child.permission
                    FROM sys_menus child
                    JOIN enabled_menus parent ON parent.id = child.parent_id
                    WHERE child.status = 'enabled'
                )
                SELECT DISTINCT menu.id, menu.permission
                FROM sys_user_roles ur
                JOIN sys_roles role ON role.id = ur.role_id AND role.status = 'enabled'
                JOIN sys_role_menus role_menu ON role_menu.role_id = role.id
                JOIN enabled_menus menu ON menu.id = role_menu.menu_id
                WHERE ur.user_id = $1
                ORDER BY menu.id
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
            (
                rows.iter().map(|row| row.id).collect(),
                rows.into_iter().filter_map(|row| row.permission).collect(),
            )
        };

        let data_scope =
            crate::access::scope::resolve_user_data_scope(&self.pool, user_id, "access").await?;
        Ok(AccessSnapshot {
            version,
            user_id,
            role_codes,
            menu_ids,
            permissions,
            data_scope,
        })
    }
}

async fn increment_access_version(
    redis: &mut MultiplexedConnection,
) -> Result<(), redis::RedisError> {
    let _: i64 = redis.incr(AUTHZ_VERSION_KEY, 1_i64).await?;
    Ok(())
}

async fn load_catalog(pool: &PgPool) -> Result<AccessCatalog, AccessInitError> {
    let nodes = sqlx::query_as::<_, CatalogNodeRow>(
        "SELECT id, parent_id, menu_type, status, permission FROM sys_menus ORDER BY id",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| AccessNode {
        id: row.id,
        parent_id: row.parent_id,
        menu_type: row.menu_type,
        status: row.status,
        permission: row.permission,
    })
    .collect();
    let bindings = sqlx::query_as::<_, CatalogBindingRow>(
        "SELECT menu_id, method, path_pattern FROM sys_menu_apis ORDER BY method, path_pattern",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| AccessBinding {
        menu_id: row.menu_id,
        method: row.method,
        path: row.path_pattern,
    })
    .collect();
    Ok(AccessCatalog::from_parts(nodes, bindings)?)
}
