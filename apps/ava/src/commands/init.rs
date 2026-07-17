use anyhow::{Context, Result};
use audit::AuditService;
use auth::password::PasswordService;
use iam::{access::AccessService, users::UserService};
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

    let access = AccessService::new(pool.clone());
    let audit = AuditService::new(pool.clone());
    UserService::new(pool, access, audit, PasswordService::new())
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
