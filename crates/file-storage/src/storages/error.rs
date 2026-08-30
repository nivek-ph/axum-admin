use crate::files::ObjectStorageError;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage was not found")]
    NotFound,
    #[error("storage code already exists")]
    CodeConflict,
    #[error("storage driver, code, and object location cannot be changed")]
    ImmutableIdentity,
    #[error("the default storage cannot be disabled or deleted")]
    DefaultProtected,
    #[error("only an enabled storage can be the default")]
    DisabledDefault,
    #[error("storage is referenced by uploaded files or active upload sessions")]
    InUse,
    #[error("storage input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("storage is invalid: {0}")]
    InvalidConfiguration(#[from] ObjectStorageError),
    #[error("storage operation failed")]
    Database(#[from] sqlx::Error),
}
