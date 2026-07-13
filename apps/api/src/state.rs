use std::{env, sync::Arc};

use audit::login_logs::service::LoginLogService;
use audit::operation_logs::service::OperationLogService;
use file_storage::categories::service::CategoryService;
use file_storage::files::service::FileService;
use iam::authorization::service::AuthorizationService;
use iam::departments::service::DepartmentService;
use iam::roles::service::RoleService;
use iam::users::service::UserService;
use menu::menus::service::MenuService;
use metadata::dictionaries::service::DictionaryService;
use metadata::parameters::service::ParameterService;

use crate::routes::auth::LoginOperation;
use auth::session::AuthSessionService;

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

    pub fn get_http_addr(&self) -> String {
        format!("0.0.0.0:{}", self.http_port)
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
    pub auth_session_service: AuthSessionService,
    pub login: LoginOperation,
    pub users: UserService,
    pub roles: RoleService,
    pub departments: DepartmentService,
    pub authorization: AuthorizationService,
    pub dictionaries: DictionaryService,
    pub parameters: ParameterService,
    pub menus: MenuService,
    pub login_logs: LoginLogService,
    pub operation_logs: OperationLogService,
    pub files: FileService,
    pub attachment_categories: CategoryService,
}
