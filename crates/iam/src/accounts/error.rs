use crate::authorization::{AccountPolicyError, AuthorizationError};

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("user not found")]
    NotFound,
    #[error("user already exists")]
    AlreadyExists,
    #[error("selected roles are invalid")]
    InvalidRoles,
    #[error("only an active super_admin may perform this operation")]
    AccessDenied,
    #[error("the final active super_admin cannot be removed")]
    LastSuperAdmin,
    #[error("selected permissions are invalid")]
    InvalidPermissions,
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
}

impl From<AccountPolicyError> for AccountError {
    fn from(error: AccountPolicyError) -> Self {
        match error {
            AccountPolicyError::UserNotFound => Self::NotFound,
            AccountPolicyError::AccessDenied => Self::AccessDenied,
            AccountPolicyError::LastSuperAdmin => Self::LastSuperAdmin,
            AccountPolicyError::InvalidRoleAssignment => Self::InvalidRoles,
            AccountPolicyError::InvalidPermissionAssignment => Self::InvalidPermissions,
            AccountPolicyError::Database(source) => Self::Database(source),
            AccountPolicyError::Audit(source) => Self::Audit(source),
            AccountPolicyError::Authorization(source) => Self::Authorization(source),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RefreshIdentityError {
    #[error("user not found")]
    NotFound,
    #[error("user is disabled")]
    Disabled,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
}
