mod common;
use pdf_presenter::studio::{Action, Monitor, Studio};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
fn studio(path: &Path, cache: &Path) -> Studio {
    let mut s = Studio::with_program(
        true,
        Some(path.to_owned()),
        env!("CARGO_BIN_EXE_pdf-presenter").into(),
    );
    s.cache_path = cache.to_owned();
    s.set_monitors(
        vec![Monitor {
            id: "one".into(),
            name: "one".into(),
            origin: (0, 0),
            size: (640, 480),
        }],
        0,
    );
    s
}
fn prepare(s: &mut Studio) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !s.fully_buffered() || s.notes_loading {
        s.poll();
        assert!(
            Instant::now() < deadline,
            "{} / {}",
            s.preparation_status(),
            s.status
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn artifacts(path: &Path) -> Vec<PathBuf> {
    let mut result = vec![];
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(artifacts(&path));
        } else if path.extension().is_some_and(|e| e == "rgba") {
            result.push(path);
        }
    }
    result
}
#[test]
fn whole_deck_stays_in_ram_and_warm_open_reuses_screen_sized_cache() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("slides");
    std::fs::create_dir(&dir).unwrap();
    for i in 0..8 {
        common::image(&dir.join(format!("slide{i}.png")), [i * 30, 128, 200, 255]);
    }
    let cache = temp.path().join("processed");
    let mut first = studio(&dir, &cache);
    prepare(&mut first);
    assert_eq!(first.prepared.len(), 8);
    assert_eq!(first.target_edge, 640);
    assert!(first.preparation_status().starts_with("READY"));
    let files = artifacts(&cache);
    let timestamps: Vec<_> = files
        .iter()
        .filter(|p| p.file_name().unwrap().to_str().unwrap().contains("-640-"))
        .map(|p| (p.clone(), std::fs::metadata(p).unwrap().modified().unwrap()))
        .collect();
    assert_eq!(timestamps.len(), 8);
    for p in &files {
        if p.file_name().unwrap().to_str().unwrap().contains("-512-") {
            std::fs::remove_file(p).unwrap();
        }
    }
    drop(first);
    let mut warm = studio(&dir, &cache);
    prepare(&mut warm);
    assert_eq!(warm.cached_slides, 8);
    for (path, time) in timestamps {
        assert_eq!(std::fs::metadata(path).unwrap().modified().unwrap(), time);
    }
    assert!(
        !artifacts(&cache).iter().any(|p| p
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("-512-"))
    );
    // Once ready, even removing both sources and processed files cannot cause a slide read.
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&cache).unwrap();
    for page in [7, 0, 5, 1, 6, 2, 4, 3] {
        warm.execute(Action::Go(page));
        warm.poll();
        assert!(warm.fully_buffered());
        assert_eq!(warm.full_frame(page).unwrap().info.page, page);
    }
    assert!(!cache.exists());
}
#[test]
fn changing_screen_resolution_reprepares_and_readiness_waits_for_output() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("show.pdf");
    common::pdf(&path);
    let mut s = studio(&path, &temp.path().join("cache"));
    prepare(&mut s);
    let generation = s.buffer_generation();
    s.monitors[0].size = (800, 600);
    s.poll();
    assert!(!s.fully_buffered());
    assert_ne!(generation, s.buffer_generation());
    prepare(&mut s);
    assert_eq!(s.target_edge, 800);
    assert_eq!(s.full_frame(0).unwrap().info.width, 800);
    s.routes.presentation = vec!["output".into()];
    assert!(!s.fully_buffered());
    assert!(
        s.preparation_status()
            .contains("Buffering presentation output")
    );
    s.output_buffered = s.count();
    assert!(s.fully_buffered());
    s.output_buffered = 0;
    s.ram_blocked = true;
    assert!(s.preparation_status().contains("RAM budget reached"));
}

#[test]
fn impossibly_small_budget_reports_pressure_and_recovers() {
    let temp = tempfile::tempdir().unwrap();
    let pdf = temp.path().join("show.pdf");
    common::pdf(&pdf);
    let mut s = studio(&pdf, &temp.path().join("cache"));
    // An intentionally tiny internal test budget exercises pressure without huge allocations.
    s.settings.buffer_mib = 1;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !s.ram_blocked {
        s.poll();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!s.fully_buffered());
    assert!(s.preparation_status().contains("RAM budget reached"));
    s.settings.buffer_mib = 256;
    s.remember_buffer_budget();
    prepare(&mut s);
    assert!(s.buffer_bytes() <= 256 * 1024 * 1024);
    let frame = s.full_frame(0).unwrap();
    s.execute(Action::Go(1));
    s.poll();
    assert!(std::sync::Arc::ptr_eq(&frame, &s.full_frame(0).unwrap()));
}

#[test]
fn navigating_buffered_show_during_failed_open_does_not_cancel_open_completion() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("show.pdf");
    common::pdf(&path);
    let mut s = studio(&path, &temp.path().join("cache"));
    prepare(&mut s);
    let frame = s.full_frame(1).unwrap();
    s.execute(Action::Open(temp.path().join("missing.pdf")));
    s.poll();
    s.execute(Action::Go(1));
    let deadline = Instant::now() + Duration::from_secs(5);
    while s.loading {
        s.poll();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(s.fully_buffered());
    assert_eq!(s.requested, 1);
    assert!(std::sync::Arc::ptr_eq(&frame, &s.full_frame(1).unwrap()));
}

