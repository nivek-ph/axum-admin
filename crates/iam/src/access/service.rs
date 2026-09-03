use super::AccessEvaluationError;
use crate::authorization::Authorization;

#[derive(Clone)]
pub struct AccessService {
    authorization: Authorization,
}

impl AccessService {
    pub(crate) fn new(authorization: Authorization) -> Self {
        Self { authorization }
    }

    pub async fn require_active_user(&self, user_id: i64) -> Result<(), AccessEvaluationError> {
        match self.authorization.user_status(user_id).await {
            Some(true) => Ok(()),
            Some(false) => Err(AccessEvaluationError::UserDisabled),
            None => Err(AccessEvaluationError::UserNotFound),
        }
    }

    pub async fn authorize_permission(
        &self,
        user_id: i64,
        required_permission: &str,
    ) -> Result<(), AccessEvaluationError> {
        if self
            .authorization
            .authorize_permission(user_id, required_permission)
            .await?
        {
            Ok(())
        } else {
            Err(AccessEvaluationError::PermissionDenied)
        }
    }
}
