use argon2::Argon2;
use password_hash::{Error as PasswordHashError, PasswordHasher, PasswordVerifier};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("authentication failed")]
    AuthFailed,
    #[error("token invalid")]
    InvalidToken,
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<PasswordHashError> for AuthError {
    fn from(error: PasswordHashError) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(_error: jsonwebtoken::errors::Error) -> Self {
        Self::InvalidToken
    }
}

#[derive(Debug, Clone, Default)]
pub struct PasswordService;

impl PasswordService {
    pub fn new() -> Self {
        Self
    }

    pub fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        Ok(Argon2::default()
            .hash_password(password.as_bytes())?
            .to_string())
    }

    pub fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool, AuthError> {
        match Argon2::default().verify_password(password.as_bytes(), password_hash) {
            Ok(()) => Ok(true),
            Err(PasswordHashError::PasswordInvalid) => Ok(false),
            Err(error) => Err(AuthError::from(error)),
        }
    }
}
