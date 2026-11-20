//! Shared native presenter state, scheduling, layout and UI actions.
use crate::{
    document::{DeckInfo, Frame, FrameInfo},
    settings::{self, Layout, Routes, Settings},
    worker::{Bridge, Request},
};
use std::{
    collections::{HashMap, HashSet},
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
    SwapRoutes,
    KeepRoutes,
    RevertRoutes,
    OpenBrowser,
    CloseBrowser,
    Refresh,
    Exit,
    ToggleFullscreen,
    ToggleTimer,
    RestartTimer,
}
pub struct RouteTrial {
    pub id: u64,
    pub deadline: Instant,
    previous: Routes,
    presenter_fullscreen: bool,
    presentation_fullscreen: bool,
}
pub struct Studio {
    pub settings: Settings,
    pub layout: Layout,
    pub monitors: Vec<Monitor>,
    pub routes: Routes,
    pub draft_routes: Routes,
    pub draft_presenter_fullscreen: bool,
    pub draft_presentation_fullscreen: bool,
    pub route_trial: Option<RouteTrial>,
    pub presentation_started: bool,
    next_route_trial: u64,
    pub deck: Option<DeckInfo>,
    pub requested: usize,
    pub displayed: Option<usize>,
    pub blanked: bool,
    pub status: String,
    pub awake_status: String,
    pub generation: u64,
    pub frames: HashMap<(usize, u32), Arc<Frame>>,
    pub prepared: HashSet<usize>,
    pub target_edge: u32,
    pub preparation_started: Instant,
    pub output_buffered: usize,
    pub ram_blocked: bool,
    pub cached_slides: usize,
    pub evicted_frames: usize,
    pub loading: bool,
    pub notes_loading: bool,
    pub visible_thumbs: HashSet<usize>,
    pub layout_open: bool,
    pub displays_open: bool,
    pub menu_open: bool,
    pub browser_open: bool,
    pub browser_selected: usize,
    pub browser_preview: Option<(u64, usize)>,
    pub change_on_click: bool,
    pub jump: String,
    pub timer: crate::timer::Timer,
    pub actions: Vec<Action>,
    pub settings_path: PathBuf,
    pub cache_path: PathBuf,
    pub fg: Bridge,
    pub bg: Bridge,
    pub io: Bridge,
    pub dialog: Bridge,
    pub dialog_busy: bool,
    pub dialog_owner: Option<isize>,
    fg_busy: bool,
    bg_busy: bool,
    fg_job: Option<(usize, u32)>,
    bg_job: Option<(usize, u32)>,
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
    worker_program: PathBuf,
}
impl Studio {
    pub fn new(smoke: bool, open_path: Option<PathBuf>) -> Self {
        Self::with_program(
            smoke,
            open_path,
            std::env::current_exe().unwrap_or_default(),
        )
    }
    pub fn with_program(smoke: bool, open_path: Option<PathBuf>, worker_program: PathBuf) -> Self {
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
        let io = Bridge::with_program(Duration::from_secs(10), worker_program.clone());
        io.submit(0, 0, Request::LoadSettings(settings_path.clone()));
        Self {
            settings: Settings::default(),
            layout: Layout::default(),
            monitors: vec![],
            routes: Routes::default(),
            draft_routes: Routes::default(),
            draft_presenter_fullscreen: true,
            draft_presentation_fullscreen: true,
            route_trial: None,
            presentation_started: false,
            next_route_trial: 0,
            deck: None,
            requested: 0,
            displayed: None,
            blanked: false,
            status: "Ready — open a PDF or picture folder, or drop it here.".into(),
            awake_status: "Sleep protection starting…".into(),
            generation: 0,
            frames: HashMap::new(),
            prepared: HashSet::new(),
            target_edge: 1920,
            preparation_started: Instant::now(),
            output_buffered: 0,
            ram_blocked: false,
            cached_slides: 0,
            evicted_frames: 0,
            loading: false,
            notes_loading: false,
            visible_thumbs: HashSet::new(),
            layout_open: false,
            displays_open: false,
            menu_open: false,
            browser_open: false,
            browser_selected: 0,
            browser_preview: None,
            change_on_click: false,
            jump: String::new(),
            timer: crate::timer::Timer::default(),
            actions: vec![],
            settings_path,
            cache_path,
            fg: Bridge::with_program(Duration::from_secs(30), worker_program.clone()),
            bg: Bridge::with_program(Duration::from_secs(30), worker_program.clone()),
            io,
            dialog: Bridge::with_program(Duration::from_secs(300), worker_program.clone()),
            dialog_busy: false,
            dialog_owner: None,
            fg_busy: false,
            bg_busy: false,
            fg_job: None,
            bg_job: None,
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
            worker_program,
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
        [self.target_edge, 512, 1280, 320]
            .iter()
            .find_map(|&edge| self.frames.get(&(page, edge)).cloned())
    }
    pub fn full_frame(&self, page: usize) -> Option<Arc<Frame>> {
        [self.target_edge, 512, 1280]
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
            self.execute(Action::RevertRoutes);
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
            self.blanked = recovering || !self.settings.auto_show;
            self.new_output = true;
            self.status = if recovering {
                "Screen setup changed. Output blanked; review destinations before showing."
            } else {
                "Screen setup ready."
            }
            .into();
        }
    }
    pub fn timer_limit(&self) -> Option<Duration> {
        (self.layout.timer_mode == settings::TimerMode::Countdown)
            .then(|| Duration::from_secs(self.layout.countdown_seconds as u64))
    }
    pub fn slide_played(&mut self) {
        self.timer
            .first_slide(Instant::now(), self.layout.timer_autostart);
    }
    pub fn execute(&mut self, action: Action) {
        match action {
            Action::Open(path) => {
                if self.fg_busy {
                    self.fg =
                        Bridge::with_program(Duration::from_secs(30), self.worker_program.clone());
                    self.fg_busy = false;
                    self.fg_job = None;
                }
                self.loading = true;
                self.pending_open = Some(path);
                self.status = "Opening document in an isolated worker…".into();
            }
            Action::Pick(folder) => {
                if !self.dialog_busy {
                    self.dialog_busy = self.dialog.submit(
                        0,
                        0,
                        Request::Pick {
                            folder,
                            owner: self.dialog_owner,
                        },
                    );
                    self.status = if self.dialog_busy {
                        "File chooser open — select a presentation or cancel."
                    } else {
                        "Could not open the file chooser. Try again."
                    }
                    .into();
                }
            }
            Action::Go(page) => {
                if self.route_trial.is_some() {
                    return;
                }
                if page < self.count() {
                    if page != self.requested && self.fg_busy && !self.loading {
                        self.fg = Bridge::with_program(
                            Duration::from_secs(30),
                            self.worker_program.clone(),
                        );
                        self.fg_busy = false;
                        self.fg_job = None;
                    }
                    self.presentation_started = true;
                    if !self.blanked && self.routes.presentation.is_empty() {
                        self.slide_played();
                    }
                    self.requested = page;
                    self.jump.clear();
                    self.new_output = true;
                }
            }
            Action::Blank => {
                if self.route_trial.is_some() {
                    return;
                }
                self.blanked = !self.blanked;
                if !self.blanked {
                    self.presentation_started = true;
                    if self.count() > 0 && self.routes.presentation.is_empty() {
                        self.slide_played();
                    }
                }
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
            Action::SwapRoutes => {
                if self.route_trial.is_none()
                    && !self.presentation_started
                    && !self.routes.presentation.is_empty()
                {
                    self.begin_display_edit();
                    std::mem::swap(
                        &mut self.draft_routes.presenter,
                        &mut self.draft_routes.presentation,
                    );
                    self.execute(Action::ApplyRoutes);
                }
            }
            Action::ApplyRoutes => {
                if !self.initialized {
                    return;
                }
                if self.route_trial.is_some() {
                    return;
                }
                if self.draft_routes.presenter.is_empty() {
                    self.status = "Choose at least one presenter screen.".into();
                } else if let Err(e) = self.draft_routes.validate() {
                    self.status = e;
                } else if self
                    .draft_routes
                    .presenter
                    .iter()
                    .chain(&self.draft_routes.presentation)
                    .any(|id| !self.monitors.iter().any(|m| &m.id == id))
                {
                    self.status = "A selected display is no longer connected.".into();
                } else {
                    self.next_route_trial += 1;
                    self.route_trial = Some(RouteTrial {
                        id: self.next_route_trial,
                        deadline: Instant::now() + Duration::from_secs(15),
                        previous: self.routes.clone(),
                        presenter_fullscreen: self.draft_presenter_fullscreen,
                        presentation_fullscreen: self.draft_presentation_fullscreen,
                    });
                    self.routes = self.draft_routes.clone();
                    self.dirty_routes = true;
                    self.displays_open = false;
                    self.blanked = true;
                    self.new_output = true;
                }
            }
            Action::KeepRoutes => {
                if let Some(trial) = self.route_trial.take() {
                    if Instant::now() >= trial.deadline {
                        self.route_trial = Some(trial);
                        self.execute(Action::RevertRoutes);
                        return;
                    }
                    self.settings.routes = self.routes.clone();
                    self.settings.presenter_fullscreen = trial.presenter_fullscreen;
                    self.settings.presentation_fullscreen = trial.presentation_fullscreen;
                    self.pending_save = true;
                    self.new_output = true;
                    self.status =
                        "Display setup confirmed. Output remains blank until shown.".into();
                }
            }
            Action::RevertRoutes => {
                if let Some(trial) = self.route_trial.take() {
                    self.routes = trial.previous;
                    self.begin_display_edit();
                    self.displays_open = false;
                    self.dirty_routes = true;
                    self.blanked = true;
                    self.new_output = true;
                    self.status =
                        "Display change reverted. Output remains blank until shown.".into();
                }
            }
            Action::OpenBrowser => {
                self.browser_open = true;
                self.browser_selected = self.requested;
                self.change_on_click = false;
            }
            Action::CloseBrowser => {
                self.browser_open = false;
                self.browser_preview = None;
                self.pending_save = true;
            }
            Action::Refresh => self.dirty_routes = true,
            Action::Exit => self.close_requested = true,
            Action::ToggleTimer => self.timer.toggle(Instant::now(), self.timer_limit()),
            Action::RestartTimer => self.timer.restart(Instant::now()),
            Action::ToggleFullscreen => {
                if self.route_trial.is_some() {
                    return;
                }
                let fullscreen = !(self.presenter_fullscreen() && self.presentation_fullscreen());
                self.draft_routes = self.routes.clone();
                self.draft_presenter_fullscreen = fullscreen;
                self.draft_presentation_fullscreen = fullscreen;
                self.execute(Action::ApplyRoutes);
            }
        }
    }
    pub fn display_control(&mut self, action: crate::output::DisplayAction, trial: Option<u64>) {
        use crate::output::DisplayAction;
        match action {
            DisplayAction::Swap
                if trial.is_none() && self.route_trial.is_none() && !self.presentation_started =>
            {
                self.execute(Action::SwapRoutes)
            }
            DisplayAction::Keep | DisplayAction::Revert
                if trial.is_some() && trial == self.route_trial.as_ref().map(|t| t.id) =>
            {
                self.execute(if matches!(action, DisplayAction::Keep) {
                    Action::KeepRoutes
                } else {
                    Action::RevertRoutes
                });
            }
            _ => {}
        }
    }
    pub fn begin_display_edit(&mut self) {
        self.draft_routes = self.routes.clone();
        self.draft_presenter_fullscreen = self.settings.presenter_fullscreen;
        self.draft_presentation_fullscreen = self.settings.presentation_fullscreen;
        self.displays_open = true;
    }
    pub fn presenter_fullscreen(&self) -> bool {
        self.route_trial
            .as_ref()
            .map_or(self.settings.presenter_fullscreen, |t| {
                t.presenter_fullscreen
            })
    }
    pub fn presentation_fullscreen(&self) -> bool {
        self.route_trial
            .as_ref()
            .map_or(self.settings.presentation_fullscreen, |t| {
                t.presentation_fullscreen
            })
    }
    pub fn poll(&mut self) {
        if self
            .route_trial
            .as_ref()
            .is_some_and(|t| Instant::now() >= t.deadline)
        {
            self.execute(Action::RevertRoutes);
        }
        while let Ok(c) = self.io.rx.try_recv() {
            match c.result {
                Ok(packet) => {
                    if let Some(error) = packet.reply.error {
                        self.status = format!("Settings: {error}");
                    } else if let Some(settings) = packet.reply.settings {
                        if !self.smoke {
                            self.blanked = !settings.auto_show;
                            self.settings = settings;
                            self.layout = self.settings.for_project(&self.project());
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
        if let Some(path) = self.open_path.take() {
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
            self.dialog_busy = false;
            self.status = "File chooser closed.".into();
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
                    self.fg_job = None;
                } else {
                    self.bg_busy = false;
                    self.bg_job = None;
                }
                if c.generation != self.generation {
                    continue;
                }
                match c.result {
                    Ok(packet) => {
                        if let Some(error) = packet.reply.error {
                            self.status = error;
                            if c.tag == u64::MAX {
                                self.notes_loading = false;
                            } else if self.loading && foreground {
                                self.loading = false;
                            } else {
                                self.failed.insert(((c.tag >> 32) as usize, c.tag as u32));
                            }
                            continue;
                        }
                        if let Some(info) = packet.reply.info {
                            if c.tag == u64::MAX {
                                self.notes_loading = false;
                                if let Some(deck) = &mut self.deck {
                                    deck.notes = info.notes;
                                    deck.warnings = info.warnings;
                                    if let Some(warning) = deck.warnings.first() {
                                        self.status = warning.clone();
                                    }
                                }
                                continue;
                            }
                            self.loading = false;
                            self.bg = Bridge::with_program(
                                Duration::from_secs(30),
                                self.worker_program.clone(),
                            );
                            self.bg_busy = false;
                            self.bg_job = None;
                            self.notes_loading = false;
                            self.notes_pending = true;
                            self.cached_slides = 0;
                            self.evicted_frames = 0;
                            self.generation += 1;
                            self.frames.clear();
                            self.prepared.clear();
                            self.preparation_started = Instant::now();
                            self.output_buffered = 0;
                            self.ram_blocked = false;
                            self.failed.clear();
                            self.requested = 0;
                            self.presentation_started = false;
                            self.timer = crate::timer::Timer::default();
                            self.displayed = None;
                            self.layout = self.settings.for_project(&info.key);
                            self.status = info.warnings.first().cloned().unwrap_or_else(|| {
                                format!(
                                    "Opened {} slides. Preparing first slide…",
                                    info.names.len()
                                )
                            });
                            self.deck = Some(info);
                            self.blanked = !self.settings.auto_show;
                            self.new_output = true;
                        }
                        if let Some(info) = packet.reply.frame {
                            let key = (info.page, packet.reply.edge.unwrap_or(c.tag as u32));
                            if key.1 != self.target_edge && key.1 != 512 {
                                continue;
                            }
                            if key.1 == self.target_edge
                                && packet.reply.cached
                                && !self.prepared.contains(&key.0)
                            {
                                self.cached_slides += 1;
                            }
                            let frame = Arc::new(Frame {
                                info,
                                rgba: packet.data,
                            });
                            if key.1 == self.target_edge {
                                self.frames.retain(|(page, edge), _| {
                                    *page != key.0 || *edge == self.target_edge
                                });
                            }
                            self.frames.insert(key, frame);
                            if key.1 == self.target_edge {
                                self.prepared.insert(key.0);
                            }
                            self.new_output = true;
                            if key.0 == self.requested {
                                self.new_output = true;
                                self.status = format!("Slide {} ready", self.requested + 1);
                            }
                        }
                    }
                    Err(e) => {
                        self.status = e;
                        if c.tag == u64::MAX {
                            self.notes_loading = false;
                        } else if self.loading && foreground {
                            self.loading = false;
                        } else {
                            self.failed.insert(((c.tag >> 32) as usize, c.tag as u32));
                        }
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
                let edge = self.target_edge;
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
                self.notes_loading = true;
                self.bg_busy = true;
            }
            // Use spare foreground capacity to prepare the tail while background fills forward.
            // Interactive navigation still cancels/supersedes this work immediately.
            if !self.fg_busy
                && self.prepared.len() < self.count()
                && let Some(page) = (0..self.count()).rev().find(|p| {
                    let key = (*p, self.target_edge);
                    !self.prepared.contains(p)
                        && !self.failed.contains(&key)
                        && Some(key) != self.bg_job
                })
            {
                self.schedule(page, self.target_edge, true);
            }
            if !self.bg_busy {
                let mut jobs = vec![];
                for page in [self.requested + 1, self.requested.saturating_sub(1)] {
                    if page < self.count() {
                        jobs.push((page, self.target_edge));
                    }
                }
                // Prepare the entire show, not just the currently visible neighbourhood.
                if self.prepared.len() < self.count() {
                    jobs.retain(|(p, _)| !self.prepared.contains(p));
                    jobs.extend(
                        (0..self.count())
                            .filter(|p| !self.prepared.contains(p))
                            .map(|p| (p, self.target_edge)),
                    );
                } else {
                    if self.browser_open {
                        jobs.extend(self.visible_thumbs.iter().map(|p| (*p, self.target_edge)));
                    }
                    // Refill available RAM after navigation or a budget increase.
                    jobs.extend((0..self.count()).map(|p| (p, self.target_edge)));
                }
                if let Some((page, edge)) = jobs.into_iter().find(|key| {
                    !self.frames.contains_key(key)
                        && Some(*key) != self.fg_job
                        && !self.failed.contains(key)
                        && key.0 != self.requested
                }) {
                    self.schedule(page, edge, false);
                }
            }
        }

        let edge = self
            .monitors
            .iter()
            .filter(|m| self.routes.presentation.contains(&m.id))
            .chain(
                self.monitors
                    .iter()
                    .filter(|m| self.routes.presenter.contains(&m.id)),
            )
            .map(|m| m.size.0.max(m.size.1))
            .max()
            .unwrap_or(1920)
            .clamp(512, 4096);
        if edge != self.target_edge {
            self.target_edge = edge;
            self.prepared = self
                .frames
                .keys()
                .filter(|(_, e)| *e == edge)
                .map(|(p, _)| *p)
                .collect();
            self.frames.retain(|(_, e), _| *e == edge || *e == 512);
            self.preparation_started = Instant::now();
            self.cached_slides = 0;
            self.evicted_frames = 0;
            self.output_buffered = 0;
            self.ram_blocked = false;
            self.new_output = true;
        }
    }
    fn schedule(&mut self, page: usize, edge: u32, foreground: bool) {
        // Reserve both worker slots before admission, including progressive previews.
        let reserve = 2 * edge as usize * edge as usize * 4;
        let limit = self.settings.buffer_mib as usize * 1024 * 1024;
        while self.buffer_bytes().saturating_add(reserve) > limit {
            let victim = self
                .frames
                .keys()
                .copied()
                .filter(|(p, _)| {
                    *p != self.requested && Some(*p) != self.displayed && *p != page
                    // Once processed, speculative prefetch must improve locality rather
                    // than continually swap two equally distant cached slides.
                    && (!self.prepared.contains(&page)
                        || page == self.requested
                        || p.abs_diff(self.requested) > page.abs_diff(self.requested))
                })
                .max_by_key(|(p, e)| (*e != self.target_edge, p.abs_diff(self.requested), *p));
            let Some(victim) = victim else {
                self.ram_blocked = true;
                return;
            };
            self.frames.remove(&victim);
            self.evicted_frames += 1;
            self.new_output = true;
        }
        self.ram_blocked = false;
        let request = Request::Render {
            epoch: self.generation,
            path: self.deck.as_ref().unwrap().path.clone(),
            cache: self.cache_path.clone(),
            page,
            edge,
        };
        let request = if foreground && page == self.requested && self.frame(page).is_none() {
            Request::Preview {
                epoch: self.generation,
                path: self.deck.as_ref().unwrap().path.clone(),
                cache: self.cache_path.clone(),
                page,
                edge,
            }
        } else {
            request
        };
        let bridge = if foreground { &self.fg } else { &self.bg };
        if bridge.submit(
            self.generation,
            ((page as u64) << 32) | u64::from(edge),
            request,
        ) {
            if foreground {
                self.fg_busy = true;
                self.fg_job = Some((page, edge));
            } else {
                self.bg_busy = true;
                self.bg_job = Some((page, edge));
            }
        }
    }
    pub fn buffer_generation(&self) -> u64 {
        (self.generation << 32) | u64::from(self.target_edge)
    }
    pub fn save_preferences(&mut self) {
        self.pending_save = true;
    }
    pub fn remember_buffer_budget(&mut self) {
        let reserved: usize = self
            .fg_job
            .iter()
            .chain(self.bg_job.iter())
            .map(|(_, edge)| *edge as usize * *edge as usize * 4)
            .sum();
        let limit = (self.settings.buffer_mib as usize * 1024 * 1024).saturating_sub(reserved);
        while self.buffer_bytes() > limit {
            let victim = self
                .frames
                .keys()
                .copied()
                .filter(|(p, _)| *p != self.requested && Some(*p) != self.displayed)
                .max_by_key(|(p, _)| p.abs_diff(self.requested));
            let Some(victim) = victim else {
                break;
            };
            self.frames.remove(&victim);
            self.evicted_frames += 1;
        }
        self.pending_save = true;
        self.ram_blocked = false;
        self.new_output = true;
    }
    pub fn buffer_bytes(&self) -> usize {
        self.frames.values().map(|f| f.rgba.len()).sum()
    }
    pub fn resident_slides(&self) -> usize {
        self.frames
            .keys()
            .filter(|(_, edge)| *edge == self.target_edge)
            .count()
    }
    fn rolling_status(&self, french: bool) -> String {
        let processed = self.prepared.len();
        let total = self.count();
        let eta = if processed > 0 && processed < total {
            format!(
                " · ETA ~{} s",
                (self.preparation_started.elapsed().as_secs_f64() / processed as f64
                    * (total - processed) as f64)
                    .ceil() as u64
            )
        } else {
            String::new()
        };
        if french {
            format!(
                "Cache tournant — {processed}/{total} préparées · {}/{} en RAM ({} Mio) · {} libérées · rechargement du cache à la demande{eta}",
                self.resident_slides(),
                total,
                self.buffer_bytes() / (1024 * 1024),
                self.evicted_frames
            )
        } else {
            format!(
                "Rolling buffer — {processed}/{total} prepared · {}/{} in RAM ({} MiB) · {} evicted · cached slides reload on demand{eta}",
                self.resident_slides(),
                total,
                self.buffer_bytes() / (1024 * 1024),
                self.evicted_frames
            )
        }
    }
    pub fn fully_buffered(&self) -> bool {
        !self.loading
            && self.count() > 0
            && self.resident_slides() == self.count()
            && (self.routes.presentation.is_empty() || self.output_buffered == self.count())
    }
    fn preparation_status_fr(&self) -> String {
        if self.loading {
            return "Ouverture et vérification du cache… Estimation en cours".into();
        }
        if self.deck.is_none() {
            return self.status.clone();
        }
        let n = self.prepared.len();
        let total = self.count();
        let ram = self.buffer_bytes() / (1024 * 1024);
        if self.notes_loading {
            return format!("Lecture des notes · {n}/{total} en mémoire · diffusion disponible");
        }
        if self.fully_buffered() {
            return format!(
                "PRÊT — {total} diapositives en RAM · {ram} Mio · {} px",
                self.target_edge
            );
        }
        if self.evicted_frames > 0 {
            return self.rolling_status(self.settings.language.french());
        }
        if self.ram_blocked {
            return format!(
                "Budget RAM atteint — {n}/{total} en mémoire. Mémoire insuffisante pour les images actives et les rendus en cours."
            );
        }
        if !self.failed.is_empty() {
            return format!(
                "Préparation incomplète — {n}/{total} en mémoire. {}",
                self.status
            );
        }
        let count = if n == total { self.output_buffered } else { n };
        let eta = if count > 0 && count < total {
            format!(
                "~{} s restantes",
                (self.preparation_started.elapsed().as_secs_f64() / count as f64
                    * (total - count) as f64)
                    .ceil() as u64
            )
        } else {
            "Estimation en cours…".into()
        };
        if n == total {
            format!(
                "Chargement de la sortie — {count}/{total} · {ram} Mio dans le présentateur · {eta}"
            )
        } else {
            format!(
                "Préparation {} px — {n}/{total} · {ram} Mio · {eta} · {} en cache",
                self.target_edge, self.cached_slides
            )
        }
    }
    pub fn preparation_status(&self) -> String {
        if self.settings.language.french() {
            return self.preparation_status_fr();
        }
        if self.loading {
            return "Opening source and verifying cached slide identity… ETA estimating".into();
        }
        if self.deck.is_none() {
            return self.status.clone();
        }
        let n = self.prepared.len();
        let total = self.count();
        let ram = self.buffer_bytes() / (1024 * 1024);
        if self.notes_loading {
            return format!(
                "Reading presenter notes · {n}/{total} slides buffered · playback remains available"
            );
        }
        if self.fully_buffered() {
            return format!(
                "READY — all {total} slides buffered in RAM · {ram} MiB · {} px",
                self.target_edge
            );
        }
        if self.evicted_frames > 0 {
            return self.rolling_status(self.settings.language.french());
        }
        if self.ram_blocked {
            return format!(
                "RAM budget reached — {n}/{total} buffered. Cannot fit active frames and render reservations; increase the budget."
            );
        }
        if !self.failed.is_empty() {
            return format!(
                "Preparation incomplete — {n}/{total} buffered; a slide failed. {}",
                self.status
            );
        }
        let eta = if n >= 2 && n < total {
            format!(
                "~{} s remaining",
                (self.preparation_started.elapsed().as_secs_f64() / n as f64 * (total - n) as f64)
                    .ceil() as u64
            )
        } else {
            "estimating…".into()
        };
        if n == total {
            format!(
                "Buffering presentation output — {}/{total} slides · {ram} MiB in presenter RAM · {}",
                self.output_buffered,
                if self.output_buffered > 0 {
                    format!(
                        "~{} s remaining",
                        (self.preparation_started.elapsed().as_secs_f64()
                            / self.output_buffered as f64
                            * total.saturating_sub(self.output_buffered) as f64)
                            .ceil() as u64
                    )
                } else {
                    "ETA estimating…".into()
                }
            )
        } else {
            format!(
                "Preparing {} px — {n}/{total} slides · {ram} MiB · ETA {eta} · {} cached",
                self.target_edge, self.cached_slides
            )
        }
    }
}
