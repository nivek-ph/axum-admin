use admin_httpz::{AppError, ErrorSpec};

use crate::auth::session::AuthSessionError;

pub mod auth {
    use super::*;

    pub const LOGIN_REQUIRED: ErrorSpec =
        ErrorSpec::unauthorized("LOGIN_REQUIRED", "login required");
    pub const TOKEN_INVALID: ErrorSpec =
        ErrorSpec::unauthorized("TOKEN_INVALID", "session expired");
    pub const TOKEN_REVOKED: ErrorSpec =
        ErrorSpec::unauthorized("TOKEN_REVOKED", "session expired");
    pub const SESSION_INVALID: ErrorSpec =
        ErrorSpec::unauthorized("SESSION_INVALID", "session expired");
    pub const AUTH_RESOLVE_FAILED: ErrorSpec = ErrorSpec::internal(
        "AUTH_RESOLVE_FAILED",
        "failed to resolve authenticated user",
    );
    pub const PERMISSION_DENIED: ErrorSpec =
        ErrorSpec::forbidden("PERMISSION_DENIED", "permission denied");
    pub const LOGIN_OPERATION_FAILED: ErrorSpec =
        ErrorSpec::internal("LOGIN_OPERATION_FAILED", "login failed");

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

    pub const ID_REQUIRED: ErrorSpec = ErrorSpec::bad_request("ID_REQUIRED", "id is required");
    pub const AUTHORITY_ID_REQUIRED: ErrorSpec =
        ErrorSpec::bad_request("AUTHORITY_ID_REQUIRED", "authorityId is required");
    pub const MULTIPART_FIELD_FAILED: ErrorSpec =
        ErrorSpec::bad_request("MULTIPART_FIELD_FAILED", "failed to read upload content");

    pub fn multipart_field_error(error: axum::extract::multipart::MultipartError) -> AppError {
        MULTIPART_FIELD_FAILED.into_error().with_source(error)
    }
}
