#[derive(Debug, thiserror::Error)]
#[error("parameter storage operation failed")]
pub struct ParameterError(#[from] sqlx::Error);
