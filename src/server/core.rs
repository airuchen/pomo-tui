use crate::protocol::{Request, Response};
use crate::timer::{Timer};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct PomoServer {
    timer: Arc<Mutex<Timer>>,
}

impl PomoServer {
    pub fn new() -> Self {
        Self {
            timer: Arc::new(Mutex::new(Timer::new())),
        }
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

    #[tokio::test]
    async fn test_ping() {
        let server = PomoServer::new();
        let response = server.process_request(Request::Ping).await;
        assert!(matches!(response, Response::Pong));
    }

    #[tokio::test]
    async fn test_preset() {
        use crate::timer::Preset;
        let server = PomoServer::new();
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
