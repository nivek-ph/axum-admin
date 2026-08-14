#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("uploaded file is too large")]
    TooLarge,
    #[error("file storage configuration is invalid: {0}")]
    InvalidConfiguration(&'static str),
    #[error("file storage operation failed")]
    Database(#[from] sqlx::Error),
    #[error("file storage operation failed")]
    Storage(#[from] opendal::Error),
}

#[cfg(test)]
mod tests {
    use opendal::ErrorKind;

    use super::FileError;

    #[test]
    fn adapter_failure_keeps_a_stable_capability_message_and_source() {
        let error = FileError::from(opendal::Error::new(ErrorKind::Unexpected, "disk detail"));

        assert_eq!(error.to_string(), "file storage operation failed");
        assert!(matches!(error, FileError::Storage(_)));
    }
}
