use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    pub code: String,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: "OK".to_string(),
            message: "ok".to_string(),
            data: Some(data),
        }
    }

    pub fn new(code: impl Into<String>, message: impl Into<String>, data: Option<T>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data,
        }
    }
}

impl ApiResponse<Value> {
    pub fn ok_message(message: impl Into<String>) -> Self {
        Self::new("OK", message, None)
    }
}

pub type ApiErrorResponse = ApiResponse<Value>;
