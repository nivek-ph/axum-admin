use admin_httpz::{AppError, ErrorSpec};
use iam::users::LoginError;

use crate::errors::users::{
    INVALID_CREDENTIALS, INVALID_PASSWORD, USER_ALREADY_EXISTS, USER_DISABLED,
};

const USER_NOT_FOUND: ErrorSpec = ErrorSpec::not_found("USER_NOT_FOUND", "user not found");
const OPERATION_FAILED: ErrorSpec =
    ErrorSpec::internal("USER_OPERATION_FAILED", "user operation failed");
const INVALID_ROLES: ErrorSpec =
    ErrorSpec::validation("INVALID_ROLES", "at least one enabled role is required");

pub fn map_error(error: LoginError) -> AppError {
    match error {
        LoginError::InvalidCredentials => INVALID_CREDENTIALS.into(),
        LoginError::Disabled => USER_DISABLED.into(),
        LoginError::UserNotFound => USER_NOT_FOUND.into(),
        LoginError::UserAlreadyExists => USER_ALREADY_EXISTS.into(),
        LoginError::InvalidPassword => INVALID_PASSWORD.into(),
        LoginError::InvalidRoles => INVALID_ROLES.into(),
        LoginError::Auth(source) => OPERATION_FAILED.into_error().with_source(source),
        LoginError::Database(source) => OPERATION_FAILED.into_error().with_source(source),
    }
}
