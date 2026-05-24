use std::sync::Arc;

use api::auth::session::AuthSessionService;
use auth::password::PasswordService;

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = api::state::AppConfig::from_env().expect("config should load from environment");
    info!(bind_addr = %config.bind_addr, "starting api bootstrap");
    info!("connecting to database");
    let pool = db::connect(&config.database_url)
        .await
        .expect("database pool should connect");
    info!("database connected");
    let redis_client =
        redis::Client::open(config.redis_url.clone()).expect("redis client should construct");
    let redis_connection = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("redis connection should connect");
    info!("running database migrations");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("database migrations should run");
    info!("database migrations complete");

    let password_service = PasswordService::new();
    let auth_session_service = AuthSessionService::new(&config.jwt_secret, redis_connection);

    let app_state = api::state::AppState {
        config: Arc::new(config.clone()),
        pool,
        auth_session_service,
        password_service,
    };

    let app = api::router::build_router(app_state);
    info!(bind_addr = %config.bind_addr, "binding api listener");
    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("listener should bind");
    info!(listen_addr = %listener.local_addr().expect("listener should expose local addr"), "api server listening");

    axum::serve(listener, app).await.expect("server should run");
}
