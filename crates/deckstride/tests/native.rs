mod common;
use std::{
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn run_native(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_deckstride"))
        .args(args)
        .stdin(Stdio::null())
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
    if std::env::var_os("DECKSTRIDE_GUI_TESTS").is_none() {
        eprintln!("Native smoke not requested; set DECKSTRIDE_GUI_TESTS=1 with a display server.");
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
    let result = Command::new(env!("CARGO_BIN_EXE_deckstride"))
        .arg("--bad-option")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("Unknown option"));
}

#[test]
fn independent_output_acknowledges_frames_and_blanking() {
    if std::env::var_os("DECKSTRIDE_GUI_TESTS").is_none() {
        return;
    }
    use deckstride::{
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
    for page in 0..8 {
        common::image(
            &temp.path().join(format!("slide{page}.png")),
            [page * 30, 128, 220, 255],
        );
    }
    let doc = Document::open(temp.path(), temp.path().join("cache")).unwrap();
    let frames: Vec<_> = (0..8).map(|page| doc.render(page, 320).unwrap()).collect();
    let blank_picture = deckstride::blank::prepare(
        Some(&temp.path().join("slide0.png")),
        &temp.path().join("blank-picture.rgba"),
    )
    .unwrap();
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_deckstride"))
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
    write_json(
        &mut input,
        &HostState {
            generation: 99,
            frame: Some(frames[0].info.clone()),
            preload: frames.iter().map(|f| f.info.clone()).collect(),
            buffer_mib: 256,
            monitors: vec![monitor.clone()],
            blanked: true,
            ..Default::default()
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let ack = rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("full output buffer deadline");
        if ack.generation == 99 && ack.buffered == 8 {
            break;
        }
    }
    // Rolling-buffer updates evict absent entries, then reload them from processed
    // files when they re-enter the working set, without marking them as failed.
    for (sequence, count, page) in [(90, 2, 0), (91, 8, 7)] {
        write_json(
            &mut input,
            &HostState {
                sequence,
                page,
                generation: 99,
                frame: Some(frames[page].info.clone()),
                preload: frames[..count].iter().map(|f| f.info.clone()).collect(),
                buffer_mib: 256,
                monitors: vec![monitor.clone()],
                fullscreen: false,
                blanked: false,
                ..Default::default()
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let ack = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("output working-set reload deadline");
            assert!(ack.status.is_empty(), "{}", ack.status);
            if ack.sequence == sequence && ack.buffered == count && ack.page == Some(page) {
                break;
            }
        }
    }
    // The independent output must navigate entirely from RAM after preparation.
    std::fs::remove_dir_all(temp.path().join("cache")).unwrap();
    for page in 0..8 {
        std::fs::remove_file(temp.path().join(format!("slide{page}.png"))).unwrap();
    }
    for (sequence, page, blanked) in [
        (1, 7, false),
        (2, 0, false),
        (3, 6, false),
        (4, 1, false),
        (5, 5, false),
        (6, 2, false),
        (7, 4, false),
        (8, 3, true),
    ] {
        write_json(
            &mut input,
            &HostState {
                zoom: deckstride::zoom::Sync {
                    enabled: true,
                    view: deckstride::zoom::View {
                        scale: 2.,
                        center: [0.5; 2],
                    },
                    epoch: sequence * 2,
                    serial: 0,
                    keep_fitted_area: false,
                },
                sequence,
                page,
                generation: 99,
                preload: frames.iter().map(|f| f.info.clone()).collect(),
                buffer_mib: 256,
                frame: Some(frames[page].info.clone()),
                monitors: vec![monitor.clone()],
                fullscreen: false,
                blanked,
                blank_picture: Some(blank_picture.info.clone()),
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
        // Zoom-only commands carry no frame, preload, routes or visibility fields.
        // They must retain every buffered frame after all source/cache files are gone.
        write_json(
            &mut input,
            &HostState {
                zoom_only: true,
                generation: 99,
                page,
                zoom: deckstride::zoom::Sync {
                    enabled: true,
                    view: deckstride::zoom::View {
                        scale: 4.,
                        center: [0.4, 0.6],
                    },
                    epoch: sequence * 2 + 1,
                    serial: 0,
                    keep_fitted_area: false,
                },
                ..Default::default()
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let ack = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("zoom retained buffer deadline");
            assert!(ack.status.is_empty(), "{}", ack.status);
            if ack.sequence == sequence
                && ack.generation == 99
                && ack.buffered == 8
                && ack.page == (!blanked).then_some(page)
            {
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

#[cfg(target_os = "linux")]
#[test]
fn orphaned_output_holds_until_closed_then_exits_on_close_or_escape() {
    if std::env::var_os("DECKSTRIDE_GUI_TESTS").is_none() {
        return;
    }
    use deckstride::{output::HostState, worker::write_json};
    use x11_dl::xlib;
    let x = xlib::Xlib::open().unwrap();
    // A private X11 connection is used only by this test thread.
    let display = unsafe { (x.XOpenDisplay)(std::ptr::null()) };
    assert!(!display.is_null());
    struct DisplayGuard<'a>(&'a xlib::Xlib, *mut xlib::Display);
    impl Drop for DisplayGuard<'_> {
        fn drop(&mut self) {
            unsafe {
                (self.0.XCloseDisplay)(self.1);
            }
        }
    }
    let _display = DisplayGuard(&x, display);
    // Other desktop clients can destroy windows between enumeration and property
    // lookup. BadWindow is an expected race; other protocol errors still fail.
    static X_ERROR: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
    unsafe extern "C" fn on_x_error(
        _: *mut xlib::Display,
        event: *mut xlib::XErrorEvent,
    ) -> std::os::raw::c_int {
        let code = unsafe { (*event).error_code };
        if code != xlib::BadWindow {
            X_ERROR.store(code, std::sync::atomic::Ordering::Relaxed);
        }
        0
    }
    let previous_handler = unsafe { (x.XSetErrorHandler)(Some(on_x_error)) };
    struct ErrorGuard<'a>(
        &'a xlib::Xlib,
        Option<
            unsafe extern "C" fn(*mut xlib::Display, *mut xlib::XErrorEvent) -> std::os::raw::c_int,
        >,
    );
    impl Drop for ErrorGuard<'_> {
        fn drop(&mut self) {
            unsafe {
                (self.0.XSetErrorHandler)(self.1);
            }
        }
    }
    let _errors = ErrorGuard(&x, previous_handler);

    let root = unsafe { (x.XDefaultRootWindow)(display) };
    let pid_atom = unsafe { (x.XInternAtom)(display, c"_NET_WM_PID".as_ptr(), 0) };
    fn find(
        x: &xlib::Xlib,
        d: *mut xlib::Display,
        w: xlib::Window,
        atom: xlib::Atom,
        pid: u32,
    ) -> Option<xlib::Window> {
        // Xlib owns returned buffers; free them after copying their contents.
        unsafe {
            let (mut kind, mut format, mut n, mut after) = (0, 0, 0, 0);
            let mut data = std::ptr::null_mut();
            (x.XGetWindowProperty)(
                d,
                w,
                atom,
                0,
                1,
                0,
                xlib::XA_CARDINAL,
                &mut kind,
                &mut format,
                &mut n,
                &mut after,
                &mut data,
            );
            let matches = !data.is_null()
                && format == 32
                && n == 1
                && *(data as *const std::os::raw::c_ulong) == u64::from(pid);
            if !data.is_null() {
                (x.XFree)(data.cast());
            }
            if matches {
                return Some(w);
            }
            let (mut root, mut parent, mut count) = (0, 0, 0);
            let mut children = std::ptr::null_mut();
            if (x.XQueryTree)(d, w, &mut root, &mut parent, &mut children, &mut count) == 0 {
                return None;
            }
            let found = if children.is_null() {
                None
            } else {
                std::slice::from_raw_parts(children, count as usize)
                    .iter()
                    .find_map(|child| find(x, d, *child, atom, pid))
            };
            if !children.is_null() {
                (x.XFree)(children.cast());
            }
            found
        }
    }
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
    for escape in [false, true] {
        struct ChildGuard(std::process::Child);
        impl Drop for ChildGuard {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut child = ChildGuard(
            Command::new(env!("CARGO_BIN_EXE_deckstride"))
                .env("WINIT_UNIX_BACKEND", "x11")
                .arg("--presentation-host")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let mut input = child.0.stdin.take().unwrap();
        write_json(
            &mut input,
            &HostState {
                monitors: vec![format!("{name}#1")],
                fullscreen: false,
                ..Default::default()
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let window = loop {
            if let Some(window) = find(&x, display, root, pid_atom, child.0.id()) {
                break window;
            }
            assert!(Instant::now() < deadline, "output window not created");
            std::thread::sleep(Duration::from_millis(10));
        };
        // Verify the icon actually reached X11, not just WindowAttributes.
        unsafe {
            let icon_atom = (x.XInternAtom)(display, c"_NET_WM_ICON".as_ptr(), 0);
            let (mut kind, mut format, mut count, mut after) = (0, 0, 0, 0);
            let mut data = std::ptr::null_mut();
            (x.XGetWindowProperty)(
                display,
                window,
                icon_atom,
                0,
                2,
                0,
                xlib::XA_CARDINAL,
                &mut kind,
                &mut format,
                &mut count,
                &mut after,
                &mut data,
            );
            assert!(!data.is_null());
            let dimensions =
                std::slice::from_raw_parts(data.cast::<std::os::raw::c_ulong>(), count as usize)
                    .to_vec();
            (x.XFree)(data.cast());
            assert_eq!(format, 32);
            assert_eq!(dimensions, [256, 256]);
            assert_eq!(after, 256 * 256 * 4);
        }
        assert_eq!(X_ERROR.load(std::sync::atomic::Ordering::Relaxed), 0);
        // EOF models a vanished presenter. Keep the output alive until the operator closes it.
        drop(input);
        std::thread::sleep(Duration::from_millis(300));
        assert!(child.0.try_wait().unwrap().is_none());
        unsafe {
            let mut event: xlib::XEvent = std::mem::zeroed();
            if escape {
                event.key = xlib::XKeyEvent {
                    type_: xlib::KeyPress,
                    display,
                    window,
                    root,
                    keycode: (x.XKeysymToKeycode)(display, 0xff1b).into(),
                    same_screen: 1,
                    ..std::mem::zeroed()
                };
            } else {
                let mut data = xlib::ClientMessageData::new();
                data.set_long(
                    0,
                    (x.XInternAtom)(display, c"WM_DELETE_WINDOW".as_ptr(), 0) as _,
                );
                event.client_message = xlib::XClientMessageEvent {
                    type_: xlib::ClientMessage,
                    display,
                    window,
                    message_type: (x.XInternAtom)(display, c"WM_PROTOCOLS".as_ptr(), 0),
                    format: 32,
                    data,
                    ..std::mem::zeroed()
                };
            }
            (x.XSendEvent)(
                display,
                window,
                0,
                if escape { xlib::KeyPressMask } else { 0 },
                &mut event,
            );
            (x.XFlush)(display);
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while child.0.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            child.0.try_wait().unwrap().is_some(),
            "orphan ignored {}",
            if escape { "Escape" } else { "Close" }
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn wayland_browser_parents_to_presenter_before_first_buffer() {
    if std::env::var_os("DECKSTRIDE_GUI_TESTS").is_none()
        || std::env::var_os("WAYLAND_DISPLAY").is_none()
    {
        return;
    }
    let trace = tempfile::NamedTempFile::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_deckstride"))
        .arg("--smoke-test")
        .env("WAYLAND_DEBUG", "client")
        .stdout(Stdio::null())
        .stderr(trace.reopen().unwrap())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "{}",
                std::fs::read_to_string(trace.path()).unwrap()
            );
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Wayland browser smoke timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let log = std::fs::read_to_string(trace.path()).unwrap();
    let top = |title: &str| {
        log.lines()
            .find(|line| line.contains(&format!(".set_title(\"{title}\")")))
            .unwrap()
            .split(" -> ")
            .nth(1)
            .unwrap()
            .split('.')
            .next()
            .unwrap()
    };
    let presenter = top("Deckstride — Presenter");
    let browser = top("Deckstride — All slides");
    assert_ne!(browser, presenter);
    let parent_at = log
        .find(&format!("{browser}.set_parent({presenter})"))
        .unwrap();
    let xdg = log
        .lines()
        .find(|l| l.contains(&format!(".get_toplevel(new id {browser})")))
        .unwrap()
        .split(" -> ")
        .nth(1)
        .unwrap()
        .split('.')
        .next()
        .unwrap();
    let surface = log
        .lines()
        .find(|l| l.contains(&format!(".get_xdg_surface(new id {xdg},")))
        .unwrap()
        .split(", ")
        .last()
        .unwrap()
        .trim_end_matches(')');
    let buffer_at = log.find(&format!("{surface}.attach(wl_buffer")).unwrap();
    assert!(
        parent_at < buffer_at,
        "The parent hint must precede mapping"
    );
    assert!(
        log.contains(&format!("{browser}.set_max_size(0, 0)")),
        "Browser must remain resizable"
    );
    assert!(
        !log.contains(&format!("{presenter}.set_parent(")),
        "Only the browser is parented"
    );
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn unsupported_sleep_backend_reports_unavailable_and_exits_on_parent_eof() {
    let output = run_native(&["--keep-awake"]);
    assert!(output.status.success());
    let status: String = deckstride::worker::read_json(&mut output.stdout.as_slice()).unwrap();
    assert!(status.starts_with("Sleep protection unavailable:"));
    assert!(status.contains(std::env::consts::OS));
}

#[test]
fn keep_awake_status_uses_private_length_prefixed_json() {
    let mut framed = Vec::new();
    deckstride::worker::write_json(&mut framed, &"Sleep protection test").unwrap();
    assert!(serde_json::from_slice::<String>(&framed).is_err());
    let decoded: String = deckstride::worker::read_json(&mut framed.as_slice()).unwrap();
    assert_eq!(decoded, "Sleep protection test");
}
