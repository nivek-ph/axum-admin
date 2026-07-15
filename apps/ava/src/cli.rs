use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ava", version, about = "Axum Vue Admin command line")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the HTTP API server.
    Serve,
    /// Initialize the database and administrator account.
    Init,
}
