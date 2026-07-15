#[derive(Debug, thiserror::Error)]
#[error("operation log storage operation failed")]
pub struct OperationLogError(#[from] sqlx::Error);
