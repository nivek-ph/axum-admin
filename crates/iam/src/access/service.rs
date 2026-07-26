use std::{collections::BTreeSet, sync::Arc};

use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use super::{
    AccessEvaluationError, AccessInitError, AccessPropagationError,
    catalog::{AccessBinding, AccessCatalog, AccessNode, CatalogError, PermissionCatalogEntry},
};
use crate::{access::scope::ResolvedDataScope, authorization::Authorization};

const ACCESS_CONTEXT_VERSION_KEY: &str = "ava:access_context:version";
const ACCESS_CONTEXT_KEY_PREFIX: &str = "ava:access_context:user:";
const ACCESS_CONTEXT_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessSnapshot {
    pub version: i64,
    pub user_id: i64,
    pub system_managed: bool,
    pub role_codes: BTreeSet<String>,
    pub menu_ids: BTreeSet<i64>,
    pub permissions: BTreeSet<String>,
    pub data_scope: ResolvedDataScope,
}

#[derive(Debug, FromRow)]
struct CatalogNodeRow {
    id: i64,
    parent_id: Option<i64>,
    title: String,
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
struct RoleRow {
    code: String,
    is_system: bool,
}

#[derive(Clone)]
pub struct AccessService {
    pool: PgPool,
    catalog: Arc<AccessCatalog>,
    authorization: Authorization,
    redis: Option<MultiplexedConnection>,
}

impl AccessService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            authorization: Authorization::new(pool.clone()),
            pool,
            catalog: Arc::new(AccessCatalog::new(Vec::new()).expect("empty catalog is valid")),
            redis: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_catalog(pool: PgPool, catalog: AccessCatalog) -> Self {
        Self {
            authorization: Authorization::new(pool.clone()),
            pool,
            catalog: Arc::new(catalog),
            redis: None,
        }
    }

    pub async fn load(
        pool: PgPool,
        authorization: Authorization,
        mut redis: MultiplexedConnection,
    ) -> Result<Self, AccessInitError> {
        let catalog = Arc::new(load_catalog(&pool).await?);
        let _: bool = redis.set_nx(ACCESS_CONTEXT_VERSION_KEY, 0_i64).await?;
        Ok(Self {
            pool,
            catalog,
            authorization,
            redis: Some(redis),
        })
    }

    pub async fn load_without_cache(pool: PgPool) -> Result<Self, AccessInitError> {
        Ok(Self {
            catalog: Arc::new(load_catalog(&pool).await?),
            authorization: Authorization::new(pool.clone()),
            pool,
            redis: None,
        })
    }

