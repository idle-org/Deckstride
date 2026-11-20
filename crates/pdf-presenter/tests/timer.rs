use pdf_presenter::timer::Timer;
use std::time::{Duration, Instant};

#[test]
fn elapsed_autostart_pause_resume_restart_use_monotonic_time() {
    let now = Instant::now();
    let mut timer = Timer::default();
    assert_eq!(timer.seconds(now + Duration::from_secs(50), None), 0);
    timer.first_slide(now, true);
    assert!(timer.running(now, None));
    timer.first_slide(now + Duration::from_secs(4), true);
    assert_eq!(timer.seconds(now + Duration::from_secs(5), None), 5);
    timer.toggle(now + Duration::from_secs(5), None);
    assert!(!timer.running(now + Duration::from_secs(20), None));
    timer.first_slide(now + Duration::from_secs(10), true);
    assert_eq!(timer.seconds(now + Duration::from_secs(20), None), 5);
    timer.toggle(now + Duration::from_secs(20), None);
    assert_eq!(timer.seconds(now + Duration::from_secs(23), None), 8);
    timer.restart(now + Duration::from_secs(23));
    assert_eq!(timer.seconds(now + Duration::from_secs(25), None), 2);
}

#[test]
fn countdown_manual_start_preserves_fractional_seconds_and_stops_at_zero() {
    let now = Instant::now();
    let limit = Some(Duration::from_secs(90));
    let mut timer = Timer::default();
    timer.first_slide(now, false);
    assert!(!timer.running(now, limit));
    assert_eq!(timer.seconds(now + Duration::from_secs(5), limit), 90);
    timer.toggle(now, limit);
    assert_eq!(timer.seconds(now + Duration::from_millis(100), limit), 90);
    timer.toggle(now + Duration::from_millis(1500), limit);
    assert_eq!(timer.seconds(now + Duration::from_secs(20), limit), 89);
    timer.toggle(now + Duration::from_secs(20), limit);
    assert_eq!(timer.seconds(now + Duration::from_millis(20500), limit), 88);
    assert_eq!(timer.seconds(now + Duration::from_secs(200), limit), 0);
    assert!(!timer.running(now + Duration::from_secs(200), limit));
    timer.toggle(now + Duration::from_secs(200), limit);
    assert_eq!(timer.seconds(now + Duration::from_secs(200), limit), 90);
    assert!(timer.running(now + Duration::from_secs(200), limit));
}
