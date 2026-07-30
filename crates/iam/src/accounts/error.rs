use crate::authorization::AuthorizationError;

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("user not found")]
    NotFound,
    #[error("user already exists")]
    AlreadyExists,
    #[error("at least one enabled role is required")]
    InvalidRoles,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
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
