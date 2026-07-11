use std::{env, sync::Arc};

use auth::password::PasswordService;
use db::DbPool;

use crate::auth::session::AuthSessionService;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub http_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
}

#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    pub database_url: String,
    pub admin_username: String,
    pub admin_password: String,
    pub admin_nickname: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("required environment variable {0} is not set")]
    Missing(String),
    #[error("environment variable HTTP_PORT must be a valid port, got {0}")]
    InvalidHttpPort(String),
}

/// Require an environment variable to be set.
fn require_env(key: &str) -> Result<String, ConfigError> {
    let value = env::var(key).map_err(|_| ConfigError::Missing(key.to_string()))?;
    if value.trim().is_empty() {
        return Err(ConfigError::Missing(key.to_string()));
    }
    Ok(value)
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let http_port = require_env("HTTP_PORT")?;
        Ok(Self {
            http_port: http_port
                .parse()
                .map_err(|_| ConfigError::InvalidHttpPort(http_port))?,
            database_url: require_env("DATABASE_URL")?,
            redis_url: require_env("REDIS_URL")?,
            jwt_secret: require_env("JWT_SECRET")?,
        })
    }
}

impl BootstrapConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            database_url: require_env("DATABASE_URL")?,
            admin_username: env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string()),
            admin_password: require_env("ADMIN_PASSWORD")?,
            admin_nickname: env::var("ADMIN_NICKNAME")
                .unwrap_or_else(|_| "Administrator".to_string()),
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub pool: DbPool,
    pub auth_session_service: AuthSessionService,
    pub password_service: PasswordService,
}
