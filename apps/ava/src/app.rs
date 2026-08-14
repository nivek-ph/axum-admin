use std::time::Duration;

use anyhow::{Context, Result};
use audit::{AuditAnalyzer, AuditService};
use auth::{captcha::CaptchaService, password::PasswordService, token::TokenService};
use file_storage::files::{FileService, S3StorageConfig};
use iam::departments::DepartmentService;
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};
use tracing::{info, warn};

use crate::{ServeConfig, commands::serve::FileStorageDriver};

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
    // 1. standalone services (no cross-service deps)
    let password_service = PasswordService::new();
    let tokens = TokenService::new(&config.jwt_secret, redis_connection.clone());
    let captcha = CaptchaService::new(redis_connection.clone());
    let audits = AuditService::new(pool.clone());
    let dictionaries = DictionaryService::new(pool.clone());
    let parameters = ParameterService::new(pool.clone());
    let audit_analyzer = AuditAnalyzer::new(&config.ollama_base_url, &config.ollama_model);
    let files = match config.file_storage_driver {
        FileStorageDriver::Local => {
            FileService::local(pool.clone(), &config.file_storage_local_root)
        }
        FileStorageDriver::S3 => FileService::s3(
            pool.clone(),
            S3StorageConfig {
                bucket: config.s3_bucket.clone().unwrap_or_default(),
                region: config.s3_region.clone(),
                endpoint: config.s3_endpoint.clone(),
                root: config.s3_root.clone(),
                public_base_url: config.s3_public_base_url.clone().unwrap_or_default(),
                access_key_id: config.s3_access_key_id.clone(),
                secret_access_key: config.s3_secret_access_key.clone(),
                session_token: config.s3_session_token.clone(),
                enable_virtual_host_style: config.s3_virtual_host_style,
            },
        ),
    }
    .context("file storage should configure")?;

    // 2. authorization catalog (needed by IAM services below)
    let iam = iam::Iam::load(pool.clone())
        .await
        .context("IAM should initialize")?;
    if let Err(error) = iam.start_policy_sync(&config.redis_url, Duration::from_secs(30)) {
        warn!(
            error = ?error,
            "Casbin Redis watcher unavailable; periodic policy reload remains active"
        );
    }

    // 3. IAM services that depend on access
    let departments = DepartmentService::new(pool);

    Ok(api::AppState {
        redis: redis_connection,
        public_base_url: config.public_base_url(),
        tokens,
        captcha,
        passwords: password_service,
        accounts: iam.accounts,
        roles: iam.roles,
        departments,
        access: iam.access,
        dictionaries,
        parameters,
        menus: iam.menus,
        audits,
        audit_analyzer,
        files,
    })
}
