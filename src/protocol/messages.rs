use crate::timer::{Preset, TimerStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum Request {
    GetStatus,
    Start,
    Pause,
    Resume,
    Reset,
    SwitchMode,
    SetTask(String),
    SetPreset(Preset),
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Status(TimerStatus),
    Error(String),
    Pong,
}

#[derive(Deserialize)]
pub struct SetTaskRequest {
    pub task: String,
}

#[derive(Deserialize)]
pub struct SetPresetRequest {
    pub preset: Preset,
}
