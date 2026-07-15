use sqlx::FromRow;
#[derive(Debug, Clone, FromRow)]
pub struct StoredFile {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub ext: String,
    pub tag: String,
    pub category: String,
    pub updated_at: String,
}
