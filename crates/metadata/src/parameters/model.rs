use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct SysParam {
    pub id: i64,
    pub name: String,
    pub key: String,
    pub value: String,
    pub desc: String,
}
