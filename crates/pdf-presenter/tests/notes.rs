mod common;
use pdf_presenter::{
    document::Document,
    notes,
    studio::{Action, Studio},
};
use std::time::{Duration, Instant};

#[test]
fn pdf_sidecar_precedence_creation_all_pages_and_source_preservation() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("presentation.X.pdf");
    let cache = temp.path().join("cache");
    common::pdf(&source);
    let original = std::fs::read(&source).unwrap();
    let doc = Document::open(&source, cache.clone()).unwrap();
    assert_eq!(doc.info.notes, vec!["Speaker note\n", ""]);
    let snapshot = notes::prepare(&source, cache.clone(), &doc.info.sha256).unwrap();
    assert_eq!(snapshot.path, temp.path().join("presentation.X.notes.toml"));
    let file: toml::Value =
        toml::from_str(&std::fs::read_to_string(&snapshot.path).unwrap()).unwrap();
    assert_eq!(file["version"].as_integer(), Some(1));
    assert_eq!(file["notes"]["1"].as_str(), Some("Speaker note\n"));
    assert_eq!(file["notes"]["2"].as_str(), Some(""));
    let saved = notes::save(
        &source,
        cache.clone(),
        &doc.info.sha256,
        &snapshot.revision,
        1,
        "Second\n\"quoted\" é漢字".into(),
    )
    .unwrap();
    assert_eq!(saved.notes[0], "Speaker note\n");
    let saved = notes::save(
        &source,
        cache.clone(),
        &doc.info.sha256,
        &saved.revision,
        0,
        String::new(),
    )
    .unwrap();
    assert_eq!(
        Document::open(&source, cache.clone()).unwrap().info.notes,
        saved.notes
    );
    assert_eq!(saved.notes[0], ""); // An explicit empty note hides the PDF annotation.
    std::fs::write(&snapshot.path, "version=1\n[notes]\n\"2\"='Override'").unwrap();
    let reopened = Document::open(&source, cache).unwrap();
    assert_eq!(reopened.info.notes, vec!["Speaker note\n", "Override"]);
    assert_eq!(original, std::fs::read(&source).unwrap());
}

#[test]
fn editor_refuses_conflicts_invalid_sidecars_and_changed_sources() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("talk.pdf");
    let cache = temp.path().join("cache");
    common::pdf(&source);
    let sha = Document::open(&source, cache.clone()).unwrap().info.sha256;
    let snapshot = notes::prepare(&source, cache.clone(), &sha).unwrap();
    let external = "version=1\n[notes]\n\"1\"='External edit'\n\"99\"='Keep unknown page'";
    std::fs::write(&snapshot.path, external).unwrap();
    assert!(
        notes::save(
            &source,
            cache.clone(),
            &sha,
            &snapshot.revision,
            0,
            "Old edit".into()
        )
        .unwrap_err()
        .contains("changed on disk")
    );
    assert_eq!(std::fs::read_to_string(&snapshot.path).unwrap(), external);
    let fresh = notes::prepare(&source, cache.clone(), &sha).unwrap();
    notes::save(
        &source,
        cache.clone(),
        &sha,
        &fresh.revision,
        1,
        "New".into(),
    )
    .unwrap();
    assert!(
        std::fs::read_to_string(&snapshot.path)
            .unwrap()
            .contains("Keep unknown page")
    );
    for bad in [
        "not toml ]",
        "version=2\n[notes]",
        "version=1\nextra=true\n[notes]",
    ] {
        std::fs::write(&snapshot.path, bad).unwrap();
        let doc = Document::open(&source, cache.clone()).unwrap();
        assert_eq!(doc.info.notes[0], "Speaker note\n");
        assert!(!doc.info.warnings.is_empty());
        assert!(notes::prepare(&source, cache.clone(), &sha).is_err());
        assert_eq!(std::fs::read_to_string(&snapshot.path).unwrap(), bad);
    }
    std::fs::remove_file(&snapshot.path).unwrap();
    assert!(
        notes::save(
            &source,
            cache.clone(),
            &sha,
            &fresh.revision,
            0,
            "No file".into()
        )
        .is_err()
    );
    assert!(notes::prepare(&source, cache.clone(), "wrong source SHA").is_err());
    let fresh = notes::prepare(&source, cache.clone(), &sha).unwrap();
    assert!(
        notes::save(
            &source,
            cache.clone(),
            &sha,
            &fresh.revision,
            7,
            "No slide".into()
        )
        .is_err()
    );
    assert!(
        notes::save(
            &source,
            cache,
            &sha,
            &fresh.revision,
            0,
            "x".repeat(notes::MAX_NOTE + 1)
        )
        .is_err()
    );
}

