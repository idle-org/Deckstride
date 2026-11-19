use pdf_presenter::settings::{self, Layout, Routes, Settings};
#[test]
fn settings_roundtrip_preserves_project_overrides_and_hidden_bars() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("nested/settings.toml");
    assert_eq!(settings::read(&path).unwrap().version, 1);
    let mut settings = Settings::default();
    settings.layout.notes.size = 42.;
    let mut project = Layout::default();
    project.current.show = false;
    project.header = false;
    project.footer = false;
    settings.projects.insert("show".into(), project.clone());
    settings.routes = Routes {
        presenter: vec!["one".into(), "two".into()],
        presentation: vec!["three".into()],
    };
    settings::write(&path, &settings).unwrap();
    let loaded = settings::read(&path).unwrap();
    assert_eq!(loaded.for_project("show"), project);
    assert_eq!(loaded.for_project("other").notes.size, 42.);
    assert_eq!(loaded.routes, settings.routes);
    let bytes = std::fs::read(&path).unwrap();
    settings.version = 2;
    assert!(settings::write(&path, &settings).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
#[test]
fn settings_reject_invalid_versions_ranges_and_duplicate_roles() {
    let mut s = Settings {
        version: 2,
        ..Default::default()
    };
    assert!(s.validate().is_err());
    s.version = 1;
    s.layout.clock.size = f32::NAN;
    assert!(s.validate().is_err());
    s.layout = Default::default();
    s.routes.presenter = vec!["x".into(), "x".into()];
    assert!(s.validate().is_err());
    s.routes.presenter.pop();
    s.routes.presentation = vec!["x".into()];
    assert!(s.validate().is_err());
    let temp = tempfile::tempdir().unwrap();
    let p = temp.path().join("bad.toml");
    std::fs::write(&p, "version = 99").unwrap();
    assert!(settings::read(&p).is_err());
    std::fs::write(&p, "invalid ]").unwrap();
    assert!(settings::read(&p).is_err());
    std::fs::write(&p, "version=1").unwrap();
    assert!(settings::read(&p).unwrap().layout.header);
}
#[test]
fn monitor_recovery_preserves_survivors_and_distinct_groups() {
    let routes = Routes {
        presenter: vec!["a".into(), "b".into()],
        presentation: vec!["c".into()],
    };
    assert_eq!(
        routes.resolve(&["b".into(), "c".into()], 0),
        Routes {
            presenter: vec!["b".into()],
            presentation: vec!["c".into()]
        }
    );
    let one = routes.resolve(&["c".into()], 0);
    assert_eq!(one.presenter, vec!["c"]);
    assert!(one.presentation.is_empty());
    assert!(routes.resolve(&[], 0).presenter.is_empty());
    let fallback = Routes::default().resolve(&["a".into(), "b".into(), "c".into()], 1);
    assert_eq!(fallback.presenter, vec!["b"]);
    assert_eq!(fallback.presentation, vec!["a"]);
    let rehearsal = Routes {
        presenter: vec!["a".into()],
        presentation: vec![],
    };
    assert!(
        rehearsal
            .resolve(&["a".into(), "b".into()], 0)
            .presentation
            .is_empty()
    );
}
