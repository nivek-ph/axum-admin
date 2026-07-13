use anyhow::Result;
use auth::password::PasswordService;
use tracing::info;
use tracing_otel_extra::Logger;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let logger = Logger::from_env(Some("LOG"))?.with_ansi(true);
    let _guard = logger.init()?;

    let config =
        api::state::BootstrapConfig::from_env().expect("config should load from environment");

    info!("connecting to database");
    let pool = db::connect(&config.database_url)
        .await
        .expect("database pool should connect");
    info!("database connected");

    iam::users::service::UserService::new(pool, PasswordService::new())
        .ensure_admin(
            &config.admin_username,
            &config.admin_password,
            &config.admin_nickname,
        )
        .await
        .expect("admin user should be bootstrapped");
    info!("super administrator bootstrapped");
    Ok(())
}
