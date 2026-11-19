//! Native two-window proof. PDF workers and production process isolation come later.
use crate::{
    paint,
    poc::{Action, Display, Model, Options, window_rect},
};
use softbuffer::{Context, Surface};
use std::{
    error::Error,
    num::NonZeroU32,
    rc::Rc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    monitor::MonitorHandle,
    window::{Fullscreen, Window, WindowId},
};

type Failure = Box<dyn Error>;

// X11 window-manager changes are asynchronous. Keep only the newest destination
// and allow exit/restore, movement and fullscreen entry separate event-loop turns.
#[cfg(target_os = "linux")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Placement {
    monitor: usize,
    fullscreen: bool,
    rect: (i32, i32, u32, u32),
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct X11Transition {
    target: Option<Placement>,
    pending: Option<(Instant, bool)>, // false: move/resize; true: enter fullscreen
}

#[cfg(target_os = "linux")]
impl X11Transition {
    const SETTLE: Duration = Duration::from_millis(120);

    fn request(&mut self, target: Placement, now: Instant) -> bool {
        if self.target.as_ref() == Some(&target) {
            return false;
        }
        self.target = Some(target);
        self.pending = Some((now + Self::SETTLE, false));
        true
    }

    fn take_due(&mut self, now: Instant) -> Option<bool> {
        let (deadline, enter) = self.pending?;
        if now < deadline {
            return None;
        }
        self.pending = if !enter && self.target.as_ref()?.fullscreen {
            Some((now + Self::SETTLE, true))
        } else {
            None
        };
        Some(enter)
    }
}

struct NativeWindow {
    // Surface must be dropped before its underlying window.
    surface: Surface<Rc<Window>, Rc<Window>>,
    window: Rc<Window>,
    #[cfg(target_os = "linux")]
    transition: X11Transition,
}

impl NativeWindow {
    fn create(event_loop: &ActiveEventLoop, title: &str) -> Result<Self, Failure> {
        let window = Rc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title(title)
                    .with_visible(false)
                    .with_inner_size(PhysicalSize::new(960, 640)),
            )?,
        );
        let context = Context::new(window.clone())?;
        let surface = Surface::new(&context, window.clone())?;
        Ok(Self {
            surface,
            window,
            #[cfg(target_os = "linux")]
            transition: X11Transition::default(),
        })
    }
    fn redraw(&mut self, model: &Model, audience: bool, status: &str) -> Result<(), Failure> {
        let size = self.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return Ok(());
        };
        if u64::from(w.get()) * u64::from(h.get()) > 40_000_000 {
            return Err("Window exceeds the proof-of-concept 40-megapixel allocation limit".into());
        }
        self.surface.resize(w, h)?;
        let mut buffer = self.surface.buffer_mut()?;
        paint::draw(&mut buffer, w.get(), h.get(), model, audience, status);
        self.window.pre_present_notify();
        buffer.present()?;
        Ok(())
    }
}

struct App {
    options: Options,
    presenter: Option<NativeWindow>,
    audience: Option<NativeWindow>,
    handles: Vec<MonitorHandle>,
    model: Option<Model>,
    cursor: (f64, f64),
    status: String,
    error: Option<String>,
    started: Instant,
    next_check: Instant,
    smoke_step: usize,
    frames: [usize; 2],
    #[cfg(target_os = "linux")]
    x11: bool,
}

fn displays(event_loop: &ActiveEventLoop) -> (Vec<MonitorHandle>, Vec<Display>, usize) {
    let handles: Vec<_> = event_loop.available_monitors().collect();
    let primary = event_loop
        .primary_monitor()
        .and_then(|p| handles.iter().position(|h| h == &p))
        .unwrap_or(0);
    let descriptions = handles
        .iter()
        .map(|h| Display {
            name: h.name().unwrap_or_else(|| "Unnamed display".into()),
            origin: h.position().into(),
            size: h.size().into(),
        })
        .collect();
    (handles, descriptions, primary)
}

