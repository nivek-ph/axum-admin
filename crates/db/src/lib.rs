use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub type DbPool = PgPool;

pub async fn connect(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(15))
        .max_connections(5)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

#[cfg(test)]
mod tests {
    #[sqlx::test(migrations = "../../migrations")]
    async fn demo_seed_runs_against_the_fresh_schema(pool: sqlx::PgPool) {
        sqlx::raw_sql(include_str!("../../../docs/sql/init.sql"))
            .execute(&pool)
            .await
            .unwrap();
    }
}
