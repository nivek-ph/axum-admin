use std::{env, sync::Arc};

use auth::password::PasswordService;
use db::DbPool;

use crate::auth::session::AuthSessionService;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub admin_username: String,
    pub admin_password: String,
    pub admin_nickname: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            bind_addr: env::var("APP_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string()),
            database_url: env::var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-env".to_string()),
            admin_username: env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string()),
            admin_password: env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "123456".to_string()),
            admin_nickname: env::var("ADMIN_NICKNAME").unwrap_or_else(|_| "系统管理员".to_string()),
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

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(AppConfig {
                bind_addr: "127.0.0.1:3000".to_string(),
                database_url: "postgres://postgres:postgres@localhost/axum_vue_admin".to_string(),
                redis_url: "redis://127.0.0.1:6379/".to_string(),
                jwt_secret: "change-me-in-env".to_string(),
                admin_username: "admin".to_string(),
                admin_password: "123456".to_string(),
                admin_nickname: "系统管理员".to_string(),
            }),
            pool: DbPool::connect_lazy("postgres://postgres:postgres@localhost/axum_vue_admin")
                .expect("lazy pool should construct"),
            auth_session_service: AuthSessionService::without_revocation_store("change-me-in-env"),
            password_service: PasswordService::new(),
        }
    }
}
