use chrono::{DateTime, Local};
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};
use uuid::Uuid;

const MIN: u64 = 60;
const MAX_EMIT_EVENTS: usize = 1000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Preset {
    #[default]
    Short,
    Long,
    Test,
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Preset::Short => f.write_str("Short"),
            Preset::Long => f.write_str("Long"),
            Preset::Test => f.write_str("Test"),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub enum LogEvent {
    #[default]
    Idle,
    Started {
        id: Uuid,
        timer_type: TimerMode,
        task: String,
        at: DateTime<Local>,
        remaining: u64,
    },
    Paused {
        id: Uuid,
        task: String,
        at: DateTime<Local>,
        remaining: u64,
    },
    Resumed {
        id: Uuid,
        task: String,
        at: DateTime<Local>,
        remaining: u64,
    },
    Terminated {
        id: Uuid,
        task: String,
        at: DateTime<Local>,
        remaining: u64,
        work_secs: u64,
    },
    Completed {
        id: Uuid,
        task: String,
        at: DateTime<Local>,
        work_secs: u64,
    },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize)]
pub enum TimerMode {
    // NOTE: pub so that we can use it outside of timer.rs module
    #[default]
    Work,
    Break,
}

impl TimerMode {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Work => Self::Break,
            Self::Break => Self::Work,
        }
    }

    fn duration(self, d: &Durations) -> Duration {
        match self {
            Self::Work => d.work,
            Self::Break => d.brk,
        }
    }
}

