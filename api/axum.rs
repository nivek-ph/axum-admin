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

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{Request, StatusCode},
        middleware,
        routing::get,
    };
    use tower::ServiceExt;

    use super::{SocketAddr, VERCEL_FORWARDED_FOR, inject_vercel_connect_info};

    #[tokio::test]
    async fn vercel_forwarded_for_is_exposed_as_connect_info() {
        let app =
            Router::new()
                .route(
                    "/",
                    get(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move {
                        peer.ip().to_string()
                    }),
                )
                .layer(middleware::from_fn(inject_vercel_connect_info));
        let request = Request::get("/")
            .header(VERCEL_FORWARDED_FOR, "192.0.2.10")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            "192.0.2.10"
        );
    }
}
