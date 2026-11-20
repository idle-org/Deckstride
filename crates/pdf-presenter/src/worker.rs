//! Bounded private pipe protocol and supervised, killable background workers.
use crate::{
    document::{self, DeckInfo, Frame, FrameInfo},
    settings::{self, Settings},
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
const MAX_JSON: usize = 16 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    Open {
        epoch: u64,
        path: PathBuf,
        cache: PathBuf,
    },
    Render {
        epoch: u64,
        path: PathBuf,
        cache: PathBuf,
        page: usize,
        edge: u32,
    },
    Notes {
        path: PathBuf,
        cache: PathBuf,
    },
    PreviewNotesImport {
        path: PathBuf,
        cache: PathBuf,
        source_sha: String,
        import: PathBuf,
    },
    ApplyNotesImport {
        path: PathBuf,
        cache: PathBuf,
        source_sha: String,
        preview: crate::notes::ImportPreview,
    },
    PickNotes {
        owner: Option<isize>,
    },
    PrepareNotes {
        path: PathBuf,
        cache: PathBuf,
        source_sha: String,
    },
    SaveNotes {
        path: PathBuf,
        cache: PathBuf,
        source_sha: String,
        revision: String,
        page: usize,
        text: String,
    },
    Preview {
        epoch: u64,
        path: PathBuf,
        cache: PathBuf,
        page: usize,
        edge: u32,
    },
    ReadFrame(PathBuf),
    LoadSettings(PathBuf),
    CleanCache {
        path: PathBuf,
        policy: crate::cache::Policy,
        protected: Vec<String>,
        manual: bool,
    },
    SaveSettings {
        path: PathBuf,
        settings: Box<Settings>,
    },
    Pick {
        folder: bool,
        /// Windows owner HWND; never a pointer dereferenced by the worker.
        owner: Option<isize>,
    },
    SleepTest(u64),
}
#[derive(Default, Serialize, Deserialize)]
pub struct Reply {
    pub error: Option<String>,
    pub info: Option<DeckInfo>,
    pub frame: Option<FrameInfo>,
    pub settings: Option<Settings>,
    pub notes: Option<crate::notes::Snapshot>,
    pub notes_import: Option<crate::notes::ImportPreview>,
    pub picked: Option<PathBuf>,
    pub bytes: usize,
    pub edge: Option<u32>,
    pub cached: bool,
    pub cleanup: Option<crate::cache::Report>,
}
pub struct ResultPacket {
    pub reply: Reply,
    pub data: Vec<u8>,
}
pub struct Job {
    pub generation: u64,
    pub tag: u64,
    pub request: Request,
}
pub struct Completion {
    pub generation: u64,
    pub tag: u64,
    pub result: Result<ResultPacket, String>,
}
pub fn write_json<T: Serialize>(writer: &mut impl Write, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_JSON {
        return Err("IPC message too large".into());
    }
    writer
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .and_then(|_| writer.write_all(&bytes))
        .and_then(|_| writer.flush())
        .map_err(|e| e.to_string())
}
pub fn read_json<T: serde::de::DeserializeOwned>(reader: &mut impl Read) -> Result<T, String> {
    let mut prefix = [0; 4];
    reader.read_exact(&mut prefix).map_err(|e| e.to_string())?;
    let len = u32::from_le_bytes(prefix) as usize;
    if len > MAX_JSON {
        return Err("IPC message exceeds limit".into());
    }
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
fn packet(reader: &mut impl Read) -> Result<ResultPacket, String> {
    let reply: Reply = read_json(reader)?;
    if reply.bytes > document::MAX_FRAME {
        return Err("IPC frame exceeds limit".into());
    }
    let mut data = vec![0; reply.bytes];
    reader.read_exact(&mut data).map_err(|e| e.to_string())?;
    if let Some(frame) = &reply.frame {
        document::validate_frame(frame.width, frame.height, data.len())?;
    } else if !data.is_empty() {
        return Err("Unexpected IPC payload".into());
    }
    Ok(ResultPacket { reply, data })
}
pub fn limit_worker() {
    #[cfg(target_os = "linux")]
    unsafe {
        let parent = libc::getppid();
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
        if libc::getppid() != parent {
            std::process::exit(1);
        }
    }
    #[cfg(unix)]
    {
        let limits = libc::rlimit {
            rlim_cur: 3 * 1024 * 1024 * 1024,
            rlim_max: 3 * 1024 * 1024 * 1024,
        }; // Worker-only address space ceiling.
        unsafe {
            libc::setrlimit(libc::RLIMIT_AS, &limits);
        }
    }
}
pub fn run() -> Result<(), String> {
    limit_worker();
    let mut source: Option<document::Document> = None;
    let mut source_epoch = 0;
    let mut input = std::io::stdin().lock();
    let mut out = std::io::stdout().lock();
    while let Ok(request) = read_json::<Request>(&mut input) {
        let result = (|| -> Result<ResultPacket, String> {
            let mut reply = Reply::default();
            let mut data = vec![];
            let preview = matches!(&request, Request::Preview { .. });
            match request {
                Request::Open { epoch, path, cache } => {
                    let doc = document::Document::open_slides(&path, cache)?;
                    reply.info = Some(doc.info.clone());
                    source = Some(doc);
                    source_epoch = epoch;
                }
                Request::Preview {
                    epoch,
                    path,
                    cache,
                    page,
                    edge,
                }
                | Request::Render {
                    epoch,
                    path,
                    cache,
                    page,
                    edge,
                } => {
                    if source_epoch != epoch || source.as_ref().is_none_or(|d| d.info.path != path)
                    {
                        source = Some(document::Document::open_slides(&path, cache)?);
                        source_epoch = epoch;
                    }
                    let doc = source.as_ref().unwrap();
                    let (frame, actual_edge, cached) =
                        if let Some(frame) = doc.cached(page, edge)? {
                            (frame, edge, true)
                        } else {
                            let actual_edge = if preview { 512.min(edge) } else { edge };
                            (doc.render(page, actual_edge)?, actual_edge, false)
                        };
                    let Frame { info, rgba } = frame;
                    reply.edge = Some(actual_edge);
                    reply.cached = cached;
                    reply.frame = Some(info);
                    data = rgba;
                }
                Request::Notes { path, cache } => {
                    reply.info = Some(document::Document::open(&path, cache)?.info);
                }
                Request::PreviewNotesImport {
                    path,
                    cache,
                    source_sha,
                    import,
                } => {
                    reply.notes_import = Some(crate::notes::preview_import(
                        &path,
                        cache,
                        &source_sha,
                        &import,
                    )?);
                }
                Request::ApplyNotesImport {
                    path,
                    cache,
                    source_sha,
                    preview,
                } => {
                    reply.notes = Some(crate::notes::apply_import(
                        &path,
                        cache,
                        &source_sha,
                        preview,
                    )?);
                }
                Request::PickNotes { owner } => {
                    reply.picked = picker_dialog(owner)?
                        .add_filter("Notes (TXT, Markdown)", &["txt", "md"])
                        .pick_file();
                }
                Request::PrepareNotes {
                    path,
                    cache,
                    source_sha,
                } => {
                    reply.notes = Some(crate::notes::prepare(&path, cache, &source_sha)?);
                }
                Request::SaveNotes {
                    path,
                    cache,
                    source_sha,
                    revision,
                    page,
                    text,
                } => {
                    reply.notes = Some(crate::notes::save(
                        &path,
                        cache,
                        &source_sha,
                        &revision,
                        page,
                        text,
                    )?);
                }
                Request::ReadFrame(path) => {
                    let (width, height, rgba) = document::read_frame(&path)?;
                    reply.frame = Some(FrameInfo {
                        page: 0,
                        width,
                        height,
                        path,
                        sha256: String::new(),
                    });
                    data = rgba;
                }
                Request::CleanCache {
                    path,
                    policy,
                    protected,
                    manual,
                } => {
                    reply.cleanup = Some(crate::cache::clean(
                        &path,
                        &policy,
                        &protected,
                        manual,
                        std::time::SystemTime::now(),
                    )?);
                }
                Request::LoadSettings(path) => reply.settings = Some(settings::read(&path)?),
                Request::SaveSettings { path, settings } => settings::write(&path, &settings)?,
                Request::Pick { folder, owner } => {
                    let dialog = picker_dialog(owner)?;
                    reply.picked = if folder {
                        dialog.pick_folder()
                    } else {
                        dialog.add_filter("PDF", &["pdf"]).pick_file()
                    };
                }
                Request::SleepTest(ms) => std::thread::sleep(Duration::from_millis(ms.min(60_000))),
            }
            reply.bytes = data.len();
            Ok(ResultPacket { reply, data })
        })();
        let result = result.unwrap_or_else(|error| ResultPacket {
            reply: Reply {
                error: Some(error),
                ..Default::default()
            },
            data: vec![],
        });
        write_json(&mut out, &result.reply)?;
        out.write_all(&result.data)
            .and_then(|_| out.flush())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(crate) fn quiet_child(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let _ = command;
}
pub fn spawn_mode(mode: &str) -> Result<Child, String> {
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    quiet_child(&mut command);
    command.spawn().map_err(|e| e.to_string())
}
struct Process {
    child: Child,
    input: std::process::ChildStdin,
    replies: Receiver<Result<ResultPacket, String>>,
}
impl Process {
    fn new(program: &std::path::Path) -> Result<Self, String> {
        let mut command = Command::new(program);
        command
            .arg("--worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        quiet_child(&mut command);
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let input = child.stdin.take().ok_or("Worker stdin missing")?;
        let mut output = child.stdout.take().ok_or("Worker stdout missing")?;
        let (tx, replies) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            loop {
                let result = packet(&mut output);
                let failed = result.is_err();
                if tx.send(result).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            input,
            replies,
        })
    }
    fn execute(
        &mut self,
        request: &Request,
        stop: &AtomicBool,
        timeout: Duration,
    ) -> Result<ResultPacket, String> {
        write_json(&mut self.input, request)?;
        let deadline = Instant::now() + timeout;
        loop {
            if stop.load(Ordering::Relaxed) {
                return Err("Cancelled".into());
            }
            match self.replies.recv_timeout(Duration::from_millis(20)) {
                Ok(r) => return r,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err("Worker exited".into()),
                Err(_) => {}
            }
            if Instant::now() >= deadline {
                return Err("Worker deadline exceeded; process terminated".into());
            }
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub struct Bridge {
    pub tx: SyncSender<Job>,
    pub rx: Receiver<Completion>,
    stop: Arc<AtomicBool>,
}
impl Bridge {
    pub fn new(timeout: Duration) -> Self {
        Self::with_program(timeout, std::env::current_exe().unwrap_or_default())
    }
    pub fn with_program(timeout: Duration, program: PathBuf) -> Self {
        let (tx, jobs) = mpsc::sync_channel::<Job>(4);
        let (results, rx) = mpsc::sync_channel(4);
        let stop = Arc::new(AtomicBool::new(false));
        let end = stop.clone();
        std::thread::spawn(move || {
            let mut process: Option<Process> = None;
            while !end.load(Ordering::Relaxed) {
                let job = match jobs.recv_timeout(Duration::from_millis(20)) {
                    Ok(j) => j,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => break,
                };
                let result = (|| {
                    if process.is_none() {
                        process = Some(Process::new(&program)?);
                    }
                    process
                        .as_mut()
                        .unwrap()
                        .execute(&job.request, &end, timeout)
                })();
                if result.is_err() {
                    process = None;
                }
                let mut completion = Completion {
                    generation: job.generation,
                    tag: job.tag,
                    result,
                };
                loop {
                    match results.try_send(completion) {
                        Ok(()) => break,
                        Err(mpsc::TrySendError::Full(c)) => {
                            completion = c;
                            if end.load(Ordering::Relaxed) {
                                return;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => return,
                    }
                }
            }
        });
        Self { tx, rx, stop }
    }
    pub fn submit(&self, generation: u64, tag: u64, request: Request) -> bool {
        self.tx
            .try_send(Job {
                generation,
                tag,
                request,
            })
            .is_ok()
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn picker_dialog(owner: Option<isize>) -> Result<rfd::FileDialog, String> {
    let dialog = rfd::FileDialog::new().set_title("Open presentation");
    #[cfg(target_os = "windows")]
    {
        use winit::raw_window_handle::{
            DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
            Win32WindowHandle, WindowHandle,
        };
        struct Owner(std::num::NonZeroIsize);
        impl HasWindowHandle for Owner {
            fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
                // The parent passes its live presenter HWND over private same-build IPC.
                // rfd copies this OS identifier; no worker memory is dereferenced.
                Ok(unsafe {
                    WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(self.0)))
                })
            }
        }
        impl HasDisplayHandle for Owner {
            fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
                Ok(DisplayHandle::windows())
            }
        }
        let owner = owner
            .and_then(std::num::NonZeroIsize::new)
            .ok_or("No presenter window available for the file chooser")?;
        Ok(dialog.set_parent(&Owner(owner)))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = owner;
        Ok(dialog)
    }
}