impl App {
    fn new(options: Options) -> Self {
        let now = Instant::now();
        Self {
            options,
            presenter: None,
            audience: None,
            handles: vec![],
            model: None,
            cursor: (0., 0.),
            status: String::new(),
            error: None,
            started: now,
            next_check: now,
            smoke_step: 0,
            frames: [0; 2],
            #[cfg(target_os = "linux")]
            x11: false,
        }
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl std::fmt::Display) {
        self.error = Some(error.to_string());
        event_loop.exit();
    }
    fn refresh(&mut self, event_loop: &ActiveEventLoop, initial: bool) -> Result<(), Failure> {
        let (handles, descriptions, primary) = displays(event_loop);
        for (i, d) in descriptions.iter().enumerate() {
            println!(
                "Display {}: {} {}x{} at {},{}{}",
                i + 1,
                d.name,
                d.size.0,
                d.size.1,
                d.origin.0,
                d.origin.1,
                if i == primary {
                    " [primary/default]"
                } else {
                    ""
                }
            );
        }
        if self.options.list {
            event_loop.exit();
            return Ok(());
        }
        let mut model = Model::new(
            descriptions,
            primary,
            if initial {
                self.options.presenter
            } else {
                None
            },
            if initial { self.options.audience } else { None },
            self.options.windowed,
        )?;
        if let Some(previous) = &self.model {
            model.page = previous.page;
            model.blanked = true;
        }
        self.handles = handles;
        self.model = Some(model);
        if self.presenter.is_none() {
            self.presenter = Some(NativeWindow::create(
                event_loop,
                "PDF Presenter - presenter controls",
            )?);
        }
        if self.audience.is_none() {
            self.audience = Some(NativeWindow::create(
                event_loop,
                "PDF Presenter - mock audience output",
            )?);
        }
        self.place();
        self.presenter.as_ref().unwrap().window.focus_window();
        Ok(())
    }
    fn place(&mut self) {
        let Some(model) = &self.model else {
            return;
        };
        let rehearsal = model.displays.len() == 1;
        for (is_audience, native, index, fullscreen) in [
            (
                false,
                &mut self.presenter,
                model.presenter,
                model.presenter_fullscreen,
            ),
            (true, &mut self.audience, model.audience, model.fullscreen),
        ] {
            let Some(native) = native else {
                continue;
            };
            let window = &native.window;
            if is_audience && model.disconnected {
                #[cfg(target_os = "linux")]
                {
                    native.transition = X11Transition::default();
                }
                window.set_visible(false);
                window.set_fullscreen(None);
                continue;
            }
            let (x, y, w, h) = window_rect(&model.displays[index], is_audience, rehearsal);
            #[cfg(target_os = "linux")]
            if self.x11 {
                let target = Placement {
                    monitor: index,
                    fullscreen,
                    rect: (x, y, w, h),
                };
                if native.transition.request(target, Instant::now()) {
                    window.set_fullscreen(None);
                    window.set_cursor_visible(true);
                    window.request_redraw();
                }
                continue;
            }
            if fullscreen {
                window.set_fullscreen(Some(Fullscreen::Borderless(Some(
                    self.handles[index].clone(),
                ))));
            } else {
                window.set_fullscreen(None);
                window.set_outer_position(PhysicalPosition::new(x, y));
                let _ = window.request_inner_size(PhysicalSize::new(w, h));
            }
            window.set_cursor_visible(!is_audience || !fullscreen);
            window.set_visible(true);
            window.request_redraw();
        }
        self.status = format!(
            "Requested presenter {} / audience {}. Verify placement.",
            model.presenter + 1,
            model.audience + 1
        );
        println!("{}", self.status);
    }
    #[cfg(target_os = "linux")]
    fn finish_x11_placement(&mut self, now: Instant) {
        for (is_audience, native) in [(false, &mut self.presenter), (true, &mut self.audience)] {
            let Some(native) = native else {
                continue;
            };
            let Some(enter) = native.transition.take_due(now) else {
                continue;
            };
            let target = native.transition.target.as_ref().unwrap();
            if enter {
                if let Some(monitor) = self.handles.get(target.monitor) {
                    native
                        .window
                        .set_fullscreen(Some(Fullscreen::Borderless(Some(monitor.clone()))));
                    native.window.set_cursor_visible(!is_audience);
                }
            } else {
                let (x, y, w, h) = target.rect;
                native
                    .window
                    .set_outer_position(PhysicalPosition::new(x, y));
                let _ = native.window.request_inner_size(PhysicalSize::new(w, h));
                native.window.set_visible(true);
            }
            native.window.request_redraw();
        }
    }

