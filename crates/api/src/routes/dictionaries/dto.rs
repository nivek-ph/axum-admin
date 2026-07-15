use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct DictionaryResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub dictionary_type: String,
    pub status: Option<bool>,
    pub desc: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
}

impl From<metadata::dictionaries::SysDictionary> for DictionaryResponse {
    fn from(v: metadata::dictionaries::SysDictionary) -> Self {
        Self {
            id: v.id,
            name: v.name,
            dictionary_type: v.dict_type,
            status: v.status,
            desc: v.desc,
            parent_id: v.parent_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DictionaryDetailResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub label: String,
    pub value: String,
    pub extend: String,
    pub status: Option<bool>,
    pub sort: i32,
    #[serde(rename = "sysDictionaryId")]
    pub dictionary_id: i64,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
    pub level: i32,
    pub path: String,
    pub children: Vec<DictionaryDetailResponse>,
}

impl From<metadata::dictionaries::SysDictionaryDetail> for DictionaryDetailResponse {
    fn from(v: metadata::dictionaries::SysDictionaryDetail) -> Self {
        Self {
            id: v.id,
            label: v.label,
            value: v.value,
            extend: v.extend,
            status: v.status,
            sort: v.sort,
            dictionary_id: v.sys_dictionary_id,
            parent_id: v.parent_id,
            level: v.level,
            path: v.path,
            children: v.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl DictionaryDetailResponse {
    fn flatten(items: Vec<metadata::dictionaries::SysDictionaryDetail>, output: &mut Vec<Self>) {
        for item in items {
            let children = item.children;
            output.push(Self {
                id: item.id,
                label: item.label,
                value: item.value,
                extend: item.extend,
                status: item.status,
                sort: item.sort,
                dictionary_id: item.sys_dictionary_id,
                parent_id: item.parent_id,
                level: item.level,
                path: item.path,
                children: Vec::new(),
            });
            Self::flatten(children, output);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DictionaryWithDetailsResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub dictionary_type: String,
    pub status: Option<bool>,
    pub desc: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
    #[serde(rename = "sysDictionaryDetails")]
    pub details: Vec<DictionaryDetailResponse>,
}
impl From<metadata::dictionaries::DictionaryWithDetails> for DictionaryWithDetailsResponse {
    fn from(v: metadata::dictionaries::DictionaryWithDetails) -> Self {
        let mut details = Vec::new();
        DictionaryDetailResponse::flatten(v.details, &mut details);
        Self {
            id: v.dictionary.id,
            name: v.dictionary.name,
            dictionary_type: v.dictionary.dict_type,
            status: v.dictionary.status,
            desc: v.dictionary.desc,
            parent_id: v.dictionary.parent_id,
            details,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DictionaryPayload {
    #[serde(rename = "id", default)]
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub dictionary_type: String,
    pub status: Option<bool>,
    pub desc: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
}

impl From<DictionaryPayload> for metadata::dictionaries::SysDictionary {
    fn from(value: DictionaryPayload) -> Self {
        Self {
            id: value.id,
            name: value.name,
            dict_type: value.dictionary_type,
            status: value.status,
            desc: value.desc,
            parent_id: value.parent_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DictionaryListQuery {
    pub page: Option<i64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
    pub name: Option<String>,
}

impl From<DictionaryListQuery> for metadata::dictionaries::DictionaryListQuery {
    fn from(value: DictionaryListQuery) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            name: value.name,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ImportDictionaryPayload {
    pub json: String,
}

impl From<ImportDictionaryPayload> for metadata::dictionaries::ImportDictionaryPayload {
    fn from(value: ImportDictionaryPayload) -> Self {
        Self { json: value.json }
    }
}

#[derive(Debug, Deserialize)]
pub struct DictionaryDetailPayload {
    #[serde(rename = "id", default)]
    pub id: i64,
    pub label: String,
    pub value: String,
    pub extend: String,
    pub status: Option<bool>,
    pub sort: i32,
    #[serde(rename = "sysDictionaryId")]
    pub dictionary_id: i64,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
}

impl From<DictionaryDetailPayload> for metadata::dictionaries::SysDictionaryDetail {
    fn from(value: DictionaryDetailPayload) -> Self {
        Self {
            id: value.id,
            label: value.label,
            value: value.value,
            extend: value.extend,
            status: value.status,
            sort: value.sort,
            sys_dictionary_id: value.dictionary_id,
            parent_id: value.parent_id,
            level: 0,
            path: String::new(),
            children: Vec::new(),
        }
    }
}
