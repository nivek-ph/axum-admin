use super::catalog::CatalogError;

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
