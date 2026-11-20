//! Display selection and controls for the native dual-screen proof of concept.

/// A desktop display as reported by the native window backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Display {
    /// Human-readable backend name, when available.
    pub name: String,
    /// Physical desktop origin (may be negative).
    pub origin: (i32, i32),
    /// Physical pixel dimensions.
    pub size: (u32, u32),
}

/// Mock deck length; no PDF is opened by this proof of concept.
pub const SLIDES: usize = 12;

/// Operator actions, shared by keyboard, pointer and smoke checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Previous mock page.
    Previous,
    /// Next mock page.
    Next,
    /// Toggle audience blanking.
    Blank,
    /// Exchange the two display assignments.
    Swap,
    /// Cycle the presenter display, keeping roles separate when possible.
    PresenterDisplay,
    /// Cycle the Slides screen, keeping roles separate when possible.
    AudienceDisplay,
    /// Toggle audience fullscreen.
    Fullscreen,
    /// Toggle presenter fullscreen (also useful on Wayland).
    PresenterFullscreen,
    /// Leave fullscreen and blank the audience.
    Escape,
}

/// All mutable mock presentation state, independent of the OS event loop.
#[derive(Clone, Debug)]
pub struct Model {
    /// Available displays, in the same order as native handles.
    pub displays: Vec<Display>,
    /// Presenter display index.
    pub presenter: usize,
    /// Audience display index.
    pub audience: usize,
    /// Current mock page, zero-based.
    pub page: usize,
    /// Audience is intentionally black.
    pub blanked: bool,
    /// Audience fullscreen requested.
    pub fullscreen: bool,
    /// Presenter fullscreen requested.
    pub presenter_fullscreen: bool,
    /// Topology changed; output needs explicit reassignment.
    pub disconnected: bool,
}

impl Model {
    /// Choose primary/secondary defaults and validate explicit zero-based indices.
    pub fn new(
        displays: Vec<Display>,
        primary: usize,
        presenter: Option<usize>,
        audience: Option<usize>,
        windowed: bool,
    ) -> Result<Self, String> {
        if displays.is_empty() {
            return Err("No displays reported by the window system.".into());
        }
        let presenter = presenter.unwrap_or(primary.min(displays.len() - 1));
        let audience = audience.unwrap_or_else(|| {
            (0..displays.len())
                .find(|&i| i != presenter)
                .unwrap_or(presenter)
        });
        if presenter >= displays.len() || audience >= displays.len() {
            return Err("Display index is out of range; use --list-monitors.".into());
        }
        if displays.len() > 1 && presenter == audience {
            return Err("Choose different Presenter and Slides screens.".into());
        }
        let fullscreen = !windowed;
        Ok(Self {
            displays,
            presenter,
            audience,
            page: 0,
            blanked: false,
            fullscreen,
            presenter_fullscreen: fullscreen,
            disconnected: false,
        })
    }

    /// Apply a control. Returns whether window placement needs updating.
    pub fn apply(&mut self, action: Action) -> bool {
        match action {
            Action::Previous => self.page = self.page.saturating_sub(1),
            Action::Next => self.page = (self.page + 1).min(SLIDES - 1),
            Action::Blank => {
                if !self.disconnected {
                    self.blanked = !self.blanked;
                }
            }
            Action::Escape => {
                self.blanked = true;
                self.fullscreen = false;
                self.presenter_fullscreen = false;
                return true;
            }
            Action::Swap => {
                std::mem::swap(&mut self.presenter, &mut self.audience);
                return true;
            }
            Action::PresenterDisplay => {
                self.presenter = (self.presenter + 1) % self.displays.len();
                if self.displays.len() > 1 && self.presenter == self.audience {
                    self.audience = (self.audience + 1) % self.displays.len();
                }
                return true;
            }
            Action::AudienceDisplay => {
                self.audience = (self.audience + 1) % self.displays.len();
                if self.displays.len() > 1 && self.audience == self.presenter {
                    self.presenter = (self.presenter + 1) % self.displays.len();
                }
                return true;
            }
            Action::Fullscreen => {
                if !self.disconnected {
                    self.fullscreen = !self.fullscreen;
                }
                return true;
            }
            Action::PresenterFullscreen => {
                self.presenter_fullscreen = !self.presenter_fullscreen;
                return true;
            }
        }
        false
    }

    /// Blank and suspend output if the topology changes. Refresh is explicit.
    pub fn topology_changed(&mut self, displays: &[Display]) -> bool {
        if self.displays == displays {
            return false;
        }
        self.disconnected = true;
        self.blanked = true;
        self.fullscreen = false;
        self.presenter_fullscreen = false;
        true
    }
}

/// Position and size a window inside a display in physical pixels.
pub fn window_rect(display: &Display, audience: bool, rehearsal: bool) -> (i32, i32, u32, u32) {
    let (width, height) = display.size;
    let area = if rehearsal { width / 2 } else { width };
    let w = area.saturating_sub(64).clamp(160, 1000);
    let h = height.saturating_sub(120).clamp(120, 700);
    let offset = if rehearsal && audience { area } else { 0 };
    (
        display
            .origin
            .0
            .saturating_add(offset as i32)
            .saturating_add(24),
        display.origin.1.saturating_add(48),
        w,
        h,
    )
}

/// Startup options. Display indices presented to users are one-based.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Options {
    /// Show command help without starting a desktop event loop.
    pub help: bool,
    /// List native monitor information, then exit.
    pub list: bool,
    /// Presenter display override.
    pub presenter: Option<usize>,
    /// Audience display override.
    pub audience: Option<usize>,
    /// Keep both windows decorated instead of fullscreen.
    pub windowed: bool,
    /// Exercise the real native windows and exit automatically.
    pub smoke: bool,
}

impl Options {
    /// Parse startup arguments without initializing native UI libraries.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut result = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" | "--version" => result.help = true,
                "--list-monitors" => result.list = true,
                "--windowed" => result.windowed = true,
                "--smoke-test" => {
                    result.smoke = true;
                    result.windowed = true;
                }
                "--presenter" | "--audience" => {
                    let value = args
                        .next()
                        .ok_or_else(|| format!("{arg} needs a one-based display number"))?;
                    let index = value
                        .parse::<usize>()
                        .ok()
                        .and_then(|n| n.checked_sub(1))
                        .ok_or_else(|| format!("Invalid display number: {value}"))?;
                    if arg == "--presenter" {
                        result.presenter = Some(index);
                    } else {
                        result.audience = Some(index);
                    }
                }
                _ => return Err(format!("Unknown option: {arg}. Use --help.")),
            }
        }
        Ok(result)
    }
}
