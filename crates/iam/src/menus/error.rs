use crate::access::AccessEvaluationError;

#[derive(Debug, thiserror::Error)]
pub enum MenuError {
    #[error("menu not found")]
    NotFound,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    AccessEvaluation(#[from] AccessEvaluationError),
    #[error("invalid menu payload")]
    InvalidPayload,
}
