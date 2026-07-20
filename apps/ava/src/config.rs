use std::env;

use tracing::Level;

#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub log_level: Level,
}

#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub http_port: u16,
    pub public_base_url: String,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub ollama_base_url: String,
    pub ollama_model: String,
}

#[derive(Debug, Clone)]
pub struct InitConfig {
    pub database_url: String,
    pub admin_username: String,
    pub admin_password: String,
    pub admin_nickname: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("required environment variable {0} is not set")]
    Missing(String),
    #[error("environment variable HTTP_PORT must be a valid port, got {0}")]
    InvalidHttpPort(String),
    #[error("environment variable LOG_LEVEL must be a valid tracing level, got {0}")]
    InvalidLogLevel(String),
}

pub fn load_env_file() {
    dotenvy::dotenv().ok();
}

impl LoggerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let value = env::var("LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
        let log_level = value
            .parse()
            .map_err(|_| ConfigError::InvalidLogLevel(value))?;
        Ok(Self { log_level })
    }
}

impl ServeConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let http_port = require_env("HTTP_PORT")?;
        let http_port: u16 = http_port
            .parse()
            .map_err(|_| ConfigError::InvalidHttpPort(http_port))?;
        let default_public_base_url = format!("http://127.0.0.1:{http_port}");
        Ok(Self {
            http_port,
            public_base_url: env::var("PUBLIC_BASE_URL")
                .ok()
                .map(|value| value.trim().trim_end_matches('/').to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or(default_public_base_url),
            database_url: require_env("DATABASE_URL")?,
            redis_url: require_env("REDIS_URL")?,
            jwt_secret: require_env("JWT_SECRET")?,
            ollama_base_url: env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string()),
            ollama_model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3.5:4b".to_string()),
        })
    }

    pub fn api_server(&self) -> api::ServerConfig {
        api::ServerConfig {
            listen_addr: format!("0.0.0.0:{}", self.http_port),
            public_url: self.public_base_url.clone(),
        }
    }
}

impl InitConfig {
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

fn require_env(key: &str) -> Result<String, ConfigError> {
    let value = env::var(key).map_err(|_| ConfigError::Missing(key.to_string()))?;
    if value.trim().is_empty() {
        return Err(ConfigError::Missing(key.to_string()));
    }
    Ok(value)
}
