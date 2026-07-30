use anyhow::{Context, Result};
use auth::password::PasswordService;
use clap::{Parser, builder::NonEmptyStringValueParser};
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
pub(crate) async fn execute(config: InitConfig) -> Result<()> {
    info!("connecting to database");
    let pool = db::connect(&config.database_url)
        .await
        .context("database pool should connect")?;
    db::migrate(&pool)
        .await
        .context("database migrations should run")?;

    let iam = iam::Iam::load(pool)
        .await
        .context("IAM should initialize")?;
    let password_hash = PasswordService::new()
        .hash_password(&config.admin_password)
        .context("admin password should be hashed")?;
    iam.accounts
        .ensure_admin(
            &config.admin_username,
            password_hash,
            &config.admin_nickname,
        )
        .await
        .context("admin user should be initialized")?;
    info!("super administrator initialized");
    Ok(())
}
