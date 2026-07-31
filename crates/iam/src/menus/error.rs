#[derive(Debug, thiserror::Error)]
pub enum MenuError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Authorization(#[from] crate::authorization::AuthorizationError),
}
