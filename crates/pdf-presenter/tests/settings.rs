use pdf_presenter::settings::{self, Layout, Routes, Settings};
#[test]
fn settings_roundtrip_preserves_project_overrides_and_hidden_bars() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("nested/settings.toml");
    assert_eq!(settings::read(&path).unwrap().version, 1);
    let mut settings = Settings::default();
    settings.layout.notes.size = 42.;
    settings.buffer_mib = 2048;
    settings.auto_show = false;
    settings.theme = pdf_presenter::theme::Theme::CorporateLight;
    settings.language = pdf_presenter::localization::Language::Fr;
    let mut project = Layout::default();
    project.current.show = false;
    project.header = false;
    project.footer = false;
    project.elapsed_position = settings::TimerPosition::AboveSlides;
    project.clock_position = settings::TimerPosition::BelowNotes;
    project.slide_progress = true;
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
    assert_eq!(loaded.buffer_mib, 2048);
    assert!(!loaded.auto_show);
    assert_eq!(loaded.theme, settings.theme);
    assert_eq!(loaded.language, settings.language);
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
    s.buffer_mib = 128;
    assert!(s.validate().is_err());
    s.buffer_mib = 1024;
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
    assert_eq!(settings::read(&p).unwrap().buffer_mib, 1024);
    assert!(settings::read(&p).unwrap().auto_show);
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

#[test]
fn legacy_layouts_default_timer_positions_and_progress_is_opt_in() {
    let legacy: Settings = toml::from_str("version=1\n[layout]\nheader=false").unwrap();
    assert_eq!(
        legacy.layout.elapsed_position,
        settings::TimerPosition::Header
    );
    assert_eq!(
        legacy.layout.clock_position,
        settings::TimerPosition::Header
    );
    assert!(!legacy.layout.slide_progress);
    assert!(toml::from_str::<Settings>("[layout]\nclock_position='anywhere'").is_err());
    let example: Settings = toml::from_str(include_str!(
        "../../../docs/interfaces/examples/settings-v1.toml"
    ))
    .unwrap();
    example.validate().unwrap();
    assert_eq!(
        example.layout,
        toml::from_str::<Settings>(&toml::to_string(&example).unwrap())
            .unwrap()
            .layout
    );
}
