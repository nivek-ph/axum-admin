#[derive(Debug, thiserror::Error)]
#[error("login log storage operation failed")]
pub struct LoginLogError(#[from] sqlx::Error);
