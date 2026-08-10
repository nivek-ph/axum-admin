use std::net::{IpAddr, SocketAddr};

use ava::{ServeConfig, app};
use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use tower::ServiceBuilder;
use vercel_runtime::{Error, axum::VercelLayer};

const VERCEL_FORWARDED_FOR: &str = "x-vercel-forwarded-for";

async fn inject_vercel_connect_info(mut request: Request, next: Next) -> Response {
    if let Some(client_ip) = request
        .headers()
        .get(VERCEL_FORWARDED_FOR)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<IpAddr>().ok())
    {
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::new(client_ip, 0)));
    }
    next.run(request).await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    ava::install_crypto_provider();
    let config = ServeConfig::from_env();
    let state = app::boot(&config).await?;
    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(api::router(state).layer(axum::middleware::from_fn(inject_vercel_connect_info)));
    vercel_runtime::run(app).await
}
