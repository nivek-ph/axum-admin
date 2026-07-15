use admin_httpz::{AppError, ErrorSpec};

use ::auth::token::TokenError;

pub const INTERNAL_SERVER_ERROR: ErrorSpec =
    ErrorSpec::internal("INTERNAL_SERVER_ERROR", "internal server error");

pub mod auth {
    use super::*;
    use axum::http::StatusCode;

    pub const LOGIN_REQUIRED: ErrorSpec =
        ErrorSpec::unauthorized("LOGIN_REQUIRED", "login required");
    pub const TOKEN_INVALID: ErrorSpec =
        ErrorSpec::unauthorized("TOKEN_INVALID", "session expired");
    pub const TOKEN_REVOKED: ErrorSpec =
        ErrorSpec::unauthorized("TOKEN_REVOKED", "session expired");
    pub const SESSION_INVALID: ErrorSpec =
        ErrorSpec::unauthorized("SESSION_INVALID", "session expired");
    pub const PERMISSION_DENIED: ErrorSpec =
        ErrorSpec::forbidden("PERMISSION_DENIED", "permission denied");
    pub const AUTHORIZATION_CONFIG_INVALID: ErrorSpec = ErrorSpec::internal(
        "AUTHORIZATION_CONFIG_INVALID",
        "authorization configuration is invalid",
    );
    pub const AUTHORIZATION_UNAVAILABLE: ErrorSpec = ErrorSpec::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "AUTHORIZATION_UNAVAILABLE",
        "authorization service is unavailable",
    );
    pub const LOGIN_OPERATION_FAILED: ErrorSpec =
        ErrorSpec::internal("LOGIN_OPERATION_FAILED", "login failed");
    pub const CAPTCHA_REQUIRED: ErrorSpec =
        ErrorSpec::bad_request("CAPTCHA_REQUIRED", "captcha is required");
    pub const CAPTCHA_INVALID: ErrorSpec =
        ErrorSpec::bad_request("CAPTCHA_INVALID", "captcha is invalid or expired");
    pub const CAPTCHA_OPERATION_FAILED: ErrorSpec =
        ErrorSpec::internal("CAPTCHA_OPERATION_FAILED", "captcha operation failed");

    pub fn map_token_error(error: TokenError) -> AppError {
        match error {
            TokenError::Auth(source) => TOKEN_INVALID.into_error().with_source(source),
            TokenError::Revoked => TOKEN_REVOKED.into_error(),
            TokenError::RevocationStoreUnavailable | TokenError::Redis(_) => {
                AUTHORIZATION_UNAVAILABLE.into_error().with_source(error)
            }
        }
    }
}

pub mod request {
    use super::*;

    pub const MULTIPART_FIELD_FAILED: ErrorSpec =
        ErrorSpec::bad_request("MULTIPART_FIELD_FAILED", "failed to read upload content");

    pub fn multipart_field_error(error: axum::extract::multipart::MultipartError) -> AppError {
        MULTIPART_FIELD_FAILED.into_error().with_source(error)
    }
}

pub mod users {
    use super::*;

    pub const INVALID_CREDENTIALS: ErrorSpec =
        ErrorSpec::unauthorized("INVALID_CREDENTIALS", "invalid username or password");
    pub const USER_DISABLED: ErrorSpec = ErrorSpec::forbidden("USER_DISABLED", "user is disabled");
    pub const USER_ALREADY_EXISTS: ErrorSpec =
        ErrorSpec::conflict("USER_ALREADY_EXISTS", "user already exists");
    pub const INVALID_PASSWORD: ErrorSpec =
        ErrorSpec::bad_request("INVALID_PASSWORD", "invalid password");
}
