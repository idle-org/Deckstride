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
    path::PathBuf,
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
fn navigation_direction(key: &Key, modifiers: egui::Modifiers) -> Option<i32> {
    if modifiers.ctrl || modifiers.alt || modifiers.command {
        return None;
    }
    match key {
        Key::Named(NamedKey::Space) if modifiers.shift => Some(-1),
        Key::Named(NamedKey::ArrowRight | NamedKey::PageDown | NamedKey::Space) => Some(1),
        Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => Some(-1),
        _ => None,
    }
}
// Bound zero-delay animation requests; idle windows have no repaint deadline.
fn repaint_deadline(now: Instant, delay: Duration) -> Option<Instant> {
    (delay != Duration::MAX)
        .then(|| now.checked_add(delay.max(Duration::from_millis(16))))
        .flatten()
}
type Failure = Box<dyn std::error::Error>;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Presenter,
    Browser,
    Presentation,
}
const OUTPUT_BASE_CACHE_MAX_PIXELS: usize = 4096 * 4096;

#[cfg(target_os = "windows")]
const PRESENTER_FOCUS_RETRY_DELAYS: [Duration; 5] = [
    Duration::from_millis(80),
    Duration::from_millis(150),
    Duration::from_millis(250),
    Duration::from_millis(400),
    Duration::from_millis(650),
];

#[cfg(target_os = "windows")]
struct PresenterFocusRetry {
    window: WindowId,
    next: Instant,
    attempt: usize,
}

#[cfg(target_os = "windows")]
impl PresenterFocusRetry {
    fn new(window: WindowId, now: Instant) -> Self {
        Self {
            window,
            next: now + PRESENTER_FOCUS_RETRY_DELAYS[0],
            attempt: 0,
        }
    }

    fn take_attempt(&mut self, now: Instant) -> bool {
        if now < self.next {
            return false;
        }
        self.attempt += 1;
        if let Some(delay) = PRESENTER_FOCUS_RETRY_DELAYS.get(self.attempt) {
            self.next = now + *delay;
        }
        true
    }

    fn finished(&self) -> bool {
        self.attempt >= PRESENTER_FOCUS_RETRY_DELAYS.len()
    }
}

