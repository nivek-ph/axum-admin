use crate::access::{AccessPropagationError, CatalogError};

#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("role not found")]
    NotFound,
    #[error("system role cannot be deleted")]
    Immutable,
    #[error("role is assigned to users")]
    InUse,
    #[error(transparent)]
    AccessPropagation(#[from] AccessPropagationError),
    #[error(transparent)]
    InvalidMenuAssignment(#[from] CatalogError),
}
