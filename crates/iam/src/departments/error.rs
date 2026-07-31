#[derive(Debug, thiserror::Error)]
pub enum DeptError {
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("invalid department parent")]
    InvalidParent,
    #[error("department has {descendant_count} descendant departments")]
    HasDescendants { descendant_count: i64 },
}
