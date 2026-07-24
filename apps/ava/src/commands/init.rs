use anyhow::{Context, Result};
use audit::AuditService;
use auth::password::PasswordService;
use clap::{Parser, builder::NonEmptyStringValueParser};
use iam::{access::AccessService, users::UserService};
use tracing::info;

#[derive(Debug, Clone, Parser)]
#[command(about = "Initialize the database and administrator account")]
pub struct InitConfig {
    /// Database URL
    #[arg(
        long,
        env = "DATABASE_URL",
        value_parser = NonEmptyStringValueParser::new(),
        hide_env_values = true
    )]
    pub database_url: String,

    /// Admin username
    #[arg(
        long,
        env = "ADMIN_USERNAME",
        default_value = "admin",
        hide_env_values = true
    )]
    pub admin_username: String,

    /// Admin password
    #[arg(
        long,
        env = "ADMIN_PASSWORD",
        value_parser = NonEmptyStringValueParser::new(),
        hide_env_values = true
    )]
    pub admin_password: String,

    /// Admin nickname
    #[arg(
        long,
        env = "ADMIN_NICKNAME",
        default_value = "Administrator",
        hide_env_values = true
    )]
    pub admin_nickname: String,
}

/// Execute the `init` command.
pub async fn execute(config: InitConfig) -> Result<()> {
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
