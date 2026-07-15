use super::catalog::CatalogError;

#[derive(Debug, thiserror::Error)]
pub enum AccessInitError {
    #[error("authorization database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("authorization cache is unavailable")]
    Cache(#[from] redis::RedisError),
    #[error("authorization catalog is invalid")]
    Catalog(#[from] CatalogError),
}

#[derive(Debug, thiserror::Error)]
pub enum AccessEvaluationError {
    #[error("authorization database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("authorization cache is unavailable")]
    Cache(#[from] redis::RedisError),
    #[error("authorization cache payload is invalid")]
    Serialization(#[from] serde_json::Error),
    #[error("authorization catalog is invalid")]
    Catalog(#[from] CatalogError),
    #[error("authorization user does not exist")]
    UserNotFound,
    #[error("authorization user is disabled")]
    UserDisabled,
}

#[derive(Debug, thiserror::Error)]
pub enum AccessPropagationError {
    #[error("authorization change propagation failed")]
    Cache(#[from] redis::RedisError),
}
