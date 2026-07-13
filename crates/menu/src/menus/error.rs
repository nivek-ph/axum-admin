#[derive(Debug, thiserror::Error)]
pub enum MenuError {
    #[error("menu not found")]
    NotFound,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Authorization(#[from] iam::authorization::AuthorizationError),
    #[error("invalid menu payload")]
    InvalidPayload,
}
