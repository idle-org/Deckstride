//! Shared native presenter state, scheduling, layout and UI actions.
use crate::{
    document::{DeckInfo, Frame, FrameInfo},
    settings::{self, Layout, Routes, Settings},
    worker::{Bridge, Request},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Clone, Debug)]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub origin: (i32, i32),
    pub size: (u32, u32),
}
#[derive(Clone, Debug)]
pub enum Action {
    Open(PathBuf),
    Pick(bool),
    Go(usize),
    Blank,
    SaveProject,
    SaveGlobal,
    UseGlobal,
    ApplyRoutes,
    OpenBrowser,
    CloseBrowser,
    Refresh,
    Exit,
    Windowed,
}
pub struct Studio {
    pub settings: Settings,
    pub layout: Layout,
    pub monitors: Vec<Monitor>,
    pub routes: Routes,
    pub draft_routes: Routes,
    pub deck: Option<DeckInfo>,
    pub requested: usize,
    pub displayed: Option<usize>,
    pub blanked: bool,
    pub status: String,
    pub awake_status: String,
    pub generation: u64,
    pub frames: HashMap<(usize, u32), Arc<Frame>>,
    order: VecDeque<(usize, u32)>,
    pub visible_thumbs: HashSet<usize>,
    pub layout_open: bool,
    pub displays_open: bool,
    pub menu_open: bool,
    pub browser_open: bool,
    pub browser_selected: usize,
    pub change_on_click: bool,
    pub jump: String,
    pub started: Instant,
    pub actions: Vec<Action>,
    pub settings_path: PathBuf,
    pub cache_path: PathBuf,
    pub fg: Bridge,
    pub bg: Bridge,
    pub io: Bridge,
    pub dialog: Bridge,
    fg_busy: bool,
    bg_busy: bool,
    notes_pending: bool,
    pending_open: Option<PathBuf>,
    pending_save: bool,
    pub initialized: bool,
    pub open_path: Option<PathBuf>,
    pub dirty_routes: bool,
    pub privacy_hold: bool,
    pub output_sequence: u64,
    pub new_output: bool,
    pub close_requested: bool,
    failed: HashSet<(usize, u32)>,
    pub smoke: bool,
    pub startup: Instant,
    _smoke_dir: Option<tempfile::TempDir>,
}
impl Studio {
    pub fn new(smoke: bool, open_path: Option<PathBuf>) -> Self {
        let (mut settings_path, mut cache_path) = settings::paths().unwrap_or_else(|_| {
            (
                PathBuf::from("settings.toml"),
                std::env::temp_dir().join("pdf-presenter-cache"),
            )
        });
        let smoke_dir = if smoke {
            tempfile::tempdir().ok()
        } else {
            None
        };
        if let Some(dir) = &smoke_dir {
            settings_path = dir.path().join("settings.toml");
            cache_path = dir.path().join("cache");
        }
        let io = Bridge::new(Duration::from_secs(10));
        io.submit(0, 0, Request::LoadSettings(settings_path.clone()));
        Self {
            settings: Settings::default(),
            layout: Layout::default(),
            monitors: vec![],
            routes: Routes::default(),
            draft_routes: Routes::default(),
            deck: None,
            requested: 0,
            displayed: None,
            blanked: true,
            status: "Ready — open a PDF or picture folder, or drop it here.".into(),
            awake_status: "Sleep protection starting…".into(),
            generation: 0,
            frames: HashMap::new(),
            order: VecDeque::new(),
            visible_thumbs: HashSet::new(),
            layout_open: false,
            displays_open: false,
            menu_open: false,
            browser_open: false,
            browser_selected: 0,
            change_on_click: false,
            jump: String::new(),
            started: Instant::now(),
            actions: vec![],
            settings_path,
            cache_path,
            fg: Bridge::new(Duration::from_secs(30)),
            bg: Bridge::new(Duration::from_secs(30)),
            io,
            dialog: Bridge::new(Duration::from_secs(300)),
            fg_busy: false,
            bg_busy: false,
            notes_pending: false,
            pending_open: None,
            pending_save: false,
            initialized: false,
            open_path,
            dirty_routes: true,
            privacy_hold: false,
            output_sequence: 0,
            new_output: true,
            close_requested: false,
            failed: HashSet::new(),
            smoke,
            startup: Instant::now(),
            _smoke_dir: smoke_dir,
        }
    }
    pub fn count(&self) -> usize {
        self.deck.as_ref().map_or(0, |d| d.names.len())
    }
    pub fn project(&self) -> String {
        self.deck
            .as_ref()
            .map_or_else(|| "empty".into(), |d| d.key.clone())
    }
    pub fn frame(&self, page: usize) -> Option<Arc<Frame>> {
        [2560, 1280, 320]
            .iter()
            .find_map(|&edge| self.frames.get(&(page, edge)).cloned())
    }
    pub fn full_frame(&self, page: usize) -> Option<Arc<Frame>> {
        [2560, 1280]
            .iter()
            .find_map(|&edge| self.frames.get(&(page, edge)).cloned())
    }
    pub fn frame_info(&self) -> Option<FrameInfo> {
        self.full_frame(self.requested).map(|f| f.info.clone())
    }
    pub fn set_monitors(&mut self, monitors: Vec<Monitor>, primary: usize) {
        let ids = monitors.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
        let changed = self
            .monitors
            .iter()
            .map(|m| (&m.id, m.origin, m.size))
            .collect::<Vec<_>>()
            != monitors
                .iter()
                .map(|m| (&m.id, m.origin, m.size))
                .collect::<Vec<_>>();
        let previous_outputs = self.routes.presentation.clone();
        let recovering = !self.monitors.is_empty();
        self.monitors = monitors;
        if changed {
            self.routes = self.settings.routes.resolve(&ids, primary);
            // A lost operator screen must not turn a previously public output into a notes window.
            if recovering {
                self.routes
                    .presenter
                    .retain(|id| !previous_outputs.contains(id));
                if self.routes.presenter.is_empty()
                    && let Some(id) = ids.iter().find(|id| !previous_outputs.contains(id))
                {
                    self.routes.presenter.push(id.clone());
                    self.routes.presentation.retain(|output| output != id);
                }
            }
            self.privacy_hold = recovering && self.routes.presenter.is_empty();
            self.draft_routes = self.routes.clone();
            self.dirty_routes = true;
            self.blanked = true;
            self.new_output = true;
            self.status="Screen setup refreshed. Presentation is blanked; review destinations and Show when ready.".into();
        }
    }
    pub fn execute(&mut self, action: Action) {
        match action {
            Action::Open(path) => {
                if self.fg_busy {
                    self.fg = Bridge::new(Duration::from_secs(30));
                    self.fg_busy = false;
                }
                self.pending_open = Some(path);
                self.status = "Opening document in an isolated worker…".into();
            }
            Action::Pick(folder) => {
                if !self.dialog.submit(0, 0, Request::Pick { folder }) {
                    self.status = "A file chooser is already active.".into();
                }
            }
            Action::Go(page) => {
                if page < self.count() {
                    if page != self.requested && self.fg_busy {
                        self.fg = Bridge::new(Duration::from_secs(30));
                        self.fg_busy = false;
                    }
                    self.requested = page;
                    self.jump.clear();
                    self.new_output = true;
                }
            }
            Action::Blank => {
                self.blanked = !self.blanked;
                self.new_output = true;
            }
            Action::SaveProject => {
                let key = self.project();
                self.settings.projects.insert(key, self.layout.clone());
                self.pending_save = true;
            }
            Action::SaveGlobal => {
                self.settings.layout = self.layout.clone();
                self.pending_save = true;
            }
            Action::UseGlobal => {
                self.settings.projects.remove(&self.project());
                self.layout = self.settings.layout.clone();
                self.pending_save = true;
            }
            Action::ApplyRoutes => {
                if self.draft_routes.presenter.is_empty() {
                    self.status = "Choose at least one presenter screen.".into();
                } else if let Err(e) = self.draft_routes.validate() {
                    self.status = e;
                } else {
                    self.routes = self.draft_routes.clone();
                    self.settings.routes = self.routes.clone();
                    self.dirty_routes = true;
                    self.displays_open = false;
                    self.blanked = true;
                    self.new_output = true;
                    self.pending_save = true;
                }
            }
            Action::OpenBrowser => {
                self.browser_open = true;
                self.browser_selected = self.requested;
                self.change_on_click = false;
            }
            Action::CloseBrowser => {
                self.browser_open = false;
                self.pending_save = true;
            }
            Action::Refresh => self.dirty_routes = true,
            Action::Exit => self.close_requested = true,
            Action::Windowed => {
                self.settings.presenter_fullscreen = false;
                self.settings.presentation_fullscreen = false;
                self.blanked = true;
                self.new_output = true;
                self.dirty_routes = true;
            }
        }
    }
    pub fn poll(&mut self) {
        while let Ok(c) = self.io.rx.try_recv() {
            match c.result {
                Ok(packet) => {
                    if let Some(error) = packet.reply.error {
                        self.status = format!("Settings: {error}");
                    } else if let Some(settings) = packet.reply.settings {
                        if !self.smoke {
                            self.settings = settings;
                            self.layout = self.settings.layout.clone();
                            self.routes = Routes::default();
                        }
                        self.dirty_routes = true;
                    } else {
                        self.status =
                            "Settings saved. Project layouts override the all-project default."
                                .into();
                    }
                }
                Err(e) => self.status = format!("Settings: {e}"),
            };
            self.initialized = true;
        }
        if self.initialized
            && let Some(path) = self.open_path.take()
        {
            self.execute(Action::Open(path));
        }
        if self.pending_save
            && self.io.submit(
                0,
                1,
                Request::SaveSettings {
                    path: self.settings_path.clone(),
                    settings: Box::new(self.settings.clone()),
                },
            )
        {
            self.pending_save = false;
        }
        while let Ok(c) = self.dialog.rx.try_recv() {
            match c.result {
                Ok(p) => {
                    if let Some(path) = p.reply.picked {
                        self.execute(Action::Open(path));
                    }
                    if let Some(e) = p.reply.error {
                        self.status = e;
                    }
                }
                Err(e) => self.status = e,
            }
        }
        for foreground in [true, false] {
            loop {
                let received = if foreground {
                    self.fg.rx.try_recv()
                } else {
                    self.bg.rx.try_recv()
                };
                let Ok(c) = received else {
                    break;
                };
                if foreground {
                    self.fg_busy = false;
                } else {
                    self.bg_busy = false;
                }
                if c.generation != self.generation {
                    continue;
                }
                match c.result {
                    Ok(packet) => {
                        if let Some(error) = packet.reply.error {
                            self.status = error;
                            self.failed.insert(((c.tag >> 32) as usize, c.tag as u32));
                            continue;
                        }
                        if let Some(info) = packet.reply.info {
                            if c.tag == u64::MAX {
                                if let Some(deck) = &mut self.deck {
                                    deck.notes = info.notes;
                                    deck.warnings = info.warnings;
                                    if let Some(warning) = deck.warnings.first() {
                                        self.status = warning.clone();
                                    }
                                }
                                continue;
                            }
                            self.notes_pending = true;
                            self.generation += 1;
                            self.frames.clear();
                            self.order.clear();
                            self.failed.clear();
                            self.requested = 0;
                            self.displayed = None;
                            self.layout = self.settings.for_project(&info.key);
                            self.status = info.warnings.first().cloned().unwrap_or_else(|| {
                                format!(
                                    "Opened {} slides. Preparing first slide…",
                                    info.names.len()
                                )
                            });
                            self.deck = Some(info);
                            self.blanked = true;
                            self.new_output = true;
                        }
                        if let Some(info) = packet.reply.frame {
                            let key = (info.page, c.tag as u32);
                            let frame = Arc::new(Frame {
                                info,
                                rgba: packet.data,
                            });
                            self.frames.insert(key, frame);
                            self.order.retain(|k| *k != key);
                            self.order.push_back(key);
                            self.trim_cache();
                            if key.0 == self.requested {
                                self.new_output = true;
                                self.status = format!(
                                    "Slide {} ready · {} cached frames",
                                    self.requested + 1,
                                    self.frames.len()
                                );
                            }
                        }
                    }
                    Err(e) => {
                        self.status = e;
                        self.failed.insert(((c.tag >> 32) as usize, c.tag as u32));
                    }
                }
            }
        }
        if !self.fg_busy
            && let Some(path) = self.pending_open.take()
        {
            if self.fg.submit(
                self.generation,
                0,
                Request::Open {
                    epoch: self.generation + 1,
                    path: path.clone(),
                    cache: self.cache_path.clone(),
                },
            ) {
                self.fg_busy = true;
            } else {
                self.pending_open = Some(path);
            }
        }
        if self.deck.is_some() && self.pending_open.is_none() {
            if !self.fg_busy {
                let edge = if self.frames.contains_key(&(self.requested, 1280)) {
                    2560
                } else {
                    1280
                };
                if !self.frames.contains_key(&(self.requested, edge))
                    && !self.failed.contains(&(self.requested, edge))
                {
                    self.schedule(self.requested, edge, true);
                }
            }
            if !self.bg_busy
                && self.notes_pending
                && self.full_frame(self.requested).is_some()
                && self.bg.submit(
                    self.generation,
                    u64::MAX,
                    Request::Notes {
                        path: self.deck.as_ref().unwrap().path.clone(),
                        cache: self.cache_path.clone(),
                    },
                )
            {
                self.notes_pending = false;
                self.bg_busy = true;
            }
            if !self.bg_busy {
                let mut jobs = vec![];
                if self.requested + 1 < self.count() {
                    jobs.push((self.requested + 1, 1280));
                }
                if self.requested > 0 {
                    jobs.push((self.requested - 1, 1280));
                }
                for &page in &self.visible_thumbs {
                    if page < self.count() {
                        jobs.push((page, 320));
                    }
                }
                if let Some((page, edge)) = jobs
                    .into_iter()
                    .find(|key| !self.frames.contains_key(key) && !self.failed.contains(key))
                {
                    self.schedule(page, edge, false);
                }
            }
        }
    }
    fn schedule(&mut self, page: usize, edge: u32, foreground: bool) {
        let request = Request::Render {
            epoch: self.generation,
            path: self.deck.as_ref().unwrap().path.clone(),
            cache: self.cache_path.clone(),
            page,
            edge,
        };
        let bridge = if foreground { &self.fg } else { &self.bg };
        if bridge.submit(
            self.generation,
            ((page as u64) << 32) | u64::from(edge),
            request,
        ) {
            if foreground {
                self.fg_busy = true;
            } else {
                self.bg_busy = true;
            }
        }
    }
    fn trim_cache(&mut self) {
        let mut bytes: usize = self.frames.values().map(|f| f.rgba.len()).sum();
        let count = self.order.len();
        for _ in 0..count {
            if bytes <= 128 * 1024 * 1024 {
                break;
            }
            if let Some(key) = self.order.pop_front() {
                if key.0.abs_diff(self.requested) <= 1 {
                    self.order.push_back(key);
                } else if let Some(frame) = self.frames.remove(&key) {
                    bytes -= frame.rgba.len();
                }
            }
        }
    }
}
