#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    #[error("dictionary {dictionary_id} was not found")]
    DictionaryNotFound { dictionary_id: i64 },
    #[error("dictionary detail {detail_id} was not found in dictionary {dictionary_id}")]
    DetailNotFound { dictionary_id: i64, detail_id: i64 },
    #[error("dictionary storage operation failed")]
    Database(#[from] sqlx::Error),
}
