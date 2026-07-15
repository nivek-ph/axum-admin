use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct DeptResponse {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub status: String,
}
impl From<iam::departments::Dept> for DeptResponse {
    fn from(v: iam::departments::Dept) -> Self {
        Self {
            id: v.id,
            parent_id: v.parent_id,
            name: v.name,
            code: v.code,
            sort: v.sort,
            status: v.status,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DeptNodeResponse {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub status: String,
    pub children: Vec<DeptNodeResponse>,
}
impl From<iam::departments::DeptNode> for DeptNodeResponse {
    fn from(v: iam::departments::DeptNode) -> Self {
        Self {
            id: v.id,
            parent_id: v.parent_id,
            name: v.name,
            code: v.code,
            sort: v.sort,
            status: v.status,
            children: v.children.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeptPayload {
    pub parent_id: Option<i64>,
    pub name: String,
    pub code: String,
    pub sort: Option<i32>,
    pub status: Option<String>,
}
impl From<DeptPayload> for iam::departments::CreateDeptPayload {
    fn from(v: DeptPayload) -> Self {
        Self {
            parent_id: v.parent_id,
            name: v.name,
            code: v.code,
            sort: v.sort,
            status: v.status,
        }
    }
}
impl From<DeptPayload> for iam::departments::UpdateDeptPayload {
    fn from(v: DeptPayload) -> Self {
        Self {
            parent_id: v.parent_id,
            name: v.name,
            code: v.code,
            sort: v.sort,
            status: v.status,
        }
    }
}
