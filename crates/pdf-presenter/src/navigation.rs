//! Constant-memory hold acceleration, independent of OS keyboard repeat rates.
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug)]
pub struct Hold {
    pub direction: i32,
    started: Instant,
    next: Instant,
}
impl Hold {
    /// The caller performs the initial single step immediately.
    pub fn new(direction: i32, now: Instant) -> Self {
        Self {
            direction: direction.signum(),
            started: now,
            next: now + Duration::from_millis(400),
        }
    }
    /// Native hosts can redraw a held button only when its next step is due.
    pub fn due(&self, now: Instant) -> bool {
        now >= self.next
    }
    /// At most one step per call; stalled event loops never replay missed steps.
    pub fn tick(&mut self, now: Instant, count: usize) -> Option<i32> {
        if !self.due(now) {
            return None;
        }
        let elapsed = now.saturating_duration_since(self.started);
        let (batch, interval) = if elapsed >= Duration::from_secs(5) {
            (10, 160)
        } else if elapsed >= Duration::from_secs(2) {
            (5, 160)
        } else {
            (1, 100)
        };
        self.next = now + Duration::from_millis(interval);
        let batch = batch.min((count / 10).clamp(1, 10));
        Some(self.direction * batch as i32)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delay_acceleration_small_decks_and_stalls_are_bounded() {
        let t = Instant::now();
        let mut h = Hold::new(1, t);
        assert!(!h.due(t + Duration::from_millis(399)));
        assert!(h.due(t + Duration::from_millis(400)));
        assert_eq!(h.tick(t + Duration::from_millis(399), 500), None);
        assert_eq!(h.tick(t + Duration::from_millis(400), 500), Some(1));
        assert_eq!(h.tick(t + Duration::from_secs(2), 500), Some(5));
        assert_eq!(h.tick(t + Duration::from_secs(5), 500), Some(10));
        assert_eq!(h.tick(t + Duration::from_secs(60), 500), Some(10));
        assert_eq!(h.tick(t + Duration::from_secs(60), 500), None);
        let mut reverse = Hold::new(-1, t);
        assert_eq!(reverse.tick(t + Duration::from_secs(6), 15), Some(-1));
        assert_eq!(reverse.tick(t + Duration::from_secs(7), 30), Some(-3));
    }
}
