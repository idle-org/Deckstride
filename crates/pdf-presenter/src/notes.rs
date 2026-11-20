//! Versioned presenter-only notes sidecars. All I/O runs in isolated workers.
use crate::document::{DeckInfo, Document, read_limited};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

pub const MAX_NOTE: usize = 256 * 1024;
const MAX_TEXT: usize = 8 * 1024 * 1024;
const MAX_FILE: u64 = 16 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NotesFile {
    version: u32,
    notes: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub path: PathBuf,
    pub revision: String,
    pub notes: Vec<String>,
}
/// PDF: sibling `talk.notes.toml`; image folder: sibling `folder.notes.toml`.
/// Called only in workers, because distinguishing folders requires filesystem access.
pub fn sidecar_path(source: &Path) -> PathBuf {
    if source.is_dir() {
        let mut name = source.file_name().unwrap_or_default().to_os_string();
        name.push(".notes.toml");
        source.with_file_name(name)
    } else {
        source.with_extension("notes.toml")
    }
}
fn keys(source: &Path, names: &[String]) -> Vec<String> {
    if source.is_dir() {
        names.to_vec()
    } else {
        (1..=names.len()).map(|n| n.to_string()).collect()
    }
}
fn revision(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn validate(file: &NotesFile) -> Result<(), String> {
    if file.version != 1 {
        return Err(format!("Unsupported notes version {}", file.version));
    }
    if file.notes.len() > 10000
        || file.notes.values().any(|n| n.len() > MAX_NOTE)
        || file.notes.values().map(String::len).sum::<usize>() > MAX_TEXT
    {
        return Err("Notes exceed limits (256 KiB per slide, 8 MiB total, 10000 entries)".into());
    }
    Ok(())
}
fn read(path: &Path) -> Result<Option<(NotesFile, Vec<u8>)>, String> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
        Ok(meta) if !meta.file_type().is_file() => {
            return Err("Notes sidecar must be a regular file, not a symlink or directory".into());
        }
        _ => {}
    }
    let bytes = read_limited(path, MAX_FILE)?;
    let file: NotesFile = toml::from_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    validate(&file)?;
    Ok(Some((file, bytes)))
}
/// Named sidecar entries override extracted notes, including explicitly empty entries.
pub fn overlay(source: &Path, names: &[String], notes: &mut [String]) -> Result<(), String> {
    if let Some((file, _)) = read(&sidecar_path(source))? {
        let mut merged = notes.to_vec();
        for (key, note) in keys(source, names).iter().zip(&mut merged) {
            if let Some(value) = file.notes.get(key) {
                *note = value.clone();
            }
        }
        if merged.iter().map(String::len).sum::<usize>() > MAX_TEXT {
            return Err("Combined notes exceed 8 MiB".into());
        }
        notes.clone_from_slice(&merged);
    }
    Ok(())
}
type LoadedNotes = (DeckInfo, Option<(NotesFile, Vec<u8>)>);
fn document(source: &Path, cache: PathBuf, expected_source: &str) -> Result<LoadedNotes, String> {
    let path = sidecar_path(source);
    let before = read(&path)?;
    let info = Document::open(source, cache)?.info;
    if info.sha256 != expected_source {
        return Err("Presentation changed on disk. Reopen it before editing notes.".into());
    }
    if let Some(warning) = info
        .warnings
        .iter()
        .find(|w| w.starts_with("Presentation notes:"))
    {
        return Err(warning.clone());
    }
    let after = read(&path)?;
    if before.as_ref().map(|(_, b)| b) != after.as_ref().map(|(_, b)| b) {
        return Err("Notes changed while loading. Retry to read the latest notes.".into());
    }
    Ok((info, after))
}
fn bytes(file: &NotesFile) -> Result<Vec<u8>, String> {
    validate(file)?;
    let bytes = toml::to_string_pretty(file)
        .map_err(|e| e.to_string())?
        .into_bytes();
    if bytes.len() as u64 > MAX_FILE {
        return Err("Serialized notes exceed 16 MiB".into());
    }
    Ok(bytes)
}
fn temporary(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile, String> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or("No notes parent folder")?)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    Ok(file)
}
pub fn prepare(source: &Path, cache: PathBuf, expected_source: &str) -> Result<Snapshot, String> {
    let (info, existing) = document(source, cache, expected_source)?;
    let path = sidecar_path(&info.path);
    let bytes = match existing {
        Some((_, bytes)) => bytes,
        None => {
            let file = NotesFile {
                version: 1,
                notes: keys(&info.path, &info.names)
                    .into_iter()
                    .zip(info.notes.iter().cloned())
                    .collect(),
            };
            let bytes = bytes(&file)?;
            // Never replace a sidecar that appeared while extraction was running.
            temporary(&path, &bytes)?
                .persist_noclobber(&path)
                .map_err(|e| {
                    format!("Could not create notes (retry if the file appeared meanwhile): {e}")
                })?;
            bytes
        }
    };
    Ok(Snapshot {
        path,
        revision: revision(&bytes),
        notes: info.notes,
    })
}
pub fn save(
    source: &Path,
    cache: PathBuf,
    expected_source: &str,
    expected_revision: &str,
    page: usize,
    text: String,
) -> Result<Snapshot, String> {
    if text.len() > MAX_NOTE {
        return Err("This note exceeds 256 KiB".into());
    }
    let (mut info, existing) = document(source, cache, expected_source)?;
    let key = keys(&info.path, &info.names)
        .get(page)
        .cloned()
        .ok_or("Slide no longer exists")?;
    let path = sidecar_path(&info.path);
    let (mut file, previous) = existing.ok_or("Notes file was removed. Reload before saving.")?;
    if revision(&previous) != expected_revision {
        return Err("Notes changed on disk. Copy your edits, then reload before saving.".into());
    }
    info.notes[page] = text.clone();
    if info.notes.iter().map(String::len).sum::<usize>() > MAX_TEXT {
        return Err("Combined notes exceed 8 MiB; changes were not saved.".into());
    }
    file.notes.insert(key, text.clone());
    let bytes = bytes(&file)?;
    let temp = temporary(&path, &bytes)?;
    let (_, latest) = read(&path)?.ok_or("Notes file was removed while saving")?;
    if latest != previous {
        return Err("Notes changed while saving; edits were not written.".into());
    }
    temp.persist(&path).map_err(|e| e.to_string())?;
    Ok(Snapshot {
        path,
        revision: revision(&bytes),
        notes: info.notes,
    })
}
