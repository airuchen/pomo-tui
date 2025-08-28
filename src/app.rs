use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum TimerState {
    STOPPED,
    RUNNING,
    PAUSED,
    IDLE,
    // TODO: do we need complete?
}

#[derive(Debug, Clone)]
pub enum SessionType {
    WORK,
    BREAK,
}

#[derive(Debug, Clone)]
pub struct Pomodoro {
    // Configuration
    pub duration: Duration,

    // State
    pub timer_state: TimerState,
    pub session_type: SessionType,

    // Timing
    pub elapsed: Duration,

    // Setting
    pub current_task: Option<String>,
    pub auto_continue: bool,
    pub enable_notification: bool,
}

impl Default for Pomodoro {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(25 * 60),
            timer_state: TimerState::IDLE,
            session_type: SessionType::WORK,
            elapsed: Duration::ZERO,
            current_task: None,
            auto_continue: true,
            enable_notification: true,
        }
    }
}

impl Pomodoro {
    pub fn get_duration(&self) -> Duration {
        self.duration
    }

    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration
    }

    pub fn start(&mut self) {
        match self.timer_state {
            TimerState::RUNNING => {
                print!("RUNNING")
            }
            TimerState::PAUSED | TimerState::STOPPED | TimerState::IDLE => {
                self.timer_state = TimerState::RUNNING;
                print!("IDLE")
            }
        }
    }

    pub fn start_current_session(&mut self) {
        self.timer_state = TimerState::RUNNING;
    }

    pub fn update(&mut self) {
        if let TimerState::RUNNING = self.timer_state {
            self.elapsed += Duration::from_secs(1);
        }
    }

    pub fn complete_session(&mut self) {
        self.timer_state = TimerState::STOPPED;
    }
}
