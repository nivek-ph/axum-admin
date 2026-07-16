use file_storage::files::{FileListQuery, ImportFileUrl, RenameFile};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

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

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct UploadMetadataRequest {
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub category: String,
}

#[derive(Debug, ToSchema)]
pub struct UploadFileRequest {
    #[schema(value_type = String, format = Binary)]
    #[schema(example = "example.png")]
    #[allow(dead_code)]
    pub file: Vec<u8>,
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
