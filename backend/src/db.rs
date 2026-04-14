use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

pub async fn init(database_url: &str) -> Result<SqlitePool> {
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
    }

    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    tracing::info!("Database initialized at {}", database_url);
    Ok(pool)
}
