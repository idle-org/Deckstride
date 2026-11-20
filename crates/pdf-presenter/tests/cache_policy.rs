mod common;
use pdf_presenter::{
    cache::{self, Policy},
    document::Document,
    settings,
    studio::Studio,
};
use std::{
    fs::{self, File, FileTimes},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
fn entry(root: &Path, project: u8, page: u8, bytes: u64, age_days: u64) -> PathBuf {
    let dir = root
        .join(format!("{project:064x}"))
        .join(format!("{:064x}", 1));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{page}-512-hayro09.rgba"));
    let f = File::create(&path).unwrap();
    f.set_len(bytes).unwrap();
    f.set_times(
        FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(age_days * 86400)),
    )
    .unwrap();
    path
}
#[test]
fn expiration_and_quota_remove_oldest_and_protect_active_projects() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let old = entry(root, 1, 0, 1024, 40);
    let older = entry(root, 1, 1, 40 * 1024 * 1024, 3);
    let newer = entry(root, 2, 0, 40 * 1024 * 1024, 2);
    let protected = entry(root, 3, 0, 10 * 1024 * 1024, 40);
    let locked = entry(root, 4, 0, 1024, 40);
    let lease = cache::lease(root, &format!("{:064x}", 4)).unwrap();
    let policy = Policy {
        max_mib: 64,
        max_days: 30,
    };
    let r = cache::clean(
        root,
        &policy,
        &[format!("{:064x}", 3)],
        false,
        SystemTime::now(),
    )
    .unwrap();
    assert_eq!(r.removed_files, 2);
    assert!(!old.exists() && !older.exists());
    assert!(newer.exists() && protected.exists() && locked.exists());
    assert_eq!(r.remaining_bytes, 50 * 1024 * 1024 + 1024);
    assert_eq!(r.skipped_files, 2);
    drop(lease);
    let r = cache::clean(root, &policy, &[], true, SystemTime::now()).unwrap();
    assert_eq!(r.removed_files, 3);
    assert_eq!(r.remaining_bytes, 0);
}
#[test]
fn unknown_files_symlinks_missing_roots_and_invalid_policies_are_safe() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("cache");
    assert_eq!(
        cache::clean(&root, &Policy::default(), &[], true, SystemTime::now())
            .unwrap()
            .removed_files,
        0
    );
    let file = entry(&root, 1, 0, 3, 50);
    let unknown = file.with_file_name("notes.json");
    fs::write(&unknown, "private notes").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&unknown, file.with_file_name("1-512-hayro09.rgba")).unwrap();
        std::os::unix::fs::symlink(file.parent().unwrap(), root.join(format!("{:064x}", 2)))
            .unwrap();
    }
    cache::clean(&root, &Policy::default(), &[], true, SystemTime::now()).unwrap();
    assert!(unknown.exists());
    assert!(!file.exists());
    assert!(
        Policy {
            max_mib: 0,
            max_days: 30
        }
        .validate()
        .is_err()
    );
    assert!(
        Policy {
            max_mib: 64,
            max_days: 0
        }
        .validate()
        .is_err()
    );
    assert!(
        Policy {
            max_mib: 1048577,
            max_days: 30
        }
        .validate()
        .is_err()
    );
    assert!(
        Policy {
            max_mib: 64,
            max_days: 3651
        }
        .validate()
        .is_err()
    );
}
#[test]
fn settings_migrate_validate_and_round_trip() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.toml");
    fs::write(&path, "version = 1").unwrap();
    let mut s = settings::read(&path).unwrap();
    assert_eq!(s.disk_cache.max_mib, 10240);
    assert_eq!(s.disk_cache.max_days, 30);
    s.disk_cache = Policy {
        max_mib: 256,
        max_days: 7,
    };
    settings::write(&path, &s).unwrap();
    let s = settings::read(&path).unwrap();
    assert_eq!(s.disk_cache.max_mib, 256);
    assert_eq!(s.disk_cache.max_days, 7);
    fs::write(&path, "version=1\n[disk_cache]\nmax_days=0").unwrap();
    assert!(settings::read(&path).is_err());
    let example = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/interfaces/examples/settings-v1.toml");
    settings::read(&example).unwrap();
}
#[test]
fn manual_worker_cleanup_preserves_live_document_and_regenerates_after_expunging() {
    let temp = tempfile::tempdir().unwrap();
    let images = temp.path().join("images");
    fs::create_dir(&images).unwrap();
    common::image(&images.join("1.png"), [20, 40, 60, 255]);
    let root = temp.path().join("cache");
    let doc = Document::open(&images, root.clone()).unwrap();
    let frame = doc.render(0, 512).unwrap();
    let mut studio = Studio::with_program(true, None, env!("CARGO_BIN_EXE_pdf-presenter").into());
    studio.cache_path = root.clone();
    studio.initialized = true;
    let clean = |s: &mut Studio| {
        s.clean_cache(true);
        assert!(s.cache_busy);
        let deadline = Instant::now() + Duration::from_secs(10);
        while s.cache_busy {
            s.poll();
            assert!(Instant::now() < deadline, "{}", s.cache_status);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            s.cache_status.starts_with("Disk cache:"),
            "{}",
            s.cache_status
        );
    };
    clean(&mut studio);
    assert!(frame.info.path.exists());
    drop(doc);
    clean(&mut studio);
    assert!(!frame.info.path.exists());
    let doc = Document::open(&images, root).unwrap();
    assert_eq!(frame.rgba, doc.render(0, 512).unwrap().rgba);
}

#[test]
fn changed_policy_triggers_background_expiration_and_errors_are_reported() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("cache");
    let expired = entry(&root, 1, 0, 30, 40);
    let fresh = entry(&root, 1, 1, 30, 0);
    let mut s = Studio::with_program(true, None, env!("CARGO_BIN_EXE_pdf-presenter").into());
    s.cache_path = root;
    s.initialized = true;
    s.remember_cache_policy();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !s.cache_status.starts_with("Disk cache:") {
        s.poll();
        assert!(Instant::now() < deadline, "{}", s.cache_status);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!expired.exists());
    assert!(fresh.exists());
    s.cache_path = fresh;
    s.clean_cache(true);
    while s.cache_busy {
        s.poll();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        s.cache_status.contains("not a regular directory"),
        "{}",
        s.cache_status
    );
}
