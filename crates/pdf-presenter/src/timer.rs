//! Monotonic presentation timer; no threads or wall-clock dependence.
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Timer {
    accumulated: Duration,
    running_since: Option<Instant>,
    pub waiting_for_slide: bool,
}
impl Default for Timer {
    fn default() -> Self {
        Self {
            accumulated: Duration::ZERO,
            running_since: None,
            waiting_for_slide: true,
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
    pub fn restart(&mut self, now: Instant) {
        self.accumulated = Duration::ZERO;
        self.running_since = Some(now);
        self.waiting_for_slide = false;
    }
    pub fn toggle(&mut self, now: Instant, limit: Option<Duration>) {
        self.waiting_for_slide = false;
        if self.running(now, limit) {
            self.accumulated = self.elapsed(now);
            self.running_since = None;
        } else if limit.is_some_and(|limit| self.elapsed(now) >= limit) {
            self.restart(now);
        } else {
            self.running_since = Some(now);
        }
    }
    pub fn first_slide(&mut self, now: Instant, automatic: bool) {
        if self.waiting_for_slide {
            self.waiting_for_slide = false;
            if automatic {
                self.running_since = Some(now);
            }
        }
    }
}
