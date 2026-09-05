#[derive(Debug, Clone)]
pub struct ParamListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub name: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParameterInput {
    pub name: String,
    pub key: String,
    pub value: String,
    pub desc: String,
}
