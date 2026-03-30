use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct TodoRow {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub done: i32,
    pub priority: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn insert_todo(
    pool: &SqlitePool,
    parent_id: Option<&str>,
    title: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Get next sort_order among siblings
    let max_order: (i64,) = if let Some(pid) = parent_id {
        sqlx::query_as("SELECT COALESCE(MAX(sort_order), -1) FROM todos WHERE parent_id = ?")
            .bind(pid)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_as("SELECT COALESCE(MAX(sort_order), -1) FROM todos WHERE parent_id IS NULL")
            .fetch_one(pool)
            .await?
    };
    let sort_order = max_order.0 + 1;

    sqlx::query(
        "INSERT INTO todos (id, parent_id, title, done, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, 0, ?, ?, ?)",
    )
    .bind(&id)
    .bind(parent_id)
    .bind(title)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(id)
}

pub async fn update_todo_title(pool: &SqlitePool, id: &str, title: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    sqlx::query("UPDATE todos SET title = ?, updated_at = ? WHERE id = ?")
        .bind(title)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn toggle_todo_done(pool: &SqlitePool, id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    sqlx::query("UPDATE todos SET done = 1 - done, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_todo(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM todos WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_all_todos(pool: &SqlitePool) -> Result<Vec<TodoRow>> {
    let rows = sqlx::query_as::<_, TodoRow>(
        "SELECT id, parent_id, title, done, priority, sort_order, created_at, updated_at \
         FROM todos ORDER BY priority ASC, sort_order ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn cycle_todo_priority(pool: &SqlitePool, id: &str) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT priority FROM todos WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    let new_priority = match row.0.as_str() {
        "B" => "A",
        "A" => "C",
        "C" => "B",
        _ => "B",
    };
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    sqlx::query("UPDATE todos SET priority = ?, updated_at = ? WHERE id = ?")
        .bind(new_priority)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(new_priority.to_string())
}

pub async fn link_todo_session(pool: &SqlitePool, todo_id: &str, session_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    sqlx::query(
        "INSERT OR IGNORE INTO todo_sessions (todo_id, session_id, linked_at) VALUES (?, ?, ?)",
    )
    .bind(todo_id)
    .bind(session_id)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct TodoStats {
    pub session_count: i64,
    pub total_work_secs: i64,
}

pub async fn get_todo_stats(pool: &SqlitePool, todo_id: &str) -> Result<TodoStats> {
    let row: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(s.work_secs), 0) \
         FROM todo_sessions ts \
         JOIN sessions s ON ts.session_id = s.session_id \
         WHERE ts.todo_id = ? AND s.final_event = 'Completed'",
    )
    .bind(todo_id)
    .fetch_one(pool)
    .await?;
    Ok(TodoStats {
        session_count: row.0,
        total_work_secs: row.1,
    })
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DailyStats {
    pub date: String,
    pub session_count: i64,
    pub total_work_secs: i64,
}

pub async fn get_daily_stats(pool: &SqlitePool, days: u32) -> Result<Vec<DailyStats>> {
    let rows = sqlx::query_as::<_, DailyStats>(
        "SELECT DATE(started_at) as date, \
         COUNT(*) as session_count, \
         COALESCE(SUM(work_secs), 0) as total_work_secs \
         FROM sessions \
         WHERE timer_type = 'Work' AND final_event = 'Completed' \
         AND started_at >= DATE('now', '-' || ? || ' days') \
         GROUP BY DATE(started_at) \
         ORDER BY date DESC",
    )
    .bind(days as i64)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_session_count_for_todo(pool: &SqlitePool, todo_id: &str) -> Result<i64> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todo_sessions WHERE todo_id = ?")
        .bind(todo_id)
        .fetch_one(pool)
        .await?;
    Ok(count.0)
}

pub async fn get_latest_session_id(pool: &SqlitePool) -> Result<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT session_id FROM events ORDER BY at DESC, id DESC LIMIT 1")
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::pool::PoolOptions;

    async fn test_pool() -> SqlitePool {
        let pool = PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_insert_and_get_todos() {
        let pool = test_pool().await;
        let id = insert_todo(&pool, None, "Root task").await.unwrap();
        let todos = get_all_todos(&pool).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].id, id);
        assert_eq!(todos[0].title, "Root task");
        assert_eq!(todos[0].done, 0);
        assert!(todos[0].parent_id.is_none());
    }

    #[tokio::test]
    async fn test_insert_child() {
        let pool = test_pool().await;
        let parent_id = insert_todo(&pool, None, "Parent").await.unwrap();
        let child_id = insert_todo(&pool, Some(&parent_id), "Child").await.unwrap();
        let todos = get_all_todos(&pool).await.unwrap();
        assert_eq!(todos.len(), 2);
        let child = todos.iter().find(|t| t.id == child_id).unwrap();
        assert_eq!(child.parent_id.as_deref(), Some(parent_id.as_str()));
    }

    #[tokio::test]
    async fn test_sort_order_auto_increments() {
        let pool = test_pool().await;
        insert_todo(&pool, None, "First").await.unwrap();
        insert_todo(&pool, None, "Second").await.unwrap();
        insert_todo(&pool, None, "Third").await.unwrap();
        let todos = get_all_todos(&pool).await.unwrap();
        assert_eq!(todos[0].sort_order, 0);
        assert_eq!(todos[1].sort_order, 1);
        assert_eq!(todos[2].sort_order, 2);
    }

    #[tokio::test]
    async fn test_toggle_done() {
        let pool = test_pool().await;
        let id = insert_todo(&pool, None, "Task").await.unwrap();
        toggle_todo_done(&pool, &id).await.unwrap();
        let todos = get_all_todos(&pool).await.unwrap();
        assert_eq!(todos[0].done, 1);
        toggle_todo_done(&pool, &id).await.unwrap();
        let todos = get_all_todos(&pool).await.unwrap();
        assert_eq!(todos[0].done, 0);
    }

    #[tokio::test]
    async fn test_update_title() {
        let pool = test_pool().await;
        let id = insert_todo(&pool, None, "Old title").await.unwrap();
        update_todo_title(&pool, &id, "New title").await.unwrap();
        let todos = get_all_todos(&pool).await.unwrap();
        assert_eq!(todos[0].title, "New title");
    }

    #[tokio::test]
    async fn test_delete_cascades_children() {
        let pool = test_pool().await;
        // Enable foreign keys for CASCADE
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        let parent_id = insert_todo(&pool, None, "Parent").await.unwrap();
        insert_todo(&pool, Some(&parent_id), "Child").await.unwrap();
        delete_todo(&pool, &parent_id).await.unwrap();
        let todos = get_all_todos(&pool).await.unwrap();
        assert!(todos.is_empty());
    }

    #[tokio::test]
    async fn test_link_session() {
        let pool = test_pool().await;
        let todo_id = insert_todo(&pool, None, "Task").await.unwrap();
        link_todo_session(&pool, &todo_id, "session-1")
            .await
            .unwrap();
        link_todo_session(&pool, &todo_id, "session-2")
            .await
            .unwrap();
        // Duplicate should be ignored
        link_todo_session(&pool, &todo_id, "session-1")
            .await
            .unwrap();
        let count = get_session_count_for_todo(&pool, &todo_id).await.unwrap();
        assert_eq!(count, 2);
    }
}
