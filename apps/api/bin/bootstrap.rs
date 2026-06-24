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

    info!("connecting to database");
    let pool = db::connect(&config.database_url)
        .await
        .expect("database pool should connect");
    info!("database connected");

    system::authority::ensure_default_authority(&pool)
        .await
        .expect("default authority should be bootstrapped");
    system::menu::ensure_default_menu(&pool)
        .await
        .expect("default menu should be bootstrapped");
    let password_service = PasswordService::new();
    system::users::ensure_admin_user(
        &pool,
        &password_service,
        &config.admin_username,
        &config.admin_password,
        &config.admin_nickname,
    )
    .await
    .expect("admin user should be bootstrapped");

    info!("default system data bootstrapped");
}
