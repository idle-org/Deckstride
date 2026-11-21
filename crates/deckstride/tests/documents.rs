mod common;
use deckstride::{
    document::{Document, natural_cmp, read_frame, validate_frame},
    settings,
};
#[test]
fn natural_order_handles_arbitrary_digit_lengths_and_ties() {
    let mut names = vec![
        "slide10.png",
        "slide2.png",
        "slide01.png",
        "slide1.png",
        "slide999999999999999999999999.png",
        "é2.png",
        "é10.png",
    ];
    names.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(
        names,
        vec![
            "slide01.png",
            "slide1.png",
            "slide2.png",
            "slide10.png",
            "slide999999999999999999999999.png",
            "é2.png",
            "é10.png"
        ]
    );
    assert!(natural_cmp("a", "a2").is_lt());
    assert!(natural_cmp("B", "a").is_gt());
}
#[test]
fn folder_notes_order_and_cache_integrity() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("deck");
    std::fs::create_dir(&dir).unwrap();
    common::image(&dir.join("slide10.png"), [255, 0, 0, 255]);
    common::image(&dir.join("slide2.png"), [0, 255, 0, 255]);
    std::fs::write(
        dir.join("notes.json"),
        r#"{"slide2.png":"global","slide10.png":"last"}"#,
    )
    .unwrap();
    std::fs::write(dir.join("slide2.txt"), "sidecar").unwrap();
    let doc = Document::open(&dir, temp.path().join("cache")).unwrap();
    assert_eq!(doc.info.names, vec!["slide2.png", "slide10.png"]);
    assert_eq!(doc.info.notes, vec!["sidecar", "last"]);
    let before = std::fs::read(dir.join("slide2.png")).unwrap();
    let first = doc.render(0, 320).unwrap();
    let cached = doc.render(0, 320).unwrap();
    assert_eq!(first.rgba, cached.rgba);
    assert_eq!(read_frame(&first.info.path).unwrap().2, first.rgba);
    let mut damaged = std::fs::read(&first.info.path).unwrap();
    damaged[20] ^= 1;
    std::fs::write(&first.info.path, damaged).unwrap();
    assert!(read_frame(&first.info.path).is_err());
    let repaired = doc.render(0, 320).unwrap();
    assert_eq!(repaired.rgba, first.rgba);
    assert_eq!(std::fs::read(dir.join("slide2.png")).unwrap(), before);
    assert!(doc.render(5, 320).is_err());
}
#[test]
fn missing_conflicting_and_bad_notes_do_not_block_images() {
    let temp = tempfile::tempdir().unwrap();
    common::image(&temp.path().join("a.png"), [0, 0, 255, 255]);
    std::fs::write(temp.path().join("notes.toml"), "\"a.png\" = \"from toml\"").unwrap();
    let load = || Document::open(temp.path(), temp.path().join("cache")).unwrap();
    assert_eq!(load().info.notes[0], "from toml");
    std::fs::write(temp.path().join("a.txt"), "").unwrap();
    assert_eq!(load().info.notes[0], "");
    std::fs::write(temp.path().join("notes.json"), "broken").unwrap();
    assert!(!load().info.warnings.is_empty());
    std::fs::remove_file(temp.path().join("notes.toml")).unwrap();
    assert!(!load().info.warnings.is_empty());
    std::fs::write(temp.path().join("a.txt"), [255]).unwrap();
    assert!(load().info.warnings.len() >= 2);
}
#[test]
fn pdf_renders_actual_pages_and_extracts_annotation_notes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.pdf");
    common::pdf(&path);
    let bytes = std::fs::read(&path).unwrap();
    let doc = Document::open(&path, temp.path().join("cache")).unwrap();
    assert_eq!(doc.info.names.len(), 2);
    assert!(doc.info.notes[0].contains("Speaker note"));
    let first = doc.render(0, 320).unwrap();
    let second = doc.render(1, 320).unwrap();
    assert_ne!(first.rgba, second.rgba);
    assert_eq!((first.info.width, first.info.height), (320, 180));
    assert_eq!(bytes, std::fs::read(path).unwrap());
}
#[test]
fn invalid_sources_and_frames_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    assert!(Document::open(temp.path(), temp.path().join("cache")).is_err());
    let path = temp.path().join("bad.pdf");
    std::fs::write(&path, b"not a pdf").unwrap();
    assert!(Document::open(&path, temp.path().join("cache")).is_err());
    std::fs::write(&path, b"small").unwrap();
    assert!(read_frame(&path).is_err());
    assert!(validate_frame(0, 1, 0).is_err());
    assert!(validate_frame(1, 1, 3).is_err());
    assert!(validate_frame(4097, 1, 4097 * 4).is_err());
    assert!(validate_frame(1, 1, 4).is_ok());
    assert_ne!(
        settings::project_key(&path),
        settings::project_key(temp.path())
    );
}

#[test]
fn fast_open_defers_notes_and_versioned_examples_are_valid() {
    let temp = tempfile::tempdir().unwrap();
    common::image(&temp.path().join("slide.png"), [0, 255, 0, 255]);
    std::fs::write(temp.path().join("slide.txt"), "Deferred note").unwrap();
    let fast = Document::open_slides(temp.path(), temp.path().join("cache")).unwrap();
    assert_eq!(fast.info.notes, vec![""]);
    assert!(!fast.render(0, 320).unwrap().rgba.is_empty());
    let notes = Document::open(temp.path(), temp.path().join("cache")).unwrap();
    assert_eq!(notes.info.notes, vec!["Deferred note"]);
    let settings: settings::Settings = toml::from_str(include_str!(
        "../../../docs/interfaces/examples/settings-v1.toml"
    ))
    .unwrap();
    settings.validate().unwrap();
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/interfaces/examples/extraction-v1.json"
    ))
    .unwrap();
    assert_eq!(manifest["version"], 1);
    assert_eq!(manifest["width"], 1280);
}
