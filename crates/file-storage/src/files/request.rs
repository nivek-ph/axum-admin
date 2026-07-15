use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FileListQuery {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub keyword: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FileEditPayload {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ImportUrlPayload {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub category: String,
}
