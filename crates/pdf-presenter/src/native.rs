//! Native multiwindow host using winit, egui and cached software composition.
use crate::{
    document::Frame,
    output::{Ack, DisplayAction, HostState, OutputLink, SleepLease},
    presenter_ui::{self, UiState},
    studio::{Action, Monitor, Studio},
    worker::{self, Bridge, Request},
};
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU32,
    rc::Rc,
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    monitor::MonitorHandle,
    window::{Fullscreen, Window, WindowId},
};
type Failure = Box<dyn std::error::Error>;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Presenter,
    Browser,
    Presentation,
}
struct NativeWindow {
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    window: Rc<Window>,
    ctx: egui::Context,
    input: egui_winit::State,
    painter: EguiSoftwareRender,
    ui: UiState,
    role: Role,
    monitor: String,
    target: Option<(String, bool)>,
    transition: Option<(Instant, bool)>,
    frames: usize,
    display_action: Option<DisplayAction>,
}
impl NativeWindow {
    fn create(el: &ActiveEventLoop, role: Role, monitor: String) -> Result<Self, Failure> {
        let title = match role {
            Role::Presenter => "PDF Presenter — Presenter",
            Role::Browser => "PDF Presenter — All slides",
            Role::Presentation => "PDF Presenter — Presentation",
        };
        let window = Rc::new(
            el.create_window(
                crate::artwork::window_attributes()
                    .with_title(title)
                    .with_window_level(if role == Role::Browser {
                        winit::window::WindowLevel::AlwaysOnTop
                    } else {
                        winit::window::WindowLevel::Normal
                    })
                    .with_visible(false)
                    .with_inner_size(PhysicalSize::new(1100, 750)),
            )?,
        );
        let surface =
            softbuffer::Surface::new(&softbuffer::Context::new(window.clone())?, window.clone())?;
        let ctx = egui::Context::default();
        presenter_ui::theme(&ctx);
        let input = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(4096),
        );
        Ok(Self {
            surface,
            window,
            ctx,
            input,
            painter: EguiSoftwareRender::new(ColorFieldOrder::Bgra),
            ui: UiState::default(),
            role,
            monitor,
            target: None,
            transition: None,
            frames: 0,
            display_action: None,
        })
    }
    fn place(&mut self, handle: &MonitorHandle, id: &str, fullscreen: bool, x11: bool) {
        let target = (id.to_owned(), fullscreen);
        if self.target.as_ref() == Some(&target) {
            return;
        }
        self.target = Some(target);
        self.monitor = id.into();
        if x11 {
            self.window.set_fullscreen(None);
            self.transition = Some((Instant::now() + Duration::from_millis(120), false));
        } else {
            self.move_to(handle, fullscreen);
        }
        self.window.request_redraw();
    }
    fn move_to(&self, handle: &MonitorHandle, fullscreen: bool) {
        if fullscreen {
            self.window
                .set_fullscreen(Some(Fullscreen::Borderless(Some(handle.clone()))));
        } else {
            self.window.set_fullscreen(None);
            let p = handle.position();
            let size = handle.size();
            self.window
                .set_outer_position(PhysicalPosition::new(p.x + 40, p.y + 40));
            let _ = self.window.request_inner_size(PhysicalSize::new(
                size.width.saturating_sub(80).clamp(320, 1100),
                size.height.saturating_sub(80).clamp(240, 750),
            ));
        }
        self.window.set_visible(true);
        if self.role == Role::Browser {
            self.window.focus_window();
        }
        self.window
            .set_cursor_visible(self.role != Role::Presentation);
    }
    fn transition(&mut self, handles: &[(Monitor, MonitorHandle)], now: Instant) {
        if let Some((deadline, enter)) = self.transition
            && now >= deadline
        {
            if let Some((_, handle)) = handles.iter().find(|(m, _)| m.id == self.monitor) {
                let full = self.target.as_ref().is_some_and(|(_, f)| *f);
                if enter {
                    self.move_to(handle, full);
                    self.transition = None;
                } else {
                    self.move_to(handle, false);
                    self.transition = full.then_some((now + Duration::from_millis(120), true));
                }
                self.window.request_redraw();
            } else {
                self.window.set_visible(false);
                self.transition = None;
            }
        }
    }
    fn render(
        &mut self,
        mut studio: Option<&mut Studio>,
        frame: Option<&Arc<Frame>>,
        blank: bool,
        setup: Option<&HostState>,
    ) -> Result<(), Failure> {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        if u64::from(size.width) * u64::from(size.height) > 40_000_000 {
            return Err("Display exceeds 40 megapixels".into());
        }
        if let Some(s) = studio.as_ref() {
            s.settings.theme.apply(&self.ctx);
            crate::localization::apply(&self.ctx, &s.settings.language);
        }
        let input = self.input.take_egui_input(&self.window);
        let role = self.role;
        let ui_state = &mut self.ui;
        let ctx = self.ctx.clone();
        let display_action = &mut self.display_action;
        let output = ctx.run_ui(input, |ui| match role {
            Role::Presenter => {
                presenter_ui::presenter(ui, studio.as_deref_mut().unwrap(), ui_state)
            }
            Role::Browser => presenter_ui::browser(ui, studio.as_deref_mut().unwrap(), ui_state),
            Role::Presentation => {
                ui.painter()
                    .rect_filled(ui.max_rect(), 0., egui::Color32::BLACK);
                if let Some(setup) = setup {
                    *display_action = presenter_ui::output_display_recovery(
                        ui,
                        setup.setup_controls,
                        setup.route_trial.is_some(),
                    );
                }
                if !blank && let Some(frame) = frame {
                    use std::hash::{Hash, Hasher};
                    let mut hash = std::hash::DefaultHasher::new();
                    frame.info.path.hash(&mut hash);
                    let key = (
                        hash.finish(),
                        frame.info.page,
                        frame.info.width.max(frame.info.height),
                    );
                    let tex = ui_state.textures.entry(key).or_insert_with(|| {
                        ctx.load_texture(
                            format!("output-{}", frame.info.path.display()),
                            egui::ColorImage::from_rgba_unmultiplied(
                                [frame.info.width as usize, frame.info.height as usize],
                                &frame.rgba,
                            ),
                            egui::TextureOptions::LINEAR,
                        )
                    });
                    presenter_ui::paint_slide(
                        ui.painter(),
                        ui.max_rect(),
                        tex.id(),
                        [frame.info.width, frame.info.height],
                        setup.map(|s| s.slide_colors).unwrap_or_default(),
                    );
                }
            }
        });
        self.input
            .handle_platform_output(&self.window, output.platform_output);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        self.surface.resize(
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        )?;
        let mut buffer = self.surface.buffer_mut()?;
        buffer.fill(0);
        self.painter.render(
            &mut BufferMutRef::new(
                bytemuck::cast_slice_mut(&mut buffer),
                size.width as usize,
                size.height as usize,
            ),
            &primitives,
            &output.textures_delta,
            output.pixels_per_point,
        );
        self.window.pre_present_notify();
        buffer.present()?;
        self.frames += 1;
        Ok(())
    }
}
fn monitors(el: &ActiveEventLoop) -> (Vec<(Monitor, MonitorHandle)>, usize) {
    let primary = el.primary_monitor();
    let mut names = HashMap::new();
    let handles: Vec<_> = el
        .available_monitors()
        .map(|h| {
            let name = h.name().unwrap_or_else(|| "Display".into());
            let count = names.entry(name.clone()).or_insert(0);
            *count += 1;
            let m = Monitor {
                id: format!("{name}#{}", count),
                name,
                origin: h.position().into(),
                size: h.size().into(),
            };
            (m, h)
        })
        .collect();
    let primary = primary
        .and_then(|p| handles.iter().position(|(_, h)| *h == p))
        .unwrap_or(0);
    (handles, primary)
}
struct Host {
    parent_connected: bool,
    close_requested: bool,
    commands: Receiver<HostState>,
    acks: SyncSender<Ack>,
    state: HostState,
    reader: Bridge,
    reading: Option<std::path::PathBuf>,
    cache: HashMap<std::path::PathBuf, Arc<Frame>>,
    frame: Option<Arc<Frame>>,
    presented: HashSet<WindowId>,
    failed_reads: HashSet<std::path::PathBuf>,
    preload_cursor: usize,
}
// The last visible frame may outlive its cache entry during a working-set change.
// Count that allocation and pin it until the requested replacement is available.
fn make_output_room(
    cache: &mut HashMap<std::path::PathBuf, Arc<Frame>>,
    displayed: Option<&Arc<Frame>>,
    state: &HostState,
    incoming: usize,
    limit: usize,
) -> bool {
    loop {
        let bytes = cache.values().map(|f| f.rgba.len()).sum::<usize>()
            + displayed
                .filter(|f| !cache.contains_key(&f.info.path))
                .map_or(0, |f| f.rgba.len());
        if bytes.saturating_add(incoming) <= limit {
            return true;
        }
        let victim = cache
            .iter()
            .filter(|(path, _)| {
                state.frame.as_ref().is_none_or(|f| &f.path != *path)
                    && displayed.is_none_or(|f| &f.info.path != *path)
            })
            .max_by_key(|(_, f)| f.info.page.abs_diff(state.page))
            .map(|(p, _)| p.clone());
        let Some(victim) = victim else {
            return false;
        };
        cache.remove(&victim);
    }
}

