use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ParamListQuery {
    pub(crate) page: Option<i64>,
    #[serde(rename = "pageSize")]
    pub(crate) page_size: Option<i64>,
    pub(crate) name: Option<String>,
    pub(crate) key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ParameterInput {
    pub(crate) name: String,
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) desc: String,
}
