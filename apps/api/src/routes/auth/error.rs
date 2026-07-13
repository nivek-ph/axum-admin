use admin_httpz::AppError;

use super::operation::LoginOperationError;
use crate::errors::auth::{
    CAPTCHA_INVALID, CAPTCHA_OPERATION_FAILED, CAPTCHA_REQUIRED, LOGIN_OPERATION_FAILED,
    map_session_error,
};
use crate::errors::users::{
    INVALID_CREDENTIALS, INVALID_PASSWORD, USER_ALREADY_EXISTS, USER_DISABLED,
};

pub fn map_error(error: LoginOperationError) -> AppError {
    match error {
        LoginOperationError::CaptchaRequired => CAPTCHA_REQUIRED.into(),
        LoginOperationError::CaptchaInvalid => CAPTCHA_INVALID.into(),
        LoginOperationError::Captcha(source) => {
            CAPTCHA_OPERATION_FAILED.into_error().with_source(source)
        }
        LoginOperationError::Identity(source) => match source {
            iam::users::LoginError::InvalidCredentials | iam::users::LoginError::UserNotFound => {
                INVALID_CREDENTIALS.into()
            }
            iam::users::LoginError::Disabled => USER_DISABLED.into(),
            iam::users::LoginError::UserAlreadyExists => USER_ALREADY_EXISTS.into(),
            iam::users::LoginError::InvalidPassword => INVALID_PASSWORD.into(),
            iam::users::LoginError::InvalidRoles => LOGIN_OPERATION_FAILED.into_error(),
            iam::users::LoginError::Auth(_) | iam::users::LoginError::Database(_) => {
                LOGIN_OPERATION_FAILED.into_error().with_source(source)
            }
        },
        LoginOperationError::Session(source) => map_session_error(source),
    }
}
