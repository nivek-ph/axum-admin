use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ParameterListRequest {
    pub page: Option<i64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
    pub name: Option<String>,
    pub key: Option<String>,
}

impl From<ParameterListRequest> for metadata::parameters::ParamListQuery {
    fn from(value: ParameterListRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            name: value.name,
            key: value.key,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ParameterRequest {
    pub name: String,
    pub key: String,
    pub value: String,
    pub desc: String,
}

impl From<ParameterRequest> for metadata::parameters::ParameterInput {
    fn from(value: ParameterRequest) -> Self {
        Self {
            name: value.name,
            key: value.key,
            value: value.value,
            desc: value.desc,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ParameterByKeyRequest {
    /// Parameter key
    pub key: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParamResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub name: String,
    pub key: String,
    pub value: String,
    pub desc: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ParameterListData {
    pub list: Vec<ParamResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmptyParameter {}

#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum ParameterDetailData {
    Parameter(ParamResponse),
    Empty(EmptyParameter),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParameterByKeyData {
    #[serde(rename = "sysParam")]
    pub parameter: Option<ParamResponse>,
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

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct IdsRequest {
    #[serde(rename = "IDs", alias = "ids")]
    pub ids: Vec<i64>,
}
