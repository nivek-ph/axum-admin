use std::{collections::BTreeSet, sync::Arc};

use sqlx::PgPool;

use super::catalog::{AccessCatalog, CatalogError, PermissionCatalogEntry};
use crate::{access::scope::ResolvedDataScope, authorization::Authorization};

#[derive(Debug, thiserror::Error)]
pub enum AccessEvaluationError {
    #[error("authorization policy evaluation failed")]
    Authorization(#[from] crate::authorization::AuthorizationError),
    #[error("authorization database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("authorization catalog is invalid")]
    Catalog(#[from] CatalogError),
    #[error("authorization user does not exist")]
    UserNotFound,
    #[error("authorization user is disabled")]
    UserDisabled,
    #[error("request permission is denied")]
    PermissionDenied { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessContext {
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

    pub async fn evaluate(
        &self,
        user_id: i64,
        method: &str,
        path: &str,
    ) -> Result<AccessContext, AccessEvaluationError> {
        let enabled = sqlx::query_scalar::<_, bool>("select enable from sys_users where id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AccessEvaluationError::UserNotFound)?;
        if !enabled {
            return Err(AccessEvaluationError::UserDisabled);
        }

        let method = method.trim().to_ascii_uppercase();
        let path = normalize_request_path(path);
        let required_permission = if is_self_service_endpoint(&method, &path) {
            None
        } else {
            let menu_id = self.catalog.resolve(&method, &path)?;
            Some(self.catalog.permission_for_menu(menu_id)?)
        };
        let active_role_ids = self.authorization.active_user_role_ids(user_id).await?;
        if let Some(permission) = required_permission
            && !self
                .authorization
                .enforce_with_active_roles(user_id, permission, &active_role_ids)
                .await?
        {
            return Err(AccessEvaluationError::PermissionDenied { path });
        }
        let data_scope =
            crate::access::scope::resolve_user_data_scope(&self.pool, user_id, &active_role_ids)
                .await?;
        Ok(AccessContext { data_scope })
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

    pub(crate) fn permission_catalog(
        &self,
        visible_page_ids: &BTreeSet<i64>,
        role_enabled: bool,
    ) -> Vec<PermissionCatalogEntry> {
        self.catalog
            .permission_catalog(visible_page_ids, role_enabled)
    }
}

fn is_self_service_endpoint(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("GET", "/api/users/me")
            | ("PUT", "/api/users/me")
            | ("PUT", "/api/users/me/password")
            | ("PUT", "/api/users/me/settings")
            | ("GET", "/api/menus/current")
            | ("POST", "/api/auth/logout")
    )
}

fn normalize_request_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    let normalized = if trimmed.is_empty() { "/api" } else { trimmed };
    if normalized == "/api" || normalized.starts_with("/api/") {
        normalized.to_string()
    } else if normalized.starts_with('/') {
        format!("/api{normalized}")
    } else {
        format!("/api/{normalized}")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{super::catalog::AccessBinding, *};
    use crate::access::{CatalogError, tests::AccessNode};

    fn access_service(
        pool: PgPool,
        authorization: Authorization,
        catalog: AccessCatalog,
    ) -> AccessService {
        AccessService::from_catalog(pool, authorization, Arc::new(catalog))
    }

    async fn insert_user(pool: &PgPool, user_id: i64, enabled: bool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img,
                enable, dept_id, is_system
            )
            values (
                $1, $2, $2, 'hash', 'Request Access User', '',
                $3, 1, false
            )
            "#,
        )
        .bind(user_id)
        .bind(format!("request-access-user-{user_id}"))
        .bind(enabled)
        .execute(pool)
        .await
        .unwrap();
    }

    fn protected_catalog() -> AccessCatalog {
        AccessCatalog::from_parts(
            vec![AccessNode {
                id: 2,
                parent_id: None,
                title: "Users".to_string(),
                menu_type: "page".to_string(),
                status: "enabled".to_string(),
                permission: Some("system:user:list".to_string()),
            }],
            vec![AccessBinding {
                menu_id: 2,
                method: "GET".to_string(),
                path: "/api/users".to_string(),
            }],
        )
        .unwrap()
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn self_service_evaluation_returns_the_users_data_scope(pool: PgPool) {
        let user_id = 901_231_i64;
        insert_user(&pool, user_id, true).await;
        let access = access_service(
            pool.clone(),
            Authorization::new(pool),
            AccessCatalog::from_parts(Vec::new(), Vec::new()).unwrap(),
        );

        let context = access
            .evaluate(user_id, "GET", "/api/users/me")
            .await
            .unwrap();

        assert_eq!(context.data_scope(), ResolvedDataScope::Owner(user_id));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn protected_route_evaluation_enforces_the_resolved_permission(pool: PgPool) {
        let user_id = 901_232_i64;
        insert_user(&pool, user_id, true).await;
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values ('p', $1, 'system:user:list', '', '', '', '')
            "#,
        )
        .bind(format!("user:{user_id}"))
        .execute(&pool)
        .await
        .unwrap();
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let access = access_service(pool, authorization, protected_catalog());

        let context = access.evaluate(user_id, "GET", "/api/users").await.unwrap();

        assert_eq!(context.data_scope(), ResolvedDataScope::Owner(user_id));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn protected_route_evaluation_returns_the_roles_broader_data_scope(pool: PgPool) {
        let user_id = 901_239_i64;
        let role_id = 901_239_i64;
        insert_user(&pool, user_id, true).await;
        sqlx::query(
            r#"
            insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
            values ($1, $2, 'Request Access Role', 'enabled', 10, 'all', false)
            "#,
        )
        .bind(role_id)
        .bind(format!("request-access-role-{role_id}"))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values
                ('g', $1, $2, '', '', '', ''),
                ('p', $2, 'system:user:list', '', '', '', '')
            "#,
        )
        .bind(format!("user:{user_id}"))
        .bind(format!("role:{role_id}"))
        .execute(&pool)
        .await
        .unwrap();
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let access = access_service(pool, authorization, protected_catalog());

        let context = access.evaluate(user_id, "GET", "/api/users").await.unwrap();

        assert_eq!(context.data_scope(), ResolvedDataScope::All);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn protected_route_evaluation_rejects_denied_permission(pool: PgPool) {
        let user_id = 901_233_i64;
        insert_user(&pool, user_id, true).await;
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let access = access_service(pool, authorization, protected_catalog());

        assert!(matches!(
            access.evaluate(user_id, "GET", "/api/users").await,
            Err(AccessEvaluationError::PermissionDenied { path })
                if path == "/api/users"
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn protected_route_evaluation_fails_closed_when_authorization_is_unavailable(
        pool: PgPool,
    ) {
        let user_id = 901_234_i64;
        insert_user(&pool, user_id, true).await;
        let unavailable_pool =
            PgPool::connect_lazy("postgres://postgres:postgres@127.0.0.1/ava").unwrap();
        unavailable_pool.close().await;
        let access = access_service(
            pool,
            Authorization::new(unavailable_pool),
            protected_catalog(),
        );

        assert!(matches!(
            access.evaluate(user_id, "GET", "/api/users").await,
            Err(AccessEvaluationError::Authorization(
                crate::authorization::AuthorizationError::Database(sqlx::Error::PoolClosed)
            ))
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn protected_route_evaluation_fails_closed_for_missing_binding(pool: PgPool) {
        let user_id = 901_235_i64;
        insert_user(&pool, user_id, true).await;
        let access = access_service(
            pool.clone(),
            Authorization::new(pool),
            AccessCatalog::from_parts(Vec::new(), Vec::new()).unwrap(),
        );

        assert!(matches!(
            access.evaluate(user_id, "GET", "/api/users").await,
            Err(AccessEvaluationError::Catalog(CatalogError::Unbound))
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn protected_route_evaluation_fails_closed_for_ambiguous_binding(pool: PgPool) {
        let user_id = 901_236_i64;
        insert_user(&pool, user_id, true).await;
        let catalog = AccessCatalog::from_parts(
            vec![
                AccessNode {
                    id: 2,
                    parent_id: None,
                    title: "Area settings".to_string(),
                    menu_type: "page".to_string(),
                    status: "enabled".to_string(),
                    permission: Some("system:area:settings".to_string()),
                },
                AccessNode {
                    id: 3,
                    parent_id: None,
                    title: "Admin resource".to_string(),
                    menu_type: "page".to_string(),
                    status: "enabled".to_string(),
                    permission: Some("system:admin:resource".to_string()),
                },
            ],
            vec![
                AccessBinding {
                    menu_id: 2,
                    method: "GET".to_string(),
                    path: "/api/{area}/settings".to_string(),
                },
                AccessBinding {
                    menu_id: 3,
                    method: "GET".to_string(),
                    path: "/api/admin/{resource}".to_string(),
                },
            ],
        )
        .unwrap();
        let access = access_service(pool.clone(), Authorization::new(pool), catalog);

        assert!(matches!(
            access.evaluate(user_id, "GET", "/api/admin/settings").await,
            Err(AccessEvaluationError::Catalog(CatalogError::Ambiguous))
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn missing_user_is_rejected_before_access_is_evaluated(pool: PgPool) {
        let access = access_service(
            pool.clone(),
            Authorization::new(pool),
            AccessCatalog::from_parts(Vec::new(), Vec::new()).unwrap(),
        );

        assert!(matches!(
            access.evaluate(901_237, "GET", "/api/users/me").await,
            Err(AccessEvaluationError::UserNotFound)
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn disabled_user_is_rejected_before_access_is_evaluated(pool: PgPool) {
        let user_id = 901_238_i64;
        insert_user(&pool, user_id, true).await;
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
            access.evaluate(user_id, "GET", "/api/users/me").await,
            Err(AccessEvaluationError::UserDisabled)
        ));
    }
}
