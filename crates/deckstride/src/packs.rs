//! Bounded, data-only language and theme packs compatible with Canvas Pilot manifests.
use crate::theme::Palette;
use egui::Color32;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    sync::{LazyLock, RwLock},
};

const MAX_ARCHIVE: u64 = 2 * 1024 * 1024;
const MAX_EXPANDED: u64 = 4 * 1024 * 1024;
const MAX_PACKS: usize = 32;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LanguageManifest {
    version: u32,
    id: String,
    revision: u64,
    languages: Vec<LanguageDescription>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageDescription {
    pub locale: String,
    pub name: String,
    pub native_name: String,
}

#[derive(Clone, Debug)]
pub struct InstalledLanguagePack {
    pub id: String,
    pub revision: u64,
    pub languages: Vec<LanguageDescription>,
}

#[derive(Clone)]
struct LanguagePack {
    manifest: LanguageManifest,
    catalogs: BTreeMap<String, BTreeMap<String, &'static str>>,
    path: PathBuf,
    fingerprint: [u8; 32],
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeManifest {
    version: u32,
    id: String,
    revision: u64,
    themes: Vec<ThemeDescription>,
    #[serde(default)]
    notices: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDescription {
    pub id: String,
    pub name: String,
    pub light: bool,
    colors: ThemeColors,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeColors {
    canvas: String,
    banner: String,
    grid: String,
    layer: String,
    bg: String,
    panel: String,
    field: String,
    line: String,
    text: String,
    muted: String,
    cyan: String,
    amber: String,
    purple: String,
    pattern: [String; 2],
}

#[derive(Clone, Debug)]
pub struct InstalledThemePack {
    pub id: String,
    pub revision: u64,
    pub themes: Vec<ThemeDescription>,
    pub notices: String,
}

#[derive(Clone)]
struct ThemePack {
    manifest: ThemeManifest,
    path: PathBuf,
    fingerprint: [u8; 32],
}

static LANGUAGE_PACKS: LazyLock<RwLock<Vec<LanguagePack>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));
static THEME_PACKS: LazyLock<RwLock<Vec<ThemePack>>> = LazyLock::new(|| RwLock::new(Vec::new()));
#[derive(Default)]
struct InternPool {
    values: BTreeMap<String, &'static str>,
    bytes: usize,
}
static INTERNED: LazyLock<RwLock<InternPool>> = LazyLock::new(Default::default);

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 63
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

pub fn valid_theme_choice(id: &str) -> bool {
    id.split_once('/')
        .is_some_and(|(pack, theme)| valid_id(pack) && valid_id(theme))
}

fn bounded_read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(MAX_ARCHIVE + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err("Pack exceeds 2 MiB".into());
    }
    Ok(bytes)
}

fn fingerprint(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn intern(text: String) -> Result<&'static str, String> {
    let mut pool = INTERNED.write().unwrap();
    if let Some(value) = pool.values.get(&text) {
        return Ok(value);
    }
    if pool.bytes.saturating_add(text.len()) > 32 * 1024 * 1024 {
        return Err(
            "Runtime language-pack text limit reached; restart before importing more".into(),
        );
    }
    let leaked = Box::leak(text.clone().into_boxed_str());
    pool.bytes += text.len();
    pool.values.insert(text, leaked);
    Ok(leaked)
}

fn parse_language(bytes: &[u8], path: PathBuf) -> Result<LanguagePack, String> {
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err("Language pack exceeds 2 MiB".into());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if zip.is_empty() || zip.len() > 17 {
        return Err("Expected manifest.json and at most 16 catalogs".into());
    }
    let mut entries = BTreeMap::new();
    let mut expanded = 0_u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
        let name = entry.name().to_owned();
        if name.contains(['/', '\\'])
            || !name.ends_with(".json")
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Pack entries must be regular JSON files at the ZIP root".into());
        }
        let mut data = Vec::new();
        (&mut entry)
            .take(MAX_EXPANDED.saturating_sub(expanded) + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        expanded += data.len() as u64;
        if expanded > MAX_EXPANDED {
            return Err("Expanded language pack exceeds 4 MiB".into());
        }
        if entries.insert(name, data).is_some() {
            return Err("Duplicate ZIP entry".into());
        }
    }
    let manifest: LanguageManifest = serde_json::from_slice(
        &entries
            .remove("manifest.json")
            .ok_or("Missing manifest.json")?,
    )
    .map_err(|e| e.to_string())?;
    if manifest.version != 2
        || manifest.revision == 0
        || !valid_id(&manifest.id)
        || manifest.languages.is_empty()
        || manifest.languages.len() > 16
    {
        return Err("Invalid language-pack manifest or unsupported version".into());
    }
    let builtins = ["system", "en", "fr", "es", "ja", "zh-cn", "de", "pt-br"];
    let mut locales = BTreeSet::new();
    let mut catalogs = BTreeMap::new();
    for language in &manifest.languages {
        if !valid_id(&language.locale)
            || builtins.contains(&language.locale.as_str())
            || !locales.insert(language.locale.clone())
        {
            return Err("Invalid, duplicate, or built-in locale identifier".into());
        }
        for name in [&language.name, &language.native_name] {
            if name.trim().is_empty() || name.len() > 160 || name.chars().any(char::is_control) {
                return Err("Invalid language display name".into());
            }
        }
        let messages: BTreeMap<String, String> = serde_json::from_slice(
            &entries
                .remove(&format!("{}.json", language.locale))
                .ok_or("Missing language catalog")?,
        )
        .map_err(|e| e.to_string())?;
        if messages.is_empty() || messages.len() > 4096 {
            return Err("Language catalog is empty or too large".into());
        }
        let mut checked = BTreeMap::new();
        for (key, value) in messages {
            if key.trim().is_empty()
                || key.len() > 16_384
                || key.chars().any(char::is_control)
                || value.trim().is_empty()
                || value.len() > 16_384
                || value
                    .chars()
                    .any(|c| c.is_control() && c != '\n' && c != '\t')
            {
                return Err("Invalid translation key or text".into());
            }
            checked.insert(key, intern(value)?);
        }
        catalogs.insert(language.locale.clone(), checked);
    }
    if !entries.is_empty() {
        return Err("Unexpected files in language pack".into());
    }
    Ok(LanguagePack {
        manifest,
        catalogs,
        path,
        fingerprint: fingerprint(bytes),
    })
}

