use pdf_presenter::settings::{self, Layout, Routes, Settings};

#[test]
fn every_interface_language_round_trips_through_settings_v1() {
    let temp = tempfile::tempdir().unwrap();
    for language in pdf_presenter::localization::Language::ALL {
        let path = temp.path().join(format!("{language:?}.toml"));
        let settings = Settings {
            language,
            ..Default::default()
        };
        settings::write(&path, &settings).unwrap();
        assert_eq!(settings::read(&path).unwrap().language, language);
    }
}

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
    project.previous.show = true; // Explicit opt-in survives saving and loading.
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
    s.buffer_mib = 63;
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
    assert!(!settings::read(&p).unwrap().layout.previous.show);
    assert!(!Layout::default().previous.show);
    assert_eq!(
        settings::read(&p).unwrap().buffer_mib,
        Settings::default().buffer_mib
    );
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

#[test]
fn slide_colors_migrate_roundtrip_and_reject_invalid_channels() {
    use settings::SlideColors;
    let old: Settings = toml::from_str("version = 1").unwrap();
    assert_eq!(old.slide_colors, SlideColors::default());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.toml");
    let mut s = old;
    s.slide_colors = SlideColors {
        page_background: [10, 20, 30],
        fit_padding: [40, 50, 60],
    };
    settings::write(&path, &s).unwrap();
    assert_eq!(settings::read(&path).unwrap().slide_colors, s.slide_colors);
    let state = pdf_presenter::output::HostState {
        slide_colors: s.slide_colors,
        ..Default::default()
    };
    let restored: pdf_presenter::output::HostState =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    assert_eq!(restored.slide_colors, s.slide_colors);
    for value in [
        "[256, 0, 0]",
        "[-1, 0, 0]",
        "[0, 0]",
        "[0, 0, 0, 0]",
        "[0.5, 0, 0]",
    ] {
        assert!(
            toml::from_str::<Settings>(&format!(
                "version=1\n[slide_colors]\npage_background={value}"
            ))
            .is_err()
        );
    }
}

#[test]
fn automatic_buffer_budget_softens_floor_under_memory_pressure() {
    let budget = |mib: u64| settings::buffer_budget_mib(Some(mib * 1024 * 1024));
    assert_eq!(budget(8192), 1228); // 15% on a roomy system.
    assert_eq!(budget(3072), 512); // Soft floor, still below 20%.
    assert_eq!(budget(2048), 409); // Floor yields to memory pressure.
    assert_eq!(budget(1024), 204);
    assert_eq!(budget(256), 64);
    assert_eq!(budget(0), 64);
    assert_eq!(budget(1024 * 1024), 16384);
    assert_eq!(settings::buffer_budget_mib(None), 256);
    assert_eq!(settings::buffer_budget_mib(Some(u64::MAX)), 16384);
    let legacy: Settings = toml::from_str("version=1\nbuffer_mib=1024").unwrap();
    assert_eq!(legacy.buffer_mib, 1024); // Explicit saved budgets remain authoritative.
    for value in [64, 204, 512, 16384] {
        Settings {
            buffer_mib: value,
            ..Default::default()
        }
        .validate()
        .unwrap();
    }
}

#[test]
fn side_timer_positions_and_large_sizes_roundtrip() {
    for position in settings::TimerPosition::ALL {
        let mut s = Settings::default();
        s.layout.elapsed_position = position;
        s.layout.clock_position = position;
        s.layout.elapsed.size = 320.;
        s.layout.clock.size = 320.;
        s.validate().unwrap();
        let restored: Settings = toml::from_str(&toml::to_string(&s).unwrap()).unwrap();
        assert_eq!(restored.layout, s.layout);
        s.layout.elapsed.size = 321.;
        assert!(s.validate().is_err());
    }
}

#[test]
fn timer_and_progress_settings_migrate_validate_and_roundtrip() {
    let legacy: Settings = toml::from_str("version=1").unwrap();
    assert_eq!(legacy.layout.timer_mode, settings::TimerMode::Elapsed);
    assert_eq!(legacy.layout.countdown_seconds, 1200);
    assert!(legacy.layout.timer_autostart);
    assert_eq!(legacy.layout.slide_progress_height, 24.);
    let mut s = legacy;
    s.layout.timer_mode = settings::TimerMode::Countdown;
    s.layout.countdown_seconds = 90;
    s.layout.timer_autostart = false;
    s.layout.slide_progress_height = 120.;
    s.projects.insert("demo".into(), s.layout.clone());
    s.validate().unwrap();
    let loaded: Settings = toml::from_str(&toml::to_string(&s).unwrap()).unwrap();
    assert_eq!(s.layout, loaded.layout);
    assert_eq!(s.projects, loaded.projects);
    for duration in [0, 86401] {
        s.layout.countdown_seconds = duration;
        assert!(s.validate().is_err());
    }
    s.layout.countdown_seconds = 1;
    for height in [7., 121., f32::NAN] {
        s.layout.slide_progress_height = height;
        assert!(s.validate().is_err());
    }
    assert!(toml::from_str::<Settings>("[layout]\ntimer_mode='unknown'").is_err());
}

