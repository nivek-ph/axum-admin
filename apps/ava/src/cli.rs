use clap::{Parser, Subcommand};

use crate::commands::{init, serve};

#[derive(Debug, Parser)]
#[command(name = "ava", version, about = "Axum Vue Admin command line")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Work seamlessly with ava from the command line.
///
/// See `ava --help` for more information.
#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(name = "serve", about = "Start the API server")]
    Serve(serve::ServeConfig),
    #[command(
        name = "init",
        about = "Initialize the database and administrator account"
    )]
    Init(init::InitConfig),
}

/// Parse CLI options and run the chosen command.
pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Serve(config) => serve::execute(config).await?,
        Command::Init(config) => init::execute(config).await?,
    }
    Ok(())
}
