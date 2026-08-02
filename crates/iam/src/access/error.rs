use matchit::InsertError;

use crate::authorization::AuthorizationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CatalogError {
    #[error("access catalog contains conflicting route bindings")]
    ConflictingBinding,
    #[error("request route is not bound to an access node")]
    Unbound,
    #[error("access catalog contains an invalid route binding")]
    InvalidBinding,
    #[error("access catalog contains an invalid menu tree")]
    InvalidTree,
}

impl From<InsertError> for CatalogError {
    fn from(error: InsertError) -> Self {
        match error {
            InsertError::Conflict { .. } => Self::ConflictingBinding,
            _ => Self::InvalidBinding,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AccessEvaluationError {
    #[error("authorization policy evaluation failed")]
    Authorization(#[from] AuthorizationError),
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
