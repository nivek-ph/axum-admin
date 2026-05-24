use admin_httpz::{AppError, ErrorSpec};

use crate::auth::session::AuthSessionError;

pub mod auth {
    use super::*;

    pub const LOGIN_REQUIRED: ErrorSpec = ErrorSpec::unauthorized("LOGIN_REQUIRED", "请先登录");
    pub const TOKEN_INVALID: ErrorSpec = ErrorSpec::unauthorized("TOKEN_INVALID", "登录已失效");
    pub const TOKEN_REVOKED: ErrorSpec = ErrorSpec::unauthorized("TOKEN_REVOKED", "登录已失效");
    pub const SESSION_INVALID: ErrorSpec = ErrorSpec::unauthorized("SESSION_INVALID", "登录已失效");
    pub const AUTH_RESOLVE_FAILED: ErrorSpec =
        ErrorSpec::internal("AUTH_RESOLVE_FAILED", "用户鉴权失败");
    pub const LOGIN_OPERATION_FAILED: ErrorSpec =
        ErrorSpec::internal("LOGIN_OPERATION_FAILED", "登录失败");

    impl From<AuthSessionError> for AppError {
        fn from(error: AuthSessionError) -> Self {
            match error {
                AuthSessionError::Auth(error) => TOKEN_INVALID.into_error().with_source(error),
                AuthSessionError::Revoked => TOKEN_REVOKED.into_error(),
                AuthSessionError::RevocationStoreUnavailable | AuthSessionError::Redis(_) => {
                    AUTH_RESOLVE_FAILED.into_error().with_source(error)
                }
            }
        }
    }
}

pub mod request {
    use super::*;

    pub const ID_REQUIRED: ErrorSpec = ErrorSpec::bad_request("ID_REQUIRED", "缺少ID");
    pub const AUTHORITY_ID_REQUIRED: ErrorSpec =
        ErrorSpec::bad_request("AUTHORITY_ID_REQUIRED", "缺少authorityId");
    pub const MULTIPART_FIELD_FAILED: ErrorSpec =
        ErrorSpec::bad_request("MULTIPART_FIELD_FAILED", "上传内容读取失败");

    pub fn multipart_field_error(error: axum::extract::multipart::MultipartError) -> AppError {
        MULTIPART_FIELD_FAILED.into_error().with_source(error)
    }
}
