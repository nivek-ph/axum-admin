use anyhow::{Context, Result};
use audit::{login_logs::LoginLogService, operation_logs::OperationLogService};
use auth::{captcha::CaptchaService, password::PasswordService, token::TokenService};
use file_storage::files::FileService;
use iam::{
    access::AccessService, departments::DepartmentService, menus::MenuService, roles::RoleService,
    users::UserService,
};
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};
use tracing::info;

use crate::config::ServeConfig;

pub async fn run(config: ServeConfig) -> Result<()> {
    let pool = db::connect(&config.database_url)
        .await
        .context("database pool should connect")?;
    info!("database connected");

    let redis_client =
        redis::Client::open(config.redis_url.clone()).context("redis client should construct")?;
    let redis_connection = redis_client
        .get_multiplexed_async_connection()
        .await
        .context("redis connection should connect")?;

    info!("running database migrations");
    db::migrate(&pool)
        .await
        .context("database migrations should run")?;
    info!("database migrations complete");

    let password_service = PasswordService::new();
    let tokens = TokenService::new(&config.jwt_secret, redis_connection.clone());
    let captcha = CaptchaService::new(redis_connection.clone());
    let access = AccessService::load(pool.clone(), redis_connection)
        .await
        .context("authorization catalog and cache should initialize")?;
    let users = UserService::new(pool.clone(), password_service);
    let login_logs = LoginLogService::new(pool.clone());
    let operation_logs = OperationLogService::new(pool.clone());
    let state = api::AppState {
        tokens,
        captcha,
        users,
        roles: RoleService::with_access(pool.clone(), access.clone()),
        departments: DepartmentService::new(pool.clone()),
        access: access.clone(),
        dictionaries: DictionaryService::new(pool.clone()),
        parameters: ParameterService::new(pool.clone()),
        menus: MenuService::new(pool.clone(), access),
        login_logs,
        operation_logs,
        files: FileService::new(pool, "./uploads"),
    };

    api::serve(config.api_server(), state)
        .await
        .context("api server should run")
}
