use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SysDictionary {
    #[serde(rename = "id", default)]
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub dict_type: String,
    pub status: Option<bool>,
    pub desc: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SysDictionaryDetail {
    #[serde(rename = "id")]
    pub id: i64,
    pub label: String,
    pub value: String,
    pub extend: String,
    pub status: Option<bool>,
    pub sort: i32,
    #[serde(rename = "sysDictionaryId")]
    pub sys_dictionary_id: i64,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
    pub level: i32,
    pub path: String,
    pub children: Vec<SysDictionaryDetail>,
}

#[derive(Debug, Clone)]
pub struct DictionaryWithDetails {
    pub dictionary: SysDictionary,
    pub details: Vec<SysDictionaryDetail>,
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct SysDictionaryDetailRow {
    pub id: i64,
    pub label: String,
    pub value: String,
    pub extend: String,
    pub status: Option<bool>,
    pub sort: i32,
    pub sys_dictionary_id: i64,
    pub parent_id: Option<i64>,
    pub level: i32,
    pub path: String,
}
