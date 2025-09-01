use std::fmt;
use std::time::{Duration, Instant};

const MIN: u64 = 60;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Preset {
    #[default]
    Short,
    Long,
    Test,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum TimerState {
    // NOTE: pub so that we can use it outside of timer.rs module
    #[default]
    Work,
    Break,
}

impl TimerState {
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

impl fmt::Display for TimerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimerState::Work => f.write_str("Work"),
            TimerState::Break => f.write_str("Break"),
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

#[derive(Debug, Default, Clone)]
pub struct Timer {
    started_at: Option<Instant>,
    remaining: Duration,
    state: TimerState,
    timeset: Preset,
    durs: Durations,
    paused: bool,
    auto_continue: bool,
}

impl Timer {
    pub fn new() -> Self {
        let durs = Durations::default();
        let state = TimerState::Work;
        Self {
            started_at: None,
            remaining: state.duration(&durs),
            state: state,
            timeset: Preset::default(),
            durs: durs,
            paused: false,
            auto_continue: true,
        }
    }

    pub fn start(&mut self) -> bool {
        if self.started_at.is_none() {
            self.started_at = Some(Instant::now());
            return true;
        }
        false
    }

    pub fn stop(&mut self) -> bool {
        if let Some(t0) = self.started_at.take() {
            // TODO: check what take() does
            self.remaining -= t0.elapsed();
            return true;
        }
        false
    }

    pub fn toggle(&mut self) -> bool {
        if self.is_running() {
            self.paused = self.stop();
            return self.paused;
        }
        self.paused = !self.start();
        self.paused
    }

    pub fn switch_mode(&mut self) {
        self.state = self.state.toggle();
        self.reset();
        if (self.auto_continue) {
            self.start();
        }
    }

    pub fn is_running(&self) -> bool {
        self.started_at.is_some()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn reset(&mut self) {
        self.remaining = self.state.duration(&self.durs);
        self.started_at = None;
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
        if let Some(t0) = self.started_at.as_ref() {
            if self.remaining < t0.elapsed() {
                self.switch_mode();
            }
        }
    }

    pub fn get_state(&self) -> TimerState {
        self.state
    }

    pub fn set_preset(&mut self, p: Preset) {
        if self.timeset == p {
            log::info!("Already using {:?} preset.", p);
            return;
        }
        let new = Durations::for_preset(p);
        self.durs = new;
        self.timeset = p;
        self.reset();
    }
}

#[test]
fn init_timer_state() {
    let t = Timer::new();
    assert_eq!(t.state, TimerState::Work);
    assert_eq!(t.remaining, Duration::from_secs(25 * MIN));
    assert_eq!(t.durs, Durations::default());
}

#[test]
fn timer_state_toggle() {
    let mut ts = TimerState::default();
    assert_eq!(ts, TimerState::Work);
    ts = ts.toggle();
    assert_eq!(ts, TimerState::Break);
    ts = ts.toggle();
    assert_eq!(ts, TimerState::Work);
}
