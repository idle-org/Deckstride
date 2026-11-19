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
    let output = run_native(&["--smoke-test"]);
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
