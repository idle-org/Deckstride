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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layout {
    pub previous: Widget,
    pub current: Widget,
    pub next: Widget,
    pub notes: Widget,
    pub elapsed: Widget,
    pub clock: Widget,
    pub header: bool,
    pub footer: bool,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            previous: Widget {
                show: true,
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
            header: true,
            footer: true,
        }
    }
}
impl Layout {
    pub fn validate(&self) -> Result<(), String> {
        for (w, min, max) in [
            (&self.previous, 30., 160.),
            (&self.current, 60., 200.),
            (&self.next, 30., 160.),
            (&self.notes, 12., 65.),
            (&self.elapsed, 16., 72.),
            (&self.clock, 16., 72.),
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub layout: Layout,
    pub projects: BTreeMap<String, Layout>,
    pub routes: Routes,
    pub presenter_fullscreen: bool,
    pub presentation_fullscreen: bool,
    pub browser_monitor: Option<String>,
    pub thumbnail_size: f32,
    pub buffer_mib: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            layout: Layout::default(),
            projects: BTreeMap::new(),
            routes: Routes::default(),
            presenter_fullscreen: true,
            presentation_fullscreen: true,
            browser_monitor: None,
            thumbnail_size: 220.,
            buffer_mib: 1024,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("Unsupported settings version {}", self.version));
        }
        if !(256..=16384).contains(&self.buffer_mib) {
            return Err("RAM buffer must be 256–16384 MiB per process".into());
        }
        if self.projects.len() > 4096 || !(100. ..=400.).contains(&self.thumbnail_size) {
            return Err("Invalid settings limits".into());
        }
        self.routes.validate()?;
        self.layout.validate()?;
        for layout in self.projects.values() {
            layout.validate()?;
        }
        Ok(())
    }
    pub fn for_project(&self, key: &str) -> Layout {
        self.projects.get(key).unwrap_or(&self.layout).clone()
    }
}
pub fn paths() -> Result<(PathBuf, PathBuf), String> {
    let dirs = directories::ProjectDirs::from("com", "Beffroi", "PdfPresenter")
        .ok_or("No user configuration directory")?;
    #[cfg(target_os = "linux")]
    let config = directories::BaseDirs::new()
        .ok_or("No user configuration directory")?
        .config_dir()
        .join("pdf-presenter");
    #[cfg(not(target_os = "linux"))]
    let config = dirs.config_dir().to_path_buf();
    Ok((
        config.join("settings.toml"),
        dirs.cache_dir().join("slides-v1"),
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