#[test]
fn display_layout_profiles_roundtrip_migrate_and_validate() {
    use settings::DisplayLayouts;
    let mut s: Settings =
        toml::from_str("version=1\n[layout.notes]\nshow=true\nsize=40.0").unwrap();
    assert!(s.display_layouts.is_empty());
    let mut legacy_project = s.layout.clone();
    legacy_project.notes.size = 55.;
    s.projects.insert("show".into(), legacy_project.clone());
    assert_eq!(s.for_display("main#1", "show"), legacy_project);
    let mut alternate = Layout::default();
    alternate.current.size = 180.;
    alternate.notes.show = false;
    alternate.header = false;
    alternate.clock_position = settings::TimerPosition::RightOfSlides;
    let mut project = alternate.clone();
    project.slide_progress = true;
    s.display_layouts.insert(
        "Side screen#1".into(),
        DisplayLayouts {
            layout: alternate.clone(),
            projects: [("show".into(), project.clone())].into(),
        },
    );
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.toml");
    settings::write(&path, &s).unwrap();
    let loaded = settings::read(&path).unwrap();
    assert_eq!(loaded.for_display("Side screen#1", "show"), project);
    assert_eq!(loaded.for_display("Side screen#1", "other"), alternate);
    assert_eq!(loaded.for_display("main#1", "show"), legacy_project);
    let mut bad = loaded.clone();
    bad.display_layouts
        .get_mut("Side screen#1")
        .unwrap()
        .layout
        .notes
        .size = f32::NAN;
    assert!(bad.validate().is_err());
    bad = loaded.clone();
    bad.display_layouts
        .get_mut("Side screen#1")
        .unwrap()
        .projects
        .get_mut("show")
        .unwrap()
        .current
        .size = 0.;
    assert!(bad.validate().is_err());
    bad = loaded.clone();
    bad.display_layouts.insert("".into(), Default::default());
    assert!(bad.validate().is_err());
    bad = loaded.clone();
    for i in 0..128 {
        bad.display_layouts
            .insert(format!("display-{i}"), Default::default());
    }
    assert!(bad.validate().is_err());
    bad = loaded;
    bad.display_layouts
        .get_mut("Side screen#1")
        .unwrap()
        .projects = (0..4097)
        .map(|i| (format!("project-{i}"), Layout::default()))
        .collect();
    assert!(bad.validate().is_err());
}

#[test]
fn screen_modes_migrate_validate_and_roundtrip() {
    use settings::ScreenMode::{AlwaysFullscreen, AlwaysWindowed, FollowGlobal};
    let old: Settings = toml::from_str("version = 1").unwrap();
    assert!(old.screen_modes.is_empty());
    for global in [false, true] {
        assert_eq!(FollowGlobal.fullscreen(global), global);
        assert!(AlwaysFullscreen.fullscreen(global));
        assert!(!AlwaysWindowed.fullscreen(global));
    }
    let mut s = old;
    s.screen_modes = [
        ("one".into(), AlwaysFullscreen),
        ("two".into(), AlwaysWindowed),
        ("three".into(), FollowGlobal),
    ]
    .into();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.toml");
    settings::write(&path, &s).unwrap();
    assert_eq!(settings::read(&path).unwrap().screen_modes, s.screen_modes);
    assert!(toml::from_str::<Settings>("[screen_modes]\none = 'invalid'").is_err());
    for id in [String::new(), "a".repeat(1025)] {
        s.screen_modes.insert(id.clone(), AlwaysWindowed);
        assert!(s.validate().is_err());
        s.screen_modes.remove(&id);
    }
    s.screen_modes = (0..129)
        .map(|i| (i.to_string(), AlwaysFullscreen))
        .collect();
    assert!(s.validate().is_err());
    s.screen_modes.remove("128");
    assert!(s.validate().is_ok());
}

#[test]
fn output_screen_modes_roundtrip_and_fall_back_to_global() {
    use pdf_presenter::output::HostState;
    let state = HostState {
        fullscreen: true,
        screen_fullscreen: [("one".into(), false), ("two".into(), true)].into(),
        ..Default::default()
    };
    let loaded: HostState = serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    assert!(!loaded.fullscreen_on("one"));
    assert!(loaded.fullscreen_on("two"));
    assert!(loaded.fullscreen_on("unknown"));
    let mut legacy = serde_json::to_value(state).unwrap();
    legacy.as_object_mut().unwrap().remove("screen_fullscreen");
    assert!(
        serde_json::from_value::<HostState>(legacy)
            .unwrap()
            .fullscreen_on("one")
    );
}

#[test]
fn single_screen_preferences_are_additive_bounded_and_roundtrip() {
    let old: Settings = toml::from_str("version = 1").unwrap();
    assert!(old.single_screens.is_empty());
    let mut s = old;
    s.single_screens = vec!["one".into(), "disconnected#1".into()];
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.toml");
    settings::write(&path, &s).unwrap();
    assert_eq!(
        settings::read(&path).unwrap().single_screens,
        s.single_screens
    );
    for ids in [
        vec!["".into()],
        vec!["a".repeat(1025)],
        vec!["one".into(), "one".into()],
        (0..33).map(|i| i.to_string()).collect(),
    ] {
        s.single_screens = ids;
        assert!(s.validate().is_err());
    }
    s.single_screens = (0..32).map(|i| i.to_string()).collect();
    assert!(s.validate().is_ok());
}
