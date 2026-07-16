use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type DictionaryListRequest = metadata::dictionaries::DictionaryListQuery;

#[derive(Debug, Deserialize, ToSchema)]
pub struct DictionaryRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub dictionary_type: String,
    pub status: Option<bool>,
    pub desc: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
}

impl From<DictionaryRequest> for metadata::dictionaries::DictionaryInput {
    fn from(value: DictionaryRequest) -> Self {
        Self {
            name: value.name,
            dict_type: value.dictionary_type,
            status: value.status,
            desc: value.desc,
            parent_id: value.parent_id,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ImportDictionaryRequest {
    pub json: String,
}

impl ImportDictionaryRequest {
    pub fn into_input(self) -> Result<metadata::dictionaries::DictionaryInput, serde_json::Error> {
        let document: DictionaryImportDocument = serde_json::from_str(&self.json)?;
        Ok(document.dictionary.into())
    }
}

#[derive(Debug, Deserialize)]
struct DictionaryImportDocument {
    dictionary: DictionaryRequest,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
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
    #[schema(no_recursion)]
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

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct DictionaryDetailRequest {
    #[serde(rename = "id", default)]
    pub id: i64,
    pub label: String,
    pub value: String,
    pub extend: String,
    pub status: Option<bool>,
    pub sort: i32,
    #[serde(rename = "sysDictionaryId")]
    #[serde(default)]
    pub dictionary_id: Option<i64>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
}

impl From<DictionaryDetailRequest> for metadata::dictionaries::DictionaryDetailInput {
    fn from(value: DictionaryDetailRequest) -> Self {
        Self {
            label: value.label,
            value: value.value,
            extend: value.extend,
            status: value.status,
            sort: value.sort,
            parent_id: value.parent_id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DictionaryDetailData {
    #[serde(rename = "resysDictionary")]
    pub dictionary: DictionaryDetailValue,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum DictionaryDetailValue {
    Dictionary(DictionaryWithDetailsResponse),
    Empty(EmptyDictionary),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmptyDictionary {}

#[derive(Debug, Serialize, ToSchema)]
pub struct DictionaryImportData {}

#[derive(Debug, Serialize, ToSchema)]
pub struct DictionaryTreeData {
    pub list: Vec<DictionaryDetailResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DictionaryNodeData {
    #[serde(rename = "reSysDictionaryDetail")]
    pub detail: DictionaryDetailResponse,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct DictionaryExportData {
    pub dictionary: DictionaryResponse,
    pub details: Vec<DictionaryDetailResponse>,
}

impl From<metadata::dictionaries::DictionaryWithDetails> for DictionaryExportData {
    fn from(value: metadata::dictionaries::DictionaryWithDetails) -> Self {
        Self {
            dictionary: value.dictionary.into(),
            details: value.details.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum DictionaryExportValue {
    Dictionary(DictionaryExportData),
    Empty(EmptyDictionary),
}
