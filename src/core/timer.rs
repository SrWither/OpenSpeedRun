use chrono::{DateTime, Utc, Duration};

#[derive(Debug, PartialEq)]
pub enum TimerState {
    NotStarted,
    Running,
    Paused,
    Ended,
}

pub struct Timer {
    pub state: TimerState,
    pub start_time: Option<DateTime<Utc>>,
    pub elapsed: Duration,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            state: TimerState::NotStarted,
            start_time: None,
            elapsed: Duration::zero(),
        }
    }

    pub fn start_with_offset(&mut self, offset_millis: i64) {
        self.start_time = Some(Utc::now() - Duration::milliseconds(offset_millis));
        self.elapsed = Duration::zero();
        self.state = TimerState::Running;
    }

    pub fn start(&mut self) {
        self.start_time = Some(Utc::now());
        self.state = TimerState::Running;
    }

    pub fn pause(&mut self) {
        if let Some(start) = self.start_time {
            self.elapsed = Utc::now() - start + self.elapsed;
            self.start_time = None;
            self.state = TimerState::Paused;
        }
    }

    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed = Duration::zero();
        self.state = TimerState::NotStarted;
    }

    pub fn current_time(&self) -> Duration {
        match self.state {
            TimerState::Running => {
                if let Some(start) = self.start_time {
                    Utc::now() - start + self.elapsed
                } else {
                    self.elapsed
                }
            }
            _ => self.elapsed,
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == TimerState::Running
    }
    pub fn is_paused(&self) -> bool {
        self.state == TimerState::Paused
    }
    pub fn is_ended(&self) -> bool {
        self.state == TimerState::Ended
    }
}
