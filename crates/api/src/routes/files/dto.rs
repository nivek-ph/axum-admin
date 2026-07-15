use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FileResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub name: String,
    pub url: String,
    pub ext: String,
    pub tag: String,
    pub category: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}
impl From<file_storage::files::StoredFile> for FileResponse {
    fn from(v: file_storage::files::StoredFile) -> Self {
        Self {
            id: v.id,
            name: v.name,
            url: v.url,
            ext: v.ext,
            tag: v.tag,
            category: v.category,
            updated_at: v.updated_at,
        }
    }
}
