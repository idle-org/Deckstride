//! Independent presentation host IPC and process-owned sleep protection.
use crate::{
    document::FrameInfo,
    worker::{read_json, spawn_mode, write_json},
};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum DisplayAction {
    Swap,
    Keep,
    Revert,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct HostState {
    #[serde(default)]
    pub zoom: crate::zoom::Sync,
    #[serde(default)]
    pub zoom_only: bool,
    #[serde(default)]
    pub slide_colors: crate::settings::SlideColors,
    #[serde(default)]
    pub setup_controls: bool,
    #[serde(default)]
    pub route_trial: Option<u64>,
    pub sequence: u64,
    #[serde(default)]
    pub generation: u64,
    pub page: usize,
    pub frame: Option<FrameInfo>,
    pub preload: Vec<FrameInfo>,
    #[serde(default)]
    pub buffer_mib: u32,
    pub monitors: Vec<String>,
    pub fullscreen: bool,
    pub blanked: bool,
    pub quit: bool,
}
/// Actual size of the reference presentation window, never presenter content.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputViewport {
    pub monitor: String,
    pub size: [u32; 2],
    pub fullscreen: bool,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Ack {
    #[serde(default)]
    pub viewport: Option<OutputViewport>,
    #[serde(default)]
    pub zoom: Option<crate::zoom::Sync>,
    #[serde(default)]
    pub zoom_slow: bool,
    /// Explicit operator blanking, separate from diagnostic status messages.
    #[serde(default)]
    pub request_blank: bool,
    #[serde(default)]
    pub display_action: Option<DisplayAction>,
    #[serde(default)]
    pub route_trial: Option<u64>,
    pub sequence: u64,
    pub page: Option<usize>,
    pub status: String,
    pub awake: String,
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub buffered: usize,
}
/// A lightweight crop update must never discard queued frame/routing state.
pub fn coalesce(latest: &mut Option<HostState>, command: HostState) {
    if command.zoom_only
        && let Some(previous) = latest
    {
        if previous.generation == command.generation && previous.page == command.page {
            previous.zoom.accept_presenter(command.zoom);
        }
    } else {
        *latest = Some(command);
    }
}
pub struct OutputLink {
    pub tx: SyncSender<HostState>,
    pub rx: Receiver<Ack>,
    stop: Arc<AtomicBool>,
    manager: Option<std::thread::JoinHandle<()>>,
    child: Arc<Mutex<Option<Child>>>,
}
impl Default for OutputLink {
    fn default() -> Self {
        Self::new()
    }
}
impl OutputLink {
    pub fn new() -> Self {
        Self::with_program(std::env::current_exe().unwrap_or_default())
    }
    pub fn with_program(program: PathBuf) -> Self {
        let (tx, commands) = mpsc::sync_channel::<HostState>(2);
        let (results, rx) = mpsc::sync_channel(8);
        let stop = Arc::new(AtomicBool::new(false));
        let end = stop.clone();
        let child = Arc::new(Mutex::new(None));
        let managed_child = child.clone();
        let manager = std::thread::spawn(move || {
            let mut attempts = 0;
            while !end.load(Ordering::Relaxed) && attempts < 3 {
                attempts += 1;
                let mut command = Command::new(&program);
                command
                    .arg("--presentation-host")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null());
                crate::worker::quiet_child(&mut command);
                let mut process = match command.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = results.try_send(Ack {
                            status: e.to_string(),
                            ..Default::default()
                        });
                        return;
                    }
                };
                let mut stdin = process.stdin.take().unwrap();
                let mut stdout = process.stdout.take().unwrap();
                *managed_child.lock().unwrap() = Some(process);
                let (ack_tx, ack_rx) = mpsc::sync_channel(8);
                std::thread::spawn(move || {
                    while let Ok(ack) = read_json::<Ack>(&mut stdout) {
                        let _ = ack_tx.try_send(ack);
                    }
                });
                let mut heartbeat = Instant::now();
                let mut last: Option<HostState> = None;
                loop {
                    if end.load(Ordering::Relaxed) {
                        let _ = write_json(
                            &mut stdin,
                            &HostState {
                                quit: true,
                                ..Default::default()
                            },
                        );
                        std::thread::sleep(Duration::from_millis(300));
                        terminate_output(&managed_child);
                        return;
                    }
                    let mut latest = None;
                    while let Ok(command) = commands.try_recv() {
                        coalesce(&mut latest, command);
                    }
                    if let Some(command) = latest {
                        if write_json(&mut stdin, &command).is_err() {
                            break;
                        }
                        last = Some(command);
                    }
                    while let Ok(ack) = ack_rx.try_recv() {
                        heartbeat = Instant::now();
                        let _ = results.try_send(ack);
                    }
                    if managed_child
                        .lock()
                        .unwrap()
                        .as_mut()
                        .is_none_or(|c| c.try_wait().ok().flatten().is_some())
                        || heartbeat.elapsed() > Duration::from_secs(8)
                    {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(4));
                }
                terminate_output(&managed_child);
                let _ = results.try_send(Ack {
                    status:
                        "Presentation host stopped. Restarting with the current output visibility."
                            .into(),
                    ..Default::default()
                });
                // Restart receives fresh routes, frame and visibility from the presenter.
                drop(last);
                std::thread::sleep(Duration::from_millis(250));
            }
        });
        Self {
            tx,
            rx,
            stop,
            manager: Some(manager),
            child,
        }
    }
}
impl OutputLink {
    /// Called after the event loop exits, never from an input/render callback.
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let grace = Instant::now() + Duration::from_millis(500);
        while self.manager.as_ref().is_some_and(|t| !t.is_finished()) && Instant::now() < grace {
            std::thread::sleep(Duration::from_millis(10));
        }
        // This does not use the IPC writer: it also works when that writer is blocked.
        terminate_output(&self.child);
        let deadline = Instant::now() + Duration::from_secs(1);
        while self.manager.as_ref().is_some_and(|t| !t.is_finished()) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        if self.manager.as_ref().is_some_and(|t| t.is_finished())
            && let Some(t) = self.manager.take()
        {
            let _ = t.join();
        }
    }
}
fn terminate_output(child: &Mutex<Option<Child>>) {
    if let Some(mut process) = child.lock().unwrap().take() {
        let _ = process.kill();
        let _ = process.wait();
    }
}
impl Drop for OutputLink {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct SleepLease {
    child: Option<Child>,
    pub status: Receiver<String>,
}
impl Default for SleepLease {
    fn default() -> Self {
        Self::new()
    }
}
impl SleepLease {
    pub fn new() -> Self {
        let (tx, status) = mpsc::sync_channel(1);
        let child = match spawn_mode("--keep-awake") {
            Ok(mut child) => {
                let mut stdout = child.stdout.take().unwrap();
                std::thread::spawn(move || {
                    let result = read_json::<String>(&mut stdout)
                        .unwrap_or_else(|e| format!("Sleep protection unavailable: {e}"));
                    let _ = tx.try_send(result);
                });
                Some(child)
            }
            Err(e) => {
                let _ = tx.try_send(format!("Sleep protection unavailable: {e}"));
                None
            }
        };
        Self { child, status }
    }
}
impl Drop for SleepLease {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            drop(child.stdin.take());
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
pub fn keep_awake() -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    let guard = keepawake::Builder::default()
        .display(true)
        .idle(true)
        .reason("PDF Presenter presentation")
        .app_name("PDF Presenter")
        .app_reverse_domain("com.beffroi.pdfpresenter")
        .create();
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    let guard: Result<(), String> = Err(format!(
        "No native sleep-inhibition backend for {}",
        std::env::consts::OS
    ));
    #[cfg(target_os = "linux")]
    let x11 = X11Lease::new();
    let status = match &guard {
        Ok(_) => "Display/system idle inhibition active".to_owned(),
        Err(e) => {
            #[cfg(target_os = "linux")]
            if x11.is_some() {
                return hold_x11_fallback(
                    x11,
                    format!("X11 display/DPMS protected; system sleep unavailable: {e}"),
                );
            }
            format!("Sleep protection unavailable: {e}")
        }
    };
    write_json(&mut std::io::stdout().lock(), &status)?;
    // Holding the guard in this process ties every OS lease to its pipe/lifetime.
    let mut byte = [0];
    let _ = std::io::stdin().read(&mut byte);
    drop(guard);
    Ok(())
}

#[cfg(target_os = "linux")]
struct X11Lease {
    lib: x11_dl::xlib::Xlib,
    xss: x11_dl::xss::Xss,
    display: *mut x11_dl::xlib::Display,
}
#[cfg(target_os = "linux")]
impl X11Lease {
    fn new() -> Option<Self> {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            return None;
        }
        let lib = x11_dl::xlib::Xlib::open().ok()?;
        let xss = x11_dl::xss::Xss::open().ok()?;
        // This helper process is single-threaded; the connection and extension outlive calls.
        let display = unsafe { (lib.XOpenDisplay)(std::ptr::null()) };
        if display.is_null() {
            return None;
        }
        let (mut major, mut minor) = (0, 0);
        let available = unsafe { (xss.XScreenSaverQueryVersion)(display, &mut major, &mut minor) };
        if available == 0 || (major, minor) < (1, 1) {
            unsafe {
                (lib.XCloseDisplay)(display);
            }
            return None;
        }
        unsafe {
            (xss.XScreenSaverSuspend)(display, 1);
            (lib.XFlush)(display);
        }
        Some(Self { lib, xss, display })
    }
}
#[cfg(target_os = "linux")]
impl Drop for X11Lease {
    fn drop(&mut self) {
        unsafe {
            (self.xss.XScreenSaverSuspend)(self.display, 0);
            (self.lib.XCloseDisplay)(self.display);
        }
    }
}
#[cfg(target_os = "linux")]
fn hold_x11_fallback(_lease: Option<X11Lease>, status: String) -> Result<(), String> {
    write_json(&mut std::io::stdout().lock(), &status)?;
    let _ = std::io::stdin().read(&mut [0]);
    Ok(())
}