    fn action(&mut self, action: Action) {
        if let Some(model) = &mut self.model {
            if model.apply(action) {
                self.place();
            }
            self.request_redraw();
        }
    }
    fn request_redraw(&self) {
        for native in [&self.presenter, &self.audience].into_iter().flatten() {
            native.window.request_redraw();
        }
    }
    fn key(&mut self, event_loop: &ActiveEventLoop, key: Key) {
        let action = match key {
            Key::Named(
                NamedKey::ArrowRight | NamedKey::ArrowDown | NamedKey::PageDown | NamedKey::Space,
            ) => Some(Action::Next),
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp | NamedKey::PageUp) => {
                Some(Action::Previous)
            }
            Key::Named(NamedKey::Escape) => Some(Action::Escape),
            Key::Character(text) => match text.to_ascii_lowercase().as_str() {
                "b" => Some(Action::Blank),
                "s" => Some(Action::Swap),
                "p" => Some(Action::PresenterDisplay),
                "a" => Some(Action::AudienceDisplay),
                "f" => Some(Action::Fullscreen),
                "v" => Some(Action::PresenterFullscreen),
                "r" => {
                    if let Err(error) = self.refresh(event_loop, false) {
                        self.status = format!("Refresh failed: {error}");
                        self.request_redraw();
                    }
                    None
                }
                "q" => {
                    event_loop.exit();
                    None
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(action) = action {
            self.action(action);
        }
    }
    fn smoke(&mut self, event_loop: &ActiveEventLoop) {
        // Real event-loop actions, not simulated pixels: exercises resize/fullscreen
        // transitions, native surface presentation, and recovery to windowed mode.
        let actions = [
            Action::Next,
            Action::Previous,
            Action::Blank,
            Action::Blank,
            Action::Swap,
            Action::PresenterDisplay,
            Action::AudienceDisplay,
            Action::Fullscreen,
            Action::Escape,
            Action::PresenterFullscreen,
            Action::Escape,
        ];
        if let Some(action) = actions.get(self.smoke_step) {
            self.action(*action);
        } else if self.smoke_step == actions.len() {
            if let Err(error) = self.refresh(event_loop, false) {
                self.fail(event_loop, error);
            }
        } else {
            if self.frames.iter().any(|&frames| frames < 2) {
                self.fail(
                    event_loop,
                    "Smoke check did not present frames to both native windows",
                );
            } else {
                println!(
                    "Native smoke passed: presenter frames={}, audience frames={}, elapsed={:?}. This does not prove physical dual-screen placement.",
                    self.frames[0],
                    self.frames[1],
                    self.started.elapsed()
                );
                event_loop.exit();
            }
        }
        self.smoke_step += 1;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "linux")]
        {
            use winit::platform::x11::ActiveEventLoopExtX11;
            self.x11 = event_loop.is_x11();
        }
        if self.presenter.is_none()
            && let Err(error) = self.refresh(event_loop, true)
        {
            self.fail(event_loop, error);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let is_presenter = self.presenter.as_ref().is_some_and(|w| w.window.id() == id);
        match event {
            WindowEvent::CloseRequested => {
                if is_presenter {
                    event_loop.exit();
                } else {
                    self.action(Action::Escape);
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                self.key(event_loop, event.logical_key)
            }
            WindowEvent::CursorMoved { position, .. } if is_presenter => {
                self.cursor = (position.x, position.y)
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if is_presenter => {
                if let Some(window) = &self.presenter {
                    let size = window.window.inner_size();
                    if let Some(action) =
                        paint::hit_test(size.width, size.height, self.cursor.0, self.cursor.1)
                    {
                        self.action(action);
                    }
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.request_redraw()
            }
            WindowEvent::RedrawRequested => {
                let native = if is_presenter {
                    &mut self.presenter
                } else {
                    &mut self.audience
                };
                if let (Some(native), Some(model)) = (native, &self.model) {
                    match native.redraw(model, !is_presenter, &self.status) {
                        Ok(()) => {
                            self.frames[usize::from(!is_presenter)] += 1;
                        }
                        Err(error) => self.fail(event_loop, error),
                    }
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_check {
            self.next_check =
                now + Duration::from_millis(if self.options.smoke { 250 } else { 750 });
            if let Some(model) = &mut self.model {
                let (_, current, _) = displays(event_loop);
                if !model.disconnected && model.topology_changed(&current) {
                    self.status = "Display topology changed. Output hidden; R to refresh.".into();
                    self.place();
                    self.request_redraw();
                }
            }
            if self.options.smoke && self.model.is_some() {
                self.smoke(event_loop);
            }
        }
        #[cfg(target_os = "linux")]
        self.finish_x11_placement(now);
        let deadline = self.next_check;
        #[cfg(target_os = "linux")]
        let deadline = [&self.presenter, &self.audience]
            .into_iter()
            .flatten()
            .filter_map(|native| native.transition.pending.map(|(when, _)| when))
            .fold(deadline, Instant::min);
        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
    }
}

/// Run the native proof. Returns errors rather than panicking on missing displays.
pub fn run(options: Options) -> Result<(), Failure> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new(options);
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.error {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    fn target(monitor: usize, fullscreen: bool) -> Placement {
        Placement {
            monitor,
            fullscreen,
            rect: (24, 48, 900, 600),
        }
    }

    #[test]
    fn fullscreen_moves_after_exit_and_enters_after_movement() {
        let now = Instant::now();
        let mut t = X11Transition::default();
        assert!(t.request(target(1, true), now));
        assert_eq!(t.take_due(now), None);
        assert_eq!(t.take_due(now + X11Transition::SETTLE), Some(false));
        assert_eq!(t.take_due(now + X11Transition::SETTLE), None);
        assert_eq!(t.take_due(now + X11Transition::SETTLE * 2), Some(true));
        assert_eq!(t.take_due(now + X11Transition::SETTLE * 3), None);
        assert!(!t.request(target(1, true), now));
        assert!(t.pending.is_none());
    }

    #[test]
    fn newer_request_cancels_pending_fullscreen_and_unrelated_windows_stay_put() {
        let now = Instant::now();
        let mut presenter = X11Transition::default();
        let mut audience = X11Transition::default();
        presenter.request(target(0, false), now);
        audience.request(target(1, true), now);
        assert_eq!(presenter.take_due(now + X11Transition::SETTLE), Some(false));
        assert_eq!(audience.take_due(now + X11Transition::SETTLE), Some(false));
        let changed = now + X11Transition::SETTLE;
        assert!(audience.request(target(0, false), changed));
        assert!(!presenter.request(target(0, false), changed));
        assert!(presenter.pending.is_none());
        assert_eq!(
            audience.take_due(changed + X11Transition::SETTLE),
            Some(false)
        );
        assert!(audience.pending.is_none());
        assert_eq!(audience.target, Some(target(0, false)));
    }
}
