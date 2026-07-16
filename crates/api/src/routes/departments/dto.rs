use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct DeptNodeResponse {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub status: String,
    #[schema(no_recursion)]
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

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct DeptTreeData {
    pub list: Vec<DeptNodeResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmptyDept {}

#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum DeptDetail {
    Dept(DeptResponse),
    Empty(EmptyDept),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeptDetailData {
    pub dept: DeptDetail,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeptMutationData {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn department_tree_and_empty_detail_keep_transport_shape() {
        let tree = serde_json::to_value(DeptTreeData { list: Vec::new() })
            .expect("department tree should serialize");
        assert_eq!(tree, serde_json::json!({ "list": [] }));

        let detail = serde_json::to_value(DeptDetailData {
            dept: DeptDetail::Empty(EmptyDept {}),
        })
        .expect("empty department detail should serialize");
        assert_eq!(detail, serde_json::json!({ "dept": {} }));
    }

    #[test]
    fn department_mutation_keeps_null_data() {
        let response = serde_json::to_value(crate::ApiResponse::<DeptMutationData>::new(
            "OK", "updated", None,
        ))
        .expect("mutation response should serialize");
        assert!(response["data"].is_null());
    }
}
