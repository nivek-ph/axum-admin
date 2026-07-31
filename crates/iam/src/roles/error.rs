use crate::{
    access::CatalogError,
    authorization::{AuthorizationError, RolePolicyError},
};

#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("role not found")]
    NotFound,
    #[error("protected role cannot be changed")]
    Immutable,
    #[error("only an active super_admin may manage role access")]
    AccessDenied,
    #[error(transparent)]
    InvalidMenuAssignment(#[from] CatalogError),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("selected permissions are invalid")]
    InvalidPermissions,
}

impl From<RolePolicyError> for RoleError {
    fn from(error: RolePolicyError) -> Self {
        match error {
            RolePolicyError::RoleNotFound => Self::NotFound,
            RolePolicyError::RoleImmutable => Self::Immutable,
            RolePolicyError::AccessDenied => Self::AccessDenied,
            RolePolicyError::Database(source) => Self::Database(source),
            RolePolicyError::Authorization(source) => Self::Authorization(source),
        }
    }
}