#[cfg(target_os = "windows")]
fn activate_presenter(window: &Window) {
    use std::ffi::c_void;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    type Hwnd = *mut c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetForegroundWindow() -> Hwnd;
        fn GetWindowThreadProcessId(window: Hwnd, process: *mut u32) -> u32;
        fn GetCurrentThreadId() -> u32;
        fn AttachThreadInput(thread: u32, foreground: u32, attach: i32) -> i32;
        fn BringWindowToTop(window: Hwnd) -> i32;
        fn SetForegroundWindow(window: Hwnd) -> i32;
        fn SetActiveWindow(window: Hwnd) -> Hwnd;
        fn SetFocus(window: Hwnd) -> Hwnd;
    }

    window.set_minimized(false);
    window.focus_window();
    let Some(hwnd) = window
        .window_handle()
        .ok()
        .and_then(|handle| match handle.as_raw() {
            RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as Hwnd),
            _ => None,
        })
    else {
        return;
    };
    // SetForegroundWindow is timing-sensitive while fullscreen windows are mapping.
    // Temporarily sharing the foreground input queue lets Windows apply activation
    // to this already-visible Presenter. Calls are bounded to startup retries.
    unsafe {
        let foreground = GetForegroundWindow();
        let current_thread = GetCurrentThreadId();
        let foreground_thread = if foreground.is_null() {
            0
        } else {
            GetWindowThreadProcessId(foreground, std::ptr::null_mut())
        };
        let attached = foreground_thread != 0
            && foreground_thread != current_thread
            && AttachThreadInput(current_thread, foreground_thread, 1) != 0;
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        SetActiveWindow(hwnd);
        SetFocus(hwnd);
        if attached {
            AttachThreadInput(current_thread, foreground_thread, 0);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PresentationBaseKey {
    size: [u32; 2],
    pixels_per_point: f32,
    frame: Option<(PathBuf, String, u32, u32)>,
    blank: bool,
    colors: crate::settings::SlideColors,
    zoom: crate::zoom::Sync,
}

struct PresentationBase {
    key: PresentationBaseKey,
    pixels: Vec<u32>,
}

impl PresentationBase {
    fn restore(&self, key: &PresentationBaseKey, pixels: &mut [u32]) -> bool {
        if self.key != *key || self.pixels.len() != pixels.len() {
            return false;
        }
        pixels.copy_from_slice(&self.pixels);
        true
    }
}

struct NativeWindow {
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    window: Rc<Window>,
    ctx: egui::Context,
    overlay_ctx: egui::Context,
    input: egui_winit::State,
    painter: EguiSoftwareRender,
    ui: UiState,
    role: Role,
    monitor: String,
    target: Option<(String, bool)>,
    transition: Option<(Instant, bool)>,
    frames: usize,
    repaint_at: Option<Instant>,
    display_action: Option<DisplayAction>,
    zoom_change: Option<crate::zoom::View>,
    zoom_watch: crate::zoom::PerformanceWatch,
    zoom_slow: bool,
    presentation_base: Option<PresentationBase>,
    #[cfg(target_os = "linux")]
    browser_parent: Option<WindowId>,
}
impl NativeWindow {
    fn create(el: &ActiveEventLoop, role: Role, monitor: String) -> Result<Self, Failure> {
        let title = match role {
            Role::Presenter => "PDF Presenter — Presenter",
            Role::Browser => "PDF Presenter — All slides",
            Role::Presentation => "PDF Presenter — Slides",
        };
        let attributes = crate::artwork::window_attributes()
            .with_title(title)
            .with_window_level(if role == Role::Browser {
                winit::window::WindowLevel::AlwaysOnTop
            } else {
                winit::window::WindowLevel::Normal
            })
            .with_visible(false)
            .with_active(role != Role::Presentation)
            .with_inner_size(PhysicalSize::new(1100, 750));
        #[cfg(target_os = "linux")]
        let attributes = if role == Role::Browser {
            use winit::platform::x11::{WindowAttributesExtX11, WindowType};
            attributes.with_x11_window_type(vec![WindowType::Dialog])
        } else {
            attributes
        };
        let window = Rc::new(el.create_window(attributes)?);
        let surface =
            softbuffer::Surface::new(&softbuffer::Context::new(window.clone())?, window.clone())?;
        let ctx = egui::Context::default();
        let overlay_ctx = egui::Context::default();
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
            overlay_ctx,
            input,
            painter: EguiSoftwareRender::new(ColorFieldOrder::Bgra),
            ui: UiState::default(),
            role,
            monitor,
            target: None,
            transition: None,
            frames: 0,
            repaint_at: None,
            display_action: None,
            zoom_change: None,
            zoom_watch: Default::default(),
            zoom_slow: false,
            presentation_base: None,
            #[cfg(target_os = "linux")]
            browser_parent: None,
        })
    }
    fn repaint_if_due(&mut self, now: Instant) {
        if self.repaint_at.is_some_and(|deadline| now >= deadline) {
            self.repaint_at = None;
            self.window.request_redraw();
        }
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
        if self.role != Role::Presentation {
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
        let render_started = Instant::now();
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
        let zoom_change = &mut self.zoom_change;
        let mut presentation_view = setup.map(|s| s.zoom.view).unwrap_or_default();
        let mut overlay_rects = None;
        let output = ctx.run_ui(input, |ui| match role {
            Role::Presenter => presenter_ui::presenter_on_display(
                ui,
                studio.as_deref_mut().unwrap(),
                ui_state,
                &self.monitor,
            ),
            Role::Browser => presenter_ui::browser(ui, studio.as_deref_mut().unwrap(), ui_state),
            Role::Presentation => {
                ui.painter()
                    .rect_filled(ui.max_rect(), 0., egui::Color32::BLACK);
                if let Some(setup) = setup {
                    *display_action = presenter_ui::output_display_recovery(
                        ui,
                        setup.setup_controls && !setup.zoom.enabled,
                        setup.route_trial.is_some(),
                    );
                }
                if let Some(frame) = frame {
                    let setup = if blank { None } else { setup };
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
                    let mut view = presentation_view;
                    if let Some(setup) = setup
                        && setup.zoom.enabled
                        && setup.route_trial.is_none()
                        && setup
                            .frame
                            .as_ref()
                            .is_some_and(|f| f.path == frame.info.path)
                    {
                        let (next, response) = presenter_ui::zoom_gestures_in(
                            ui,
                            ui.max_rect(),
                            [frame.info.width, frame.info.height],
                            view,
                            setup.zoom.keep_fitted_area,
                        );
                        if setup.setup_controls {
                            response.context_menu(|ui| {
                                if ui.button("Swap Presenter and Slides screens…").clicked() {
                                    *display_action = Some(DisplayAction::Swap);
                                    ui.close();
                                }
                            });
                        }
                        if next != view {
                            *zoom_change = Some(next);
                            view = next;
                        }
                    }
                    presentation_view = view;
                    presenter_ui::paint_slide_adapted(
                        ui.painter(),
                        ui.max_rect(),
                        tex.id(),
                        [frame.info.width, frame.info.height],
                        setup
                            .map(|s| s.slide_colors)
                            .unwrap_or(crate::settings::SlideColors {
                                page_background: [0; 3],
                                fit_padding: [0; 3],
                            }),
                        crate::zoom::Sync {
                            view,
                            ..setup.map(|s| s.zoom).unwrap_or_default()
                        },
                    );
                    if let Some(setup) = setup {
                        let (target, uv) = presenter_ui::zoom_geometry(
                            ui.max_rect(),
                            [frame.info.width, frame.info.height],
                            view,
                            setup.zoom.keep_fitted_area,
                        );
                        overlay_rects = Some((ui.max_rect(), target, uv));
                    }
                }
            }
        });
        self.repaint_at = repaint_deadline(
            Instant::now(),
            output
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .map_or(Duration::MAX, |viewport| viewport.repaint_delay),
        );
        self.input
            .handle_platform_output(&self.window, output.platform_output);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        self.surface.resize(
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        )?;
        let mut buffer = self.surface.buffer_mut()?;
        let pixel_count = size.width as usize * size.height as usize;
        let cache_key = (role == Role::Presentation
            && pixel_count <= OUTPUT_BASE_CACHE_MAX_PIXELS
            && setup.is_some_and(|s| !s.setup_controls && s.route_trial.is_none()))
        .then(|| PresentationBaseKey {
            size: [size.width, size.height],
            pixels_per_point: output.pixels_per_point,
            frame: frame.map(|frame| {
                (
                    frame.info.path.clone(),
                    frame.info.sha256.clone(),
                    frame.info.width,
                    frame.info.height,
                )
            }),
            blank,
            colors: setup.map(|s| s.slide_colors).unwrap_or_default(),
            zoom: crate::zoom::Sync {
                view: presentation_view,
                ..setup.map(|s| s.zoom).unwrap_or_default()
            },
        });
        let restored = cache_key.as_ref().is_some_and(|key| {
            self.presentation_base
                .as_ref()
                .is_some_and(|cached| cached.restore(key, &mut buffer))
        });
        if !restored {
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
            self.presentation_base = cache_key.map(|key| PresentationBase {
                key,
                pixels: buffer.to_vec(),
            });
        } else if !output.textures_delta.is_empty() {
            self.painter.render(
                &mut BufferMutRef::new(
                    bytemuck::cast_slice_mut(&mut buffer),
                    size.width as usize,
                    size.height as usize,
                ),
                &[],
                &output.textures_delta,
                output.pixels_per_point,
            );
        }
        if let (Some(setup), Some((screen, target, uv))) = (setup, overlay_rects)
            && (setup.overlay.pointer.is_some() || !setup.overlay.marks.is_empty())
        {
            let overlay = self.overlay_ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| crate::tools::paint(ui.painter(), target, uv, &setup.overlay),
            );
            let overlay_primitives = self
                .overlay_ctx
                .tessellate(overlay.shapes, output.pixels_per_point);
            self.painter.render(
                &mut BufferMutRef::new(
                    bytemuck::cast_slice_mut(&mut buffer),
                    size.width as usize,
                    size.height as usize,
                ),
                &overlay_primitives,
                &overlay.textures_delta,
                output.pixels_per_point,
            );
        }
        self.window.pre_present_notify();
        buffer.present()?;
        self.frames += 1;
        let zoomed = if role == Role::Presenter {
            studio
                .as_ref()
                .is_some_and(|s| s.zoom.enabled && s.zoom.view.scale > 1.)
        } else {
            !blank && setup.is_some_and(|s| s.zoom.enabled && s.zoom.view.scale > 1.)
        };
        if self.zoom_watch.observe(zoomed, render_started.elapsed()) {
            self.zoom_slow = true;
            if let Some(s) = studio {
                s.zoom_warning = true;
            }
        }
        Ok(())
    }
}
/// Associate a separate, resizable browser with its presenter before first mapping.
/// No modal/child-window style: the operator can still use both windows.
#[cfg(target_os = "linux")]
fn linux_browser_parent(browser: &Window, parent: &Window) -> Result<(), Failure> {
    use winit::raw_window_handle::{
        HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
    };
    match browser.display_handle()?.as_raw() {
        RawDisplayHandle::Wayland(display) => {
            use wayland_client::{
                Connection, Proxy,
                backend::{Backend, ObjectId},
            };
            use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
            use winit::platform::wayland::WindowExtWayland;
            let child = browser
                .xdg_toplevel()
                .ok_or("Missing browser Wayland toplevel")?;
            let owner = parent
                .xdg_toplevel()
                .ok_or("Missing presenter Wayland toplevel")?;
            // SAFETY: both live Winit windows belong to this event loop/display. These
            // temporary wrappers borrow its handles; they never destroy proxies, dispatch
            // events, disconnect the display, or outlive either Window. Winit flushes the
            // queued request with its next surface commit; there is no roundtrip here.
            unsafe {
                let connection = Connection::from_backend(Backend::from_foreign_display(
                    display.display.as_ptr().cast(),
                ));
                let child = XdgToplevel::from_id(
                    &connection,
                    ObjectId::from_ptr(XdgToplevel::interface(), child.as_ptr().cast())?,
                )?;
                let owner = XdgToplevel::from_id(
                    &connection,
                    ObjectId::from_ptr(XdgToplevel::interface(), owner.as_ptr().cast())?,
                )?;
                child.set_parent(Some(&owner));
            }
        }
        RawDisplayHandle::Xlib(display) => {
            let (RawWindowHandle::Xlib(child), RawWindowHandle::Xlib(owner)) = (
                browser.window_handle()?.as_raw(),
                parent.window_handle()?.as_raw(),
            ) else {
                return Err("Mismatched browser/presenter window backends".into());
            };
            let display = display.display.ok_or("Missing X11 display")?;
            let xlib = x11_dl::xlib::Xlib::open()?;
            // SAFETY: borrowed Xlib connection and live window IDs from this event loop.
            // This sets a WM hint only; it neither reparents nor destroys either window.
            unsafe {
                (xlib.XSetTransientForHint)(display.as_ptr().cast(), child.window, owner.window);
            }
        }
        _ => return Err("Unsupported Linux window backend".into()),
    }
    Ok(())
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
    blank: crate::blank::Output,
    reader: Bridge,
    reading: Option<std::path::PathBuf>,
    cache: HashMap<std::path::PathBuf, Arc<Frame>>,
    frame: Option<Arc<Frame>>,
    presented: HashSet<WindowId>,
    failed_reads: HashSet<std::path::PathBuf>,
    preload_cursor: usize,
    zoom_pending: bool,
    zoom_slow: bool,
    next_zoom_ack: Instant,
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
    navigation_hold: Option<(WindowId, Key, crate::navigation::Hold)>,
    #[cfg(target_os = "windows")]
    presenter_focus: Option<PresenterFocusRetry>,
}
impl App {
    fn redraw(&self) {
        for native in &self.windows {
            native.window.request_redraw();
        }
    }
    #[cfg(target_os = "windows")]
    fn retry_presenter_focus(&mut self, now: Instant) {
        let Some(retry) = &mut self.presenter_focus else {
            return;
        };
        if self
            .windows
            .iter()
            .any(|window| window.role == Role::Browser && window.window.has_focus())
        {
            self.presenter_focus = None;
            return;
        }
        if !retry.take_attempt(now) {
            return;
        }
        if !self
            .windows
            .iter()
            .any(|window| window.role == Role::Presenter && window.window.has_focus())
            && let Some(window) = self
                .windows
                .iter()
                .find(|window| window.window.id() == retry.window)
        {
            activate_presenter(&window.window);
        }
        if retry.finished() {
            self.presenter_focus = None;
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
            self.error = Some("Choose distinct, connected Presenter and Slides screens".into());
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
                    self.windowed = false;
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
                            self.error =
                                Some("Slides output requires a separate Presenter screen".into());
                            el.exit();
                            return;
                        }
                    }
                }
                s.draft_routes = s.routes.clone();
            }
        }
        if changed {
            if let Some(host) = &mut self.host {
                host.presented.clear();
            }
            for w in &mut self.windows {
                // The OS may move a surviving window when another display is
                // removed. Its route ID/fullscreen flag can remain unchanged,
                // so discard placement cached for the old desktop topology.
                w.target = None;
                w.transition = None;
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
                #[cfg(target_os = "windows")]
                let first_presenter = role == Role::Presenter
                    && !self
                        .windows
                        .iter()
                        .any(|window| window.role == Role::Presenter);
                self.windows
                    .push(NativeWindow::create(el, role, id.clone())?);
                #[cfg(target_os = "windows")]
                if first_presenter {
                    self.presenter_focus = Some(PresenterFocusRetry::new(
                        self.windows.last().unwrap().window.id(),
                        Instant::now(),
                    ));
                }
                self.windows.len() - 1
            };
            #[cfg(target_os = "linux")]
            if role == Role::Browser {
                // Prefer the Presenter on the chosen monitor, never a Slides window.
                let parent = self
                    .windows
                    .iter()
                    .filter(|w| w.role == Role::Presenter)
                    .min_by_key(|w| w.monitor != id)
                    .map(|w| w.window.clone());
                if let Some(parent) = parent {
                    let browser = &mut self.windows[index];
                    if browser.browser_parent != Some(parent.id()) {
                        if let Err(error) = linux_browser_parent(&browser.window, &parent)
                            && let Some(s) = &mut self.studio
                        {
                            s.status = format!(
                                "Could not associate All slides with the presenter: {error}"
                            );
                        }
                        browser.browser_parent = Some(parent.id());
                    }
                }
            }
            self.windows[index].place(handle, &id, full, self.x11);
        }
        if let Some(s) = &mut self.studio {
            s.dirty_routes = false;
        }
        Ok(())
    }
    fn key(&mut self, role: Role, key: Key, consumed: bool, modifiers: egui::Modifiers) {
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
            if matches!(key, Key::Named(NamedKey::Escape)) {
                // Focus may move here as All slides closes. Escape must never blank
                // a connected show; an orphan can still be dismissed safely.
                if let Some(h) = &mut self.host {
                    h.close_requested = !h.parent_connected;
                }
            } else if matches!(&key, Key::Character(c) if c.eq_ignore_ascii_case("b")) {
                if let Some(h) = &mut self.host {
                    h.state.blanked = true;
                    let _ = h.acks.try_send(Ack {
                        request_blank: true,
                        status: "Slides blanked from output keyboard.".into(),
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
        // Notes editing must never turn typing or Escape into presentation commands.
        // Unsaved text is only discarded through the explicit editor control.
        if s.notes_editor.is_some() || s.notes_import.is_some() {
            return;
        }
        if modifiers.ctrl || modifiers.alt || modifiers.command {
            return;
        }
        // Keep the live show responsive while an operator is using All slides or
        // has a presenter menu open. Text editors and modified shortcuts above
        // retain priority; focused controls may still consume all other keys.
        if let Some(delta) = navigation_direction(&key, modifiers) {
            s.execute(Action::Navigate {
                delta,
                repeated: false,
            });
            self.redraw();
            return;
        }
        if role == Role::Browser && matches!(key, Key::Named(NamedKey::Escape)) {
            if !s.jump.is_empty() {
                s.jump.clear();
            } else if s.browser_preview.take().is_none() {
                s.execute(Action::CloseBrowser);
            }
            return;
        }
        if consumed {
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
                } else if s.layout_open || s.displays_open || s.menu_open || s.toolbox.open {
                    s.layout_open = false;
                    s.displays_open = false;
                    s.menu_open = false;
                    s.toolbox.open = false;
                }
            }
            Key::Character(c) if c.eq_ignore_ascii_case("p") => {
                let tool = if s.toolbox.tool == crate::tools::Tool::Pointer {
                    crate::tools::Tool::Off
                } else {
                    crate::tools::Tool::Pointer
                };
                if s.toolbox.select(tool) {
                    s.new_overlay = true;
                }
                s.toolbox.open = true;
            }
            Key::Character(c) if c.eq_ignore_ascii_case("a") => {
                if s.toolbox.select(crate::tools::Tool::Arrow) {
                    s.new_overlay = true;
                }
                s.toolbox.open = true;
            }
            Key::Character(c) if c.eq_ignore_ascii_case("h") => {
                if s.toolbox.select(crate::tools::Tool::Highlighter) {
                    s.new_overlay = true;
                }
                s.toolbox.open = true;
            }
            Key::Character(c) if c.eq_ignore_ascii_case("b") => s.execute(Action::Blank),
            _ => {}
        }
        self.redraw();
    }
    fn stop_navigation(&mut self) {
        self.navigation_hold = None;
        if let Some(s) = &mut self.studio {
            s.execute(Action::FinishNavigation);
        }
    }
    fn repeat_navigation(&mut self, now: Instant) {
        let Some((id, _, _)) = &self.navigation_hold else {
            return;
        };
        let valid = self
            .windows
            .iter()
            .any(|w| w.window.id() == *id && w.window.has_focus())
            && self.studio.as_ref().is_some_and(|s| {
                s.notes_editor.is_none()
                    && s.notes_import.is_none()
                    && !s.dialog_busy
                    && !s.loading
                    && s.route_trial.is_none()
            });
        if !valid {
            self.stop_navigation();
            return;
        }
        let s = self.studio.as_mut().unwrap();
        if let Some(delta) = self
            .navigation_hold
            .as_mut()
            .unwrap()
            .2
            .tick(now, s.count())
        {
            s.execute(Action::Navigate {
                delta,
                repeated: true,
            });
            self.redraw();
        }
    }
    fn poll(&mut self, el: &ActiveEventLoop) {
        let mut repaint = false;
        if let Some(s) = &mut self.studio {
            repaint |= s.new_zoom || s.new_overlay || std::mem::take(&mut s.presenter_views_dirty);
            let before = (
                s.generation,
                s.frames.len(),
                s.requested,
                s.blanked,
                s.status.clone(),
                s.layout_open,
                s.browser_open,
                s.route_trial.as_ref().map(|trial| trial.id),
            );
            let chooser_was_open = s.dialog_busy;
            s.poll();
            let actions = std::mem::take(&mut s.actions);
            repaint |= !actions.is_empty();
            for action in actions {
                #[cfg(target_os = "windows")]
                if matches!(
                    action,
                    Action::Pick(_) | Action::PickNotes | Action::PickBlankPicture
                ) && !s.dialog_busy
                {
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
                    if let Some(viewport) = ack.viewport {
                        let before = s.presentation_viewport();
                        s.output_viewport = Some(viewport);
                        repaint |= before != s.presentation_viewport();
                    }
                    if let Some(zoom) = ack.zoom {
                        if ack.generation == s.buffer_generation() && ack.page == Some(s.requested)
                        {
                            if s.zoom.accept_output(zoom) {
                                s.presentation_action();
                                repaint = true;
                            }
                            if zoom.epoch == s.zoom.epoch {
                                s.zoom_warning |= ack.zoom_slow;
                                s.new_zoom = true; // Echo newest serial so output can stop retrying.
                                repaint |= ack.zoom_slow;
                            }
                        }
                        continue;
                    }
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
                        s.new_output = true;
                    }
                    if ack.request_blank {
                        s.presentation_action();
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
                        zoom: s.zoom,
                        overlay: s.toolbox.overlay.clone(),
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
                        blank_picture: s.blank_picture.as_ref().map(|f| f.info.clone()),
                        ..Default::default()
                    };
                    if output.tx.try_send(state).is_ok() {
                        s.new_output = false;
                        s.new_zoom = false;
                        s.new_overlay = false;
                    } else {
                        s.output_sequence -= 1;
                    }
                }
            }
            if (s.new_zoom || s.new_overlay)
                && !s.new_output
                && let Some(output) = &self.output
            {
                let state = HostState {
                    zoom_only: s.new_zoom,
                    zoom: s.zoom,
                    overlay_only: s.new_overlay,
                    overlay: s.toolbox.overlay.clone(),
                    generation: s.buffer_generation(),
                    page: s.requested,
                    ..Default::default()
                };
                if output.tx.try_send(state).is_ok() {
                    s.new_zoom = false;
                    s.new_overlay = false;
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
                    s.route_trial.as_ref().map(|trial| trial.id),
                );
        }
        if let Some(host) = &mut self.host {
            loop {
                let mut state = match host.commands.try_recv() {
                    Ok(state) => state,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        host.parent_connected = false;
                        host.state.route_trial = None;
                        host.state.setup_controls = false;
                        break;
                    }
                };
                if state.zoom_only || state.overlay_only {
                    if state.generation == host.state.generation && state.page == host.state.page {
                        if state.zoom_only {
                            let before = host.state.zoom;
                            host.state.zoom.accept_presenter(state.zoom);
                            host.zoom_pending = host.state.zoom != state.zoom.normalized();
                            repaint |= before != host.state.zoom;
                        }
                        if state.overlay_only {
                            let overlay = state.overlay.normalized();
                            repaint |= host.state.overlay != overlay;
                            host.state.overlay = overlay;
                        }
                    }
                    continue;
                }
                host.state.zoom.accept_presenter(state.zoom);
                host.zoom_pending = host.state.zoom != state.zoom.normalized();
                state.zoom = host.state.zoom;
                state.overlay = state.overlay.normalized();
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
                if host.blank.set(state.blank_picture.as_ref()) {
                    for w in &mut self.windows {
                        w.ui.textures.clear();
                    }
                }
                host.state = state;
            }
            if let Some(result) = host.blank.poll() {
                repaint = true;
                if let Err(error) = result {
                    let _ = host.acks.try_send(Ack {
                        status: format!("Blank picture: {error}"),
                        ..Default::default()
                    });
                }
            }
            if host.zoom_pending && host.parent_connected && Instant::now() >= host.next_zoom_ack {
                let _ = host.acks.try_send(Ack {
                    zoom: Some(host.state.zoom),
                    zoom_slow: host.zoom_slow,
                    generation: host.state.generation,
                    page: Some(host.state.page),
                    ..Default::default()
                });
                host.next_zoom_ack = Instant::now() + Duration::from_millis(33);
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
                        let limit = host.state.buffer_mib.max(64) as usize * 1024 * 1024;
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
                                status: "Slides RAM cannot fit active frames; increase the budget."
                                    .into(),
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
        if let Err(e) = self.sync_windows(el) {
            self.error = Some(e.to_string());
            el.exit();
        }
        // Include newly created mirrors in the same state update.
        if repaint {
            self.redraw();
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
        if matches!(&event, WindowEvent::KeyboardInput { event, is_synthetic: true, .. } if event.state == ElementState::Pressed)
        {
            return;
        }
        // Navigation belongs to the bounded native hold controller, not also to
        // egui's focused button activation or the OS repeat stream.
        let navigation_event = role != Role::Presentation
            && self.studio.as_ref().is_some_and(|s| {
                s.notes_editor.is_none()
                    && s.notes_import.is_none()
                    && !s.dialog_busy
                    && s.route_trial.is_none()
            })
            && matches!(&event, WindowEvent::KeyboardInput { event, .. } if navigation_direction(&event.logical_key, self.windows[index].input.egui_input().modifiers).is_some());
        let repaint = if navigation_event {
            false
        } else {
            let n = &mut self.windows[index];
            n.input.on_window_event(&n.window, &event).repaint
        };
        // egui-winit reports repaint=true for RedrawRequested itself. The
        // current render satisfies that request: requeueing it creates an idle
        // paint loop and can starve later Windows WM_PAINT messages.
        if repaint && !matches!(event, WindowEvent::RedrawRequested) {
            self.windows[index].window.request_redraw();
        }
        match event {
            WindowEvent::CloseRequested => {
                if role == Role::Presenter {
                    if let Some(s) = &mut self.studio {
                        s.execute(Action::Exit);
                        if s.close_requested {
                            el.exit();
                        } else {
                            self.redraw();
                        }
                    }
                } else if role == Role::Browser {
                    if let Some(s) = &mut self.studio {
                        s.execute(Action::CloseBrowser);
                    }
                } else if let Some(h) = &mut self.host {
                    h.close_requested = true;
                    h.state.blanked = true;
                    let _ = h.acks.try_send(Ack {
                        request_blank: true,
                        status: "Slides blanked from output window.".into(),
                        ..Default::default()
                    });
                    self.redraw();
                }
            }
            WindowEvent::DroppedFile(path) => {
                if role == Role::Presenter
                    && let Some(s) = &mut self.studio
                {
                    s.execute(if crate::notes::is_import(&path) {
                        Action::ImportNotes(path)
                    } else {
                        Action::Open(path)
                    });
                }
            }
            WindowEvent::Focused(false) => {
                self.stop_navigation();
                self.windows[index].ui.navigation_hold = None;
            }
            WindowEvent::ModifiersChanged(_) => self.stop_navigation(),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if event.state == ElementState::Released {
                    if self
                        .navigation_hold
                        .as_ref()
                        .is_some_and(|(owner, key, _)| *owner == id && *key == event.logical_key)
                    {
                        self.stop_navigation();
                    }
                } else if !event.repeat && !is_synthetic {
                    self.stop_navigation();
                    let native = &self.windows[index];
                    let modifiers = native.input.egui_input().modifiers;
                    let consumed = native.ctx.egui_wants_keyboard_input();
                    let direction = navigation_direction(&event.logical_key, modifiers);
                    let allowed = role != Role::Presentation
                        && self.studio.as_ref().is_some_and(|s| {
                            s.notes_editor.is_none()
                                && s.notes_import.is_none()
                                && !s.dialog_busy
                                && s.route_trial.is_none()
                        });
                    if allowed && let Some(direction) = direction {
                        self.navigation_hold = Some((
                            id,
                            event.logical_key.clone(),
                            crate::navigation::Hold::new(direction, Instant::now()),
                        ));
                    }
                    let local_dismissed = matches!(event.logical_key, Key::Named(NamedKey::Escape))
                        && !modifiers.ctrl
                        && !modifiers.alt
                        && !modifiers.command
                        && role == Role::Presenter
                        && self.studio.as_mut().is_some_and(|s| {
                            if s.route_trial.is_some()
                                || s.notes_editor.is_some()
                                || s.notes_import.is_some()
                                || !s.jump.is_empty()
                            {
                                return false;
                            }
                            let Some(view) =
                                s.presenter_views.get_mut(&self.windows[index].monitor)
                            else {
                                return false;
                            };
                            let open = view.layout_open || view.menu_open;
                            view.layout_open = false;
                            view.menu_open = false;
                            open
                        });
                    if local_dismissed {
                        self.redraw();
                    } else {
                        self.key(role, event.logical_key, consumed, modifiers);
                    }
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.windows[index].window.request_redraw()
            }
            WindowEvent::RedrawRequested => {
                let frame = self.host.as_ref().and_then(|h| {
                    if h.state.blanked {
                        h.blank.picture.clone()
                    } else {
                        h.frame.clone()
                    }
                });
                let blank = self.host.as_ref().is_none_or(|h| h.state.blanked);
                let result = self.windows[index].render(
                    self.studio.as_mut(),
                    frame.as_ref(),
                    blank,
                    self.host.as_ref().map(|h| &h.state),
                );
                if let Some(host) = &mut self.host {
                    if let Some(view) = self.windows[index].zoom_change.take() {
                        host.state.zoom.gesture(view);
                        host.zoom_pending = true;
                        self.windows.iter().for_each(|w| w.window.request_redraw());
                    }
                    if self.windows[index].zoom_slow && !host.zoom_slow {
                        host.zoom_slow = true;
                        host.zoom_pending = true;
                    }
                }
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
            self.repeat_navigation(now);
            // egui repaint timers alone do not wake this custom native host.
            // Redraw held mouse controls at their repeat deadline, not at frame rate.
            for window in &self.windows {
                if window.ui.navigation_hold.is_some_and(|hold| hold.due(now)) {
                    window.window.request_redraw();
                }
            }
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
                let viewport = host.state.monitors.iter().find_map(|id| {
                    self.windows
                        .iter()
                        .find(|w| w.role == Role::Presentation && &w.monitor == id)
                        .map(|w| {
                            let size = w.window.inner_size();
                            crate::output::OutputViewport {
                                monitor: id.clone(),
                                size: [size.width, size.height],
                                fullscreen: host.state.fullscreen,
                                pixels_per_point: w.window.scale_factor() as f32,
                            }
                        })
                });
                let _ = host.acks.try_send(Ack {
                    viewport,
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
            window.repaint_if_due(now);
            window.transition(&self.handles, now);
        }
        #[cfg(target_os = "windows")]
        self.retry_presenter_focus(now);
        if self.smoke
            && self.started.elapsed() > Duration::from_millis((self.smoke_stage as u64 + 1) * 300)
            && let Some(s) = &mut self.studio
        {
            match self.smoke_stage {
                0 => {
                    for view in s.presenter_views.values_mut() {
                        view.layout_open = true;
                    }
                }
                1 => {
                    for view in s.presenter_views.values_mut() {
                        view.layout_open = false;
                    }
                    s.displays_open = true;
                }
                2 => {
                    s.displays_open = false;
                    s.execute(Action::OpenBrowser);
                }
                3 => s.execute(Action::CloseBrowser),
                4 => {
                    for view in s.presenter_views.values_mut() {
                        view.layout.header = false;
                        view.layout.footer = false;
                    }
                }
                5 => {
                    for view in s.presenter_views.values_mut() {
                        view.layout = Default::default();
                    }
                    // Escape dismisses transient UI; fullscreen is an explicit control.
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
        navigation_hold: None,
        #[cfg(target_os = "windows")]
        presenter_focus: None,
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
            blank: Default::default(),
            reader: Bridge::new(Duration::from_secs(10)),
            reading: None,
            cache: HashMap::new(),
            frame: None,
            presented: HashSet::new(),
            failed_reads: HashSet::new(),
            preload_cursor: 0,
            zoom_pending: false,
            zoom_slow: false,
            next_zoom_ack: now,
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
        navigation_hold: None,
        #[cfg(target_os = "windows")]
        presenter_focus: None,
    };
    el.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod buffer_tests {
    use super::*;
    #[test]
    fn presentation_base_restores_only_an_exact_frame() {
        let key = PresentationBaseKey {
            size: [2, 2],
            pixels_per_point: 1.,
            frame: Some(("slide.rgba".into(), "sha".into(), 2, 2)),
            blank: false,
            colors: Default::default(),
            zoom: Default::default(),
        };
        let cached = PresentationBase {
            key: key.clone(),
            pixels: vec![1, 2, 3, 4],
        };
        let mut pixels = vec![0; 4];
        assert!(cached.restore(&key, &mut pixels));
        assert_eq!(pixels, [1, 2, 3, 4]);
        let changed = PresentationBaseKey { blank: true, ..key };
        assert!(!cached.restore(&changed, &mut pixels));
        assert!(!cached.restore(&cached.key, &mut [0; 3]));
    }
    #[test]
    fn repaint_deadlines_bound_animation_and_leave_idle_windows_asleep() {
        let now = Instant::now();
        assert_eq!(repaint_deadline(now, Duration::MAX), None);
        assert_eq!(
            repaint_deadline(now, Duration::ZERO),
            Some(now + Duration::from_millis(16))
        );
        assert_eq!(
            repaint_deadline(now, Duration::from_secs(2)),
            Some(now + Duration::from_secs(2))
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn three_native_windows_render_without_clicks_and_reset_placement_on_topology_change() {
        if std::env::var_os("PDF_PRESENTER_GUI_TESTS").is_none() {
            return;
        }
        use winit::platform::x11::EventLoopBuilderExtX11;
        struct Harness {
            app: App,
            started: Instant,
            topology_checked: bool,
        }
        impl ApplicationHandler for Harness {
            fn resumed(&mut self, el: &ActiveEventLoop) {
                self.app.handles = monitors(el).0;
                let id = self.app.handles[0].0.id.clone();
                for index in 0..3 {
                    let mut window =
                        NativeWindow::create(el, Role::Presentation, id.clone()).unwrap();
                    window.target = Some((id.clone(), false));
                    let _ = window
                        .window
                        .request_inner_size(PhysicalSize::new(180, 120));
                    window
                        .window
                        .set_outer_position(PhysicalPosition::new(20 + index * 200, 20));
                    window.window.set_visible(true);
                    window.window.request_redraw();
                    self.app.windows.push(window);
                }
            }
            fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
                self.app.window_event(el, id, event);
            }
            fn about_to_wait(&mut self, el: &ActiveEventLoop) {
                let now = Instant::now();
                for window in &mut self.app.windows {
                    window.repaint_if_due(now);
                }
                if !self.topology_checked
                    && now.duration_since(self.started) >= Duration::from_millis(200)
                {
                    // Simulate a previous desktop layout. Route IDs and fullscreen
                    // are unchanged, but all three placements must be invalidated.
                    self.app.handles[0].0.origin.0 += 1;
                    self.app.refresh(el);
                    for window in &self.app.windows {
                        assert!(window.target.is_none());
                        assert!(window.transition.is_none());
                    }
                    self.topology_checked = true;
                }
                if now.duration_since(self.started) >= Duration::from_secs(1) {
                    el.exit();
                }
                el.set_control_flow(ControlFlow::WaitUntil(now + Duration::from_millis(8)));
            }
        }
        let now = Instant::now();
        let mut harness = Harness {
            started: now,
            topology_checked: false,
            app: App {
                studio: None,
                host: None,
                output: None,
                lease: None,
                windows: vec![],
                handles: vec![],
                next_poll: now,
                next_clock: now,
                next_monitors: now,
                x11: true,
                error: None,
                smoke: false,
                smoke_stage: 0,
                started: now,
                windowed: true,
                override_presenter: None,
                override_output: None,
                navigation_hold: None,
                #[cfg(target_os = "windows")]
                presenter_focus: None,
            },
        };
        EventLoop::builder()
            .with_x11()
            .with_any_thread(true)
            .build()
            .unwrap()
            .run_app(&mut harness)
            .unwrap();
        assert!(harness.app.error.is_none(), "{:?}", harness.app.error);
        assert!(harness.topology_checked);
        assert_eq!(harness.app.windows.len(), 3);
        for window in &harness.app.windows {
            assert!(window.frames > 0, "Every mirror must paint without a click");
            assert!(
                window.frames < 30,
                "Idle window repainted {} times",
                window.frames
            );
        }
    }

    #[test]
    fn space_navigation_respects_shift_focus_roles_and_blanking() {
        let now = Instant::now();
        let mut studio = Studio::new(true, None);
        studio.deck = Some(crate::document::DeckInfo {
            path: "test.pdf".into(),
            key: "test".into(),
            title: "Test".into(),
            names: vec!["1".into(), "2".into(), "3".into()],
            notes: vec![],
            warnings: vec![],
            sha256: String::new(),
        });
        studio.blanked = true;
        let mut app = App {
            studio: Some(studio),
            host: None,
            output: None,
            lease: None,
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
            windowed: true,
            override_presenter: None,
            override_output: None,
            navigation_hold: None,
            #[cfg(target_os = "windows")]
            presenter_focus: None,
        };
        for (role, consumed, modifiers, expected) in [
            (Role::Presenter, false, egui::Modifiers::NONE, 1),
            (Role::Presenter, false, egui::Modifiers::SHIFT, 0),
            (Role::Browser, false, egui::Modifiers::NONE, 1),
            (Role::Browser, false, egui::Modifiers::SHIFT, 0),
            (Role::Presenter, true, egui::Modifiers::NONE, 1),
            (Role::Presenter, true, egui::Modifiers::SHIFT, 0),
            (
                Role::Presenter,
                false,
                egui::Modifiers::SHIFT | egui::Modifiers::CTRL,
                0,
            ),
            (
                Role::Presenter,
                false,
                egui::Modifiers::SHIFT | egui::Modifiers::ALT,
                0,
            ),
            (Role::Presentation, false, egui::Modifiers::NONE, 0),
            (Role::Presenter, false, egui::Modifiers::NONE, 1),
            (Role::Presenter, false, egui::Modifiers::NONE, 2),
        ] {
            app.key(role, Key::Named(NamedKey::Space), consumed, modifiers);
            assert_eq!(app.studio.as_ref().unwrap().requested, expected);
            assert!(app.studio.as_ref().unwrap().blanked);
        }
        let s = app.studio.as_mut().unwrap();
        s.settings.presenter_fullscreen = true;
        s.settings.presentation_fullscreen = true;
        s.blanked = false;
        s.jump = "12".into();
        app.key(
            Role::Presenter,
            Key::Named(NamedKey::Escape),
            false,
            egui::Modifiers::NONE,
        );
        assert!(app.studio.as_ref().unwrap().jump.is_empty());
        app.key(
            Role::Presenter,
            Key::Named(NamedKey::Escape),
            false,
            egui::Modifiers::NONE,
        );
        let s = app.studio.as_ref().unwrap();
        assert!(s.presenter_fullscreen() && s.presentation_fullscreen());
        assert!(!s.blanked);
        for consumed in [false, true] {
            let s = app.studio.as_mut().unwrap();
            s.browser_open = true;
            s.browser_preview = Some((s.generation, 1));
            app.key(
                Role::Browser,
                Key::Named(NamedKey::Escape),
                consumed,
                egui::Modifiers::NONE,
            );
            let s = app.studio.as_ref().unwrap();
            assert!(s.browser_open && s.browser_preview.is_none() && !s.blanked);
            app.key(
                Role::Browser,
                Key::Named(NamedKey::Escape),
                consumed,
                egui::Modifiers::NONE,
            );
            let s = app.studio.as_ref().unwrap();
            assert!(!s.browser_open && !s.blanked && s.presentation_fullscreen());
        }
        let (_tx, commands) = mpsc::sync_channel(4);
        let (acks, rx) = mpsc::sync_channel(8);
        app.host = Some(Host {
            parent_connected: true,
            close_requested: false,
            commands,
            acks,
            state: HostState {
                blanked: false,
                ..Default::default()
            },
            blank: Default::default(),
            reader: Bridge::new(Duration::from_secs(10)),
            reading: None,
            cache: HashMap::new(),
            frame: None,
            presented: HashSet::new(),
            failed_reads: HashSet::new(),
            preload_cursor: 0,
            zoom_pending: false,
            zoom_slow: false,
            next_zoom_ack: now,
        });
        app.key(
            Role::Presentation,
            Key::Named(NamedKey::Escape),
            false,
            egui::Modifiers::NONE,
        );
        assert!(!app.host.as_ref().unwrap().state.blanked);
        assert!(!app.host.as_ref().unwrap().close_requested);
        assert!(
            rx.try_recv().is_err(),
            "Focus transfer must not request audience blanking"
        );
        app.key(
            Role::Presentation,
            Key::Character("b".into()),
            false,
            egui::Modifiers::NONE,
        );
        assert!(rx.try_recv().unwrap().request_blank);
        app.host.as_mut().unwrap().parent_connected = false;
        app.key(
            Role::Presentation,
            Key::Named(NamedKey::Escape),
            false,
            egui::Modifiers::NONE,
        );
        assert!(app.host.as_ref().unwrap().close_requested);
    }
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
