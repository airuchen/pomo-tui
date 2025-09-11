use chrono::{DateTime, Local};
use serde::Serialize;
use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};
use uuid::Uuid;

const MIN: u64 = 60;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Preset {
    #[default]
    Short,
    Long,
    Test,
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
    },
    Paused {
        id: Uuid,
        task: String,
        at: DateTime<Local>,
    },
    Resumed {
        id: Uuid,
        task: String,
        at: DateTime<Local>,
    },
    Terminated {
        id: Uuid,
        task: String,
        at: DateTime<Local>,
        work_secs: Duration,
    },
    Completed {
        id: Uuid,
        task: String,
        at: DateTime<Local>,
        work_secs: Duration,
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

// TODO: maybe a session struct for a session name, id, time...

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
            mode: mode,
            timeset: Preset::default(),
            durs: durs,
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
    }

    // TODO: how is this done?
    pub fn drain_events(&mut self) -> impl Iterator<Item = LogEvent> {
        std::mem::take(&mut self.events).into_iter()
    }

    fn start(&mut self) {
        self.idle = false;
        self.started_at = Some(Instant::now());
        self.id = Some(Uuid::new_v4());
        self.emit(LogEvent::Started {
            id: self.id.unwrap(),
            timer_type: self.mode,
            task: self.task_name.clone(),
            at: Local::now(),
        });
    }

    fn resume(&mut self) {
        self.started_at = Some(Instant::now());
        self.emit(LogEvent::Resumed {
            id: self.id.unwrap(),
            task: self.task_name.clone(),
            at: Local::now(),
        });
    }

    fn stop(&mut self) {
        self.emit(LogEvent::Paused {
            id: self.id.unwrap(),
            task: self.task_name.clone(),
            at: Local::now(),
        });
        if let Some(t0) = self.started_at.take() {
            // TODO: check what take() does
            self.remaining -= t0.elapsed();
        }
    }

    pub fn toggle(&mut self) {
        if !self.is_running() && !self.is_paused() {
            self.start();
            return;
        }

        if self.is_running() {
            self.stop();
        } else {
            self.resume();
        }
        self.paused = !self.paused;
    }

    pub fn switch_mode(&mut self) {
        if self.id.is_some() {
            self.persist_termination();
        }
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
            Some(t0) => {
                if self.remaining > t0.elapsed() {
                    return self.remaining - t0.elapsed();
                }
                Duration::ZERO
            }
            None => self.remaining,
        }
    }

    pub fn update(&mut self) {
        if let Some(t0) = self.started_at.as_ref()
            && self.remaining < t0.elapsed()
        {
            self.emit(LogEvent::Completed {
                id: self.id.unwrap(),
                task: self.task_name.clone(),
                at: Local::now(),
                work_secs: self.mode.duration(&self.durs),
            });
            self.switch_mode();
            if self.auto_continue {
                self.toggle();
            }
        }
    }

    // maybe return a struct with all the info needed?
    // TODO: should I always return ref?
    pub fn get_mode(&self) -> &TimerMode {
        &self.mode
    }

    pub fn get_task_name(&self) -> &str {
        &self.task_name
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
            id: self.id.unwrap(),
            task: self.task_name.clone(),
            at: Local::now(),
            work_secs: (self.mode.duration(&self.durs) - self.get_remaining()),
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
