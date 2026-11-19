mod common;
use std::{
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn run_native(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdf-presenter"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!(
                "Native test exceeded 20 seconds: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn native_windows_smoke_when_requested() {
    if std::env::var_os("PDF_PRESENTER_GUI_TESTS").is_none() {
        eprintln!(
            "Native smoke not requested; set PDF_PRESENTER_GUI_TESTS=1 with a display server."
        );
        return;
    }
    let listed = run_native(&["--list-monitors"]);
    assert!(
        listed.status.success(),
        "{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    assert!(String::from_utf8_lossy(&listed.stdout).contains("Display 1:"));
    let proof = run_native(&["--display-proof", "--smoke-test"]);
    assert!(
        proof.status.success(),
        "{}",
        String::from_utf8_lossy(&proof.stderr)
    );
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("native.pdf");
    common::pdf(&path);
    let output = run_native(&["--smoke-test", "--open", path.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Native smoke passed"));
}

#[test]
fn invalid_cli_fails_cleanly() {
    let result = Command::new(env!("CARGO_BIN_EXE_pdf-presenter"))
        .arg("--bad-option")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("Unknown option"));
}

#[test]
fn independent_output_acknowledges_frames_and_blanking() {
    if std::env::var_os("PDF_PRESENTER_GUI_TESTS").is_none() {
        return;
    }
    use pdf_presenter::{
        document::Document,
        output::{Ack, HostState},
        worker::{read_json, write_json},
    };
    let listing = run_native(&["--list-monitors"]);
    let listing = String::from_utf8(listing.stdout).unwrap();
    let line = listing
        .lines()
        .find(|l| l.starts_with("Display 1:"))
        .unwrap();
    let name = line
        .split(" at ")
        .next()
        .unwrap()
        .rsplit_once(' ')
        .unwrap()
        .0
        .split_once(": ")
        .unwrap()
        .1;
    let monitor = format!("{name}#1");
    let temp = tempfile::tempdir().unwrap();
    let pdf = temp.path().join("host.pdf");
    common::pdf(&pdf);
    let doc = Document::open(&pdf, temp.path().join("cache")).unwrap();
    let frames = [doc.render(0, 320).unwrap(), doc.render(1, 320).unwrap()];
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_pdf-presenter"))
            .arg("--presentation-host")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut input = child.0.stdin.take().unwrap();
    let mut output = child.0.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(a) = read_json::<Ack>(&mut output) {
            if tx.send(a).is_err() {
                break;
            }
        }
    });
    for (sequence, page, blanked) in [(1, 0, false), (2, 1, false), (3, 1, true)] {
        write_json(
            &mut input,
            &HostState {
                sequence,
                page,
                frame: Some(frames[page].info.clone()),
                monitors: vec![monitor.clone()],
                fullscreen: false,
                blanked,
                ..Default::default()
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let ack = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("host frame deadline");
            if ack.sequence == sequence && ack.page == (!blanked).then_some(page) {
                break;
            }
        }
    }
    write_json(
        &mut input,
        &HostState {
            quit: true,
            ..Default::default()
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.0.try_wait().unwrap().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(child.0.try_wait().unwrap().is_some(), "host did not exit");
}
