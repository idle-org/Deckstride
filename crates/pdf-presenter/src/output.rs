//! Independent presentation host IPC and process-owned sleep protection.
use crate::{
    document::FrameInfo,
    worker::{read_json, spawn_mode, write_json},
};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    process::Child,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct HostState {
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
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Ack {
    pub sequence: u64,
    pub page: Option<usize>,
    pub status: String,
    pub awake: String,
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub buffered: usize,
}
pub struct OutputLink {
    pub tx: SyncSender<HostState>,
    pub rx: Receiver<Ack>,
    stop: Arc<AtomicBool>,
    manager: Option<std::thread::JoinHandle<()>>,
}
impl Default for OutputLink {
    fn default() -> Self {
        Self::new()
    }
}
impl OutputLink {
    pub fn new() -> Self {
        let (tx, commands) = mpsc::sync_channel::<HostState>(2);
        let (results, rx) = mpsc::sync_channel(8);
        let stop = Arc::new(AtomicBool::new(false));
        let end = stop.clone();
        let manager = std::thread::spawn(move || {
            let mut attempts = 0;
            while !end.load(Ordering::Relaxed) && attempts < 3 {
                attempts += 1;
                let mut child = match spawn_mode("--presentation-host") {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = results.try_send(Ack {
                            status: e,
                            ..Default::default()
                        });
                        return;
                    }
                };
                let mut stdin = child.stdin.take().unwrap();
                let mut stdout = child.stdout.take().unwrap();
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
                        let _ = child.kill();
                        let _ = child.wait();
                        return;
                    }
                    let mut latest = None;
                    while let Ok(command) = commands.try_recv() {
                        latest = Some(command);
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
                    if child.try_wait().ok().flatten().is_some()
                        || heartbeat.elapsed() > Duration::from_secs(8)
                    {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(4));
                }
                let _ = child.kill();
                let _ = child.wait();
                let _=results.try_send(Ack{status:"Presentation host stopped. Output will restart blanked; review screens before resuming.".into(),..Default::default()});
                // Restart receives a fresh state from the presenter; never silently resume output.
                drop(last);
                std::thread::sleep(Duration::from_millis(250));
            }
        });
        Self {
            tx,
            rx,
            stop,
            manager: Some(manager),
        }
    }
}
impl OutputLink {
    /// Called after the event loop exits, never from an input/render callback.
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let deadline = Instant::now() + Duration::from_secs(2);
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
impl Drop for OutputLink {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
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
    let guard = keepawake::Builder::default()
        .display(true)
        .idle(true)
        .reason("PDF Presenter presentation")
        .app_name("PDF Presenter")
        .app_reverse_domain("com.beffroi.pdfpresenter")
        .create();
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
