use crate::authorization::AuthorizationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CatalogError {
    #[error("access catalog contains an invalid menu tree")]
    InvalidTree,
}

#[derive(Debug, thiserror::Error)]
pub enum AccessEvaluationError {
    #[error("authorization policy evaluation failed")]
    Authorization(#[from] AuthorizationError),
    #[error("authorization database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("authorization user does not exist")]
    UserNotFound,
    #[error("authorization user is disabled")]
    UserDisabled,
    #[error("request permission is denied")]
    PermissionDenied,
}
