mod cli;
mod commands;
mod config;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use tracing_otel::Logger;

#[tokio::main]
async fn main() -> Result<()> {
    config::load_env_file();

    let logger = Logger::from_env(Some("LOG"))?.with_ansi(true);
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
