use sqlx::PgPool;

use super::AccessEvaluationError;
use crate::authorization::Authorization;

#[derive(Clone)]
pub struct AccessService {
    pool: PgPool,
    authorization: Authorization,
}

impl AccessService {
    pub(crate) fn new(pool: PgPool, authorization: Authorization) -> Self {
        Self {
            pool,
            authorization,
        }
    }

    pub async fn require_active_user(&self, user_id: i64) -> Result<(), AccessEvaluationError> {
        let enabled = sqlx::query_scalar::<_, bool>("select enable from sys_users where id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AccessEvaluationError::UserNotFound)?;
        if !enabled {
            return Err(AccessEvaluationError::UserDisabled);
        }
        Ok(())
    }

    pub async fn authorize_permission(
        &self,
        user_id: i64,
        required_permission: &str,
    ) -> Result<(), AccessEvaluationError> {
        let active_role_ids = self.authorization.active_user_role_ids(user_id).await?;
        if self
            .authorization
            .enforce_with_active_roles(user_id, required_permission, &active_role_ids)
            .await?
        {
            Ok(())
        } else {
            Err(AccessEvaluationError::PermissionDenied)
        }
    }
}
