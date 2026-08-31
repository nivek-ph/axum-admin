use std::sync::Arc;

use sqlx::PgPool;

use super::{AccessCatalog, AccessEvaluationError, catalog::normalize_request_path};
use crate::authorization::Authorization;

#[derive(Clone)]
pub struct AccessService {
    pool: PgPool,
    authorization: Authorization,
    access_catalog: Arc<AccessCatalog>,
}

impl AccessService {
    pub(crate) fn from_catalog(
        pool: PgPool,
        authorization: Authorization,
        access_catalog: Arc<AccessCatalog>,
    ) -> Self {
        Self {
            pool,
            authorization,
            access_catalog,
        }
    }

    pub async fn evaluate(
        &self,
        user_id: i64,
        method: &str,
        path: &str,
    ) -> Result<(), AccessEvaluationError> {
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
        if is_self_service_endpoint(&method, &path) {
            return Ok(());
        }
        let required_permission = self.access_catalog.required_permission(&method, &path)?;
        let active_role_ids = self.authorization.active_user_role_ids(user_id).await?;
        if self
            .authorization
            .enforce_with_active_roles(user_id, required_permission, &active_role_ids)
            .await?
        {
            Ok(())
        } else {
            Err(AccessEvaluationError::PermissionDenied { path })
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_service_routes_are_explicit() {
        assert!(is_self_service_endpoint("GET", "/api/users/me"));
        assert!(!is_self_service_endpoint("GET", "/api/users"));
    }
}