    pub async fn snapshot(&self, user_id: i64) -> Result<AccessSnapshot, AccessEvaluationError> {
        let enabled = sqlx::query_scalar::<_, bool>("select enable from sys_users where id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AccessEvaluationError::UserNotFound)?;
        if !enabled {
            return Err(AccessEvaluationError::UserDisabled);
        }
        let active_role_ids = self.authorization.active_user_role_ids(user_id).await?;
        let data_scope =
            crate::access::scope::resolve_user_data_scope(&self.pool, user_id, &active_role_ids)
                .await?;
        let Some(mut redis) = self.redis.clone() else {
            return self
                .load_snapshot(user_id, 0, active_role_ids, data_scope)
                .await;
        };
        let cache_key = format!("{ACCESS_CONTEXT_KEY_PREFIX}{user_id}");
        let (version, cached): (Option<i64>, Option<String>) = redis::cmd("MGET")
            .arg(ACCESS_CONTEXT_VERSION_KEY)
            .arg(&cache_key)
            .query_async(&mut redis)
            .await?;
        let version = version.unwrap_or(0);
        if let Some(cached) = cached {
            let mut snapshot: AccessSnapshot = serde_json::from_str(&cached)?;
            if snapshot.version == version {
                snapshot.data_scope = data_scope;
                return Ok(snapshot);
            }
        }
        let snapshot = self
            .load_snapshot(user_id, version, active_role_ids, data_scope)
            .await?;
        let payload = serde_json::to_string(&snapshot)?;
        let _: () = redis
            .set_ex(cache_key, payload, ACCESS_CONTEXT_TTL_SECONDS)
            .await?;
        Ok(snapshot)
    }

    pub async fn bump_version(&self) -> Result<(), AccessPropagationError> {
        if let Some(mut redis) = self.redis.clone() {
            let _: i64 = redis.incr(ACCESS_CONTEXT_VERSION_KEY, 1_i64).await?;
        }
        Ok(())
    }

    pub fn required_menu(&self, method: &str, path: &str) -> Result<i64, AccessEvaluationError> {
        Ok(self.catalog.resolve(method, path)?)
    }

    pub fn required_permission(
        &self,
        method: &str,
        path: &str,
    ) -> Result<&str, AccessEvaluationError> {
        Ok(self
            .catalog
            .permission_for_menu(self.required_menu(method, path)?)?)
    }

    pub(crate) fn validate_menu_assignment(
        &self,
        menu_ids: &BTreeSet<i64>,
    ) -> Result<(), CatalogError> {
        self.catalog
            .validate_assignment(&menu_ids.iter().copied().collect())
    }

    pub(crate) fn effective_role_menu_ids(
        &self,
        configured_menu_ids: &BTreeSet<i64>,
        role_enabled: bool,
        system_managed: bool,
    ) -> BTreeSet<i64> {
        if system_managed && role_enabled {
            self.catalog.system_page_access()
        } else {
            self.catalog
                .effective_page_access(&configured_menu_ids.iter().copied().collect(), role_enabled)
        }
    }

    pub(crate) fn validate_permission_assignment(
        &self,
        permissions: &BTreeSet<String>,
    ) -> Result<(), CatalogError> {
        self.catalog.validate_permission_assignment(permissions)
    }

    pub(crate) fn enabled_permissions(&self) -> BTreeSet<String> {
        self.catalog.enabled_permissions().iter().cloned().collect()
    }

    pub(crate) fn permission_catalog(
        &self,
        visible_page_ids: &BTreeSet<i64>,
        role_enabled: bool,
    ) -> Vec<PermissionCatalogEntry> {
        self.catalog
            .permission_catalog(visible_page_ids, role_enabled)
    }

    async fn load_snapshot(
        &self,
        user_id: i64,
        version: i64,
        role_ids: Vec<i64>,
        data_scope: ResolvedDataScope,
    ) -> Result<AccessSnapshot, AccessEvaluationError> {
        let roles = sqlx::query_as::<_, RoleRow>(
            r#"
            select code, is_system
            from sys_roles
            where id = any($1)
            order by code
            "#,
        )
        .bind(&role_ids)
        .fetch_all(&self.pool)
        .await?;
        let role_codes = roles.iter().map(|role| role.code.clone()).collect();
        let system_managed = roles.iter().any(|role| role.is_system);

        let menu_ids = if system_managed {
            self.catalog.system_page_access()
        } else {
            let configured = sqlx::query_scalar::<_, i64>(
                r#"
                select distinct menu_id
                from sys_role_menus
                where role_id = any($1)
                order by menu_id
                "#,
            )
            .bind(&role_ids)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect::<BTreeSet<_>>();
            self.catalog
                .effective_page_access(&configured.into_iter().collect(), true)
        };

        let permissions = if system_managed {
            self.enabled_permissions()
        } else {
            let enabled_permissions = self.enabled_permissions();
            self.authorization
                .effective_permissions(user_id)
                .await?
                .into_iter()
                .filter(|permission| enabled_permissions.contains(permission))
                .collect()
        };

        Ok(AccessSnapshot {
            version,
            user_id,
            system_managed,
            role_codes,
            menu_ids,
            permissions,
            data_scope,
        })
    }
}

async fn load_catalog(pool: &PgPool) -> Result<AccessCatalog, AccessInitError> {
    let nodes = sqlx::query_as::<_, CatalogNodeRow>(
        "select id, parent_id, title, menu_type, status, permission from sys_menus order by id",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| AccessNode {
        id: row.id,
        parent_id: row.parent_id,
        title: row.title,
        menu_type: row.menu_type,
        status: row.status,
        permission: row.permission,
    })
    .collect();
    let bindings = sqlx::query_as::<_, CatalogBindingRow>(
        "select menu_id, method, path_pattern from sys_menu_apis order by method, path_pattern",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn disabled_user_is_rejected_even_when_navigation_snapshot_is_cached(pool: PgPool) {
        let user_id = 901_234_i64;
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img,
                enable, dept_id, is_system
            )
            values ($1, 'cached-user', 'cached-user', 'hash', 'Cached User', '', true, 1, false)
            "#,
        )
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let client = redis::Client::open(redis_url).unwrap();
        let mut redis = client.get_multiplexed_async_connection().await.unwrap();
        let cache_key = format!("{ACCESS_CONTEXT_KEY_PREFIX}{user_id}");
        let _: () = redis.del(&cache_key).await.unwrap();
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let access = AccessService::load(pool.clone(), authorization, redis.clone())
            .await
            .unwrap();

        access.snapshot(user_id).await.unwrap();
        sqlx::query("update sys_users set enable = false where id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(matches!(
            access.snapshot(user_id).await,
            Err(AccessEvaluationError::UserDisabled)
        ));
        let _: () = redis.del(cache_key).await.unwrap();
    }
}
