#[test]
fn executable_identifies_its_actual_status() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_pdf-presenter"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains(env!("CARGO_PKG_VERSION")));
    assert!(text.contains("dual-screen proof"));
}