fn color(text: &str) -> Result<Color32, String> {
    if text.len() != 7
        || !text.starts_with('#')
        || !text[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(format!("Expected opaque #RRGGBB color: {text}"));
    }
    let value = u32::from_str_radix(&text[1..], 16).map_err(|e| e.to_string())?;
    Ok(Color32::from_rgb(
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ))
}

impl ThemeColors {
    fn palette(&self) -> Result<Palette, String> {
        Ok(Palette {
            canvas: color(&self.canvas)?,
            banner: color(&self.banner)?,
            grid: color(&self.grid)?,
            layer: color(&self.layer)?,
            bg: color(&self.bg)?,
            panel: color(&self.panel)?,
            field: color(&self.field)?,
            line: color(&self.line)?,
            text: color(&self.text)?,
            muted: color(&self.muted)?,
            cyan: color(&self.cyan)?,
            amber: color(&self.amber)?,
            purple: color(&self.purple)?,
            pattern: [color(&self.pattern[0])?, color(&self.pattern[1])?],
        })
    }
}

fn parse_theme(bytes: &[u8], path: PathBuf) -> Result<ThemePack, String> {
    if bytes.len() as u64 > MAX_ARCHIVE {
        return Err("Theme pack exceeds 2 MiB".into());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if zip.len() != 1 {
        return Err("Expected only manifest.json in the theme pack".into());
    }
    let mut entry = zip.by_index(0).map_err(|e| e.to_string())?;
    if entry.name() != "manifest.json"
        || entry.is_dir()
        || entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
    {
        return Err("Expected a regular manifest.json at the ZIP root".into());
    }
    let mut data = Vec::new();
    (&mut entry)
        .take(MAX_EXPANDED + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_EXPANDED {
        return Err("Expanded theme pack exceeds 4 MiB".into());
    }
    let manifest: ThemeManifest = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    if manifest.version != 1
        || manifest.revision == 0
        || !valid_id(&manifest.id)
        || manifest.themes.is_empty()
        || manifest.themes.len() > 16
        || manifest.notices.len() > 65_536
        || manifest
            .notices
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err("Invalid theme-pack manifest or unsupported version".into());
    }
    let mut ids = BTreeSet::new();
    for theme in &manifest.themes {
        if !valid_id(&theme.id)
            || !ids.insert(theme.id.clone())
            || theme.name.trim().is_empty()
            || theme.name.len() > 160
            || theme.name.chars().any(char::is_control)
        {
            return Err("Invalid or duplicate theme identifier/name".into());
        }
        theme.colors.palette()?;
    }
    Ok(ThemePack {
        manifest,
        path,
        fingerprint: fingerprint(bytes),
    })
}

fn zip_paths(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "zip"))
        .collect();
    paths.sort();
    if paths.len() > MAX_PACKS {
        return Err("Too many packs (maximum 32)".into());
    }
    Ok(paths)
}

