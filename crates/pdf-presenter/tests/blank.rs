mod common;
use pdf_presenter::{
    blank,
    studio::{Action, Studio},
};
use std::time::{Duration, Instant};

#[test]
fn managed_picture_is_bounded_durable_and_checked() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.png");
    let target = dir.path().join("blank-picture.rgba");
    image::RgbaImage::from_pixel(3000, 1500, image::Rgba([10u8, 20, 30, 128]))
        .save(&source)
        .unwrap();
    let original = std::fs::read(&source).unwrap();
    let frame = blank::prepare(Some(&source), &target).unwrap();
    assert_eq!((frame.info.width, frame.info.height), (2048, 1024));
    assert_eq!(original, std::fs::read(&source).unwrap());
    std::fs::remove_file(&source).unwrap();
    assert_eq!(blank::prepare(None, &target).unwrap().rgba, frame.rgba);
    assert!(blank::prepare(Some(&source), &target).is_err());
    assert_eq!(
        blank::prepare(None, &target).unwrap().info.sha256,
        frame.info.sha256
    );
    let mut data = std::fs::read(&target).unwrap();
    data[8] ^= 1;
    std::fs::write(&target, data).unwrap();
    assert!(
        blank::prepare(None, &target)
            .err()
            .unwrap()
            .contains("checksum")
    );
    std::fs::write(&target, b"bad").unwrap();
    assert!(blank::prepare(None, &target).is_err());
}

fn until(s: &mut Studio, done: impl Fn(&Studio) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(s) {
        assert!(Instant::now() < deadline, "{}", s.status);
        s.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn worker_import_failure_and_clear_preserve_blank_state() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("picture.png");
    common::image(&source, [20, 50, 90, 255]);
    let mut s = Studio::with_program(true, None, env!("CARGO_BIN_EXE_pdf-presenter").into());
    until(&mut s, |s| s.initialized);
    s.blanked = true;
    s.execute(Action::SetBlankPicture(source.clone()));
    until(&mut s, |s| !s.blank_busy);
    assert!(s.settings.blank_picture && s.blanked);
    let hash = s.blank_picture.as_ref().unwrap().info.sha256.clone();
    std::fs::remove_file(source.clone()).unwrap();
    s.execute(Action::SetBlankPicture(source));
    until(&mut s, |s| !s.blank_busy);
    assert_eq!(s.blank_picture.as_ref().unwrap().info.sha256, hash);
    assert!(s.blanked);
    let encoded = toml::to_string(&s.settings).unwrap();
    let decoded: pdf_presenter::settings::Settings = toml::from_str(&encoded).unwrap();
    assert!(decoded.blank_picture);
    s.execute(Action::ClearBlankPicture);
    assert!(s.blanked && s.blank_picture.is_none() && !s.settings.blank_picture);
    assert!(!pdf_presenter::settings::Settings::default().blank_picture);
}
