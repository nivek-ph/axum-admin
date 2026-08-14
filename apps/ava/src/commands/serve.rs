use anyhow::{Context, Result};
use clap::{Parser, ValueEnum, builder::NonEmptyStringValueParser};

use crate::app;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum FileStorageDriver {
    Local,
    S3,
}

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

    /// File storage adapter
    #[arg(long, env = "FILE_STORAGE_DRIVER", default_value = "local", value_enum)]
    pub(crate) file_storage_driver: FileStorageDriver,

    /// Local file storage directory
    #[arg(
        long,
        env = "FILE_STORAGE_LOCAL_ROOT",
        default_value = "./uploads",
        hide_env_values = true
    )]
    pub(crate) file_storage_local_root: String,

    /// S3 bucket name
    #[arg(long, env = "S3_BUCKET", hide_env_values = true)]
    pub(crate) s3_bucket: Option<String>,

    /// S3 region
    #[arg(long, env = "S3_REGION", hide_env_values = true)]
    pub(crate) s3_region: Option<String>,

    /// S3-compatible endpoint
    #[arg(long, env = "S3_ENDPOINT", hide_env_values = true)]
    pub(crate) s3_endpoint: Option<String>,

    /// Object prefix inside the S3 bucket
    #[arg(
        long,
        env = "S3_ROOT",
        default_value = "uploads",
        hide_env_values = true
    )]
    pub(crate) s3_root: String,

    /// Public URL for the S3 bucket or custom domain
    #[arg(long, env = "S3_PUBLIC_BASE_URL", hide_env_values = true)]
    pub(crate) s3_public_base_url: Option<String>,

    /// S3 access key ID
    #[arg(long, env = "AWS_ACCESS_KEY_ID", hide_env_values = true)]
    pub(crate) s3_access_key_id: Option<String>,

    /// S3 secret access key
    #[arg(long, env = "AWS_SECRET_ACCESS_KEY", hide_env_values = true)]
    pub(crate) s3_secret_access_key: Option<String>,

    /// Temporary S3 session token
    #[arg(long, env = "AWS_SESSION_TOKEN", hide_env_values = true)]
    pub(crate) s3_session_token: Option<String>,

    /// Use virtual-hosted-style S3 URLs
    #[arg(long, env = "S3_VIRTUAL_HOST_STYLE", default_value_t = false)]
    pub(crate) s3_virtual_host_style: bool,

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
