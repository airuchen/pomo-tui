use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::timer::LogEvent;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SessionRow {
    pub session_id: String,
    pub timer_type: Option<String>,
    pub task: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub work_secs: Option<i64>,
    pub final_event: Option<String>,
}

pub async fn insert_event(pool: &SqlitePool, event: &LogEvent) -> Result<()> {
    let (session_id, event_type, timer_type, task, at, remaining_secs, work_secs) = match event {
        LogEvent::Idle => return Ok(()),
        LogEvent::Started {
            id,
            timer_type,
            task,
            at,
            remaining,
        } => (
            id.to_string(),
            "Started",
            Some(timer_type.to_string()),
            task.as_str(),
            at.with_timezone(&Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            Some(*remaining as i64),
            None::<i64>,
        ),
        LogEvent::Paused {
            id,
            task,
            at,
            remaining,
        } => (
            id.to_string(),
            "Paused",
            None,
            task.as_str(),
            at.with_timezone(&Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            Some(*remaining as i64),
            None,
        ),
        LogEvent::Resumed {
            id,
            task,
            at,
            remaining,
        } => (
            id.to_string(),
            "Resumed",
            None,
            task.as_str(),
            at.with_timezone(&Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            Some(*remaining as i64),
            None,
        ),
        LogEvent::Terminated {
            id,
            task,
            at,
            remaining,
            work_secs,
        } => (
            id.to_string(),
            "Terminated",
            None,
            task.as_str(),
            at.with_timezone(&Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            Some(*remaining as i64),
            Some(*work_secs as i64),
        ),
        LogEvent::Completed {
            id,
            task,
            at,
            work_secs,
        } => (
            id.to_string(),
            "Completed",
            None,
            task.as_str(),
            at.with_timezone(&Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            None::<i64>,
            Some(*work_secs as i64),
        ),
    };

    sqlx::query(
        "INSERT INTO events \
         (session_id, event_type, timer_type, task, at, remaining_secs, work_secs) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(event_type)
    .bind(timer_type)
    .bind(task)
    .bind(at)
    .bind(remaining_secs)
    .bind(work_secs)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_sessions(pool: &SqlitePool, limit: u32) -> Result<Vec<SessionRow>> {
    let limit = limit.clamp(1, 100) as i64;
    let rows = sqlx::query_as::<_, SessionRow>(
        "SELECT session_id, timer_type, task, started_at, ended_at, work_secs, final_event \
         FROM sessions \
         ORDER BY started_at DESC \
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer::{LogEvent, TimerMode};
    use chrono::Local;
    use sqlx::pool::PoolOptions;
    use uuid::Uuid;

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
    async fn test_idle_is_skipped() {
        let pool = test_pool().await;
        insert_event(&pool, &LogEvent::Idle).await.unwrap();
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn test_insert_started() {
        let pool = test_pool().await;
        let id = Uuid::new_v4();
        insert_event(
            &pool,
            &LogEvent::Started {
                id,
                timer_type: TimerMode::Work,
                task: "test task".into(),
                at: Local::now(),
                remaining: 1500,
            },
        )
        .await
        .unwrap();

        let row: (String, Option<String>, String, Option<i64>, Option<i64>) = sqlx::query_as(
            "SELECT event_type, timer_type, task, remaining_secs, work_secs \
                 FROM events WHERE session_id = ?",
        )
        .bind(id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Started");
        assert_eq!(row.1.as_deref(), Some("Work"));
        assert_eq!(row.2, "test task");
        assert_eq!(row.3, Some(1500_i64));
        assert!(row.4.is_none());
    }

    #[tokio::test]
    async fn test_insert_completed() {
        let pool = test_pool().await;
        let id = Uuid::new_v4();
        insert_event(
            &pool,
            &LogEvent::Completed {
                id,
                task: "done".into(),
                at: Local::now(),
                work_secs: 1500,
            },
        )
        .await
        .unwrap();

        let row: (String, Option<i64>, Option<i64>) = sqlx::query_as(
            "SELECT event_type, remaining_secs, work_secs \
                 FROM events WHERE session_id = ?",
        )
        .bind(id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "Completed");
        assert!(row.1.is_none(), "Completed must have NULL remaining_secs");
        assert_eq!(row.2, Some(1500_i64));
    }

    #[tokio::test]
    async fn test_insert_all_variants() {
        let pool = test_pool().await;
        let id = Uuid::new_v4();
        let now = Local::now();

        let events = vec![
            LogEvent::Started {
                id,
                timer_type: TimerMode::Work,
                task: "t".into(),
                at: now,
                remaining: 1500,
            },
            LogEvent::Paused {
                id,
                task: "t".into(),
                at: now,
                remaining: 1200,
            },
            LogEvent::Resumed {
                id,
                task: "t".into(),
                at: now,
                remaining: 1200,
            },
            LogEvent::Terminated {
                id,
                task: "t".into(),
                at: now,
                remaining: 100,
                work_secs: 1400,
            },
        ];

        for e in &events {
            insert_event(&pool, e).await.unwrap();
        }

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 4);
    }

    #[tokio::test]
    async fn test_get_sessions_full() {
        let pool = test_pool().await;
        let id = Uuid::new_v4();
        let now = Local::now();

        insert_event(
            &pool,
            &LogEvent::Started {
                id,
                timer_type: TimerMode::Work,
                task: "my task".into(),
                at: now,
                remaining: 1500,
            },
        )
        .await
        .unwrap();
        insert_event(
            &pool,
            &LogEvent::Completed {
                id,
                task: "my task".into(),
                at: now,
                work_secs: 1500,
            },
        )
        .await
        .unwrap();

        let sessions = get_sessions(&pool, 20).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, id.to_string());
        assert_eq!(sessions[0].timer_type.as_deref(), Some("Work"));
        assert_eq!(sessions[0].task.as_deref(), Some("my task"));
        assert_eq!(sessions[0].work_secs, Some(1500));
        assert_eq!(sessions[0].final_event.as_deref(), Some("Completed"));
    }

    #[tokio::test]
    async fn test_get_sessions_limit() {
        let pool = test_pool().await;
        let now = Local::now();
        for _ in 0..5 {
            let id = Uuid::new_v4();
            insert_event(
                &pool,
                &LogEvent::Started {
                    id,
                    timer_type: TimerMode::Work,
                    task: "t".into(),
                    at: now,
                    remaining: 1500,
                },
            )
            .await
            .unwrap();
        }
        let sessions = get_sessions(&pool, 3).await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_get_sessions_clamps_zero_to_one() {
        let pool = test_pool().await;
        let id = Uuid::new_v4();
        insert_event(
            &pool,
            &LogEvent::Started {
                id,
                timer_type: TimerMode::Work,
                task: "t".into(),
                at: Local::now(),
                remaining: 1500,
            },
        )
        .await
        .unwrap();
        let sessions = get_sessions(&pool, 0).await.unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_get_sessions_clamps_to_100() {
        let pool = test_pool().await;
        let now = Local::now();
        for _ in 0..105 {
            let id = Uuid::new_v4();
            insert_event(
                &pool,
                &LogEvent::Started {
                    id,
                    timer_type: TimerMode::Work,
                    task: "t".into(),
                    at: now,
                    remaining: 1500,
                },
            )
            .await
            .unwrap();
        }
        let sessions = get_sessions(&pool, 200).await.unwrap();
        assert_eq!(sessions.len(), 100);
    }
}