struct App {
    studio: Option<Studio>,
    host: Option<Host>,
    output: Option<OutputLink>,
    lease: Option<SleepLease>,
    windows: Vec<NativeWindow>,
    handles: Vec<(Monitor, MonitorHandle)>,
    next_poll: Instant,
    next_clock: Instant,
    next_monitors: Instant,
    x11: bool,
    error: Option<String>,
    smoke: bool,
    smoke_stage: usize,
    started: Instant,
    windowed: bool,
    override_presenter: Option<usize>,
    override_output: Option<usize>,
}
impl App {
    fn redraw(&self) {
        for native in &self.windows {
            native.window.request_redraw();
        }
    }
    fn refresh(&mut self, el: &ActiveEventLoop) {
        let (handles, primary) = monitors(el);
        let changed = self
            .handles
            .iter()
            .map(|(m, _)| (&m.id, m.origin, m.size))
            .collect::<Vec<_>>()
            != handles
                .iter()
                .map(|(m, _)| (&m.id, m.origin, m.size))
                .collect::<Vec<_>>();
        self.handles = handles;
        if self
            .override_presenter
            .is_some_and(|i| i >= self.handles.len())
            || self
                .override_output
                .is_some_and(|i| i >= self.handles.len())
            || (self.override_presenter.is_some()
                && self.override_presenter == self.override_output)
        {
            self.error =
                Some("Choose distinct, connected presenter and presentation monitors".into());
            el.exit();
            return;
        }
        if let Some(s) = &mut self.studio {
            if changed {
                s.set_monitors(
                    self.handles.iter().map(|(m, _)| m.clone()).collect(),
                    primary,
                );
            }
            if s.dirty_routes {
                let ids = s.monitors.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
                if s.initialized && s.routes.presenter.is_empty() && !s.privacy_hold {
                    s.routes = s.settings.routes.resolve(&ids, primary);
                }
                if self.windowed {
                    s.settings.presenter_fullscreen = false;
                    s.settings.presentation_fullscreen = false;
                }
                if let Some(index) = self.override_presenter.take()
                    && let Some((m, _)) = self.handles.get(index)
                {
                    s.routes.presenter = vec![m.id.clone()];
                    s.routes.presentation.retain(|id| id != &m.id);
                }
                if let Some(index) = self.override_output.take()
                    && let Some((m, _)) = self.handles.get(index)
                {
                    s.routes.presentation = vec![m.id.clone()];
                    s.routes.presenter.retain(|id| id != &m.id);
                    if s.routes.presenter.is_empty() {
                        if let Some((presenter, _)) =
                            self.handles.iter().find(|(p, _)| p.id != m.id)
                        {
                            s.routes.presenter.push(presenter.id.clone());
                        } else {
                            self.error = Some(
                                "Presentation output requires a separate presenter monitor".into(),
                            );
                            el.exit();
                            return;
                        }
                    }
                }
                s.draft_routes = s.routes.clone();
            }
        }
        if changed && let Some(host) = &mut self.host {
            host.state.blanked = true;
            host.presented.clear();
            for w in &self.windows {
                if !self.handles.iter().any(|(m, _)| m.id == w.monitor) {
                    w.window.set_visible(false);
                }
            }
        }
    }
    fn sync_windows(&mut self, el: &ActiveEventLoop) -> Result<(), Failure> {
        let mut desired = vec![];
        if let Some(s) = &self.studio {
            for id in &s.routes.presenter {
                desired.push((Role::Presenter, id.clone(), s.presenter_fullscreen()));
            }
            if s.browser_open {
                let id = s
                    .settings
                    .browser_monitor
                    .as_ref()
                    .filter(|id| self.handles.iter().any(|(m, _)| &m.id == *id))
                    .cloned()
                    .or_else(|| s.routes.presenter.first().cloned());
                if let Some(id) = id {
                    desired.push((Role::Browser, id, false));
                }
            }
        }
        if let Some(h) = &self.host {
            for id in &h.state.monitors {
                if self.handles.iter().any(|(m, _)| &m.id == id) {
                    desired.push((Role::Presentation, id.clone(), h.state.fullscreen));
                }
            }
        }
        self.windows.retain(|w| {
            desired.iter().any(|(role, id, _)| {
                *role == w.role && (w.role == Role::Browser || *id == w.monitor)
            })
        });
        for (role, id, full) in desired {
            let Some((_, handle)) = self.handles.iter().find(|(m, _)| m.id == id) else {
                continue;
            };
            let index = if let Some(index) = self
                .windows
                .iter()
                .position(|w| w.role == role && (role == Role::Browser || w.monitor == id))
            {
                index
            } else {
                self.windows
                    .push(NativeWindow::create(el, role, id.clone())?);
                self.windows.len() - 1
            };
            self.windows[index].place(handle, &id, full, self.x11);
        }
        if let Some(s) = &mut self.studio {
            s.dirty_routes = false;
        }
        Ok(())
    }
    fn key(&mut self, role: Role, key: Key, consumed: bool, modifiers: bool) {
        if role == Role::Presentation {
            if matches!(key, Key::Named(NamedKey::Escape))
                && let Some(host) = &self.host
                && host.state.route_trial.is_some()
            {
                let _ = host.acks.try_send(Ack {
                    display_action: Some(DisplayAction::Revert),
                    route_trial: host.state.route_trial,
                    ..Default::default()
                });
                return;
            }
            if matches!(key, Key::Named(NamedKey::Escape))
                || matches!(&key,Key::Character(c) if c.eq_ignore_ascii_case("b"))
            {
                if let Some(h) = &mut self.host {
                    h.state.blanked = true;
                    h.close_requested = matches!(key, Key::Named(NamedKey::Escape));
                    let _ = h.acks.try_send(Ack {
                        status: "Presentation blanked from output keyboard.".into(),
                        ..Default::default()
                    });
                }
                self.redraw();
            }
            return;
        }
        let Some(s) = &mut self.studio else {
            return;
        };
        if matches!(key, Key::Named(NamedKey::Escape)) && s.route_trial.is_some() {
            s.execute(Action::RevertRoutes);
            return;
        }
        if modifiers || consumed {
            return;
        }
        match key {
            Key::Character(c) if c.chars().all(|c| c.is_ascii_digit()) => {
                if s.jump.len() < 6 {
                    s.jump.push_str(&c);
                }
            }
            Key::Named(NamedKey::Enter) if !s.jump.is_empty() => {
                if let Ok(number) = s.jump.parse::<usize>()
                    && number > 0
                    && number <= s.count()
                {
                    s.execute(Action::Go(number - 1));
                } else {
                    s.status = format!("Choose a slide from 1 to {}", s.count());
                }
            }
            Key::Named(NamedKey::Backspace) if !s.jump.is_empty() => {
                s.jump.pop();
            }
            Key::Named(NamedKey::Escape) => {
                if !s.jump.is_empty() {
                    s.jump.clear();
                } else if role == Role::Browser {
                    s.execute(Action::CloseBrowser);
                } else if s.layout_open || s.displays_open || s.menu_open {
                    s.layout_open = false;
                    s.displays_open = false;
                    s.menu_open = false;
                } else {
                    s.execute(Action::Windowed);
                }
            }
            Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space)
                if role == Role::Presenter =>
            {
                s.execute(Action::Go(s.requested + 1))
            }
            Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) if role == Role::Presenter => {
                s.execute(Action::Go(s.requested.saturating_sub(1)))
            }
            Key::Character(c) if c.eq_ignore_ascii_case("b") => s.execute(Action::Blank),
            _ => {}
        }
        self.redraw();
    }
    fn poll(&mut self, el: &ActiveEventLoop) {
        let mut repaint = false;
        if let Some(s) = &mut self.studio {
            let before = (
                s.generation,
                s.frames.len(),
                s.requested,
                s.blanked,
                s.status.clone(),
                s.layout_open,
                s.browser_open,
            );
            let chooser_was_open = s.dialog_busy;
            s.poll();
            let actions = std::mem::take(&mut s.actions);
            repaint |= !actions.is_empty();
            for action in actions {
                #[cfg(target_os = "windows")]
                if matches!(action, Action::Pick(_)) && !s.dialog_busy {
                    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
                    s.dialog_owner = self
                        .windows
                        .iter()
                        .filter(|w| w.role == Role::Presenter)
                        .find(|w| w.window.has_focus())
                        .or_else(|| self.windows.iter().find(|w| w.role == Role::Presenter))
                        .and_then(|w| w.window.window_handle().ok())
                        .and_then(|h| match h.as_raw() {
                            RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
                            _ => None,
                        });
                }
                if matches!(action, Action::OpenBrowser) {
                    for window in self.windows.iter().filter(|w| w.role == Role::Browser) {
                        window.window.set_minimized(false);
                        window.window.focus_window();
                    }
                }
                s.execute(action);
            }
            if chooser_was_open != s.dialog_busy {
                repaint = true;
                for window in &self.windows {
                    if window.role == Role::Browser {
                        window.window.set_window_level(if s.dialog_busy {
                            winit::window::WindowLevel::Normal
                        } else {
                            winit::window::WindowLevel::AlwaysOnTop
                        });
                    }
                    #[cfg(target_os = "windows")]
                    if !s.dialog_busy && window.role == Role::Presenter {
                        use winit::{
                            platform::windows::WindowExtWindows,
                            raw_window_handle::{HasWindowHandle, RawWindowHandle},
                        };
                        // The worker can time out while COM has the owner disabled.
                        window.window.set_enable(true);
                        if window.window.window_handle().ok().is_some_and(|h| matches!(h.as_raw(),RawWindowHandle::Win32(h) if Some(h.hwnd.get()) == s.dialog_owner)) {
                            window.window.focus_window();
                        }
                    }
                }
            }
            if let Some(output) = &self.output {
                while let Ok(ack) = output.rx.try_recv() {
                    if let Some(action) = ack.display_action {
                        s.display_control(action, ack.route_trial);
                    }
                    if ack.generation == s.buffer_generation() {
                        s.output_buffered = ack.buffered;
                    }
                    if !ack.awake.is_empty() {
                        s.awake_status = ack.awake;
                    }
                    if !ack.status.is_empty() {
                        s.status = ack.status;
                        s.output_buffered = 0;
                        s.blanked = true;
                        s.new_output = true;
                    }
                    if ack.sequence == s.output_sequence {
                        s.displayed = ack.page;
                    }
                }
                if s.new_output {
                    s.output_sequence += 1;
                    let state = HostState {
                        setup_controls: !s.presentation_started,
                        route_trial: s.route_trial.as_ref().map(|t| t.id),
                        sequence: s.output_sequence,
                        slide_colors: s.settings.slide_colors,
                        page: s.requested,
                        frame: if s.routes.presentation.is_empty() {
                            None
                        } else {
                            s.frame_info()
                        },
                        generation: s.buffer_generation(),
                        buffer_mib: s.settings.buffer_mib,
                        preload: if s.routes.presentation.is_empty() {
                            vec![]
                        } else {
                            (0..s.count())
                                .filter_map(|p| {
                                    s.frames.get(&(p, s.target_edge)).map(|f| f.info.clone())
                                })
                                .collect()
                        },
                        monitors: s.routes.presentation.clone(),
                        fullscreen: s.presentation_fullscreen(),
                        blanked: s.blanked,
                        ..Default::default()
                    };
                    if output.tx.try_send(state).is_ok() {
                        s.new_output = false;
                    } else {
                        s.output_sequence -= 1;
                    }
                }
            }
            if s.close_requested {
                el.exit();
            }
            repaint |= before
                != (
                    s.generation,
                    s.frames.len(),
                    s.requested,
                    s.blanked,
                    s.status.clone(),
                    s.layout_open,
                    s.browser_open,
                );
        }
        if let Some(host) = &mut self.host {
            loop {
                let state = match host.commands.try_recv() {
                    Ok(state) => state,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        host.parent_connected = false;
                        host.state.route_trial = None;
                        host.state.setup_controls = false;
                        break;
                    }
                };
                if !state.blanked {
                    host.close_requested = false;
                }
                repaint = true;
                if state.quit {
                    el.exit();
                    return;
                }
                if state.frame.as_ref().map(|f| &f.path)
                    != host.state.frame.as_ref().map(|f| &f.path)
                {
                    host.presented.clear();
                    for w in &mut self.windows {
                        w.ui.textures.clear();
                    }
                }
                if state.generation != host.state.generation {
                    host.cache.clear();
                    host.failed_reads.clear();
                    host.frame = None;
                }
                // Mirror the presenter working set, retiring evicted and obsolete frames.
                let retained: HashSet<_> = state
                    .preload
                    .iter()
                    .map(|f| &f.path)
                    .chain(state.frame.iter().map(|f| &f.path))
                    .collect();
                host.cache.retain(|path, _| retained.contains(path));
                host.preload_cursor = 0;
                if state.buffer_mib > host.state.buffer_mib {
                    host.failed_reads.clear();
                }
                if state.monitors.is_empty() {
                    host.frame = None;
                }
                host.state = state;
            }
            if !host.parent_connected && (host.close_requested || self.windows.is_empty()) {
                el.exit();
                return;
            }
            while let Ok(c) = host.reader.rx.try_recv() {
                repaint = true;
                let reading = host.reading.take();
                if c.generation != host.state.generation {
                    continue;
                }
                match c.result {
                    Ok(packet) if packet.reply.frame.is_some() => {
                        let info = packet.reply.frame.unwrap();
                        // Ignore completed reads that the presenter has already evicted.
                        if !host
                            .state
                            .preload
                            .iter()
                            .chain(host.state.frame.iter())
                            .any(|f| f.path == info.path)
                        {
                            continue;
                        }
                        if host.cache.contains_key(&info.path) {
                            continue;
                        }
                        let limit = host.state.buffer_mib.max(256) as usize * 1024 * 1024;
                        if make_output_room(
                            &mut host.cache,
                            host.frame.as_ref(),
                            &host.state,
                            packet.data.len(),
                            limit,
                        ) {
                            host.cache.insert(
                                info.path.clone(),
                                Arc::new(Frame {
                                    info,
                                    rgba: packet.data,
                                }),
                            );
                        } else {
                            let _ = host.acks.try_send(Ack {
                                status: "Presentation RAM cannot fit active frames; increase the budget.".into(),
                                ..Default::default()
                            });
                        }
                    }
                    _ => {
                        if let Some(path) = reading {
                            host.failed_reads.insert(path);
                            let _ = host.acks.try_send(Ack {
                                status:
                                    "Could not buffer an output slide; reopen the source to retry."
                                        .into(),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            if let Some(info) = &host.state.frame {
                if let Some(frame) = host.cache.get(&info.path) {
                    host.frame = Some(frame.clone());
                } else if host.reading.is_none()
                    && !host.failed_reads.contains(&info.path)
                    && host.reader.submit(
                        host.state.generation,
                        0,
                        Request::ReadFrame(info.path.clone()),
                    )
                {
                    host.reading = Some(info.path.clone());
                }
            }
            while host.reading.is_none() && host.preload_cursor < host.state.preload.len() {
                let info = &host.state.preload[host.preload_cursor];
                if host.cache.contains_key(&info.path) || host.failed_reads.contains(&info.path) {
                    host.preload_cursor += 1;
                } else if host.reader.submit(
                    host.state.generation,
                    0,
                    Request::ReadFrame(info.path.clone()),
                ) {
                    host.reading = Some(info.path.clone());
                    host.preload_cursor += 1;
                } else {
                    break;
                }
            }
        }
        if let Some(lease) = &self.lease {
            while let Ok(status) = lease.status.try_recv() {
                if let Some(s) = &mut self.studio {
                    s.awake_status = status.clone();
                }
                if let Some(host) = &self.host {
                    let _ = host.acks.try_send(Ack {
                        sequence: host.state.sequence,
                        awake: status,
                        ..Default::default()
                    });
                }
            }
        }
        if self.studio.as_ref().is_some_and(|s| s.dirty_routes) {
            self.refresh(el);
            if let Some(s) = &mut self.studio {
                s.new_output = true;
            }
        }
        if repaint {
            self.redraw();
        }
        if let Err(e) = self.sync_windows(el) {
            self.error = Some(e.to_string());
            el.exit();
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        #[cfg(target_os = "linux")]
        {
            use winit::platform::x11::ActiveEventLoopExtX11;
            self.x11 = el.is_x11();
        }
        self.refresh(el);
        self.poll(el);
    }
    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(index) = self.windows.iter().position(|w| w.window.id() == id) else {
            return;
        };
        let role = self.windows[index].role;
        let response = {
            let n = &mut self.windows[index];
            n.input.on_window_event(&n.window, &event)
        };
        if response.repaint {
            self.windows[index].window.request_redraw();
        }
        match event {
            WindowEvent::CloseRequested => {
                if role == Role::Presenter {
                    el.exit();
                } else if role == Role::Browser {
                    if let Some(s) = &mut self.studio {
                        s.execute(Action::CloseBrowser);
                    }
                } else if let Some(h) = &mut self.host {
                    h.close_requested = true;
                    h.state.blanked = true;
                    let _ = h.acks.try_send(Ack {
                        status: "Presentation blanked from output window.".into(),
                        ..Default::default()
                    });
                    self.redraw();
                }
            }
            WindowEvent::DroppedFile(path) => {
                if let Some(s) = &mut self.studio {
                    s.execute(Action::Open(path));
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                let native = &self.windows[index];
                self.key(
                    role,
                    event.logical_key,
                    native.ctx.egui_wants_keyboard_input(),
                    native
                        .ctx
                        .input(|i| i.modifiers.ctrl || i.modifiers.alt || i.modifiers.command),
                );
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.windows[index].window.request_redraw()
            }
            WindowEvent::RedrawRequested => {
                let frame = self.host.as_ref().and_then(|h| h.frame.clone());
                let blank = self.host.as_ref().is_none_or(|h| h.state.blanked);
                let result = self.windows[index].render(
                    self.studio.as_mut(),
                    frame.as_ref(),
                    blank,
                    self.host.as_ref().map(|h| &h.state),
                );
                if let Some(action) = self.windows[index].display_action.take()
                    && let Some(host) = &self.host
                {
                    let _ = host.acks.try_send(Ack {
                        display_action: Some(action),
                        route_trial: host.state.route_trial,
                        ..Default::default()
                    });
                }
                if let Err(e) = result {
                    self.error = Some(e.to_string());
                    el.exit();
                }
                if let Some(h) = &mut self.host
                    && h.state.frame.as_ref().map(|f| &f.path)
                        == h.frame.as_ref().map(|f| &f.info.path)
                {
                    h.presented.insert(id);
                    if h.presented.len() >= self.windows.len() {
                        let _ = h.acks.try_send(Ack {
                            sequence: h.state.sequence,
                            page: (!h.state.blanked).then_some(h.state.page),
                            ..Default::default()
                        });
                    }
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_monitors {
            self.refresh(el);
            self.next_monitors = now + Duration::from_millis(750);
        }
        if now >= self.next_poll {
            self.poll(el);
            self.next_poll = now + Duration::from_millis(8);
        }
        if now >= self.next_clock {
            self.redraw();
            self.next_clock = now + Duration::from_secs(1);
            if let Some(host) = &self.host {
                let shown = !host.state.blanked
                    && !self.windows.is_empty()
                    && host.presented.len() >= self.windows.len();
                let _ = host.acks.try_send(Ack {
                    sequence: host.state.sequence,
                    generation: host.state.generation,
                    buffered: host
                        .state
                        .preload
                        .iter()
                        .filter(|f| host.cache.contains_key(&f.path))
                        .count(),
                    page: shown.then_some(host.state.page),
                    ..Default::default()
                });
            }
        }
        for window in &mut self.windows {
            window.transition(&self.handles, now);
        }
        if self.smoke
            && self.started.elapsed() > Duration::from_millis((self.smoke_stage as u64 + 1) * 300)
            && let Some(s) = &mut self.studio
        {
            match self.smoke_stage {
                0 => s.layout_open = true,
                1 => {
                    s.layout_open = false;
                    s.displays_open = true;
                }
                2 => {
                    s.displays_open = false;
                    s.execute(Action::OpenBrowser);
                }
                3 => s.execute(Action::CloseBrowser),
                4 => {
                    s.layout.header = false;
                    s.layout.footer = false;
                }
                5 => {
                    s.layout = Default::default();
                    s.execute(Action::Windowed);
                }
                6 => {
                    if self.windows.iter().all(|w| w.frames > 0) && !self.windows.is_empty() {
                        println!(
                            "Native smoke passed: presenter frames rendered, controls and browser exercised"
                        );
                        el.exit();
                    } else {
                        self.error = Some("No native presenter frames".into());
                        el.exit();
                    }
                }
                _ => {}
            }
            self.smoke_stage += 1;
            self.redraw();
        }
        el.set_control_flow(ControlFlow::WaitUntil(self.next_poll.min(self.next_clock)));
    }
}
pub fn run(options: crate::poc::Options, path: Option<std::path::PathBuf>) -> Result<(), Failure> {
    if options.list {
        return crate::desktop::run(options);
    }
    let el = EventLoop::new()?;
    let now = Instant::now();
    let studio = Studio::new(options.smoke, path);
    let mut app = App {
        studio: Some(studio),
        host: None,
        output: Some(OutputLink::new()),
        lease: Some(SleepLease::new()),
        windows: vec![],
        handles: vec![],
        next_poll: now,
        next_clock: now,
        next_monitors: now,
        x11: false,
        error: None,
        smoke: options.smoke,
        smoke_stage: 0,
        started: now,
        windowed: options.windowed,
        override_presenter: options.presenter,
        override_output: options.audience,
    };
    let result = el.run_app(&mut app);
    if let Some(output) = &mut app.output {
        output.shutdown();
    }
    result?;
    if let Some(e) = app.error {
        return Err(e.into());
    }
    Ok(())
}
pub fn run_output() -> Result<(), Failure> {
    let (tx, commands) = mpsc::sync_channel(4);
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        while let Ok(state) = worker::read_json::<HostState>(&mut stdin) {
            if tx.send(state).is_err() {
                break;
            }
        }
    });
    let (acks, rx) = mpsc::sync_channel(8);
    std::thread::spawn(move || {
        let mut stdout = std::io::stdout().lock();
        while let Ok(ack) = rx.recv() {
            if worker::write_json(&mut stdout, &ack).is_err() {
                break;
            }
        }
    });
    let el = EventLoop::new()?;
    let now = Instant::now();
    let mut app = App {
        studio: None,
        host: Some(Host {
            parent_connected: true,
            close_requested: false,
            commands,
            acks,
            state: Default::default(),
            reader: Bridge::new(Duration::from_secs(10)),
            reading: None,
            cache: HashMap::new(),
            frame: None,
            presented: HashSet::new(),
            failed_reads: HashSet::new(),
            preload_cursor: 0,
        }),
        output: None,
        lease: Some(SleepLease::new()),
        windows: vec![],
        handles: vec![],
        next_poll: now,
        next_clock: now,
        next_monitors: now,
        x11: false,
        error: None,
        smoke: false,
        smoke_stage: 0,
        started: now,
        windowed: false,
        override_presenter: None,
        override_output: None,
    };
    el.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod buffer_tests {
    use super::*;
    fn frame(page: usize) -> Arc<Frame> {
        Arc::new(Frame {
            info: crate::document::FrameInfo {
                page,
                width: 4,
                height: 4,
                path: format!("{page}.rgba").into(),
                sha256: String::new(),
            },
            rgba: vec![0; 64],
        })
    }
    #[test]
    fn output_evicts_distant_frames_and_counts_pinned_frame_outside_cache() {
        let current = frame(1);
        let near = frame(2);
        let far = frame(20);
        let mut cache = [current.clone(), near.clone(), far.clone()]
            .into_iter()
            .map(|f| (f.info.path.clone(), f))
            .collect();
        let state = HostState {
            page: 2,
            frame: Some(near.info.clone()),
            ..Default::default()
        };
        assert!(make_output_room(
            &mut cache,
            Some(&current),
            &state,
            64,
            192
        ));
        assert!(!cache.contains_key(&far.info.path));
        assert!(cache.contains_key(&current.info.path));
        assert!(cache.contains_key(&near.info.path));
        cache.remove(&current.info.path);
        assert!(!make_output_room(
            &mut cache,
            Some(&current),
            &state,
            64,
            128
        ));
        assert!(make_output_room(
            &mut cache,
            Some(&current),
            &state,
            64,
            192
        ));
    }
}
