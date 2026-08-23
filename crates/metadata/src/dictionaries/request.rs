use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DictionaryListQuery {
    pub page: Option<i64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DictionaryInput {
    pub name: String,
    pub dict_type: String,
    pub status: Option<bool>,
    pub desc: String,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DictionaryDetailInput {
    pub label: String,
    pub value: String,
    pub extend: String,
    pub status: Option<bool>,
    pub sort: i32,
    pub parent_id: Option<i64>,
}