impl fmt::Display for TimerMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimerMode::Work => f.write_str("Work"),
            TimerMode::Break => f.write_str("Break"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Durations {
    work: Duration,
    brk: Duration,
}

impl Durations {
    pub const SHORT: Self = Self {
        work: Duration::from_secs(25 * MIN),
        brk: Duration::from_secs(5 * MIN),
    };

    pub const LONG: Self = Self {
        work: Duration::from_secs(50 * MIN),
        brk: Duration::from_secs(10 * MIN),
    };

    pub const TEST: Self = Self {
        work: Duration::from_secs(5),
        brk: Duration::from_secs(5),
    };

    pub const fn for_preset(p: Preset) -> Self {
        match p {
            Preset::Short => Self::SHORT,
            Preset::Long => Self::LONG,
            Preset::Test => Self::TEST,
        }
    }
}

impl Default for Durations {
    fn default() -> Self {
        Durations::SHORT
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerStatus {
    pub mode: String,
    pub remaining: u64,
    pub preset: String,
    pub is_paused: bool,
    pub is_idle: bool,
    pub is_running: bool,
    pub task: String,
}

#[derive(Debug, Default, Clone)]
pub struct Timer {
    started_at: Option<Instant>,
    remaining: Duration,
    mode: TimerMode,
    timeset: Preset,
    durs: Durations,
    paused: bool,
    idle: bool,
    auto_continue: bool,
    task_name: String,
    id: Option<Uuid>,
    events: VecDeque<LogEvent>,
}

impl Timer {
    pub fn new() -> Self {
        let durs = Durations::default();
        let mode = TimerMode::Work;
        Self {
            started_at: None,
            remaining: mode.duration(&durs),
            mode,
            timeset: Preset::default(),
            durs,
            paused: false,
            idle: true,
            auto_continue: true,
            task_name: String::new(),
            id: None,
            events: VecDeque::new(),
        }
    }

    // TODO: what does inline do here?
    #[inline]
    fn emit(&mut self, event: LogEvent) {
        self.events.push_back(event);

        if self.events.len() > MAX_EMIT_EVENTS {
            self.events.pop_front();
        }
    }

    // TODO: how is this done?
    pub fn drain_events(&mut self) -> impl Iterator<Item = LogEvent> {
        std::mem::take(&mut self.events).into_iter()
    }

    fn current_id(&self) -> Uuid {
        self.id.expect("Timer must have an ID when active")
    }

    fn start(&mut self) {
        self.idle = false;
        self.started_at = Some(Instant::now());
        self.id = Some(Uuid::new_v4());
        self.emit(LogEvent::Started {
            id: self.current_id(),
            timer_type: self.mode,
            task: self.task_name.clone(),
            at: Local::now(),
            remaining: self.get_remaining().as_secs(),
        });
    }

    fn resume(&mut self) {
        self.started_at = Some(Instant::now());
        self.emit(LogEvent::Resumed {
            id: self.current_id(),
            task: self.task_name.clone(),
            at: Local::now(),
            remaining: self.get_remaining().as_secs(),
        });
    }

    fn stop(&mut self) {
        self.emit(LogEvent::Paused {
            id: self.current_id(),
            task: self.task_name.clone(),
            at: Local::now(),
            remaining: self.get_remaining().as_secs(),
        });
        if let Some(t0) = self.started_at.take() {
            self.remaining = self.remaining.saturating_sub(t0.elapsed());
        }
    }

    pub fn toggle(&mut self) {
        match (self.is_running(), self.is_paused()) {
            (false, false) => {
                // Idle state -> start timer
                self.start();
                self.paused = false;
            }
            (true, false) => {
                // Running -> pause it
                self.stop();
                self.paused = true;
            }
            (false, true) => {
                // Paused resume it
                self.resume();
                self.paused = false;
            }
            (true, true) => {
                log::warn!("Invalid timer state: running and paused");
                self.paused = false;
            }
        }
    }

    pub fn switch_mode(&mut self) {
        if self.id.is_some() {
            self.persist_termination();
        }
        self.mode = self.mode.toggle();
        self.reset();
    }

    fn complet_and_switch(&mut self) {
        self.mode = self.mode.toggle();
        self.reset();
    }

    pub fn is_running(&self) -> bool {
        self.started_at.is_some()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn is_idle(&self) -> bool {
        self.idle
    }

    pub fn reset(&mut self) {
        self.idle = true;
        self.remaining = self.mode.duration(&self.durs);
        self.started_at = None;
        self.id = None;
        self.paused = false;
    }

    pub fn get_remaining(&self) -> Duration {
        match self.started_at {
            Some(t0) => self.remaining.saturating_sub(t0.elapsed()),
            None => self.remaining,
        }
    }

    pub fn update(&mut self) {
        if let Some(t0) = self.started_at.as_ref() {
            if self.remaining > t0.elapsed() {
                return;
            }
            self.emit(LogEvent::Completed {
                id: self.current_id(),
                task: self.task_name.clone(),
                at: Local::now(),
                work_secs: self.mode.duration(&self.durs).as_secs(),
            });

            let notification_msg = format!("{}: {}", self.mode, self.task_name);
            let _ = Notification::new()
                .summary("Completed")
                .body(&notification_msg)
                .icon("clock")
                .show();

            self.complet_and_switch();
            if self.auto_continue {
                self.toggle();
            }
        }
    }

    pub fn get_timer_status(&self) -> TimerStatus {
        TimerStatus {
            task: self.get_task_name().to_string(),
            remaining: self.get_remaining().as_secs(),
            preset: self.get_preset().to_string(),
            is_paused: self.is_paused(),
            is_idle: self.is_idle(),
            is_running: self.is_running(),
            mode: self.get_mode().to_string(),
        }
    }

    pub fn get_mode(&self) -> &TimerMode {
        &self.mode
    }

    pub fn get_task_name(&self) -> &str {
        &self.task_name
    }

    pub fn get_preset(&self) -> &Preset {
        &self.timeset
    }

    pub fn set_task_name(&mut self, new_task_name: &str) {
        self.task_name = new_task_name.into();
    }

    pub fn set_preset(&mut self, p: Preset) {
        if self.timeset == p {
            log::info!("Already using {:?} preset.", p);
            return;
        }
        if self.id.is_some() {
            self.persist_termination();
        }
        let new = Durations::for_preset(p);
        self.durs = new;
        self.timeset = p;
        self.reset();
    }

    // TODO: is it possible the gather the emit logic to one function?
    pub fn persist_termination(&mut self) {
        self.emit(LogEvent::Terminated {
            id: self.current_id(),
            task: self.task_name.clone(),
            at: Local::now(),
            remaining: self.get_remaining().as_secs(),
            work_secs: (self.mode.duration(&self.durs) - self.get_remaining()).as_secs(),
        });
    }
}

#[test]
fn init_timer_mode() {
    let t = Timer::new();
    assert_eq!(t.mode, TimerMode::Work);
    assert_eq!(t.remaining, Duration::from_secs(25 * MIN));
    assert_eq!(t.durs, Durations::default());
}

#[test]
fn timer_mode_toggle() {
    let mut ts = TimerMode::default();
    assert_eq!(ts, TimerMode::Work);
    ts = ts.toggle();
    assert_eq!(ts, TimerMode::Break);
    ts = ts.toggle();
    assert_eq!(ts, TimerMode::Work);
}
