use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::{
    protocol::messages::{SetPresetRequest, SetTaskRequest},
    server::core::PomoServer,
};

pub struct HttpServer {
    server: Arc<PomoServer>,
}

impl HttpServer {
    pub fn new(server: PomoServer) -> Self {
        Self {
            server: Arc::new(server),
        }
    }

    // TODO: how to warn if we call wrongly?
    pub async fn start(&self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            .with_state(self.server.clone());

        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("HttpServer listening on {}", addr);

        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn ping_handler(State(server): State<Arc<PomoServer>>) -> Json<Value> {
    server.process_request(crate::protocol::Request::Ping).await;
    Json(json!({"message": "pong"}))
}

async fn get_status_handler(State(server): State<Arc<PomoServer>>) -> Json<Value> {
    let response = server
        .process_request(crate::protocol::Request::GetStatus)
        .await;
    Json(serde_json::to_value(response).unwrap())
}

async fn start_timer_handler(State(server): State<Arc<PomoServer>>) -> Json<Value> {
    server
        .process_request(crate::protocol::Request::Start)
        .await;
    Json(json!({"success": true}))
}

async fn pause_timer_handler(State(server): State<Arc<PomoServer>>) -> Json<Value> {
    server
        .process_request(crate::protocol::Request::Pause)
        .await;
    Json(json!({"success": true}))
}

async fn resume_timer_handler(State(server): State<Arc<PomoServer>>) -> Json<Value> {
    server
        .process_request(crate::protocol::Request::Resume)
        .await;
    Json(json!({"success": true}))
}

async fn reset_timer_handler(State(server): State<Arc<PomoServer>>) -> Json<Value> {
    server
        .process_request(crate::protocol::Request::Reset)
        .await;
    Json(json!({"success": true}))
}

async fn switch_mode_timer_handler(State(server): State<Arc<PomoServer>>) -> Json<Value> {
    server
        .process_request(crate::protocol::Request::SwitchMode)
        .await;
    Json(json!({"success": true}))
}

async fn set_task_handler(
    State(server): State<Arc<PomoServer>>,
    Json(req): Json<SetTaskRequest>,
) -> Json<Value> {
    server
        .process_request(crate::protocol::Request::SetTask(req.task))
        .await;
    Json(json!({"success": true}))
}

async fn set_preset_handler(
    State(server): State<Arc<PomoServer>>,
    Json(req): Json<SetPresetRequest>,
) -> Json<Value> {
    server
        .process_request(crate::protocol::Request::SetPreset(req.preset))
        .await;
    Json(json!({"success": true}))
}
