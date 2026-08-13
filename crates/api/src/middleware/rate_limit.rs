use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::Request,
    response::{IntoResponse, Response},
};
use redis::aio::MultiplexedConnection;
use tower_rate_limiter::{
    ClientIpKeyExtractor, RateLimitError, RateLimitLayer, RedisStore, ResponseFactory,
    ResponseReason, Store, StoreFailureMode,
};

use crate::mappings::{INTERNAL_SERVER_ERROR, RATE_LIMIT_UNAVAILABLE, RATE_LIMITED};

const WINDOW: Duration = Duration::from_secs(60);
// global policy and limit
const GLOBAL_POLICY: &str = "global";
const GLOBAL_LIMIT: u64 = 60; // 60 req/min

// captcha policy and limit
const CAPTCHA_POLICY: &str = "captcha";
const CAPTCHA_LIMIT: u64 = 3; // 3 req/min

#[derive(Clone, Copy, Debug, Default)]
struct ApiRateLimitResponseFactory;

impl ResponseFactory<Body, Body> for ApiRateLimitResponseFactory {
    fn build(&self, request: Request<Body>, reason: ResponseReason) -> Response<Body> {
        match reason {
            ResponseReason::RateLimited(_) => RATE_LIMITED.into_error().into_response(),
            ResponseReason::Error(error) => {
                tracing::error!(
                    method = %request.method(),
                    path = %request.uri().path(),
                    error = %error,
                    "rate-limit request failed",
                );
                let spec = match &error {
                    RateLimitError::Key(_, _) | RateLimitError::Quota(_, _) => {
                        INTERNAL_SERVER_ERROR
                    }
                    RateLimitError::Store(_, _) => RATE_LIMIT_UNAVAILABLE,
                };
                spec.into_error().with_source(error).into_response()
            }
        }
    }
}

/// Apply the Redis-backed rate limit shared by every route nested under `/api`.
pub(crate) fn apply_global<S>(router: Router<S>, connection: &MultiplexedConnection) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    apply_policy(
        router,
        RedisStore::new(connection.clone()),
        GLOBAL_POLICY,
        GLOBAL_LIMIT,
    )
}

/// Apply the stricter Redis-backed policy for CAPTCHA creation.
pub(crate) fn apply_captcha<S>(router: Router<S>, connection: &MultiplexedConnection) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    apply_policy(
        router,
        RedisStore::new(connection.clone()),
        CAPTCHA_POLICY,
        CAPTCHA_LIMIT,
    )
}

fn apply_policy<S, T>(
    router: Router<S>,
    store: T,
    policy_name: &'static str,
    limit: u64,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    T: Store + Clone + Send + Sync + 'static,
    T::Future: Send + 'static,
{
    let layer = RateLimitLayer::builder(ClientIpKeyExtractor::new())
        .policy_name(policy_name)
        .limit(limit)
        .window(WINDOW)
        .store_failure_mode(StoreFailureMode::Allow)
        .store_failure_tracing_level(tracing::Level::ERROR)
        .with_store(store)
        .response_factory(ApiRateLimitResponseFactory)
        .build()
        .expect("API rate-limit policy should be valid");

    router.layer(layer)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        future::{Ready, ready},
        net::SocketAddr,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use axum::{
        Router,
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{Request, StatusCode},
        response::Response,
        routing::get,
    };
    use tower::ServiceExt;
    use tower_rate_limiter::{RateLimitError, Store, Usage};

    use super::{CAPTCHA_LIMIT, CAPTCHA_POLICY, GLOBAL_LIMIT, GLOBAL_POLICY, apply_policy};

    #[derive(Clone, Default)]
    struct TestStore {
        usage_by_key: Arc<Mutex<HashMap<String, u64>>>,
    }

    impl Store for TestStore {
        type Future = Ready<Result<Usage, RateLimitError>>;

        fn increment(&self, key: &str, window: Duration) -> Self::Future {
            let mut usage_by_key = self.usage_by_key.lock().unwrap();
            let used = usage_by_key.entry(key.to_string()).or_default();
            *used += 1;
            ready(Ok(Usage {
                used: *used,
                reset_after: window,
            }))
        }
    }

    fn request(path: &str, ip: &str) -> Request<Body> {
        let peer = format!("{ip}:3000")
            .parse::<SocketAddr>()
            .expect("test peer address should be valid");
        Request::get(path)
            .extension(ConnectInfo(peer))
            .body(Body::empty())
            .expect("test request should build")
    }

    fn app() -> Router {
        let store = TestStore::default();
        let captcha = apply_policy(
            Router::new().route("/captcha", get(|| async { StatusCode::OK })),
            store.clone(),
            CAPTCHA_POLICY,
            CAPTCHA_LIMIT,
        );
        let api = Router::new()
            .route("/health", get(|| async { StatusCode::OK }))
            .merge(captcha);
        apply_policy(api, store, GLOBAL_POLICY, GLOBAL_LIMIT)
    }

    #[tokio::test]
    async fn global_policy_allows_one_hundred_requests_per_ip() {
        assert_eq!(GLOBAL_LIMIT, 100);
        let app = app();
        for _ in 0..GLOBAL_LIMIT {
            let response = app
                .clone()
                .oneshot(request("/health", "192.0.2.1"))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app.oneshot(request("/health", "192.0.2.1")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key("retry-after"));
        assert_eq!(
            response.headers().get("ratelimit-policy").unwrap(),
            "\"global\";q=100;w=60"
        );
    }

    #[tokio::test]
    async fn captcha_policy_allows_three_requests_per_ip() {
        let app = app();
        for _ in 0..CAPTCHA_LIMIT {
            let response = app
                .clone()
                .oneshot(request("/captcha", "192.0.2.2"))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app.oneshot(request("/captcha", "192.0.2.2")).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key("retry-after"));
        assert_eq!(
            response.headers().get("ratelimit-policy").unwrap(),
            "\"captcha\";q=3;w=60"
        );
    }

    #[tokio::test]
    async fn rate_limited_response_uses_shared_envelope() {
        let app = app();
        for _ in 0..CAPTCHA_LIMIT {
            let response = app
                .clone()
                .oneshot(request("/captcha", "192.0.2.3"))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app.oneshot(request("/captcha", "192.0.2.3")).await.unwrap();
        assert_rate_limited(response).await;
    }

    async fn assert_rate_limited(response: Response<Body>) {
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key("retry-after"));
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({
                "code": "RATE_LIMITED",
                "message": "too many requests",
                "data": null
            })
        );
    }
}