#[test]
fn opening_document_honors_auto_show_and_navigation_preserves_manual_blank() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("show.pdf");
    common::pdf(&path);
    for auto_show in [true, false] {
        let mut s = studio(&path, &temp.path().join("cache"));
        s.settings.auto_show = auto_show;
        prepare(&mut s);
        assert_eq!(s.blanked, !auto_show);
        s.blanked = true;
        s.execute(Action::Go(1));
        assert!(s.blanked);
    }
}

#[test]
fn rolling_buffer_finishes_processing_and_reloads_evicted_slides_without_churn() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("slides");
    std::fs::create_dir(&dir).unwrap();
    for i in 0..16 {
        common::image(&dir.join(format!("slide{i}.png")), [i * 10, 128, 200, 255]);
    }
    let cache = temp.path().join("processed");
    let mut s = studio(&dir, &cache);
    s.settings.buffer_mib = 6;
    let deadline = Instant::now() + Duration::from_secs(20);
    while s.count() == 0 || s.prepared.len() < s.count() {
        s.poll();
        assert!(s.buffer_bytes() <= 6 * 1024 * 1024);
        assert!(
            Instant::now() < deadline,
            "{} / {}",
            s.preparation_status(),
            s.status
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(s.evicted_frames > 0);
    assert!(s.resident_slides() < s.count());
    assert!(!s.fully_buffered());
    assert!(s.preparation_status().contains("Rolling buffer"));
    let timestamps: Vec<_> = artifacts(&cache)
        .iter()
        .filter(|p| p.file_name().unwrap().to_str().unwrap().contains("-640-"))
        .map(|p| (p.clone(), std::fs::metadata(p).unwrap().modified().unwrap()))
        .collect();
    assert_eq!(timestamps.len(), 16);
    let missing = (0..s.count())
        .find(|p| !s.frames.contains_key(&(*p, 640)))
        .unwrap();
    for page in [missing, 15, 0, 8, 2] {
        s.displayed = Some(s.requested);
        s.execute(Action::Go(page));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !s.frames.contains_key(&(page, 640)) {
            s.poll();
            assert!(s.buffer_bytes() <= 6 * 1024 * 1024);
            assert!(
                Instant::now() < deadline,
                "{} / {}",
                s.preparation_status(),
                s.status
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(s.full_frame(page).unwrap().info.page, page);
        s.displayed = Some(page);
    }
    // Let neighbour prefetch settle, then ensure idle polls do not cycle evictions.
    for _ in 0..100 {
        s.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    let evictions = s.evicted_frames;
    for _ in 0..100 {
        s.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(s.evicted_frames, evictions);
    for (path, timestamp) in timestamps {
        assert_eq!(
            std::fs::metadata(path).unwrap().modified().unwrap(),
            timestamp
        );
    }
    s.settings.buffer_mib = 256;
    s.remember_buffer_budget();
    prepare(&mut s);
    s.settings.buffer_mib = 6;
    s.remember_buffer_budget();
    assert!(s.buffer_bytes() <= 6 * 1024 * 1024);
    assert!(s.full_frame(s.requested).is_some());
    assert!(!s.fully_buffered());
}

#[test]
#[ignore = "manual stress test: set PDF_PRESENTER_STRESS_PDF to an authorized local PDF"]
fn real_pdf_exceeds_budget_and_keeps_navigating() {
    let path = PathBuf::from(
        std::env::var_os("PDF_PRESENTER_STRESS_PDF").expect("PDF_PRESENTER_STRESS_PDF"),
    );
    let temp = tempfile::tempdir().unwrap();
    let mut s = studio(&path, &temp.path().join("cache"));
    s.monitors[0].size = (1920, 1080);
    s.settings.buffer_mib = 256;
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut peak = 0;
    while s.evicted_frames < 10 {
        s.poll();
        peak = peak.max(s.buffer_bytes());
        assert!(peak <= 256 * 1024 * 1024);
        assert!(
            Instant::now() < deadline,
            "{} / {}",
            s.preparation_status(),
            s.status
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    eprintln!(
        "Pressure reached: {} · peak {} MiB",
        s.preparation_status(),
        peak / (1024 * 1024)
    );
    assert!(s.full_frame(0).is_some());
    assert!(!s.fully_buffered());
    let missing = (0..s.count())
        .find(|p| s.prepared.contains(p) && !s.frames.contains_key(&(*p, s.target_edge)))
        .unwrap();
    for page in [missing, s.count() / 2, s.count() - 1, 0] {
        let start = Instant::now();
        s.displayed = Some(s.requested);
        s.execute(Action::Go(page));
        while !s.frames.contains_key(&(page, s.target_edge)) {
            s.poll();
            assert!(s.buffer_bytes() <= 256 * 1024 * 1024);
            assert!(
                start.elapsed() < Duration::from_secs(35),
                "{} / {}",
                s.preparation_status(),
                s.status
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        s.displayed = Some(page);
        eprintln!(
            "Slide {} available in {:?}; {}",
            page + 1,
            start.elapsed(),
            s.preparation_status()
        );
    }
    while s.prepared.len() < s.count() {
        s.poll();
        assert!(s.buffer_bytes() <= 256 * 1024 * 1024);
        assert!(
            Instant::now() < deadline,
            "{} / {}",
            s.preparation_status(),
            s.status
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    eprintln!("Whole PDF processed: {}", s.preparation_status());
    assert!(!s.fully_buffered());
}
