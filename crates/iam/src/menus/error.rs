#[derive(Debug, thiserror::Error)]
pub enum MenuError {
    #[error("menu not found")]
    NotFound,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Access(#[from] crate::access::AccessError),
    #[error("invalid menu payload")]
    InvalidPayload,
}
