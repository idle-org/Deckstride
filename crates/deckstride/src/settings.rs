//! Versioned per-user settings and deterministic monitor/layout recovery.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Widget {
    pub show: bool,
    pub size: f32,
}
/// Fixed presenter-only timer locations; body rows remain visible with hidden bars.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimerPosition {
    #[default]
    Header,
    AboveSlides,
    BelowNotes,
    LeftOfNotes,
    RightOfNotes,
    LeftOfSlides,
    RightOfSlides,
}
impl TimerPosition {
    pub const ALL: [Self; 7] = [
        Self::Header,
        Self::AboveSlides,
        Self::BelowNotes,
        Self::LeftOfNotes,
        Self::RightOfNotes,
        Self::LeftOfSlides,
        Self::RightOfSlides,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Header => "Header",
            Self::AboveSlides => "Above slides",
            Self::BelowNotes => "Below notes",
            Self::LeftOfNotes => "Left of notes",
            Self::RightOfNotes => "Right of notes",
            Self::LeftOfSlides => "Left of slides",
            Self::RightOfSlides => "Right of slides",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimerMode {
    #[default]
    Elapsed,
    Countdown,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layout {
    pub previous: Widget,
    pub current: Widget,
    pub next: Widget,
    pub notes: Widget,
    pub elapsed: Widget,
    pub clock: Widget,
    pub elapsed_position: TimerPosition,
    pub clock_position: TimerPosition,
    pub slide_progress: bool,
    pub slide_progress_height: f32,
    pub timer_mode: TimerMode,
    pub countdown_seconds: u32,
    pub timer_autostart: bool,
    pub header: bool,
    pub footer: bool,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            previous: Widget {
                show: false,
                size: 60.,
            },
            current: Widget {
                show: true,
                size: 130.,
            },
            next: Widget {
                show: true,
                size: 75.,
            },
            notes: Widget {
                show: true,
                size: 28.,
            },
            elapsed: Widget {
                show: true,
                size: 28.,
            },
            clock: Widget {
                show: true,
                size: 28.,
            },
            elapsed_position: TimerPosition::Header,
            clock_position: TimerPosition::Header,
            slide_progress: false,
            slide_progress_height: 24.,
            timer_mode: TimerMode::Elapsed,
            countdown_seconds: 1200,
            timer_autostart: true,
            header: true,
            footer: true,
        }
    }
}
impl Layout {
    pub fn validate(&self) -> Result<(), String> {
        if !self.slide_progress_height.is_finite()
            || !(8. ..=120.).contains(&self.slide_progress_height)
            || !(1..=86400).contains(&self.countdown_seconds)
        {
            return Err("Invalid timer duration or progress bar size".into());
        }
        for (w, min, max) in [
            (&self.previous, 30., 160.),
            (&self.current, 60., 200.),
            (&self.next, 30., 160.),
            (&self.notes, 12., 65.),
            (&self.elapsed, 16., 320.),
            (&self.clock, 16., 320.),
        ] {
            if !w.size.is_finite() || w.size < min || w.size > max {
                return Err("Invalid layout size".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Routes {
    pub presenter: Vec<String>,
    pub presentation: Vec<String>,
}
impl Routes {
    pub fn resolve(&self, available: &[String], primary: usize) -> Self {
        if available.is_empty() {
            return Self::default();
        }
        let mut presenter: Vec<_> = self
            .presenter
            .iter()
            .filter(|id| available.contains(id))
            .cloned()
            .collect();
        if presenter.is_empty() {
            presenter.push(available[primary.min(available.len() - 1)].clone());
        }
        let mut presentation: Vec<_> = self
            .presentation
            .iter()
            .filter(|id| available.contains(id) && !presenter.contains(id))
            .cloned()
            .collect();
        if presentation.is_empty()
            && (!self.presentation.is_empty() || self.presenter.is_empty())
            && let Some(id) = available.iter().find(|id| !presenter.contains(id))
        {
            presentation.push(id.clone());
        }
        Self {
            presenter,
            presentation,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.presenter.len() + self.presentation.len() > 32
            || self.presenter.iter().any(|p| self.presentation.contains(p))
        {
            return Err("Invalid screen roles".into());
        }
        for group in [&self.presenter, &self.presentation] {
            let mut unique = group.clone();
            unique.sort();
            unique.dedup();
            if unique.len() != group.len() {
                return Err("Duplicate screen assignment".into());
            }
        }
        Ok(())
    }
}
/// Opaque display colors; cached slide pixels retain their original transparency.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SlideColors {
    #[serde(deserialize_with = "read_rgb")]
    pub page_background: [u8; 3],
    #[serde(deserialize_with = "read_rgb")]
    pub fit_padding: [u8; 3],
}
fn read_rgb<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<[u8; 3], D::Error> {
    let channels = Vec::<u8>::deserialize(deserializer)?;
    channels
        .try_into()
        .map_err(|_| serde::de::Error::custom("RGB colors require exactly three channels"))
}
impl Default for SlideColors {
    fn default() -> Self {
        Self {
            page_background: [255; 3],
            fit_padding: [0; 3],
        }
    }
}
/// Per-monitor preference that takes priority over role-wide fullscreen commands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenMode {
    #[default]
    FollowGlobal,
    AlwaysFullscreen,
    AlwaysWindowed,
}
impl ScreenMode {
    pub fn fullscreen(self, global: bool) -> bool {
        match self {
            Self::FollowGlobal => global,
            Self::AlwaysFullscreen => true,
            Self::AlwaysWindowed => false,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::FollowGlobal => "Follow global",
            Self::AlwaysFullscreen => "Always fullscreen",
            Self::AlwaysWindowed => "Always windowed",
        }
    }
}
pub fn validate_screen_modes(modes: &BTreeMap<String, ScreenMode>) -> Result<(), String> {
    if modes.len() > 128 || modes.keys().any(|id| id.is_empty() || id.len() > 1024) {
        return Err(
            "Invalid screen modes: at most 128 nonempty monitor IDs of at most 1024 bytes".into(),
        );
    }
    Ok(())
}
pub fn validate_single_screens(displays: &[String]) -> Result<(), String> {
    let unique: std::collections::HashSet<_> = displays.iter().collect();
    if displays.len() > 32
        || unique.len() != displays.len()
        || displays.iter().any(|id| id.is_empty() || id.len() > 1024)
    {
        return Err("Invalid single-screen assignments".into());
    }
    Ok(())
}
/// Saved visual layouts for one monitor; shared timer behavior uses legacy layouts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayLayouts {
    pub layout: Layout,
    pub projects: BTreeMap<String, Layout>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub slide_colors: SlideColors,
    pub layout: Layout,
    pub projects: BTreeMap<String, Layout>,
    pub display_layouts: BTreeMap<String, DisplayLayouts>,
    pub screen_modes: BTreeMap<String, ScreenMode>,
    pub single_screens: Vec<String>,
    pub routes: Routes,
    pub presenter_fullscreen: bool,
    pub presentation_fullscreen: bool,
    pub browser_monitor: Option<String>,
    pub thumbnail_size: f32,
    pub buffer_mib: u32,
    pub disk_cache: crate::cache::Policy,
    pub auto_show: bool,
    pub autoplay_enabled: bool,
    pub autoplay_seconds: f32,
    pub autoplay_loop: bool,
    pub blank_picture: bool,
    pub zoom_enabled: bool,
    pub zoom_keep_fitted_area: bool,
    pub theme: crate::theme::Theme,
    pub language: crate::localization::Language,
}
/// Choose a per-process decoded-frame budget from available physical RAM.
/// Prefer 15%, with a soft 512 MiB floor that never exceeds 20% (except the
/// 64 MiB operational minimum). Unknown memory uses a conservative 256 MiB.
pub fn buffer_budget_mib(available_bytes: Option<u64>) -> u32 {
    let Some(bytes) = available_bytes else {
        return 256;
    };
    let mib = bytes / (1024 * 1024);
    (mib.saturating_mul(15) / 100)
        .max(512)
        .min(mib / 5)
        .clamp(64, 16384) as u32
}

fn default_buffer_mib() -> u32 {
    // Probe only RAM, once per process: no process enumeration or UI polling.
    static BUDGET: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *BUDGET.get_or_init(|| {
        let mut system = sysinfo::System::new();
        system.refresh_memory_specifics(sysinfo::MemoryRefreshKind::nothing().with_ram());
        buffer_budget_mib((system.total_memory() > 0).then(|| system.available_memory()))
    })
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            slide_colors: SlideColors::default(),
            layout: Layout::default(),
            projects: BTreeMap::new(),
            display_layouts: BTreeMap::new(),
            screen_modes: BTreeMap::new(),
            single_screens: Vec::new(),
            routes: Routes::default(),
            presenter_fullscreen: true,
            presentation_fullscreen: true,
            browser_monitor: None,
            thumbnail_size: 220.,
            buffer_mib: default_buffer_mib(),
            disk_cache: Default::default(),
            auto_show: true,
            autoplay_enabled: false,
            autoplay_seconds: 5.0,
            autoplay_loop: false,
            blank_picture: false,
            zoom_enabled: false,
            zoom_keep_fitted_area: false,
            theme: Default::default(),
            language: Default::default(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("Unsupported settings version {}", self.version));
        }
        self.disk_cache.validate()?;
        if !(64..=16384).contains(&self.buffer_mib) {
            return Err("RAM buffer must be 64–16384 MiB per process".into());
        }
        if !self.autoplay_seconds.is_finite() || !(0.25..=3600.0).contains(&self.autoplay_seconds) {
            return Err("Autoplay timing must be 0.25–3600 seconds".into());
        }
        if self.projects.len() > 4096 || !(100. ..=400.).contains(&self.thumbnail_size) {
            return Err("Invalid settings limits".into());
        }
        if self.display_layouts.len() > 128
            || self
                .display_layouts
                .values()
                .map(|profile| profile.projects.len())
                .sum::<usize>()
                > 4096
        {
            return Err("Too many display layouts".into());
        }
        for (display, profile) in &self.display_layouts {
            if display.is_empty() || display.len() > 1024 {
                return Err("Invalid display layout identifier".into());
            }
            profile.layout.validate()?;
            for layout in profile.projects.values() {
                layout.validate()?;
            }
        }
        validate_single_screens(&self.single_screens)?;
        validate_screen_modes(&self.screen_modes)?;
        self.routes.validate()?;
        self.layout.validate()?;
        for layout in self.projects.values() {
            layout.validate()?;
        }
        Ok(())
    }
    pub fn for_display(&self, display: &str, project: &str) -> Layout {
        self.display_layouts.get(display).map_or_else(
            || self.for_project(project),
            |profile| {
                profile
                    .projects
                    .get(project)
                    .unwrap_or(&profile.layout)
                    .clone()
            },
        )
    }
    pub fn for_project(&self, key: &str) -> Layout {
        self.projects.get(key).unwrap_or(&self.layout).clone()
    }
}
fn select_renamed_paths(
    settings: PathBuf,
    cache: PathBuf,
    legacy_settings: PathBuf,
    legacy_cache: PathBuf,
) -> (PathBuf, PathBuf) {
    if !settings.exists()
        && legacy_settings.is_file()
        && let Ok(value) = read(&legacy_settings)
    {
        let _ = write(&settings, &value);
    }
    let settings = if settings.exists() || !legacy_settings.is_file() {
        settings
    } else {
        legacy_settings
    };
    let cache = if cache.exists() || !legacy_cache.exists() {
        cache
    } else {
        legacy_cache
    };
    (settings, cache)
}

pub fn paths() -> Result<(PathBuf, PathBuf), String> {
    let dirs = directories::ProjectDirs::from("com", "Beffroi", "Deckstride")
        .ok_or("No user configuration directory")?;
    let legacy_dirs = directories::ProjectDirs::from("com", "Beffroi", "PdfPresenter")
        .ok_or("No user configuration directory")?;
    #[cfg(target_os = "linux")]
    let (config, legacy_config) = {
        let base = directories::BaseDirs::new().ok_or("No user configuration directory")?;
        (
            base.config_dir().join("deckstride"),
            base.config_dir().join("pdf-presenter"),
        )
    };
    #[cfg(not(target_os = "linux"))]
    let (config, legacy_config) = (
        dirs.config_dir().to_path_buf(),
        legacy_dirs.config_dir().to_path_buf(),
    );
    Ok(select_renamed_paths(
        config.join("settings.toml"),
        dirs.cache_dir().join("slides-v1"),
        legacy_config.join("settings.toml"),
        legacy_dirs.cache_dir().join("slides-v1"),
    ))
}
pub fn read(path: &Path) -> Result<Settings, String> {
    match std::fs::File::open(path) {
        Ok(file) => {
            use std::io::Read;
            let mut bytes = Vec::new();
            file.take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 4 * 1024 * 1024 {
                return Err("Settings exceed 4 MiB".into());
            }
            let value: Settings =
                toml::from_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            value.validate()?;
            Ok(value)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.to_string()),
    }
}
pub fn write(path: &Path, settings: &Settings) -> Result<(), String> {
    settings.validate()?;
    atomic_write(
        path,
        toml::to_string_pretty(settings)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let dir = path.parent().ok_or("No parent directory")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn project_key(path: &Path) -> String {
    format!("{:x}", Sha256::digest(path.as_os_str().as_encoded_bytes()))
}

#[cfg(test)]
mod rename_tests {
    use super::*;

    #[test]
    fn renamed_paths_copy_settings_and_reuse_a_warm_cache() {
        let temp = tempfile::tempdir().unwrap();
        let settings = temp.path().join("new/settings.toml");
        let cache = temp.path().join("new/cache");
        let legacy_settings = temp.path().join("legacy/settings.toml");
        let legacy_cache = temp.path().join("legacy/cache");
        write(&legacy_settings, &Settings::default()).unwrap();
        std::fs::create_dir_all(&legacy_cache).unwrap();

        let selected = select_renamed_paths(
            settings.clone(),
            cache,
            legacy_settings,
            legacy_cache.clone(),
        );

        assert_eq!(selected, (settings.clone(), legacy_cache));
        let migrated = read(&settings).unwrap();
        assert_eq!(migrated.version, 1);
        assert_eq!(migrated.layout, Layout::default());
    }
}
