//! Bounded worker-only maintenance of disposable rendered slide files.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Policy {
    pub max_mib: u32,
    pub max_days: u32,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            max_mib: 10240,
            max_days: 30,
        }
    }
}
impl Policy {
    pub fn validate(&self) -> Result<(), String> {
        if !(64..=1048576).contains(&self.max_mib) || !(1..=3650).contains(&self.max_days) {
            return Err("Disk cache must be 64–1048576 MiB and 1–3650 days".into());
        }
        Ok(())
    }
}
#[derive(Default, Serialize, Deserialize)]
pub struct Report {
    pub removed_bytes: u64,
    pub remaining_bytes: u64,
    pub removed_files: usize,
    pub skipped_files: usize,
}

fn lock_file(project: &Path) -> Result<File, String> {
    let path = project.join(".lease-v1");
    if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
        return Err("Invalid cache lease file".into());
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())
}
/// Scoped OS lock, explicitly unlocked before close so a concurrent fork cannot
/// accidentally prolong ownership until the child execs.
pub struct Lease(File);
impl Drop for Lease {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
/// A live document holds a shared OS lock; process exit releases it automatically.
pub fn lease(cache: &Path, key: &str) -> Result<Lease, String> {
    let project = cache.join(key);
    fs::create_dir_all(&project).map_err(|e| e.to_string())?;
    let file = lock_file(&project)?;
    // Only a killable worker waits here, never an input/event loop.
    file.lock_shared().map_err(|e| e.to_string())?;
    Ok(Lease(file))
}
fn hash_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn artifact(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    let Some((page, rest)) = name.split_once('-') else {
        return false;
    };
    let Some((edge, suffix)) = rest.split_once('-') else {
        return false;
    };
    page.parse::<usize>().is_ok()
        && edge.parse::<u32>().is_ok()
        && matches!(
            suffix,
            "hayro09.rgba" | "hayro09-straight.rgba" | "extraction.json"
        )
}
struct Entry {
    path: PathBuf,
    project: PathBuf,
    bytes: u64,
    modified: SystemTime,
}
/// Expire by artifact modification time, then evict oldest first to the byte target.
/// Manual expunge removes all eligible artifacts. Active projects are always retained.
pub fn clean(
    root: &Path,
    policy: &Policy,
    protected: &[String],
    manual: bool,
    now: SystemTime,
) -> Result<Report, String> {
    policy.validate()?;
    match fs::symlink_metadata(root) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Report::default()),
        Ok(m) if m.is_dir() => (),
        _ => return Err("Cache root is not a regular directory".into()),
    }
    let mut entries = Vec::new();
    let mut report = Report::default();
    let mut visited = 0usize;
    // Fixed depth and entry ceiling bound memory, including unexpected entries.
    let mut scan = vec![(root.to_owned(), 0)];
    while let Some((dir, depth)) = scan.pop() {
        for item in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            visited += 1;
            if visited > 200_000 {
                return Err("Cache scan exceeds 200000 entries; cleanup stopped".into());
            }
            let item = item.map_err(|e| e.to_string())?;
            let kind = item.file_type().map_err(|e| e.to_string())?;
            let path = item.path();
            if depth < 2 && kind.is_dir() && hash_name(&path) {
                scan.push((path, depth + 1));
            } else if depth == 2 && kind.is_file() {
                let meta = item.metadata().map_err(|e| e.to_string())?;
                report.remaining_bytes = report.remaining_bytes.saturating_add(meta.len());
                if artifact(&path) {
                    entries.push(Entry {
                        path,
                        project: dir.parent().unwrap().to_owned(),
                        bytes: meta.len(),
                        modified: meta.modified().map_err(|e| e.to_string())?,
                    });
                }
            }
        }
    }
    entries.sort_by(|a, b| {
        a.modified
            .cmp(&b.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    let max_bytes = u64::from(policy.max_mib) * 1024 * 1024;
    let lifetime = Duration::from_secs(u64::from(policy.max_days) * 86400);
    for entry in entries {
        if !manual
            && report.remaining_bytes <= max_bytes
            && now.duration_since(entry.modified).unwrap_or_default() < lifetime
        {
            continue;
        }
        let key = entry.project.file_name().unwrap().to_string_lossy();
        if protected.iter().any(|p| p == &key) {
            report.skipped_files += 1;
            continue;
        }
        let lock = lock_file(&entry.project)?;
        match lock.try_lock() {
            Ok(()) => (),
            Err(std::fs::TryLockError::WouldBlock) => {
                report.skipped_files += 1;
                continue;
            }
            Err(e) => return Err(e.to_string()),
        }
        let _lease = Lease(lock);
        // Recheck after obtaining the lock: a worker may have replaced this artifact.
        let meta = match fs::symlink_metadata(&entry.path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
        };
        if !meta.is_file()
            || meta.len() != entry.bytes
            || meta.modified().ok() != Some(entry.modified)
        {
            report.skipped_files += 1;
            continue;
        }
        fs::remove_file(&entry.path).map_err(|e| e.to_string())?;
        report.removed_bytes += entry.bytes;
        report.remaining_bytes = report.remaining_bytes.saturating_sub(entry.bytes);
        report.removed_files += 1;
        // Never remove project/lease files: unlinking a lock could split ownership.
        let _ = fs::remove_dir(entry.path.parent().unwrap());
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoped_release_is_not_prolonged_by_a_duplicated_descriptor() {
        let temp = tempfile::tempdir().unwrap();
        let key = format!("{:064x}", 1);
        let lease = lease(temp.path(), &key).unwrap();
        let inherited = lease.0.try_clone().unwrap();
        drop(lease);
        let exclusive = lock_file(&temp.path().join(key)).unwrap();
        exclusive.try_lock().unwrap();
        drop(Lease(exclusive));
        drop(inherited);
    }
}