pub fn load(root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let mut languages = Vec::new();
    match zip_paths(&root.join("language-packs")) {
        Ok(paths) => {
            for path in paths {
                match bounded_read(&path).and_then(|bytes| parse_language(&bytes, path.clone())) {
                    Ok(pack)
                        if !languages.iter().any(|old: &LanguagePack| {
                            old.manifest.id == pack.manifest.id
                                || old.manifest.languages.iter().any(|old_language| {
                                    pack.manifest
                                        .languages
                                        .iter()
                                        .any(|new| new.locale == old_language.locale)
                                })
                        }) =>
                    {
                        languages.push(pack)
                    }
                    Ok(_) => errors.push(format!("{}: duplicate pack or locale", path.display())),
                    Err(error) => errors.push(format!("{}: {error}", path.display())),
                }
            }
        }
        Err(error) => errors.push(error),
    }
    let mut themes = Vec::new();
    match zip_paths(&root.join("theme-packs")) {
        Ok(paths) => {
            for path in paths {
                match bounded_read(&path).and_then(|bytes| parse_theme(&bytes, path.clone())) {
                    Ok(pack)
                        if !themes
                            .iter()
                            .any(|old: &ThemePack| old.manifest.id == pack.manifest.id) =>
                    {
                        themes.push(pack)
                    }
                    Ok(_) => errors.push(format!("{}: duplicate pack identifier", path.display())),
                    Err(error) => errors.push(format!("{}: {error}", path.display())),
                }
            }
        }
        Err(error) => errors.push(error),
    }
    *LANGUAGE_PACKS.write().unwrap() = languages;
    *THEME_PACKS.write().unwrap() = themes;
    errors
}

fn install_bytes(
    directory: &Path,
    id: &str,
    revision: u64,
    previous: Option<(PathBuf, u64, [u8; 32])>,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let destination = previous
        .as_ref()
        .map(|(path, _, _)| path.clone())
        .unwrap_or_else(|| directory.join(format!("{id}.zip")));
    if let Some((_, old_revision, _)) = &previous
        && revision <= *old_revision
    {
        return Err(format!(
            "Installed revision {old_revision} is not older than revision {revision}"
        ));
    }
    if previous.is_none() && destination.exists() {
        return Err("Pack destination already exists; remove or rename it before importing".into());
    }
    if let Some((_, _, expected)) = previous
        && fingerprint(&bounded_read(&destination)?) != expected
    {
        return Err("Installed pack changed on disk; restart before updating it".into());
    }
    if !destination.exists() && zip_paths(directory).is_ok_and(|paths| paths.len() >= MAX_PACKS) {
        return Err("Too many packs (maximum 32)".into());
    }
    crate::settings::atomic_write(&destination, bytes)?;
    Ok(destination)
}

pub fn install_language(root: &Path, source: &Path) -> Result<String, String> {
    let bytes = bounded_read(source)?;
    let parsed = parse_language(&bytes, PathBuf::new())?;
    let installed = LANGUAGE_PACKS.read().unwrap().clone();
    for old in &installed {
        if old.manifest.id != parsed.manifest.id
            && old.manifest.languages.iter().any(|old_language| {
                parsed
                    .manifest
                    .languages
                    .iter()
                    .any(|new| new.locale == old_language.locale)
            })
        {
            return Err(format!(
                "Locale belongs to installed pack {}",
                old.manifest.id
            ));
        }
    }
    let previous = installed
        .iter()
        .find(|pack| pack.manifest.id == parsed.manifest.id)
        .map(|pack| (pack.path.clone(), pack.manifest.revision, pack.fingerprint));
    let directory = root.join("language-packs");
    let path = install_bytes(
        &directory,
        &parsed.manifest.id,
        parsed.manifest.revision,
        previous,
        &bytes,
    )?;
    let id = parsed.manifest.id.clone();
    let parsed = parse_language(&bytes, path)?;
    let mut packs = LANGUAGE_PACKS.write().unwrap();
    packs.retain(|pack| pack.manifest.id != id);
    packs.push(parsed);
    packs.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    Ok(id)
}

pub fn install_theme(root: &Path, source: &Path) -> Result<String, String> {
    let bytes = bounded_read(source)?;
    let parsed = parse_theme(&bytes, PathBuf::new())?;
    let installed = THEME_PACKS.read().unwrap().clone();
    let previous = installed
        .iter()
        .find(|pack| pack.manifest.id == parsed.manifest.id)
        .map(|pack| (pack.path.clone(), pack.manifest.revision, pack.fingerprint));
    let directory = root.join("theme-packs");
    let path = install_bytes(
        &directory,
        &parsed.manifest.id,
        parsed.manifest.revision,
        previous,
        &bytes,
    )?;
    let id = parsed.manifest.id.clone();
    let parsed = parse_theme(&bytes, path)?;
    let mut packs = THEME_PACKS.write().unwrap();
    packs.retain(|pack| pack.manifest.id != id);
    packs.push(parsed);
    packs.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    Ok(id)
}

