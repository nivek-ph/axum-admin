use file_storage::files::{FileListQuery, ImportFileUrl, RenameFile, StartUpload, UploadSession};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type FileListRequest = FileListQuery;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportFileUrlRequest {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub category: String,
}

impl From<ImportFileUrlRequest> for ImportFileUrl {
    fn from(value: ImportFileUrlRequest) -> Self {
        Self {
            name: value.name,
            url: value.url,
            tag: value.tag,
            category: value.category,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RenameFileRequest {
    pub name: String,
}

impl RenameFileRequest {
    pub fn into_input(self, id: i64) -> RenameFile {
        RenameFile {
            id,
            name: self.name,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartUploadRequest {
    pub name: String,
    pub size: i64,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub category: String,
}

impl From<StartUploadRequest> for StartUpload {
    fn from(value: StartUploadRequest) -> Self {
        Self {
            name: value.name,
            size: value.size,
            tag: value.tag,
            category: value.category,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileListData {
    pub list: Vec<FileResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadFileData {
    pub file: Option<FileResponse>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadSessionData {
    pub id: String,
    pub offset: i64,
    pub total_size: i64,
    pub chunk_size: usize,
}

impl UploadSessionData {
    pub fn from_session(session: UploadSession) -> Self {
        Self {
            id: session.id,
            offset: session.uploaded_size,
            total_size: session.total_size,
            chunk_size: file_storage::files::UPLOAD_CHUNK_BYTES,
        }
    }
}

impl FileResponse {
    pub fn from_stored(public_base_url: &str, v: file_storage::files::StoredFile) -> Self {
        Self {
            id: v.id,
            name: v.name,
            url: public_file_url(public_base_url, &v.url),
            ext: v.ext,
            tag: v.tag,
            category: v.category,
            updated_at: v.updated_at.to_jiff().to_string(),
        }
    }
}

/// External URLs are stored as is; API responses expose them under `PUBLIC_BASE_URL`.
pub fn public_file_url(public_base_url: &str, url: &str) -> String {
    if !url.starts_with("/uploads/") {
        return url.to_string();
    }
    format!("{}{url}", public_base_url.trim_end_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::public_file_url;

    #[test]
    fn public_file_url_prefixes_local_upload_paths() {
        assert_eq!(
            public_file_url("http://127.0.0.1:3000", "/uploads/demo.pdf"),
            "http://127.0.0.1:3000/uploads/demo.pdf"
        );
        assert_eq!(
            public_file_url("http://127.0.0.1:3000/", "/uploads/demo.pdf"),
            "http://127.0.0.1:3000/uploads/demo.pdf"
        );
    }

    #[test]
    fn public_file_url_keeps_external_urls() {
        assert_eq!(
            public_file_url("http://127.0.0.1:3000", "https://cdn.example.com/a.pdf"),
            "https://cdn.example.com/a.pdf"
        );
    }
}
