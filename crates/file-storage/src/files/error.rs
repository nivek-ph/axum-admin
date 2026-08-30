#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("uploaded file is too large")]
    TooLarge,
    #[error("upload session not found")]
    UploadNotFound,
    #[error("upload offset does not match")]
    OffsetMismatch,
    #[error("upload is incomplete")]
    UploadIncomplete,
    #[error("upload operation is already in progress")]
    UploadInProgress,
    #[error("uploaded chunks do not match the persisted upload state")]
    UploadCorrupt,
    #[error("file storage operation failed")]
    Database(#[from] sqlx::Error),
    #[error("file storage operation failed")]
    Io(#[from] std::io::Error),
    #[error("file storage operation failed")]
    Adapter(#[from] opendal::Error),
    #[error("file storage operation failed")]
    Storage(#[from] crate::storages::StorageError),
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::FileError;

    #[test]
    fn adapter_failure_keeps_a_stable_capability_message_and_source() {
        let error = FileError::from(std::io::Error::other("disk detail"));

        assert_eq!(error.to_string(), "file storage operation failed");
        let source = error
            .source()
            .expect("file error should keep its I/O source");
        let source = source
            .downcast_ref::<std::io::Error>()
            .expect("source should remain an I/O error");
        assert_eq!(source.to_string(), "disk detail");
    }
}
