//! Monotonic presentation timer; no threads or wall-clock dependence.
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Timer {
    accumulated: Duration,
    running_since: Option<Instant>,
    pub waiting_for_action: bool,
}
impl Default for Timer {
    fn default() -> Self {
        Self {
            accumulated: Duration::ZERO,
            running_since: None,
            waiting_for_action: true,
        }
    }
}
impl Timer {
    pub fn elapsed(&self, now: Instant) -> Duration {
        self.accumulated
            + self
                .running_since
                .map_or(Duration::ZERO, |start| now.saturating_duration_since(start))
    }
    pub fn running(&self, now: Instant, limit: Option<Duration>) -> bool {
        self.running_since.is_some() && limit.is_none_or(|limit| self.elapsed(now) < limit)
    }
    pub fn seconds(&self, now: Instant, limit: Option<Duration>) -> u64 {
        let elapsed = self.elapsed(now);
        match limit {
            Some(limit) => {
                let remaining = limit.saturating_sub(elapsed);
                remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0)
            }
            None => elapsed.as_secs(),
        }
    }
    /// Return to zero and wait for the next presentation action.
    pub fn reset(&mut self) {
        self.accumulated = Duration::ZERO;
        self.running_since = None;
        self.waiting_for_action = true;
    }
    pub fn toggle(&mut self, now: Instant, limit: Option<Duration>) {
        self.waiting_for_action = false;
        if self.running(now, limit) {
            self.accumulated = self.elapsed(now);
            self.running_since = None;
        } else if limit.is_some_and(|limit| self.elapsed(now) >= limit) {
            self.accumulated = Duration::ZERO;
            self.running_since = Some(now);
        } else {
            self.running_since = Some(now);
        }
    }
    pub fn presentation_action(&mut self, now: Instant, automatic: bool) {
        if self.waiting_for_action {
            self.waiting_for_action = false;
            if automatic {
                self.running_since = Some(now);
            }
        }
    }
}