pub fn language_packs() -> Vec<InstalledLanguagePack> {
    LANGUAGE_PACKS
        .read()
        .unwrap()
        .iter()
        .map(|pack| InstalledLanguagePack {
            id: pack.manifest.id.clone(),
            revision: pack.manifest.revision,
            languages: pack.manifest.languages.clone(),
        })
        .collect()
}

pub fn theme_packs() -> Vec<InstalledThemePack> {
    THEME_PACKS
        .read()
        .unwrap()
        .iter()
        .map(|pack| InstalledThemePack {
            id: pack.manifest.id.clone(),
            revision: pack.manifest.revision,
            themes: pack.manifest.themes.clone(),
            notices: pack.manifest.notices.clone(),
        })
        .collect()
}

pub fn translation(locale: &str, source: &str) -> Option<&'static str> {
    LANGUAGE_PACKS
        .read()
        .unwrap()
        .iter()
        .find_map(|pack| pack.catalogs.get(locale)?.get(source).copied())
}

pub fn language_name(locale: &str) -> Option<String> {
    LANGUAGE_PACKS.read().unwrap().iter().find_map(|pack| {
        pack.manifest
            .languages
            .iter()
            .find(|language| language.locale == locale)
            .map(|language| format!("{} — {}", language.native_name, language.name))
    })
}

pub fn theme_choices() -> Vec<(String, String)> {
    THEME_PACKS
        .read()
        .unwrap()
        .iter()
        .flat_map(|pack| {
            pack.manifest.themes.iter().map(|theme| {
                (
                    format!("{}/{}", pack.manifest.id, theme.id),
                    theme.name.clone(),
                )
            })
        })
        .collect()
}

pub fn packed_theme(choice: &str) -> Option<(Palette, bool, String)> {
    let (pack_id, theme_id) = choice.split_once('/')?;
    THEME_PACKS.read().unwrap().iter().find_map(|pack| {
        if pack.manifest.id != pack_id {
            return None;
        }
        let theme = pack
            .manifest
            .themes
            .iter()
            .find(|theme| theme.id == theme_id)?;
        Some((
            theme.colors.palette().ok()?,
            theme.light,
            theme.name.clone(),
        ))
    })
}

pub fn pack_root(settings_path: &Path) -> Result<&Path, String> {
    settings_path
        .parent()
        .ok_or("Settings directory unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, bytes) in files {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn example_packs_install_load_and_resolve() {
        let temp = tempfile::tempdir().unwrap();
        let language = temp.path().join("language.zip");
        let theme = temp.path().join("theme.zip");
        std::fs::write(
            &language,
            include_bytes!("../../../docs/packs/deckstride-language-pack.zip"),
        )
        .unwrap();
        std::fs::write(
            &theme,
            include_bytes!("../../../docs/packs/deckstride-theme-pack.zip"),
        )
        .unwrap();
        assert_eq!(
            install_language(temp.path(), &language).unwrap(),
            "example-language"
        );
        assert_eq!(translation("eo-example", "Close"), Some("Fermi"));
        assert_eq!(install_theme(temp.path(), &theme).unwrap(), "example-theme");
        assert!(packed_theme("example-theme/night").is_some());
        assert!(load(temp.path()).is_empty());
        assert_eq!(language_packs()[0].revision, 1);
        assert_eq!(theme_packs()[0].revision, 1);
        let ctx = egui::Context::default();
        crate::localization::apply_selected(
            &ctx,
            &crate::localization::Language::En,
            Some("eo-example"),
        );
        assert_eq!(crate::localization::tr(&ctx, "Close"), "Fermi");
        crate::theme::apply_selected(
            &ctx,
            crate::theme::Theme::Midra,
            Some("example-theme/night"),
        );
        assert_eq!(
            ctx.global_style().visuals.panel_fill,
            color("#101820").unwrap()
        );
        let mut manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/packs/language-example/manifest.json"
        ))
        .unwrap();
        manifest["revision"] = 2.into();
        let updated = archive(&[
            ("manifest.json", &serde_json::to_vec(&manifest).unwrap()),
            (
                "eo-example.json",
                include_bytes!("../../../docs/packs/language-example/eo-example.json"),
            ),
        ]);
        std::fs::write(
            temp.path().join("language-packs/example-language.zip"),
            b"changed",
        )
        .unwrap();
        std::fs::write(&language, updated).unwrap();
        assert!(
            install_language(temp.path(), &language)
                .unwrap_err()
                .contains("changed on disk")
        );
    }

    #[test]
    fn malformed_and_traversing_archives_are_rejected() {
        let bad = archive(&[("../manifest.json", b"{}")]);
        assert!(parse_language(&bad, PathBuf::new()).is_err());
        assert!(parse_theme(&bad, PathBuf::new()).is_err());
    }
}