#[test]
fn image_folder_sidecar_uses_filenames_and_keeps_original_notes() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("Images");
    std::fs::create_dir(&source).unwrap();
    common::image(&source.join("slide2.png"), [0, 0, 0, 255]);
    common::image(&source.join("slide10.png"), [0, 0, 0, 255]);
    std::fs::write(source.join("slide2.txt"), "Original").unwrap();
    std::fs::write(source.join("notes.toml"), "\"slide10.png\"='Global'").unwrap();
    let cache = temp.path().join("cache");
    let sha = Document::open(&source, cache.clone()).unwrap().info.sha256;
    let snapshot = notes::prepare(&source, cache.clone(), &sha).unwrap();
    assert_eq!(snapshot.path, temp.path().join("Images.notes.toml"));
    let saved = notes::save(
        &source,
        cache.clone(),
        &sha,
        &snapshot.revision,
        0,
        "Edited".into(),
    )
    .unwrap();
    assert_eq!(saved.notes, vec!["Edited", "Global"]);
    assert_eq!(
        Document::open(&source, cache).unwrap().info.notes,
        saved.notes
    );
    assert_eq!(
        std::fs::read_to_string(source.join("slide2.txt")).unwrap(),
        "Original"
    );
}

#[cfg(unix)]
#[test]
fn symlink_sidecars_are_never_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("talk.pdf");
    common::pdf(&source);
    let cache = temp.path().join("cache");
    let sha = Document::open(&source, cache.clone()).unwrap().info.sha256;
    let target = temp.path().join("external");
    std::fs::write(&target, "untouched").unwrap();
    std::os::unix::fs::symlink(&target, notes::sidecar_path(&source)).unwrap();
    assert!(notes::prepare(&source, cache, &sha).is_err());
    assert_eq!(std::fs::read_to_string(target).unwrap(), "untouched");
}

#[test]
fn editor_worker_saves_captured_page_and_keeps_draft_after_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("talk.pdf");
    common::pdf(&source);
    let cache = temp.path().join("cache");
    let info = Document::open(&source, cache.clone()).unwrap().info;
    let mut s = Studio::with_program(true, None, env!("CARGO_BIN_EXE_pdf-presenter").into());
    s.cache_path = cache;
    s.deck = Some(info);
    s.initialized = true;
    s.requested = 1;
    s.execute(Action::EditNotes);
    fn finish(s: &mut Studio) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while s.notes_editor.as_ref().unwrap().busy {
            s.poll();
            assert!(Instant::now() < deadline, "notes editor stalled");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    finish(&mut s);
    assert!(s.notes_editor.as_ref().unwrap().error.is_none());
    s.notes_editor.as_mut().unwrap().text = "Edited second slide".into();
    s.execute(Action::EditNotes); // Double-click again must not discard drafts.
    assert_eq!(s.notes_editor.as_ref().unwrap().text, "Edited second slide");
    s.execute(Action::Open(source.clone()));
    assert!(s.status.contains("Close the notes editor"));
    s.requested = 0; // Editor remains bound to the slide captured when it opened.
    s.execute(Action::SaveNotes);
    finish(&mut s);
    assert_eq!(s.deck.as_ref().unwrap().notes[1], "Edited second slide");
    assert_eq!(s.deck.as_ref().unwrap().notes[0], "Speaker note\n");
    let file = s
        .notes_editor
        .as_ref()
        .unwrap()
        .snapshot
        .as_ref()
        .unwrap()
        .path
        .clone();
    std::fs::write(&file, "version=1\n[notes]\n\"2\"='External'").unwrap();
    s.notes_editor.as_mut().unwrap().text = "My unsaved draft".into();
    s.execute(Action::SaveNotes);
    finish(&mut s);
    assert!(s.notes_editor.as_ref().unwrap().error.is_some());
    assert_eq!(s.notes_editor.as_ref().unwrap().text, "My unsaved draft");
    s.execute(Action::ReloadNotes);
    finish(&mut s);
    assert_eq!(s.notes_editor.as_ref().unwrap().text, "External");
}

#[test]
fn partial_sidecar_cannot_exceed_combined_notes_limit_with_fallback_text() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("Slides");
    std::fs::create_dir(&source).unwrap();
    let text = "x".repeat(notes::MAX_NOTE);
    for page in 1..=33 {
        common::image(&source.join(format!("slide{page}.png")), [0, 0, 0, 255]);
        if page <= 32 {
            std::fs::write(source.join(format!("slide{page}.txt")), &text).unwrap();
        }
    }
    let cache = temp.path().join("cache");
    let sha = Document::open(&source, cache.clone()).unwrap().info.sha256;
    let sidecar = notes::sidecar_path(&source);
    let empty = "version=1\n[notes]\n";
    std::fs::write(&sidecar, empty).unwrap();
    let snapshot = notes::prepare(&source, cache.clone(), &sha).unwrap();
    assert!(
        notes::save(
            &source,
            cache.clone(),
            &sha,
            &snapshot.revision,
            32,
            "x".into()
        )
        .unwrap_err()
        .contains("Combined notes")
    );
    assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), empty);
    std::fs::write(&sidecar, "version=1\n[notes]\n\"slide33.png\"='x'").unwrap();
    assert!(
        notes::prepare(&source, cache, &sha)
            .unwrap_err()
            .contains("Combined notes")
    );
}
