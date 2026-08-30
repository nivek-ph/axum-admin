use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct StoredFile {
    pub id: i64,
    pub storage_id: Option<i64>,
    pub name: String,
    pub url: String,
    pub ext: String,
    pub tag: String,
    pub category: String,
    pub updated_at: jiff_sqlx::Timestamp,
}

#[derive(Debug, Clone, FromRow)]
pub struct UploadSession {
    pub id: String,
    pub storage_id: i64,
    pub name: String,
    pub object_name: String,
    pub ext: String,
    pub tag: String,
    pub category: String,
    pub total_size: i64,
    pub uploaded_size: i64,
}
