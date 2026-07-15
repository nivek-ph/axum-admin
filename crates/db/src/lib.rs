use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub type DbPool = PgPool;

pub async fn connect(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}
