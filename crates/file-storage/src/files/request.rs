#[derive(Debug, Clone)]
pub struct FileListQuery {
    pub page: i64,
    pub page_size: i64,
    pub keyword: Option<String>,
    pub category: Option<String>,
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

#[derive(Debug, Clone)]
pub struct StartUpload {
    pub name: String,
    pub size: i64,
    pub tag: String,
    pub category: String,
}
