use crate::{access::CatalogError, authorization::AuthorizationError};

#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("role not found")]
    NotFound,
    #[error("system role cannot be deleted")]
    Immutable,
    #[error(transparent)]
    InvalidMenuAssignment(#[from] CatalogError),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
}
