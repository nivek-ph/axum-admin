use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DictionaryListQuery {
    pub page: Option<i64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DictionaryTreeQuery {
    pub sys_dictionary_id: i64,
}

#[derive(Debug, Clone)]
pub struct DictionaryTypeQuery {
    pub dict_type: String,
}

#[derive(Debug, Clone)]
pub struct DictionaryParentQuery {
    pub parent_id: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ImportDictionaryPayload {
    pub json: String,
}
