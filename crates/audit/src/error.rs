#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("audit time range must use RFC 3339 timestamps")]
    InvalidTimeRange(#[source] time::error::Parse),
    #[error("audit storage operation failed")]
    Database(#[from] sqlx::Error),
    #[error("audit change serialization failed")]
    Serialization(#[from] serde_json::Error),
}
