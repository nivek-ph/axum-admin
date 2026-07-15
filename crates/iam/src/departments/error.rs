use crate::access::AccessPropagationError;

#[derive(Debug, thiserror::Error)]
pub enum DeptError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("invalid department parent")]
    InvalidParent,
    #[error(transparent)]
    AccessPropagation(#[from] AccessPropagationError),
}
