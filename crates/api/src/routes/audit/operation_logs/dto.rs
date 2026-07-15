use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct OperationUserResponse {
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: String,
}

#[derive(Debug, Serialize)]
pub struct OperationLogResponse {
    #[serde(rename = "id")]
    pub id: i64,
    pub ip: String,
    pub method: String,
    pub path: String,
    pub status: i32,
    pub agent: String,
    #[serde(rename = "errorMessage")]
    pub error_message: String,
    pub body: String,
    pub resp: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub user: OperationUserResponse,
}

impl From<audit::operation_logs::OperationLogView> for OperationLogResponse {
    fn from(value: audit::operation_logs::OperationLogView) -> Self {
        Self {
            id: value.id,
            ip: value.ip,
            method: value.method,
            path: value.path,
            status: value.status,
            agent: value.agent,
            error_message: value.error_message,
            body: value.body,
            resp: value.resp,
            created_at: value.created_at,
            user: OperationUserResponse {
                user_name: value.user.user_name,
                nick_name: value.user.nick_name,
            },
        }
    }
}
