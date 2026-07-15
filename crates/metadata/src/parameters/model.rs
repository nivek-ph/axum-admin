use serde::Deserialize;
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, FromRow, Deserialize, ToSchema)]
pub struct SysParam {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    pub key: String,
    pub value: String,
    pub desc: String,
}
