use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

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

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct IdsRequest {
    #[serde(rename = "IDs", alias = "ids")]
    pub ids: Vec<i64>,
}
