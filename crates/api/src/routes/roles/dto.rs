use serde::Serialize;
use utoipa::ToSchema;

pub type RoleRequest = iam::roles::RolePayload;
pub type RoleMenuRequest = iam::roles::RoleMenuPayload;
pub type RoleDeptRequest = iam::roles::RoleDeptPayload;
pub type RoleUsersRequest = iam::roles::RoleUsersPayload;

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub sort: i32,
    pub data_scope: String,
    pub is_system: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleListData {
    pub list: Vec<RoleResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleData {
    pub role: RoleResponse,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleMenuIdsData {
    pub menu_ids: Vec<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleDeptIdsData {
    pub dept_ids: Vec<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleUserIdsData(pub Vec<i64>);

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleMutationData {}

impl From<iam::roles::RoleSummary> for RoleResponse {
    fn from(v: iam::roles::RoleSummary) -> Self {
        Self {
            id: v.id,
            code: v.code,
            name: v.name,
            status: v.status,
            sort: v.sort,
            data_scope: v.data_scope,
            is_system: v.is_system,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_ids_keep_transport_shape() {
        let menus = serde_json::to_value(RoleMenuIdsData {
            menu_ids: vec![1, 2],
        })
        .expect("menu IDs should serialize");
        assert_eq!(menus["menuIds"], serde_json::json!([1, 2]));
        assert!(menus.get("menu_ids").is_none());

        let depts = serde_json::to_value(RoleDeptIdsData { dept_ids: vec![3] })
            .expect("department IDs should serialize");
        assert_eq!(depts["deptIds"], serde_json::json!([3]));

        let users =
            serde_json::to_value(RoleUserIdsData(vec![4, 5])).expect("user IDs should serialize");
        assert_eq!(users, serde_json::json!([4, 5]));
    }

    #[test]
    fn mutation_response_keeps_null_data() {
        let response = serde_json::to_value(crate::ApiResponse::<RoleMutationData>::new(
            "OK", "saved", None,
        ))
        .expect("mutation response should serialize");

        assert!(response["data"].is_null());
    }
}
