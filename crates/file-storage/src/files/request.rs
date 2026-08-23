use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FileListQuery {
    pub(crate) page: i64,
    #[serde(rename = "pageSize")]
    pub(crate) page_size: i64,
    pub(crate) keyword: Option<String>,
    pub(crate) category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RenameFile {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ImportFileUrl {
    pub name: String,
    pub url: String,
    pub tag: String,
    pub category: String,
}
