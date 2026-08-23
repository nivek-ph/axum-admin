use ava::{ServeConfig, app};
use tower::ServiceBuilder;
use tracing_otel::Logger;
use vercel_runtime::{Error, axum::VercelLayer};

#[tokio::main]
async fn main() -> Result<(), Error> {
    ava::install_crypto_provider();
    dotenv::dotenv().ok();

    let logger = Logger::from_env(Some("LOG"))?.with_ansi(false);
    let _guard = logger.init()?;

    let config = ServeConfig::from_env();
    let state = app::boot(&config).await?;
    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(api::router(state));
    vercel_runtime::run(app).await
}
