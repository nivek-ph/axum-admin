use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ParamResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub name: String,
    pub key: String,
    pub value: String,
    pub desc: String,
}
impl From<metadata::parameters::SysParam> for ParamResponse {
    fn from(v: metadata::parameters::SysParam) -> Self {
        Self {
            id: v.id,
            name: v.name,
            key: v.key,
            value: v.value,
            desc: v.desc,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ParamPayload {
    #[serde(rename = "id", default)]
    pub id: i64,
    pub name: String,
    pub key: String,
    pub value: String,
    pub desc: String,
}

impl From<ParamPayload> for metadata::parameters::SysParam {
    fn from(value: ParamPayload) -> Self {
        Self {
            id: value.id,
            name: value.name,
            key: value.key,
            value: value.value,
            desc: value.desc,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct IdsRequest {
    #[serde(rename = "IDs", alias = "ids")]
    pub ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ParamListQuery {
    pub page: Option<i64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
    pub name: Option<String>,
    pub key: Option<String>,
}

impl From<ParamListQuery> for metadata::parameters::ParamListQuery {
    fn from(value: ParamListQuery) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            name: value.name,
            key: value.key,
        }
    }
}
