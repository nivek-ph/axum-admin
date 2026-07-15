use auth::password::PasswordError;

use crate::access::AccessPropagationError;

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("user not found")]
    NotFound,
    #[error("user already exists")]
    AlreadyExists,
    #[error("invalid password")]
    InvalidPassword,
    #[error("at least one enabled role is required")]
    InvalidRoles,
    #[error("{0}")]
    Password(#[from] PasswordError),
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
    #[error(transparent)]
    AccessPropagation(#[from] AccessPropagationError),
}

#[derive(Debug, thiserror::Error)]
pub enum AuthenticateError {
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("user is disabled")]
    Disabled,
    #[error("credential operation failed")]
    Credential(#[from] PasswordError),
    #[error("{0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum AuthSessionError {
    #[error("user not found")]
    UserNotFound,
    #[error("user is disabled")]
    UserDisabled,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
}
