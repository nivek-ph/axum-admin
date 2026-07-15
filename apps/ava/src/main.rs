mod cli;
mod commands;
mod config;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use tracing_otel_extra::Logger;

#[tokio::main]
async fn main() -> Result<()> {
    config::load_env_file();
    let logger_config = config::LoggerConfig::from_env()?;

    let logger = Logger::new("ava")
        .with_level(logger_config.log_level)
        .with_ansi(true);
    let _guard = logger.init()?;

    let cli = Cli::parse();

    match cli.command {
        Command::Serve => {
            let config = config::ServeConfig::from_env()?;
            commands::serve::run(config).await
        }
        Command::Init => {
            let config = config::InitConfig::from_env()?;
            commands::init::run(config).await
        }
    }
}
