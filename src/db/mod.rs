use anyhow::Result;
use sqlx::{
    SqlitePool,
    pool::PoolOptions,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};
use std::{path::Path, time::Duration};

pub mod events;

pub async fn init(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = PoolOptions::<sqlx::Sqlite>::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_init_creates_db_and_runs_migrations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let pool = init(&db_path).await.unwrap();

        // migrations ran — events table must exist
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn test_init_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("nested/dir/test.db");

        init(&db_path).await.unwrap();
        assert!(db_path.exists());
    }
}