#[cfg(all(test, unix))]
mod shutdown_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn shutdown_and_drop_reap_output_even_when_its_input_pipe_is_full() {
        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("unresponsive-output");
        std::fs::write(&program, "#!/bin/sh\nexec sleep 60\n").unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        for explicit in [true, false] {
            let mut link = OutputLink::with_program(program.clone());
            let child = link.child.clone();
            let deadline = Instant::now() + Duration::from_secs(5);
            let pid = loop {
                if let Some(process) = child.lock().unwrap().as_ref() {
                    break process.id();
                }
                assert!(Instant::now() < deadline, "output did not spawn");
                std::thread::sleep(Duration::from_millis(10));
            };
            link.tx
                .send(HostState {
                    monitors: vec!["x".repeat(2 * 1024 * 1024)],
                    ..Default::default()
                })
                .unwrap();
            // The child never reads stdin: this message is larger than the pipe capacity.
            std::thread::sleep(Duration::from_millis(100));
            let start = Instant::now();
            if explicit {
                link.shutdown();
                assert!(link.manager.is_none());
            }
            drop(link);
            assert!(start.elapsed() < Duration::from_secs(2));
            assert!(child.lock().unwrap().is_none());
            // The PID is only used immediately after reaping our own child.
            assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
        }
    }
}
