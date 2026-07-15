use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct FileListQuery {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub keyword: Option<String>,
    pub category: Option<String>,
}
impl From<FileListQuery> for file_storage::files::FileListQuery {
    fn from(v: FileListQuery) -> Self {
        Self {
            page: v.page,
            page_size: v.page_size,
            keyword: v.keyword,
            category: v.category,
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct FileEditPayload {
    #[serde(rename = "id")]
    pub id: i64,
    pub name: String,
}
impl From<FileEditPayload> for file_storage::files::FileEditPayload {
    fn from(v: FileEditPayload) -> Self {
        Self {
            id: v.id,
            name: v.name,
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct ImportUrlPayload {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub category: String,
}
impl From<ImportUrlPayload> for file_storage::files::ImportUrlPayload {
    fn from(v: ImportUrlPayload) -> Self {
        Self {
            name: v.name,
            url: v.url,
            tag: v.tag,
            category: v.category,
        }
    }
}
