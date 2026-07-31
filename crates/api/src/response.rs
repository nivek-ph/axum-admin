use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    pub code: String,
    pub message: String,
    pub data: Option<T>,
}

/// Marker type for responses with no data payload (`data: null`).
///
/// Used as the `T` in `ApiResponse<T>` when there is nothing to return (errors,
/// logout, etc.). Callers pass `None` for `data`; this type is only for the type
/// system and OpenAPI schema.
#[derive(Debug, Serialize, ToSchema)]
pub struct EmptyData {}

impl<T> ApiResponse<T> {
    pub fn new(code: impl Into<String>, message: impl Into<String>, data: Option<T>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data,
        }
    }

    pub fn ok(data: T) -> Self {
        Self::new("OK", "ok", Some(data))
    }

    pub fn ok_message(message: impl Into<String>) -> Self {
        Self::new("OK", message, None)
    }
}

pub type ApiErrorResponse = ApiResponse<EmptyData>;
