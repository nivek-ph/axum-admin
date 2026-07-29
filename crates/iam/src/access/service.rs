use std::{collections::BTreeSet, sync::Arc};

use sqlx::PgPool;

use super::{
    AccessEvaluationError,
    catalog::{AccessCatalog, CatalogError, PermissionCatalogEntry},
};
use crate::{access::scope::ResolvedDataScope, authorization::Authorization};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessContext {
    user_id: i64,
    active_role_ids: Vec<i64>,
    data_scope: ResolvedDataScope,
}

impl AccessContext {
    pub fn data_scope(&self) -> ResolvedDataScope {
        self.data_scope.clone()
    }
}

#[derive(Clone)]
pub struct AccessService {
    pool: PgPool,
    catalog: Arc<AccessCatalog>,
    authorization: Authorization,
}

impl AccessService {
    pub(crate) fn from_catalog(
        pool: PgPool,
        authorization: Authorization,
        catalog: Arc<AccessCatalog>,
    ) -> Self {
        Self {
            authorization,
            pool,
            catalog,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_catalog(
        pool: PgPool,
        authorization: Authorization,
        catalog: AccessCatalog,
    ) -> Self {
        Self {
            authorization,
            pool,
            catalog: Arc::new(catalog),
        }
    }

    pub async fn context(&self, user_id: i64) -> Result<AccessContext, AccessEvaluationError> {
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
        Ok(AccessContext {
            user_id,
            active_role_ids,
            data_scope,
        })
    }

    pub async fn enforce(
        &self,
        context: &AccessContext,
        permission: &str,
    ) -> Result<bool, AccessEvaluationError> {
        Ok(self
            .authorization
            .enforce_with_active_roles(context.user_id, permission, &context.active_role_ids)
            .await?)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn disabled_user_is_rejected_before_access_is_evaluated(pool: PgPool) {
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
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let (access, _) = crate::load_access_and_menus(pool.clone(), authorization)
            .await
            .unwrap();

        sqlx::query("update sys_users set enable = false where id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(matches!(
            access.context(user_id).await,
            Err(AccessEvaluationError::UserDisabled)
        ));
    }
}
