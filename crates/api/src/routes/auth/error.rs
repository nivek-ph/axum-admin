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
            LoginError::Identity(source) => source.into(),
            LoginError::Token(source) => source.into(),
        }
    }
}
