#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("audit storage operation failed")]
    Database(#[from] sqlx::Error),
    #[error("audit change serialization failed")]
    Serialization(#[from] serde_json::Error),
}
