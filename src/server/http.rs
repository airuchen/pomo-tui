// Copyright (c) 2025 Yu-Wen Chen
// Licensed under the MIT License (see LICENSE file)

use anyhow::Result;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{delete, get, post, put},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::{
    db,
    protocol::messages::{SetPresetRequest, SetTaskRequest},
    server::core::PomoServer,
};

#[derive(Clone)]
pub struct AppState {
    pub server: Arc<PomoServer>,
    pub pool: SqlitePool,
}

pub struct HttpServer {
    state: AppState,
}

impl HttpServer {
    pub fn new(server: Arc<PomoServer>, pool: SqlitePool) -> Self {
        Self {
            state: AppState { server, pool },
        }
    }

    pub async fn start(&self, addr: &str) -> Result<()> {
        let app = Router::new()
            .route("/ping", get(ping_handler))
            .route("/timer/status", get(get_status_handler))
            .route("/timer/start", post(start_timer_handler))
            .route("/timer/pause", post(pause_timer_handler))
            .route("/timer/resume", post(resume_timer_handler))
            .route("/timer/reset", post(reset_timer_handler))
            .route("/timer/switch", post(switch_mode_timer_handler))
            .route("/timer/task", put(set_task_handler))
            .route("/timer/preset", put(set_preset_handler))
            .route("/timer/history", get(get_history_handler))
            // Todo endpoints
            .route("/todos", get(get_todos_handler))
            .route("/todos", post(create_todo_handler))
            .route("/todos/{id}", put(update_todo_handler))
            .route("/todos/{id}", delete(delete_todo_handler))
            .route("/todos/{id}/toggle", post(toggle_todo_handler))
            .route("/todos/{id}/priority", post(cycle_todo_priority_handler))
            .route("/todos/{id}/stats", get(get_todo_stats_handler))
            // Stats
            .route("/stats/daily", get(get_daily_stats_handler))
            // Dashboard
            .route("/", get(dashboard_handler))
            .with_state(self.state.clone());

        let listener = TcpListener::bind(addr).await?;
        eprintln!("HttpServer listening on {}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn ping_handler(State(state): State<AppState>) -> Json<Value> {
    state
        .server
        .process_request(crate::protocol::Request::Ping)
        .await;
    Json(json!({"message": "pong"}))
}

async fn get_status_handler(State(state): State<AppState>) -> Json<Value> {
    let response = state
        .server
        .process_request(crate::protocol::Request::GetStatus)
        .await;
    Json(serde_json::to_value(response).unwrap())
}

async fn start_timer_handler(State(state): State<AppState>) -> Json<Value> {
    state
        .server
        .process_request(crate::protocol::Request::Start)
        .await;
    Json(json!({"success": true}))
}

async fn pause_timer_handler(State(state): State<AppState>) -> Json<Value> {
    state
        .server
        .process_request(crate::protocol::Request::Pause)
        .await;
    Json(json!({"success": true}))
}

async fn resume_timer_handler(State(state): State<AppState>) -> Json<Value> {
    state
        .server
        .process_request(crate::protocol::Request::Resume)
        .await;
    Json(json!({"success": true}))
}

async fn reset_timer_handler(State(state): State<AppState>) -> Json<Value> {
    state
        .server
        .process_request(crate::protocol::Request::Reset)
        .await;
    Json(json!({"success": true}))
}

async fn switch_mode_timer_handler(State(state): State<AppState>) -> Json<Value> {
    state
        .server
        .process_request(crate::protocol::Request::SwitchMode)
        .await;
    Json(json!({"success": true}))
}

async fn set_task_handler(
    State(state): State<AppState>,
    Json(req): Json<SetTaskRequest>,
) -> Json<Value> {
    state
        .server
        .process_request(crate::protocol::Request::SetTask(req.task))
        .await;
    Json(json!({"success": true}))
}

async fn set_preset_handler(
    State(state): State<AppState>,
    Json(req): Json<SetPresetRequest>,
) -> Json<Value> {
    state
        .server
        .process_request(crate::protocol::Request::SetPreset(req.preset))
        .await;
    Json(json!({"success": true}))
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<u32>,
}

async fn get_history_handler(
    State(state): State<AppState>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let limit = params.limit.unwrap_or(20);
    match db::events::get_sessions(&state.pool, limit).await {
        Ok(sessions) => Ok(Json(serde_json::to_value(sessions).unwrap())),
        Err(e) => {
            log::error!("History query failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

// --- Dashboard ---

async fn dashboard_handler() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

// --- Todo endpoints ---

#[derive(Deserialize)]
struct CreateTodoRequest {
    title: String,
    parent_id: Option<String>,
}

#[derive(Deserialize)]
struct UpdateTodoRequest {
    title: String,
}

async fn get_todos_handler(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match db::todos::get_all_todos(&state.pool).await {
        Ok(todos) => {
            let items: Vec<Value> = todos
                .into_iter()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "parent_id": t.parent_id,
                        "title": t.title,
                        "done": t.done != 0,
                        "priority": t.priority,
                        "sort_order": t.sort_order,
                        "created_at": t.created_at,
                        "updated_at": t.updated_at,
                    })
                })
                .collect();
            Ok(Json(json!(items)))
        }
        Err(e) => {
            log::error!("Get todos failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

async fn create_todo_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateTodoRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match db::todos::insert_todo(&state.pool, req.parent_id.as_deref(), &req.title).await {
        Ok(id) => Ok(Json(json!({"id": id}))),
        Err(e) => {
            log::error!("Create todo failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

async fn update_todo_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTodoRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match db::todos::update_todo_title(&state.pool, &id, &req.title).await {
        Ok(()) => Ok(Json(json!({"success": true}))),
        Err(e) => {
            log::error!("Update todo failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

async fn delete_todo_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match db::todos::delete_todo(&state.pool, &id).await {
        Ok(()) => Ok(Json(json!({"success": true}))),
        Err(e) => {
            log::error!("Delete todo failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

async fn toggle_todo_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match db::todos::toggle_todo_done(&state.pool, &id).await {
        Ok(()) => Ok(Json(json!({"success": true}))),
        Err(e) => {
            log::error!("Toggle todo failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

async fn cycle_todo_priority_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match db::todos::cycle_todo_priority(&state.pool, &id).await {
        Ok(new_priority) => Ok(Json(json!({"priority": new_priority}))),
        Err(e) => {
            log::error!("Cycle priority failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

async fn get_todo_stats_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match db::todos::get_todo_stats(&state.pool, &id).await {
        Ok(stats) => Ok(Json(serde_json::to_value(stats).unwrap())),
        Err(e) => {
            log::error!("Todo stats failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

// --- Stats endpoints ---

#[derive(Deserialize)]
struct DailyStatsQuery {
    days: Option<u32>,
}

async fn get_daily_stats_handler(
    State(state): State<AppState>,
    Query(params): Query<DailyStatsQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let days = params.days.unwrap_or(30);
    match db::todos::get_daily_stats(&state.pool, days).await {
        Ok(stats) => Ok(Json(serde_json::to_value(stats).unwrap())),
        Err(e) => {
            log::error!("Daily stats failed: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer::{LogEvent, TimerMode};
    use axum::{body::Body, http::Request};
    use chrono::Local;
    use sqlx::pool::PoolOptions;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn test_app() -> (Router, SqlitePool) {
        let pool = PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let server = Arc::new(PomoServer::new(pool.clone()));
        let state = AppState {
            server,
            pool: pool.clone(),
        };
        let app = Router::new()
            .route("/timer/history", get(get_history_handler))
            .with_state(state);
        (app, pool)
    }

    #[tokio::test]
    async fn test_history_empty() {
        let (app, _pool) = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/timer/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_history_returns_session() {
        let (app, pool) = test_app().await;
        let id = Uuid::new_v4();
        crate::db::events::insert_event(
            &pool,
            &LogEvent::Started {
                id,
                timer_type: TimerMode::Work,
                task: "test".into(),
                at: Local::now(),
                remaining: 1500,
            },
        )
        .await
        .unwrap();
        crate::db::events::insert_event(
            &pool,
            &LogEvent::Completed {
                id,
                task: "test".into(),
                at: Local::now(),
                work_secs: 1500,
            },
        )
        .await
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/timer/history?limit=5")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["timer_type"], "Work");
        assert_eq!(sessions[0]["task"], "test");
        assert_eq!(sessions[0]["final_event"], "Completed");
    }
}
