// Copyright (c) 2025 Yu-Wen Chen
// Licensed under the MIT License (see LICENSE file)

use crate::db;
use crate::protocol::{Request, Response};
use crate::timer::Timer;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, interval};

pub struct PomoServer {
    timer: Arc<Mutex<Timer>>,
}

impl PomoServer {
    pub fn new(pool: SqlitePool) -> Self {
        let timer = Arc::new(Mutex::new(Timer::new()));
        let timer_clone = timer.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let events: Vec<crate::timer::LogEvent> = {
                    let mut timer_guard = timer_clone.lock().await;
                    timer_guard.update();
                    timer_guard.drain_events().collect()
                }; // MutexGuard dropped here
                for event in events {
                    if let Err(e) = db::events::insert_event(&pool, &event).await {
                        log::error!("Failed to persist event: {e}");
                    }
                }
            }
        });

        Self { timer }
    }

    pub async fn process_request(&self, request: Request) -> Response {
        let mut timer = self.timer.lock().await;
        match request {
            Request::Ping => Response::Pong,
            Request::Start => {
                if timer.is_idle() || timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
            Request::Pause => {
                if timer.is_running() && !timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
            Request::Reset => {
                timer.reset();
                Response::Ok
            }
            Request::SetTask(name) => {
                timer.set_task_name(&name);
                Response::Ok
            }
            Request::GetStatus => Response::Status(timer.get_timer_status()),
            Request::SetPreset(preset) => {
                timer.set_preset(preset);
                Response::Ok
            }
            Request::SwitchMode => {
                timer.switch_mode();
                Response::Ok
            }
            Request::Resume => {
                if timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Request, Response};
    use sqlx::pool::PoolOptions;

    async fn test_server() -> PomoServer {
        let pool = PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        PomoServer::new(pool)
    }

    #[tokio::test]
    async fn test_ping() {
        let server = test_server().await;
        let response = server.process_request(Request::Ping).await;
        assert!(matches!(response, Response::Pong));
    }

    #[tokio::test]
    async fn test_preset() {
        use crate::timer::Preset;
        let server = test_server().await;
        let response = server
            .process_request(Request::SetPreset(Preset::Long))
            .await;
        assert!(matches!(response, Response::Ok));
        let response = server.process_request(Request::GetStatus).await;
        if let Response::Status(status) = response {
            assert_eq!(status.preset, "Long");
        }
    }
}
