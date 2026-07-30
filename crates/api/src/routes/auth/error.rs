use super::login::LoginError;
use crate::{
    AppError,
    mappings::{CAPTCHA_INVALID, CAPTCHA_REQUIRED},
};

impl From<LoginError> for AppError {
    fn from(error: LoginError) -> Self {
        match error {
            LoginError::CaptchaRequired => CAPTCHA_REQUIRED.into(),
            LoginError::CaptchaInvalid => CAPTCHA_INVALID.into(),
            LoginError::Captcha(source) => source.into(),
            LoginError::InvalidCredentials => crate::mappings::INVALID_CREDENTIALS.into(),
            LoginError::Disabled => crate::mappings::USER_DISABLED.into(),
            LoginError::Password(source) => source.into(),
            LoginError::Account(iam::accounts::AccountError::NotFound) => {
                crate::mappings::INVALID_CREDENTIALS.into()
            }
            LoginError::Account(source) => source.into(),
            LoginError::Token(source) => source.into(),
        }
    }
}
