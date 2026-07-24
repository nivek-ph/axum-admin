use anyhow::Result;
use tracing::error;
use tracing_otel::Logger;

#[tokio::main]
async fn main() -> Result<()> {
    ava::install_crypto_provider();
    ava::load_env();

    let logger = Logger::from_env(Some("LOG"))?.with_ansi(true);
    let _guard = logger.init()?;

    let result = ava::cli::run().await;
    if let Err(err) = &result {
        error!("{err:#}");
    }
    result
}
