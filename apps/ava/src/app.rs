use std::time::Duration;

use anyhow::{Context, Result};
use audit::{AuditAnalyzer, AuditService};
use auth::{captcha::CaptchaService, password::PasswordService, token::TokenService};
use file_storage::files::FileService;
use iam::{
    access::AccessService, departments::DepartmentService, menus::MenuService, roles::RoleService,
    users::UserService,
};
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};
use tracing::info;

use crate::ServeConfig;

// boot the application and return the app state
pub async fn boot(config: &ServeConfig) -> Result<api::AppState> {
    let (pool, redis_connection) = connect_stores(config).await?;

    info!("running database migrations");
    db::migrate(&pool)
        .await
        .context("database migrations should run")?;
    info!("database migrations complete");

    build_state(config, pool, redis_connection).await
}

// connect to the stores and return the pool and redis connection
async fn connect_stores(
    config: &ServeConfig,
) -> Result<(db::DbPool, redis::aio::MultiplexedConnection)> {
    let pool = db::connect(&config.database_url)
        .await
        .context("database pool should connect")?;
    info!("database connected");

    let redis_client =
        redis::Client::open(config.redis_url.clone()).context("redis client should construct")?;
    let redis_config = redis::AsyncConnectionConfig::new()
        .set_connection_timeout(Some(Duration::from_secs(10)))
        .set_response_timeout(Some(Duration::from_secs(5)));
    let redis_connection = redis_client
        .get_multiplexed_async_connection_with_config(&redis_config)
        .await
        .context("redis connection should connect")?;
    Ok((pool, redis_connection))
}

// wire up the services and return the app state
async fn build_state(
    config: &ServeConfig,
    pool: db::DbPool,
    redis_connection: redis::aio::MultiplexedConnection,
) -> Result<api::AppState> {
    let public_base_url = config.public_base_url();

    // 1. standalone services (no cross-service deps)
    let password_service = PasswordService::new();
    let tokens = TokenService::new(&config.jwt_secret, redis_connection.clone());
    let captcha = CaptchaService::new(redis_connection.clone());
    let audits = AuditService::new(pool.clone());
    let dictionaries = DictionaryService::new(pool.clone());
    let parameters = ParameterService::new(pool.clone());
    let menus = MenuService::new(pool.clone());
    let audit_analyzer = AuditAnalyzer::new(&config.ollama_base_url, &config.ollama_model);
    let files = FileService::new(pool.clone(), "./uploads");

    // 2. authorization catalog (needed by IAM services below)
    let access = AccessService::load(pool.clone(), redis_connection)
        .await
        .context("authorization catalog and cache should initialize")?;

    // 3. IAM services that depend on access / audit / password
    let users = UserService::new(
        pool.clone(),
        access.clone(),
        audits.clone(),
        password_service,
    );
    let roles = RoleService::new(pool.clone(), access.clone());
    let departments = DepartmentService::new(pool, access.clone());

    Ok(api::AppState {
        public_base_url,
        tokens,
        captcha,
        users,
        roles,
        departments,
        access,
        dictionaries,
        parameters,
        menus,
        audits,
        audit_analyzer,
        files,
    })
}
