use crate::{
    access::{AccessPropagationError, CatalogError},
    authorization::AuthorizationError,
};

#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("role not found")]
    NotFound,
    #[error("system role cannot be deleted")]
    Immutable,
    #[error(transparent)]
    AccessPropagation(#[from] AccessPropagationError),
    #[error(transparent)]
    InvalidMenuAssignment(#[from] CatalogError),
    #[error("permission assignment is invalid")]
    InvalidPermissionAssignment,
    #[error("user assignment is invalid")]
    InvalidUserAssignment,
    #[error("authorization configuration is invalid")]
    AuthorizationConfig,
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
}
