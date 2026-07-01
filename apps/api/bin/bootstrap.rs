use anyhow::Result;
use auth::password::PasswordService;
use tracing::info;
use tracing_otel_extra::Logger;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let logger = Logger::from_env(Some("LOG"))?.with_ansi(true);
    let _guard = logger.init()?;

    let config = api::state::AppConfig::from_env().expect("config should load from environment");

    info!("connecting to database");
    let pool = db::connect(&config.database_url)
        .await
        .expect("database pool should connect");
    info!("database connected");

    system::roles::ensure_builtin_roles(&pool)
        .await
        .expect("builtin roles should be bootstrapped");
    system::roles::ensure_builtin_role_permissions(&pool)
        .await
        .expect("builtin role permissions should be bootstrapped");
    system::menu::ensure_default_menu(&pool)
        .await
        .expect("default menu should be bootstrapped");
    let password_service = PasswordService::new();
    system::users::ensure_admin_user(
        &pool,
        &password_service,
        &config.admin_username,
        &config.admin_password,
        &config.admin_nickname,
    )
    .await
    .expect("admin user should be bootstrapped");
    system::users::ensure_builtin_user(&pool, &password_service, "dev", "123456", "Dev", 2)
        .await
        .expect("dev user should be bootstrapped");
    system::users::ensure_builtin_user(&pool, &password_service, "ops", "123456", "Ops", 3)
        .await
        .expect("ops user should be bootstrapped");

    info!("default system data bootstrapped");
    Ok(())
}
