use anyhow::{Context, Result};
use clap::{Parser, builder::NonEmptyStringValueParser};

use crate::app;

#[derive(Debug, Clone, Parser)]
#[command(about = "Start the API server")]
pub struct ServeConfig {
    /// HTTP listen port
    #[arg(
        short = 'p',
        long = "port",
        env = "HTTP_PORT",
        default_value_t = 3000,
        hide_env_values = true
    )]
    pub http_port: u16,

    /// Public base URL for links and Swagger (defaults to http://127.0.0.1:<port>)
    #[arg(
        long,
        env = "PUBLIC_BASE_URL",
        default_value = "",
        hide_env_values = true
    )]
    public_base_url: String,

    /// Database URL
    #[arg(
        long,
        env = "DATABASE_URL",
        value_parser = NonEmptyStringValueParser::new(),
        hide_env_values = true
    )]
    pub(crate) database_url: String,

    /// Redis URL
    #[arg(
        long,
        env = "REDIS_URL",
        value_parser = NonEmptyStringValueParser::new(),
        hide_env_values = true
    )]
    pub(crate) redis_url: String,

    /// JWT signing secret
    #[arg(
        long,
        env = "JWT_SECRET",
        value_parser = NonEmptyStringValueParser::new(),
        hide_env_values = true
    )]
    pub(crate) jwt_secret: String,

    /// Ollama OpenAI-compatible base URL
    #[arg(
        long,
        env = "OLLAMA_BASE_URL",
        default_value = "",
        hide_env_values = true
    )]
    pub(crate) ollama_base_url: String,

    /// Ollama model name
    #[arg(long, env = "OLLAMA_MODEL", default_value = "", hide_env_values = true)]
    pub(crate) ollama_model: String,
}

impl ServeConfig {
    /// Parse from environment variables (and clap defaults).
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self::parse()
    }

    /// Public base URL, falling back to `http://127.0.0.1:<port>` when unset.
    pub(crate) fn public_base_url(&self) -> String {
        let trimmed = self.public_base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            format!("http://127.0.0.1:{}", self.http_port)
        } else {
            trimmed.to_string()
        }
    }
}

/// Execute the `serve` command.
pub(crate) async fn execute(config: ServeConfig) -> Result<()> {
    let server_config = api::ServerConfig {
        listen_addr: format!("0.0.0.0:{}", config.http_port),
        public_url: config.public_base_url(),
    };
    let state = app::boot(&config).await?;
    api::serve(server_config, state)
        .await
        .context("api server should run")
}
