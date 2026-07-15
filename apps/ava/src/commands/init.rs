use anyhow::{Context, Result};
use auth::password::PasswordService;
use tracing::info;

use crate::config::InitConfig;

pub async fn run(config: InitConfig) -> Result<()> {
    info!("connecting to database");
    let pool = db::connect(&config.database_url)
        .await
        .context("database pool should connect")?;
    db::migrate(&pool)
        .await
        .context("database migrations should run")?;

    iam::users::UserService::new(pool, PasswordService::new())
        .ensure_admin(
            &config.admin_username,
            &config.admin_password,
            &config.admin_nickname,
        )
        .await
        .context("admin user should be initialized")?;
    info!("super administrator initialized");
    Ok(())
}
