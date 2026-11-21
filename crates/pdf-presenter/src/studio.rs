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
    PickNotes,
    PickBlankPicture,
    SetBlankPicture(PathBuf),
    ClearBlankPicture,
    #[cfg(target_os = "windows")]
    RegisterPdfAssociation,
    ImportNotes(PathBuf),
    ApplyNotesImport,
    Go(usize),
    Navigate {
        delta: i32,
        repeated: bool,
    },
    FinishNavigation,
    Blank,
    SaveDisplayLayout {
        display: String,
        project: bool,
        layout: Layout,
    },
    UseDisplayDefault(String),
    SaveProject,
    SaveGlobal,
    UseGlobal,
    ApplyRoutes,
    SwapRoutes,
    StartupSingleScreen(bool),
    KeepRoutes,
    RevertRoutes,
    OpenBrowser,
    CloseBrowser,
    Refresh,
    Exit,
    ConfirmExit,
    CancelExit,
    ToggleFullscreen,
    ToggleTimer,
    ResetTimer,
    EditNotes,
    ReloadNotes,
    SaveNotes,
}
pub struct NotesImport {
    pub preview: Option<crate::notes::ImportPreview>,
    pub busy: bool,
    pub error: Option<String>,
}
pub struct NotesEditor {
    pub page: usize,
    pub text: String,
    pub saved_text: String,
    pub snapshot: Option<crate::notes::Snapshot>,
    pub busy: bool,
    pub error: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlideLoadState {
    Ready,
    Preparing,
    Cached,
    Queued,
    Failed,
}
pub struct RouteTrial {
    pub id: u64,
    pub deadline: Instant,
    previous: Routes,
    presenter_fullscreen: bool,
    presentation_fullscreen: bool,
    screen_modes: std::collections::BTreeMap<String, crate::settings::ScreenMode>,
    single_screens: Vec<String>,
}
#[derive(Default)]
pub struct PresenterView {
    pub layout: Layout,
    pub layout_open: bool,
    pub layout_target: Option<String>,
    pub preferences_open: bool,
    pub menu_open: bool,
}
pub struct Studio {
    pub settings: Settings,
    pub layout: Layout,
    pub presenter_views: HashMap<String, PresenterView>,
    pub presenter_views_dirty: bool,
    pub monitors: Vec<Monitor>,
    pub routes: Routes,
    pub draft_routes: Routes,
    pub draft_single_screens: Vec<String>,
    startup_displays_observed: bool,
    pub startup_single_screen: Option<String>,
    pub draft_screen_modes: std::collections::BTreeMap<String, crate::settings::ScreenMode>,
    pub draft_presenter_fullscreen: bool,
    pub draft_presentation_fullscreen: bool,
    pub route_trial: Option<RouteTrial>,
    pub presentation_started: bool,
    next_route_trial: u64,
    pub deck: Option<DeckInfo>,
    pub requested: usize,
    pub displayed: Option<usize>,
    pub blanked: bool,
    pub blank_picture: Option<Arc<Frame>>,
    pub blank_busy: bool,
    blank_loader: Bridge,
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
    pub notes_import: Option<NotesImport>,
    pub notes_editor: Option<NotesEditor>,
    notes_io: Bridge,
    notes_edited: bool,
    pub visible_thumbs: HashSet<usize>,
    pub layout_open: bool,
    pub displays_open: bool,
    pub preferences_open: bool,
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
    cache_io: Bridge,
    pub cache_busy: bool,
    pub cache_status: String,
    startup_cache_pending: bool,
    navigation_settle: Option<Instant>,
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
    pub zoom: crate::zoom::Sync,
    pub output_viewport: Option<crate::output::OutputViewport>,
    pub new_zoom: bool,
    pub zoom_warning: bool,
    pub zoom_warning_dismissed: bool,
    pub toolbox: crate::tools::Toolbox,
    pub new_overlay: bool,
    pub new_output: bool,
    pub close_requested: bool,
    pub quit_confirmation: bool,
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
            presenter_views: HashMap::new(),
            presenter_views_dirty: false,
            monitors: vec![],
            routes: Routes::default(),
            draft_routes: Routes::default(),
            draft_single_screens: Vec::new(),
            startup_displays_observed: false,
            startup_single_screen: None,
            draft_screen_modes: Default::default(),
            draft_presenter_fullscreen: true,
            draft_presentation_fullscreen: true,
            route_trial: None,
            presentation_started: false,
            next_route_trial: 0,
            deck: None,
            requested: 0,
            displayed: None,
            blanked: false,
            blank_picture: None,
            blank_busy: false,
            blank_loader: Bridge::with_program(Duration::from_secs(15), worker_program.clone()),
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
            notes_import: None,
            notes_editor: None,
            notes_io: Bridge::with_program(Duration::from_secs(30), worker_program.clone()),
            notes_edited: false,
            visible_thumbs: HashSet::new(),
            layout_open: false,
            displays_open: false,
            preferences_open: false,
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
            cache_io: Bridge::with_program(Duration::from_secs(30), worker_program.clone()),
            cache_busy: false,
            cache_status: String::new(),
            startup_cache_pending: true,
            navigation_settle: None,
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
            zoom: Default::default(),
            output_viewport: None,
            new_zoom: false,
            zoom_warning: false,
            zoom_warning_dismissed: false,
            toolbox: Default::default(),
            new_overlay: false,
            new_output: true,
            close_requested: false,
            quit_confirmation: false,
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
    pub fn slide_load_state(&self, page: usize) -> SlideLoadState {
        if self.frame(page).is_some() {
            SlideLoadState::Ready
        } else if self.fg_job.is_some_and(|(job, _)| job == page)
            || self.bg_job.is_some_and(|(job, _)| job == page)
        {
            SlideLoadState::Preparing
        } else if self.failed.contains(&(page, self.target_edge)) {
            SlideLoadState::Failed
        } else if self.prepared.contains(&page) {
            SlideLoadState::Cached
        } else {
            SlideLoadState::Queued
        }
    }
    pub fn full_frame(&self, page: usize) -> Option<Arc<Frame>> {
        [self.target_edge, 512, 1280]
            .iter()
            .find_map(|&edge| self.frames.get(&(page, edge)).cloned())
    }
    pub fn frame_info(&self) -> Option<FrameInfo> {
        self.full_frame(self.requested).map(|f| f.info.clone())
    }
    /// Called on the first native discovery, including zero displays. Never rearmed by hotplug.
    pub fn observe_startup_displays(&mut self, count: usize, only_id: Option<&str>) {
        if !self.startup_displays_observed {
            self.startup_displays_observed = true;
            self.startup_single_screen = only_id.filter(|_| count == 1).map(str::to_owned);
        }
    }
    pub fn is_single_screen(&self, id: &str) -> bool {
        self.routes.presenter.iter().any(|display| display == id)
            && self
                .route_trial
                .as_ref()
                .map_or(&self.settings.single_screens, |trial| &trial.single_screens)
                .iter()
                .any(|display| display == id)
    }
    pub fn startup_single_offer(&self) -> bool {
        self.initialized
            && self.route_trial.is_none()
            && !self.quit_confirmation
            && self.startup_single_screen.as_ref().is_some_and(|id| {
                self.monitors.len() == 1
                    && self.monitors[0].id == *id
                    && self.routes.presenter.contains(id)
                    && !self.is_single_screen(id)
            })
    }
    pub fn swap_draft_groups(&mut self) {
        if self.draft_routes.presentation.is_empty()
            || (!self.draft_routes.presenter.is_empty()
                && self
                    .draft_routes
                    .presenter
                    .iter()
                    .all(|id| self.draft_single_screens.contains(id)))
        {
            return;
        }
        let singles: Vec<_> = self
            .draft_routes
            .presenter
            .iter()
            .filter(|id| self.draft_single_screens.contains(id))
            .cloned()
            .collect();
        self.draft_routes
            .presenter
            .retain(|id| !singles.contains(id));
        std::mem::swap(
            &mut self.draft_routes.presenter,
            &mut self.draft_routes.presentation,
        );
        self.draft_routes.presenter.extend(singles);
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
        if self
            .startup_single_screen
            .as_ref()
            .is_some_and(|id| monitors.len() != 1 || monitors[0].id != *id)
        {
            self.startup_single_screen = None;
        }
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
            if !recovering {
                self.blanked = !self.settings.auto_show;
            }
            self.new_output = true;
            self.status = if recovering {
                "Screen setup changed. Output visibility preserved; review destinations."
            } else {
                "Screen setup ready."
            }
            .into();
        }
    }
    fn ensure_display_profile(&mut self, display: &str) -> bool {
        if display.is_empty()
            || display.len() > 1024
            || (self.settings.display_layouts.len() >= 128
                && !self.settings.display_layouts.contains_key(display))
        {
            self.status = "Cannot save another display layout.".into();
            return false;
        }
        let layout = self.settings.layout.clone();
        self.settings
            .display_layouts
            .entry(display.into())
            .or_insert_with(|| settings::DisplayLayouts {
                layout,
                projects: Default::default(),
            });
        true
    }
    pub fn take_presenter_view(&mut self, display: &str) -> PresenterView {
        if let Some(view) = self.presenter_views.remove(display) {
            return view;
        }
        // Retain disconnected screens during ordinary hotplug, with bounded session state.
        if self.presenter_views.len() >= 128 {
            self.presenter_views
                .retain(|id, _| self.routes.presenter.contains(id));
        }
        PresenterView {
            layout: self.settings.for_display(display, &self.project()),
            ..Default::default()
        }
    }
    pub fn timer_limit(&self) -> Option<Duration> {
        (self.layout.timer_mode == settings::TimerMode::Countdown)
            .then(|| Duration::from_secs(self.layout.countdown_seconds as u64))
    }
    pub fn presentation_action(&mut self) {
        self.timer
            .presentation_action(Instant::now(), self.layout.timer_autostart);
    }
    pub fn execute(&mut self, action: Action) {
        match action {
            Action::Open(path) => {
                if self.notes_editor.is_some() || self.notes_import.is_some() || self.dialog_busy {
                    self.status =
                        "Close the notes editor/import or file chooser before opening another presentation.".into();
                    return;
                }
                if self.fg_busy {
                    self.fg =
                        Bridge::with_program(Duration::from_secs(30), self.worker_program.clone());
                    self.fg_busy = false;
                    self.fg_job = None;
                }
                self.navigation_settle = None;
                self.loading = true;
                self.pending_open = Some(path);
                self.status = "Opening document in an isolated worker…".into();
            }
            Action::PickBlankPicture => {
                if self.initialized && !self.dialog_busy && !self.blank_busy {
                    self.dialog_busy = self.dialog.submit(
                        0,
                        2,
                        Request::PickBlankPicture {
                            owner: self.dialog_owner,
                        },
                    );
                }
            }
            Action::SetBlankPicture(path) => {
                if self.initialized && !self.blank_busy {
                    self.load_blank_picture(Some(path));
                }
            }
            Action::ClearBlankPicture => {
                self.blank_loader =
                    Bridge::with_program(Duration::from_secs(15), self.worker_program.clone());
                self.blank_busy = false;
                self.blank_picture = None;
                self.settings.blank_picture = false;
                self.new_output = true;
                self.pending_save = true;
                self.status = "Blank slides use solid black.".into();
            }
            #[cfg(target_os = "windows")]
            Action::RegisterPdfAssociation => {
                self.status = match crate::windows_integration::register_pdf_association() {
                    Ok(()) => "Choose PDF Presenter for .pdf in Windows Default Apps. PDF files will then use its icon.".into(),
                    Err(error) => format!("Could not register PDF Presenter: {error}"),
                };
            }
            Action::PickNotes => {
                if !self.dialog_busy
                    && !self.loading
                    && self.deck.is_some()
                    && self.notes_editor.is_none()
                    && self.notes_import.is_none()
                {
                    self.dialog_busy = self.dialog.submit(
                        self.generation,
                        1,
                        Request::PickNotes {
                            owner: self.dialog_owner,
                        },
                    );
                    self.status = "Choose a TXT or Markdown notes file.".into();
                }
            }
            Action::ImportNotes(import) => {
                if self.loading
                    || self.notes_editor.is_some()
                    || self.notes_import.is_some()
                    || self.dialog_busy
                {
                    self.status = "Close the notes editor/import or file chooser and wait for the presentation to load.".into();
                    return;
                }
                let Some(deck) = &self.deck else {
                    self.status = "Open a presentation before importing notes.".into();
                    return;
                };
                if self.notes_io.submit(
                    self.generation,
                    2,
                    Request::PreviewNotesImport {
                        path: deck.path.clone(),
                        cache: self.cache_path.clone(),
                        source_sha: deck.sha256.clone(),
                        import,
                    },
                ) {
                    self.notes_import = Some(NotesImport {
                        preview: None,
                        busy: true,
                        error: None,
                    });
                    self.jump.clear();
                    self.status = "Reading notes for import…".into();
                }
            }
            Action::ApplyNotesImport => {
                if let (Some(deck), Some(import)) = (&self.deck, &mut self.notes_import)
                    && !import.busy
                    && import.error.is_none()
                    && let Some(preview) = &import.preview
                    && self.notes_io.submit(
                        self.generation,
                        3,
                        Request::ApplyNotesImport {
                            path: deck.path.clone(),
                            cache: self.cache_path.clone(),
                            source_sha: deck.sha256.clone(),
                            preview: preview.clone(),
                        },
                    )
                {
                    import.busy = true;
                    self.status = "Saving imported notes…".into();
                }
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
            Action::Navigate { delta, repeated } => {
                if self.notes_editor.is_some()
                    || self.notes_import.is_some()
                    || self.dialog_busy
                    || self.route_trial.is_some()
                {
                    return;
                }
                let page = self
                    .requested
                    .saturating_add_signed(delta as isize)
                    .min(self.count().saturating_sub(1));
                if page != self.requested {
                    if !repeated {
                        self.navigation_settle = None;
                    }
                    if repeated {
                        self.navigation_settle = Some(Instant::now() + Duration::from_millis(180));
                    }
                    self.execute(Action::Go(page));
                }
            }
            Action::FinishNavigation => {
                if self.navigation_settle.take().is_some()
                    && self.fg_job.is_some_and(|(page, _)| page != self.requested)
                    && !self.loading
                {
                    self.fg =
                        Bridge::with_program(Duration::from_secs(30), self.worker_program.clone());
                    self.fg_busy = false;
                    self.fg_job = None;
                }
            }
            Action::Go(page) => {
                if self.route_trial.is_some() {
                    return;
                }
                if page < self.count() {
                    if page != self.requested
                        && self.fg_busy
                        && !self.loading
                        && self.navigation_settle.is_none()
                    {
                        self.fg = Bridge::with_program(
                            Duration::from_secs(30),
                            self.worker_program.clone(),
                        );
                        self.fg_busy = false;
                        self.fg_job = None;
                    }
                    self.presentation_started = true;
                    if self.requested != page {
                        self.set_zoom(crate::zoom::View::default());
                        self.toolbox.clear();
                        self.new_overlay = true;
                        self.presentation_action();
                    }
                    self.requested = page;
                    self.jump.clear();
                    self.new_output = true;
                }
            }
            Action::Blank => {
                self.blanked = !self.blanked;
                self.presentation_action();
                if !self.blanked {
                    self.presentation_started = true;
                }
                self.new_output = true;
            }
            Action::SaveDisplayLayout {
                display,
                project,
                mut layout,
            } => {
                if !self.ensure_display_profile(&display) {
                    return;
                }
                let key = self.project();
                // Timer behavior remains shared; only its appearance belongs to a monitor.
                layout.timer_mode = self.layout.timer_mode;
                layout.countdown_seconds = self.layout.countdown_seconds;
                layout.timer_autostart = self.layout.timer_autostart;
                if project {
                    let total: usize = self
                        .settings
                        .display_layouts
                        .values()
                        .map(|p| p.projects.len())
                        .sum();
                    if total >= 4096
                        && !self.settings.display_layouts[&display]
                            .projects
                            .contains_key(&key)
                    {
                        self.status = "Too many saved display/project layouts.".into();
                        return;
                    }
                    let mut shared = self.settings.for_project(&key);
                    shared.timer_mode = layout.timer_mode;
                    shared.countdown_seconds = layout.countdown_seconds;
                    shared.timer_autostart = layout.timer_autostart;
                    self.settings.projects.insert(key.clone(), shared);
                    self.settings
                        .display_layouts
                        .get_mut(&display)
                        .unwrap()
                        .projects
                        .insert(key, layout);
                } else {
                    self.settings.layout.timer_mode = layout.timer_mode;
                    self.settings.layout.countdown_seconds = layout.countdown_seconds;
                    self.settings.layout.timer_autostart = layout.timer_autostart;
                    self.settings
                        .display_layouts
                        .get_mut(&display)
                        .unwrap()
                        .layout = layout;
                }
                self.pending_save = true;
            }
            Action::UseDisplayDefault(display) => {
                if !self.ensure_display_profile(&display) {
                    return;
                }
                let key = self.project();
                let profile = self.settings.display_layouts.get_mut(&display).unwrap();
                profile.projects.remove(&key);
                let layout = profile.layout.clone();
                if let Some(view) = self.presenter_views.get_mut(&display) {
                    view.layout = layout;
                }
                self.pending_save = true;
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
            Action::StartupSingleScreen(accept) => {
                if !self.startup_single_offer() {
                    return;
                }
                let id = self.startup_single_screen.take().unwrap();
                if accept {
                    self.begin_display_edit();
                    self.draft_single_screens.push(id);
                    self.execute(Action::ApplyRoutes);
                }
            }
            Action::SwapRoutes => {
                if self.route_trial.is_none()
                    && self
                        .routes
                        .presenter
                        .iter()
                        .any(|id| !self.is_single_screen(id))
                    && !self.routes.presentation.is_empty()
                {
                    self.begin_display_edit();
                    self.swap_draft_groups();
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
                    self.status = "Choose at least one presenter or single-screen display.".into();
                } else if let Err(e) =
                    crate::settings::validate_single_screens(&self.draft_single_screens)
                {
                    self.status = e;
                } else if let Err(e) =
                    crate::settings::validate_screen_modes(&self.draft_screen_modes)
                {
                    self.status = e;
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
                    self.startup_single_screen = None;
                    self.next_route_trial += 1;
                    self.route_trial = Some(RouteTrial {
                        id: self.next_route_trial,
                        deadline: Instant::now() + Duration::from_secs(15),
                        previous: self.routes.clone(),
                        presenter_fullscreen: self.draft_presenter_fullscreen,
                        presentation_fullscreen: self.draft_presentation_fullscreen,
                        screen_modes: self.draft_screen_modes.clone(),
                        single_screens: self.draft_single_screens.clone(),
                    });
                    self.routes = self.draft_routes.clone();
                    self.dirty_routes = true;
                    self.displays_open = false;
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
                    self.settings.single_screens = trial.single_screens;
                    self.settings.screen_modes = trial.screen_modes;
                    self.settings.routes = self.routes.clone();
                    self.settings.presenter_fullscreen = trial.presenter_fullscreen;
                    self.settings.presentation_fullscreen = trial.presentation_fullscreen;
                    self.pending_save = true;
                    self.new_output = true;
                    self.status = "Display setup confirmed. Output visibility preserved.".into();
                }
            }
            Action::RevertRoutes => {
                if let Some(trial) = self.route_trial.take() {
                    self.routes = trial.previous;
                    self.begin_display_edit();
                    self.displays_open = false;
                    self.dirty_routes = true;
                    self.new_output = true;
                    self.status = "Display change reverted. Output visibility preserved.".into();
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
            Action::Exit => {
                self.quit_confirmation = true;
            }
            Action::CancelExit => self.quit_confirmation = false,
            Action::ConfirmExit => {
                if self.quit_confirmation {
                    if let Some(reason) = self.quit_block_reason() {
                        self.status = reason.into();
                    } else {
                        self.quit_confirmation = false;
                        self.close_requested = true;
                    }
                }
            }
            command @ (Action::EditNotes | Action::ReloadNotes) => {
                if matches!(command, Action::EditNotes) && self.notes_editor.is_some() {
                    return;
                }
                if self.loading
                    || self.notes_import.is_some()
                    || self.dialog_busy
                    || self.notes_editor.as_ref().is_some_and(|e| e.busy)
                {
                    return;
                }
                if let Some(deck) = &self.deck {
                    let page = self
                        .notes_editor
                        .as_ref()
                        .map_or(self.requested, |e| e.page);
                    if page >= self.count() {
                        return;
                    }
                    if self.notes_io.submit(
                        self.generation,
                        0,
                        Request::PrepareNotes {
                            path: deck.path.clone(),
                            cache: self.cache_path.clone(),
                            source_sha: deck.sha256.clone(),
                        },
                    ) {
                        self.notes_editor = Some(NotesEditor {
                            page,
                            text: String::new(),
                            saved_text: String::new(),
                            snapshot: None,
                            busy: true,
                            error: None,
                        });
                        self.jump.clear();
                    }
                }
            }
            Action::SaveNotes => {
                if let (Some(deck), Some(editor)) = (&self.deck, &mut self.notes_editor)
                    && !editor.busy
                    && let Some(snapshot) = &editor.snapshot
                    && self.notes_io.submit(
                        self.generation,
                        1,
                        Request::SaveNotes {
                            path: deck.path.clone(),
                            cache: self.cache_path.clone(),
                            source_sha: deck.sha256.clone(),
                            revision: snapshot.revision.clone(),
                            page: editor.page,
                            text: editor.text.clone(),
                        },
                    )
                {
                    editor.busy = true;
                    editor.error = None;
                }
            }
            Action::ToggleTimer => self.timer.toggle(Instant::now(), self.timer_limit()),
            Action::ResetTimer => self.timer.reset(),
            Action::ToggleFullscreen => {
                if self.route_trial.is_some() {
                    return;
                }
                let fullscreen = !(self.presenter_fullscreen() && self.presentation_fullscreen());
                self.draft_routes = self.routes.clone();
                self.draft_single_screens = self.settings.single_screens.clone();
                self.draft_screen_modes = self.settings.screen_modes.clone();
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
    pub fn quit_block_reason(&self) -> Option<&'static str> {
        if self.notes_import.as_ref().is_some_and(|i| i.busy) {
            Some("Wait for notes import to finish before closing.")
        } else if self
            .notes_editor
            .as_ref()
            .is_some_and(|e| e.busy || e.text != e.saved_text)
        {
            Some("Save or discard notes before closing the presentation.")
        } else {
            None
        }
    }
    pub fn screen_fullscreen(&self, display: &str, global: bool) -> bool {
        self.route_trial
            .as_ref()
            .map_or(&self.settings.screen_modes, |trial| &trial.screen_modes)
            .get(display)
            .copied()
            .unwrap_or_default()
            .fullscreen(global)
    }
    pub fn begin_display_edit(&mut self) {
        self.draft_single_screens = self.settings.single_screens.clone();
        self.draft_screen_modes = self.settings.screen_modes.clone();
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
    pub fn clean_cache(&mut self, manual: bool) {
        if self.cache_busy || !self.initialized || self.loading {
            return;
        }
        let protected = self.deck.iter().map(|d| d.key.clone()).collect();
        if self.cache_io.submit(
            0,
            0,
            Request::CleanCache {
                path: self.cache_path.clone(),
                policy: self.settings.disk_cache.clone(),
                protected,
                manual,
            },
        ) {
            self.startup_cache_pending = false;
            self.cache_busy = true;
            self.cache_status = "Cleaning disk cache…".into();
        }
    }
    pub fn remember_cache_policy(&mut self) {
        self.save_preferences();
    }
    fn load_blank_picture(&mut self, source: Option<PathBuf>) {
        let path = self.settings_path.with_file_name("blank-picture.rgba");
        self.blank_busy = self.blank_loader.submit(
            0,
            u64::from(source.is_some()),
            Request::BlankPicture { source, path },
        );
        if self.blank_busy {
            self.status = "Preparing blank presentation picture…".into();
        }
    }
    pub fn poll(&mut self) {
        if self.initialized
            && self
                .startup_single_screen
                .as_ref()
                .is_some_and(|id| self.is_single_screen(id))
        {
            self.startup_single_screen = None;
        }
        while let Ok(c) = self.blank_loader.rx.try_recv() {
            self.blank_busy = false;
            let result = c.result.and_then(|p| {
                if let Some(error) = p.reply.error {
                    return Err(error);
                }
                let info = p.reply.frame.ok_or("Missing blank picture")?;
                Ok(Arc::new(Frame { info, rgba: p.data }))
            });
            match result {
                Ok(frame) => {
                    self.blank_picture = Some(frame);
                    self.settings.blank_picture = true;
                    self.pending_save |= c.tag == 1;
                    self.new_output = true;
                    self.status = "Blank slides picture ready.".into();
                }
                Err(error) => self.status = format!("Blank picture: {error}"),
            }
        }
        while let Ok(c) = self.cache_io.rx.try_recv() {
            self.cache_busy = false;
            self.cache_status = match c.result {
                Ok(p) => match (p.reply.error, p.reply.cleanup) {
                    (Some(e), _) => format!("Cache cleanup: {e}"),
                    (_, Some(r)) => format!(
                        "Disk cache: {} MiB remaining; removed {} files ({} MiB); {} protected/changed files retained.",
                        r.remaining_bytes.div_ceil(1024 * 1024),
                        r.removed_files,
                        r.removed_bytes.div_ceil(1024 * 1024),
                        r.skipped_files
                    ),
                    _ => "Cache cleanup returned no report.".into(),
                },
                Err(e) => format!("Cache cleanup: {e}"),
            };
        }

        while let Ok(completion) = self.notes_io.rx.try_recv() {
            if completion.generation != self.generation {
                continue;
            }
            if completion.tag >= 2 {
                if let Some(import) = &mut self.notes_import {
                    import.busy = false;
                    match completion.result {
                        Ok(packet) => {
                            import.error = packet.reply.error;
                            if let Some(preview) = packet.reply.notes_import {
                                import.preview = Some(preview);
                                self.status = "Review the notes import before saving.".into();
                            }
                            if let Some(snapshot) = packet.reply.notes {
                                if let Some(deck) = &mut self.deck {
                                    deck.notes = snapshot.notes;
                                }
                                self.notes_edited = true;
                                self.status =
                                    format!("Notes imported: {}", snapshot.path.display());
                                self.notes_import = None;
                            }
                        }
                        Err(error) => import.error = Some(error),
                    }
                    if let Some(error) = self.notes_import.as_ref().and_then(|i| i.error.as_ref()) {
                        self.status = format!("Notes import: {error}");
                    }
                }
                continue;
            }
            if let Some(editor) = &mut self.notes_editor {
                editor.busy = false;
                match completion.result {
                    Ok(packet) => {
                        editor.error = packet.reply.error;
                        if let Some(snapshot) = packet.reply.notes {
                            editor.text =
                                snapshot.notes.get(editor.page).cloned().unwrap_or_default();
                            editor.saved_text = editor.text.clone();
                            if let Some(deck) = &mut self.deck {
                                deck.notes = snapshot.notes.clone();
                            }
                            editor.snapshot = Some(snapshot);
                            self.notes_edited = true;
                            self.status = if completion.tag == 1 {
                                "Notes saved."
                            } else {
                                "Notes editor ready."
                            }
                            .into();
                        }
                    }
                    Err(error) => editor.error = Some(error),
                }
                if let Some(error) = &editor.error {
                    self.status = format!("Notes: {error}");
                }
            }
        }
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
                            if self.settings.blank_picture {
                                self.load_blank_picture(None);
                            }
                            self.set_zoom(crate::zoom::View::default());
                            self.layout = self.settings.for_project(&self.project());
                            self.presenter_views.clear();
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
                        if c.tag == 2 {
                            self.execute(Action::SetBlankPicture(path));
                        } else if c.tag == 1 {
                            if c.generation == self.generation {
                                self.execute(Action::ImportNotes(path));
                            }
                        } else {
                            self.execute(Action::Open(path));
                        }
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
                                if !self.notes_edited
                                    && let Some(deck) = &mut self.deck
                                {
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
                            self.notes_edited = false;
                            self.cached_slides = 0;
                            self.evicted_frames = 0;
                            self.generation += 1;
                            self.frames.clear();
                            self.prepared.clear();
                            self.preparation_started = Instant::now();
                            self.output_buffered = 0;
                            self.ram_blocked = false;
                            self.failed.clear();
                            self.set_zoom(crate::zoom::View::default());
                            self.toolbox.clear();
                            self.new_overlay = true;
                            self.requested = 0;
                            self.presentation_started = false;
                            self.timer = crate::timer::Timer::default();
                            self.displayed = None;
                            self.layout = self.settings.for_project(&info.key);
                            self.presenter_views.clear();
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
        if self
            .navigation_settle
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.navigation_settle = None;
        }
        if self.deck.is_some() && self.pending_open.is_none() && self.navigation_settle.is_none() {
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
        // One startup pass, after source/notes loading and initial rendering settle.
        // Prepared disk artifacts count even when the deck exceeds the RAM budget.
        if self.startup_cache_pending
            && self.open_path.is_none()
            && self.pending_open.is_none()
            && !self.fg_busy
            && !self.bg_busy
            && !self.notes_pending
            && !self.notes_loading
            && self.prepared.len() == self.count()
        {
            self.clean_cache(false);
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
    /// Use the first connected presentation route as the preview reference.
    pub fn presentation_viewport(&self) -> Option<[u32; 2]> {
        let monitor = self
            .routes
            .presentation
            .iter()
            .find_map(|id| self.monitors.iter().find(|m| &m.id == id))?;
        if let Some(viewport) = &self.output_viewport
            && viewport.monitor == monitor.id
            && viewport.fullscreen
                == self.screen_fullscreen(&monitor.id, self.presentation_fullscreen())
            && viewport.size.iter().all(|v| *v > 0)
        {
            return Some(viewport.size);
        }
        Some([monitor.size.0.max(1), monitor.size.1.max(1)])
    }
    pub fn set_zoom(&mut self, view: crate::zoom::View) {
        self.zoom.keep_fitted_area = self.settings.zoom_keep_fitted_area;
        self.zoom.set(self.settings.zoom_enabled, view);
        self.new_zoom = true;
    }
    pub fn set_zoom_from_action(&mut self, view: crate::zoom::View) {
        if self.settings.zoom_enabled && view.normalized() != self.zoom.view.normalized() {
            self.presentation_action();
        }
        self.set_zoom(view);
    }
    pub fn enable_zoom(&mut self, enabled: bool) {
        self.settings.zoom_enabled = enabled;
        self.set_zoom(crate::zoom::View::default());
        self.zoom_warning = false;
        self.zoom_warning_dismissed = false;
        self.save_preferences();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_navigation_coalesces_rendering_and_clamps_without_blanking() {
        let mut s = Studio::new(true, None);
        s.deck = Some(DeckInfo {
            path: "test.pdf".into(),
            key: "test".into(),
            title: "Test".into(),
            names: vec![String::new(); 500],
            notes: vec![],
            warnings: vec![],
            sha256: String::new(),
        });
        s.blanked = false;
        s.fg_busy = true;
        s.fg_job = Some((0, s.target_edge));
        for _ in 0..100 {
            s.execute(Action::Navigate {
                delta: 10,
                repeated: true,
            });
        }
        assert_eq!(s.requested, 499);
        assert!(!s.blanked);
        assert!(
            s.fg_busy,
            "Repeats must not repeatedly replace renderer workers"
        );
        assert_eq!(s.fg_job, Some((0, s.target_edge)));
        s.execute(Action::FinishNavigation);
        assert!(!s.fg_busy && s.fg_job.is_none());
        assert!(s.navigation_settle.is_none());
        s.execute(Action::Navigate {
            delta: -10,
            repeated: true,
        });
        s.poll();
        assert!(
            !s.fg_busy && !s.bg_busy,
            "Skipped slides must not start render jobs during a hold"
        );
        assert_eq!(s.requested, 489);
        s.execute(Action::Navigate {
            delta: -1000,
            repeated: true,
        });
        assert_eq!(s.requested, 0);
        s.execute(Action::FinishNavigation);
    }
    #[test]
    fn absent_frames_report_their_actual_load_state() {
        let mut studio = Studio::new(true, None);
        assert_eq!(studio.slide_load_state(7), SlideLoadState::Queued);

        studio.prepared.insert(7);
        assert_eq!(studio.slide_load_state(7), SlideLoadState::Cached);

        studio.fg_job = Some((7, studio.target_edge));
        assert_eq!(studio.slide_load_state(7), SlideLoadState::Preparing);

        studio.fg_job = None;
        studio.failed.insert((7, studio.target_edge));
        assert_eq!(studio.slide_load_state(7), SlideLoadState::Failed);
    }
}
