use deckstride::{
    document::{DeckInfo, Frame, FrameInfo},
    presenter_ui::{self, UiState},
    studio::{Action, Monitor, Studio},
};
use egui::{Context, Pos2, RawInput, Rect, Vec2};
use std::{collections::HashMap, sync::Arc};
fn studio() -> Studio {
    let mut s = Studio::new(true, None);
    s.initialized = true;
    s.deck = Some(DeckInfo {
        path: "demo.pdf".into(),
        key: "demo".into(),
        title: "Demo".into(),
        names: (1..=30).map(|i| format!("Slide {i}")).collect(),
        notes: vec!["A long note with unbrokenwordsandUnicodeé漢字. ".repeat(80); 30],
        warnings: vec![],
        sha256: "test".into(),
    });
    s.monitors = vec![
        Monitor {
            id: "one".into(),
            name: "Primary".into(),
            origin: (0, 0),
            size: (1920, 1080),
        },
        Monitor {
            id: "two".into(),
            name: "Secondary".into(),
            origin: (1920, 0),
            size: (1920, 1080),
        },
    ];
    s.routes.presenter = vec!["one".into()];
    s.routes.presentation = vec!["two".into()];
    s.draft_routes = s.routes.clone();
    s.frames = HashMap::from([(
        (0, 1280),
        Arc::new(Frame {
            info: FrameInfo {
                page: 0,
                width: 64,
                height: 36,
                path: "cached.rgba".into(),
                sha256: "test".into(),
            },
            rgba: vec![255; 64 * 36 * 4],
        }),
    )]);
    s
}
fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}
#[test]
fn native_ui_renders_real_previews_notes_and_all_control_panels() {
    let ctx = Context::default();
    presenter_ui::theme(&ctx);
    let mut s = studio();
    let mut ui = UiState::default();
    for size in [Vec2::new(1280., 900.), Vec2::new(800., 600.)] {
        for panel in 0..5 {
            s.menu_open = panel == 1;
            s.layout_open = panel == 2;
            s.displays_open = panel == 3;
            s.preferences_open = panel == 4;
            s.jump = "12".into();
            let output = ctx.run_ui(input(size), |root| {
                presenter_ui::presenter(root, &mut s, &mut ui)
            });
            assert!(!output.shapes.is_empty());
            assert!(
                !ctx.tessellate(output.shapes, output.pixels_per_point)
                    .is_empty()
            );
        }
    }
    s.menu_open = false;
    s.layout_open = false;
    s.displays_open = false;
    s.layout.previous.show = false;
    s.layout.current.show = false;
    s.layout.next.show = false;
    s.layout.header = false;
    s.layout.footer = false;
    let output = ctx.run_ui(input(Vec2::new(1000., 700.)), |root| {
        presenter_ui::presenter(root, &mut s, &mut ui)
    });
    assert!(!output.shapes.is_empty());
    s.layout.notes.show = false;
    let output = ctx.run_ui(input(Vec2::new(1000., 700.)), |root| {
        presenter_ui::presenter(root, &mut s, &mut ui)
    });
    assert!(!output.shapes.is_empty());
}
#[test]
fn browser_virtualizes_thumbnails_without_changing_presented_slide() {
    let ctx = Context::default();
    let mut s = studio();
    let mut ui = UiState::default();
    s.requested = 3;
    s.browser_selected = 8;
    s.deck.as_mut().unwrap().names = (0..1000).map(|i| format!("Slide {i}")).collect();
    for size in [100., 220., 400.] {
        s.settings.thumbnail_size = size;
        let output = ctx.run_ui(input(Vec2::new(900., 650.)), |root| {
            presenter_ui::browser(root, &mut s, &mut ui)
        });
        assert!(!output.shapes.is_empty());
        assert!(!s.visible_thumbs.is_empty());
        assert!(s.visible_thumbs.len() < 100);
        assert_eq!(s.requested, 3);
    }
}
#[test]
fn shared_actions_preserve_blanking_and_save_scope() {
    let mut s = studio();
    s.blanked = true;
    s.execute(Action::Go(5));
    assert_eq!(s.requested, 5);
    assert!(s.blanked);
    s.execute(Action::Go(99));
    assert_eq!(s.requested, 5);
    s.execute(Action::Blank);
    assert!(!s.blanked);
    s.layout.current.show = false;
    s.execute(Action::SaveProject);
    assert!(!s.settings.projects["demo"].current.show);
    s.execute(Action::SaveGlobal);
    assert!(!s.settings.layout.current.show);
    s.execute(Action::UseGlobal);
    assert!(!s.settings.projects.contains_key("demo"));
    s.execute(Action::OpenBrowser);
    assert_eq!(s.browser_selected, 5);
    assert!(!s.change_on_click);
    s.execute(Action::CloseBrowser);
    assert!(!s.browser_open);
    s.execute(Action::ToggleFullscreen);
    assert!(!s.presenter_fullscreen());
    s.execute(Action::KeepRoutes);
    assert!(!s.blanked);
    assert!(!s.settings.presenter_fullscreen);
    s.draft_routes.presenter.clear();
    s.execute(Action::ApplyRoutes);
    assert!(s.status.contains("presenter"));
    s.draft_routes = s.routes.clone();
    s.execute(Action::ApplyRoutes);
    assert!(!s.blanked);
    s.execute(Action::Refresh);
    assert!(s.dirty_routes);
    s.execute(Action::Exit);
    assert!(s.quit_confirmation && !s.close_requested);
    s.execute(Action::ConfirmExit);
    assert!(s.close_requested);
}

#[test]
fn lost_operator_screen_never_becomes_notes_on_previous_public_output() {
    let mut s = studio();
    s.settings.routes = s.routes.clone();
    s.set_monitors(vec![s.monitors[1].clone()], 0);
    assert!(s.routes.presenter.is_empty());
    assert!(s.privacy_hold);
    assert!(!s.blanked);
    let monitors = studio().monitors;
    s.set_monitors(monitors, 0);
    assert!(!s.privacy_hold);
    assert_eq!(s.routes.presenter, vec!["one"]);
}

#[test]
fn palettes_and_languages_apply_to_presenter_browser_and_popups() {
    use deckstride::{
        localization::{Language, apply, tr},
        theme::Theme,
    };
    let ctx = Context::default();
    let mut s = studio();
    let mut ui = UiState::default();
    for theme in Theme::ALL {
        theme.apply(&ctx);
        assert_eq!(
            ctx.global_style().visuals.text_color(),
            theme.palette().text
        );
        for language in Language::ALL {
            apply(&ctx, &language);
            if language == Language::Fr {
                assert_eq!(tr(&ctx, "Displays"), "Écrans");
            }
            s.menu_open = true;
            s.displays_open = true;
            s.layout_open = true;
            let output = ctx.run_ui(input(Vec2::new(1400., 1000.)), |root| {
                presenter_ui::presenter(root, &mut s, &mut ui)
            });
            assert!(!output.shapes.is_empty());
            let output = ctx.run_ui(input(Vec2::new(1000., 700.)), |root| {
                presenter_ui::browser(root, &mut s, &mut ui)
            });
            assert!(!output.shapes.is_empty());
        }
    }
}
#[test]
fn startup_output_is_enabled_and_disconnect_preserves_visibility() {
    let mut s = Studio::new(true, None);
    assert!(!s.blanked);
    let monitors = studio().monitors;
    s.set_monitors(monitors.clone(), 0);
    assert!(!s.blanked);
    s.set_monitors(vec![monitors[0].clone()], 0);
    assert!(!s.blanked);
    let mut opt_out = Studio::new(true, None);
    opt_out.settings.auto_show = false;
    opt_out.set_monitors(monitors, 0);
    assert!(opt_out.blanked);
}

#[test]
fn header_click_opens_slide_browser_and_menu_dropdown() {
    fn text_position(shapes: &[egui::epaint::ClippedShape], label: &str) -> Pos2 {
        fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
                _ => None,
            }
        }
        shapes.iter().find_map(|s| find(&s.shape, label)).unwrap()
    }
    let ctx = Context::default();
    let mut s = studio();
    let mut native = UiState::default();
    let size = Vec2::new(1920., 1080.);
    for label in ["All slides", "Menu"] {
        let mut output = ctx.run_ui(input(size), |ui| {
            presenter_ui::presenter(ui, &mut s, &mut native)
        });
        for _ in 0..2 {
            output = ctx.run_ui(input(size), |ui| {
                presenter_ui::presenter(ui, &mut s, &mut native)
            });
        }
        let pos = text_position(&output.shapes, label);
        for pressed in [true, false] {
            let mut raw = input(size);
            raw.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ];
            let _ = ctx.run_ui(raw, |ui| presenter_ui::presenter(ui, &mut s, &mut native));
        }
        if label == "All slides" {
            assert!(
                s.actions.iter().any(|a| matches!(a, Action::OpenBrowser)),
                "All slides click at {pos:?} must dispatch an action"
            );
        } else {
            assert!(s.menu_open, "Menu click must open the anchored dropdown");
        }
    }
}

#[test]
fn display_changes_require_confirmation_and_timeout_restores_previous_setup() {
    let mut s = studio();
    s.settings.routes = s.routes.clone();
    let original = s.routes.clone();
    s.begin_display_edit();
    s.draft_presenter_fullscreen = false;
    s.draft_presentation_fullscreen = false;
    std::mem::swap(
        &mut s.draft_routes.presenter,
        &mut s.draft_routes.presentation,
    );
    s.execute(Action::ApplyRoutes);
    assert_ne!(s.routes, original);
    assert_eq!(s.settings.routes, original);
    assert!(s.settings.presenter_fullscreen);
    assert!(!s.presenter_fullscreen());
    assert!(!s.blanked);
    s.execute(Action::Blank);
    s.execute(Action::Go(5));
    assert!(s.blanked);
    assert_eq!(s.requested, 0);
    // Render the modal even with the ordinary bars hidden.
    s.layout.header = false;
    s.layout.footer = false;
    let ctx = Context::default();
    let _ = ctx.run_ui(input(Vec2::new(1280., 720.)), |ui| {
        presenter_ui::presenter(ui, &mut s, &mut UiState::default())
    });
    s.route_trial.as_mut().unwrap().deadline = std::time::Instant::now();
    s.poll();
    assert!(s.route_trial.is_none());
    assert_eq!(s.routes, original);
    assert_eq!(s.settings.routes, original);
    assert!(s.presenter_fullscreen());
    assert!(s.blanked);
    s.execute(Action::RevertRoutes); // Idempotent recovery.
    assert_eq!(s.routes, original);
    s.begin_display_edit();
    s.draft_presenter_fullscreen = false;
    s.draft_presentation_fullscreen = false;
    s.execute(Action::ApplyRoutes);
    s.execute(Action::KeepRoutes);
    assert!(!s.settings.presenter_fullscreen);
    assert!(!s.settings.presentation_fullscreen);
}

#[test]
fn recovery_commands_are_scoped_to_the_current_trial_and_startup() {
    use deckstride::output::DisplayAction;
    let mut s = studio();
    s.settings.routes = s.routes.clone();
    let original = s.routes.clone();
    s.display_control(DisplayAction::Swap, None);
    let old_id = s.route_trial.as_ref().unwrap().id;
    s.display_control(DisplayAction::Keep, Some(old_id + 1));
    assert!(s.route_trial.is_some());
    assert_eq!(s.settings.routes, original);
    s.display_control(DisplayAction::Revert, Some(old_id));
    assert_eq!(s.routes, original);
    s.display_control(DisplayAction::Swap, None);
    let new_id = s.route_trial.as_ref().unwrap().id;
    assert_ne!(new_id, old_id);
    s.display_control(DisplayAction::Keep, Some(old_id));
    assert!(s.route_trial.is_some());
    s.display_control(DisplayAction::Keep, Some(new_id));
    assert!(s.route_trial.is_none());
    assert_eq!(s.settings.routes, s.routes);
    assert_ne!(s.routes, original);
    s.execute(Action::Go(0));
    let confirmed = s.routes.clone();
    s.display_control(DisplayAction::Swap, None);
    assert!(s.route_trial.is_none());
    assert_eq!(s.routes, confirmed);
    // Explicit display editing remains available after startup.
    s.begin_display_edit();
    s.draft_routes = original;
    s.execute(Action::ApplyRoutes);
    s.route_trial.as_mut().unwrap().deadline = std::time::Instant::now();
    s.execute(Action::KeepRoutes); // A late click must revert, never save.
    assert!(s.route_trial.is_none());
    assert_eq!(s.settings.routes, confirmed);
    assert_eq!(s.routes, confirmed);
}

#[test]
fn hotplug_cancels_a_display_trial_without_saving_the_proposal() {
    let mut s = studio();
    s.settings.routes = s.routes.clone();
    let saved = s.settings.routes.clone();
    s.execute(Action::SwapRoutes);
    assert!(s.route_trial.is_some());
    s.set_monitors(vec![s.monitors[0].clone()], 0);
    assert!(s.route_trial.is_none());
    assert_eq!(s.settings.routes, saved);
    assert!(!s.blanked);
    assert!(!s.routes.presentation.contains(&"two".to_owned()));
}

#[test]
fn output_recovery_buttons_dispatch_without_presenter_data() {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    for (label, trial) in [
        ("Keep changes", true),
        ("Revert now", true),
        ("Swap Presenter and Slides screens…", false),
    ] {
        let ctx = Context::default();
        let size = Vec2::new(1280., 720.);
        let mut action = None;
        let mut output = ctx.run_ui(input(size), |ui| {
            action = presenter_ui::output_display_recovery(ui, true, trial);
        });
        if !trial {
            for pressed in [true, false] {
                let pos = Pos2::new(300., 300.);
                let mut raw = input(size);
                raw.events = vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ];
                output = ctx.run_ui(raw, |ui| {
                    action = presenter_ui::output_display_recovery(ui, true, false);
                });
            }
        }
        for _ in 0..3 {
            output = ctx.run_ui(input(size), |ui| {
                action = presenter_ui::output_display_recovery(ui, true, trial);
            });
        }
        let pos = output
            .shapes
            .iter()
            .find_map(|s| find(&s.shape, label))
            .expect(label);
        for pressed in [true, false] {
            let mut raw = input(size);
            raw.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ];
            let _ = ctx.run_ui(raw, |ui| {
                action = presenter_ui::output_display_recovery(ui, true, trial);
            });
        }
        use deckstride::output::DisplayAction;
        assert!(matches!(
            (label, action),
            ("Keep changes", Some(DisplayAction::Keep))
                | ("Revert now", Some(DisplayAction::Revert))
                | (
                    "Swap Presenter and Slides screens…",
                    Some(DisplayAction::Swap)
                )
        ));
        let output = ctx.run_ui(input(size), |ui| {
            action = presenter_ui::output_display_recovery(ui, false, false);
        });
        assert!(action.is_none());
        assert!(
            output
                .shapes
                .iter()
                .all(|s| find(&s.shape, label).is_none())
        );
    }
}

#[test]
fn quick_settings_and_preferences_window_remain_interactive() {
    fn position(shapes: &[egui::epaint::ClippedShape], label: &str) -> Pos2 {
        fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
                _ => None,
            }
        }
        shapes
            .iter()
            .find_map(|s| find(&s.shape, label))
            .unwrap_or_else(|| panic!("Missing menu entry: {label}"))
    }
    let ctx = Context::default();
    let mut s = studio();
    s.settings.language = deckstride::localization::Language::En;
    let mut native = UiState::default();
    let mut frame = |s: &mut Studio, events| {
        let mut raw = input(Vec2::new(1920., 1080.));
        raw.events = events;
        ctx.run_ui(raw, |ui| presenter_ui::presenter(ui, s, &mut native))
    };
    let mut click = |s: &mut Studio, label: &str| {
        let mut output = frame(s, vec![]);
        for _ in 0..3 {
            output = frame(s, vec![]);
        }
        let pos = position(&output.shapes, label);
        for pressed in [true, false] {
            frame(
                s,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
    };
    click(&mut s, "Menu");
    click(&mut s, "Quit");
    assert!(
        s.actions
            .iter()
            .any(|action| matches!(action, Action::Exit)),
        "Quit should be directly reachable without scrolling at 1080p"
    );
    assert!(!s.menu_open);
    s.actions.clear();
    click(&mut s, "Menu");
    click(&mut s, "Autoplay");
    click(&mut s, "Play automatically");
    assert!(s.settings.autoplay_enabled);
    assert!(s.menu_open);
    click(&mut s, "Loop");
    assert!(s.settings.autoplay_loop);
    assert!(s.menu_open);
    click(&mut s, "Menu");
    click(&mut s, "Menu");
    click(&mut s, "Preferences…");
    assert!(s.preferences_open);
    assert!(!s.menu_open);
    click(&mut s, "Midra Dark");
    click(&mut s, "High Contrast");
    assert_eq!(s.settings.theme, deckstride::theme::Theme::HighContrast);
    assert!(s.preferences_open);
    click(&mut s, "Close");
    assert!(!s.preferences_open);
    click(&mut s, "Menu");
    let zoom_enabled = s.settings.zoom_enabled;
    click(&mut s, "Enable page zoom");
    assert_eq!(s.settings.zoom_enabled, !zoom_enabled);
    assert!(s.menu_open);
    let fitted = s.settings.zoom_keep_fitted_area;
    click(&mut s, "Keep fitted page area when zooming");
    assert_eq!(s.settings.zoom_keep_fitted_area, !fitted);
    assert!(s.menu_open);
    let auto_show = s.settings.auto_show;
    click(&mut s, "Show slides automatically when opening a document");
    assert_eq!(s.settings.auto_show, !auto_show);
    assert!(s.menu_open);
    click(&mut s, "Preferences…");
    assert!(s.preferences_open);
    assert!(!s.menu_open);
    click(&mut s, "Performance");
    click(&mut s, "Expunge unused cache now");
    assert!(s.cache_busy, "The Preferences window must submit cleanup");
    assert!(s.preferences_open);
    click(&mut s, "Close");
    assert!(!s.preferences_open);
    click(&mut s, "Reset timer");
    assert!(!s.timer.running(std::time::Instant::now(), s.timer_limit()));
    assert!(
        !s.menu_open,
        "Clicking the header command should dismiss the dropdown"
    );
    click(&mut s, "Start timer");
    assert!(s.timer.running(std::time::Instant::now(), s.timer_limit()));
    click(&mut s, "Pause timer");
    assert!(!s.timer.running(std::time::Instant::now(), s.timer_limit()));
    click(&mut s, "Menu");
    click(&mut s, "demo.pdf");
    assert!(!s.menu_open, "Outside click should dismiss the dropdown");
    click(&mut s, "Menu");
    frame(
        &mut s,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    assert!(!s.menu_open, "Escape should dismiss the dropdown");
}

#[test]
fn timer_presets_reserve_space_and_progress_tracks_navigation() {
    use deckstride::settings::TimerPosition;
    fn text_rect(shapes: &[egui::epaint::ClippedShape], label: &str) -> Option<Rect> {
        fn find(shape: &egui::Shape, label: &str) -> Option<Rect> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(Rect::from_min_size(t.pos, t.galley.size()))
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
                _ => None,
            }
        }
        shapes.iter().find_map(|s| find(&s.shape, label))
    }
    let ctx = Context::default();
    let mut s = studio();
    let mut native = UiState::default();
    s.layout.header = false;
    s.layout.footer = false;
    s.layout.elapsed_position = TimerPosition::AboveSlides;
    s.layout.clock_position = TimerPosition::BelowNotes;
    s.layout.elapsed.size = 72.;
    s.layout.clock.size = 72.;
    let mut render = |s: &mut Studio| {
        let mut output = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
            presenter_ui::presenter(ui, s, &mut native)
        });
        for _ in 0..3 {
            output = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
                presenter_ui::presenter(ui, s, &mut native)
            });
        }
        output
    };
    let output = render(&mut s);
    let elapsed = text_rect(&output.shapes, "Elapsed time").unwrap();
    let clock = text_rect(&output.shapes, "Time · HH:MM").unwrap();
    let current = text_rect(&output.shapes, "CURRENT  1 / 30").unwrap();
    assert!(elapsed.bottom() < current.top());
    assert!(clock.top() > current.bottom());
    assert!(
        text_rect(&output.shapes, "00:00").unwrap().height() >= 65.,
        "the above-slides timer must use its configured size, not button-row height"
    );
    let clock_value = chrono::Local::now().format("%H:%M").to_string();
    assert!(text_rect(&output.shapes, &clock_value).unwrap().height() >= 65.);
    for font_size in [28., 72., 160.] {
        s.layout.header = true;
        s.layout.elapsed_position = TimerPosition::Header;
        s.layout.clock_position = TimerPosition::Header;
        s.layout.elapsed.size = font_size;
        s.layout.clock.size = font_size;
        let header = render(&mut s);
        let elapsed = text_rect(&header.shapes, "00:00").unwrap();
        assert!(
            elapsed.height() >= font_size * 0.85,
            "header font {font_size}: {elapsed:?}"
        );
        assert!(elapsed.bottom() < text_rect(&header.shapes, "CURRENT  1 / 30").unwrap().top());
    }
    s.layout.header = false;
    s.layout.elapsed_position = TimerPosition::AboveSlides;
    s.layout.clock_position = TimerPosition::BelowNotes;
    s.layout.elapsed.size = 72.;
    s.layout.clock.size = 72.;
    assert!(text_rect(&output.shapes, "Slide 1 / 30").is_none());
    s.layout.slide_progress = true;
    for page in [0, 14, 29] {
        s.requested = page;
        let output = render(&mut s);
        let progress = text_rect(&output.shapes, &format!("Slide {} / 30", page + 1)).unwrap();
        assert!(progress.top() > text_rect(&output.shapes, "Time · HH:MM").unwrap().bottom());
    }
    s.requested = 0;
    s.layout.elapsed.size = 320.;
    s.layout.clock.size = 320.;
    for (left, right, anchor) in [
        (
            TimerPosition::LeftOfSlides,
            TimerPosition::RightOfSlides,
            "CURRENT  1 / 30",
        ),
        (
            TimerPosition::LeftOfNotes,
            TimerPosition::RightOfNotes,
            "PRESENTER NOTES · AUTO-FIT",
        ),
    ] {
        s.layout.elapsed_position = left;
        s.layout.clock_position = right;
        let output = render(&mut s);
        let elapsed = text_rect(&output.shapes, "Elapsed time").unwrap();
        let clock = text_rect(&output.shapes, "Time · HH:MM").unwrap();
        let pane = text_rect(&output.shapes, anchor).unwrap();
        assert!(elapsed.right() < pane.left());
        assert!(clock.left() > pane.right());
        assert!(text_rect(&output.shapes, "00:00").unwrap().height() > 72.);
        // Sharing a side stacks the controls without overlapping their labels.
        s.layout.clock_position = left;
        let output = render(&mut s);
        let elapsed = text_rect(&output.shapes, "Elapsed time").unwrap();
        let clock = text_rect(&output.shapes, "Time · HH:MM").unwrap();
        assert!(clock.top() > elapsed.bottom());
        assert!((clock.left() - elapsed.left()).abs() < 1.);
    }
    s.deck = None;
    assert!(text_rect(&render(&mut s).shapes, "Slide 0 / 0").is_some());
    s.layout.elapsed.show = false;
    s.layout.clock.show = false;
    let output = render(&mut s);
    assert!(text_rect(&output.shapes, "Elapsed time").is_none());
    assert!(text_rect(&output.shapes, "Time · HH:MM").is_none());
}

#[test]
fn end_of_slides_is_only_a_presenter_next_preview_not_a_page() {
    fn count_text(shape: &egui::Shape, label: &str) -> usize {
        match shape {
            egui::Shape::Text(t) => usize::from(t.galley.text() == label),
            egui::Shape::Vec(shapes) => shapes.iter().map(|s| count_text(s, label)).sum(),
            _ => 0,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    s.settings.language = deckstride::localization::Language::En;
    let mut native = UiState::default();
    for (total, requested, next_visible, expected) in [
        (3, 0, true, 0),
        (3, 1, true, 0),
        (3, 2, true, 1),
        (3, 2, false, 0),
        (1, 0, true, 1),
        (0, 0, true, 0),
    ] {
        s.deck.as_mut().unwrap().names = (0..total).map(|i| i.to_string()).collect();
        s.requested = requested;
        s.layout.next.show = next_visible;
        let output = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
            presenter_ui::presenter(ui, &mut s, &mut native);
        });
        assert_eq!(
            output
                .shapes
                .iter()
                .map(|s| count_text(&s.shape, "End of slides"))
                .sum::<usize>(),
            expected
        );
        assert_eq!(s.count(), total);
        s.execute(Action::Go(total));
        assert_eq!(
            s.requested, requested,
            "The placeholder cannot be navigated to"
        );
        let output = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
            presenter_ui::browser(ui, &mut s, &mut native);
        });
        assert_eq!(
            output
                .shapes
                .iter()
                .map(|s| count_text(&s.shape, "End of slides"))
                .sum::<usize>(),
            0
        );
    }
}

#[test]
fn browser_double_click_previews_without_live_navigation() {
    fn find(shape: &egui::Shape) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == "2" => Some(t.pos + Vec2::new(40., -45.)),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(find),
            _ => None,
        }
    }
    for live in [false, true] {
        let ctx = Context::default();
        let mut s = studio();
        s.change_on_click = live;
        s.browser_open = true;
        let mut native = UiState::default();
        let mut time = 0.;
        let mut frame = |s: &mut Studio, events| {
            time += 0.03;
            let mut raw = input(Vec2::new(1280., 900.));
            raw.time = Some(time);
            raw.events = events;
            ctx.run_ui(raw, |ui| presenter_ui::browser(ui, s, &mut native))
        };
        let mut output = frame(&mut s, vec![]);
        for _ in 0..3 {
            output = frame(&mut s, vec![]);
        }
        let pos = output.shapes.iter().find_map(|s| find(&s.shape)).unwrap();
        for _ in 0..2 {
            for pressed in [true, false] {
                frame(
                    &mut s,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
        }
        assert_eq!(s.browser_selected, 1);
        assert_eq!(s.requested, 0);
        if live {
            assert!(s.browser_preview.is_none());
            assert!(s.actions.iter().any(|a| matches!(a, Action::Go(1))));
        } else {
            assert_eq!(s.browser_preview, Some((s.generation, 1)));
            assert!(s.actions.is_empty());
            s.generation += 1;
            frame(&mut s, vec![]);
            assert!(s.browser_preview.is_none());
            s.browser_preview = Some((s.generation, 1));
            s.change_on_click = true;
            frame(&mut s, vec![]);
            assert!(s.browser_preview.is_none());
            s.browser_preview = Some((s.generation, 1));
            s.execute(Action::CloseBrowser);
            assert!(s.browser_preview.is_none());
        }
    }
}

#[test]
fn browser_thumbnails_fill_resized_window_without_stretching_tiles() {
    fn collect(shape: &egui::Shape, rects: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Rect(r) if r.fill == egui::Color32::from_rgb(18, 28, 31) => {
                rects.push(r.rect)
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, rects)),
            _ => {}
        }
    }
    let mut s = studio();
    s.deck.as_mut().unwrap().names = (0..1000).map(|i| format!("Slide {i}")).collect();
    for width in [500., 900., 1600.] {
        let ctx = Context::default();
        presenter_ui::theme(&ctx);
        let mut native = UiState::default();
        let mut right = 0.;
        let mut output = ctx.run_ui(input(Vec2::new(width, 900.)), |ui| {
            right = ui.max_rect().right();
            presenter_ui::browser(ui, &mut s, &mut native);
        });
        for _ in 0..3 {
            output = ctx.run_ui(input(Vec2::new(width, 900.)), |ui| {
                right = ui.max_rect().right();
                presenter_ui::browser(ui, &mut s, &mut native);
            });
        }
        let mut rects = vec![];
        for shape in &output.shapes {
            collect(&shape.shape, &mut rects);
        }
        fn right_edge_rects(
            shape: &egui::Shape,
            right: f32,
            found: &mut Vec<(Rect, egui::Color32)>,
        ) {
            match shape {
                egui::Shape::Rect(rect) if rect.rect.right() > right - 20. => {
                    found.push((rect.rect, rect.fill));
                }
                egui::Shape::Vec(shapes) => {
                    shapes
                        .iter()
                        .for_each(|shape| right_edge_rects(shape, right, found));
                }
                _ => {}
            }
        }
        let mut edge = vec![];
        for shape in &output.shapes {
            right_edge_rects(&shape.shape, right, &mut edge);
        }
        let handles = edge
            .iter()
            .filter(|(rect, fill)| {
                *fill == egui::Color32::from_rgb(70, 210, 175)
                    && (rect.width() - 8.).abs() < 0.1
                    && rect.height() >= 36.
            })
            .map(|(rect, _)| *rect)
            .collect::<Vec<_>>();
        assert_eq!(
            handles.len(),
            1,
            "an explicit scrollbar handle must be painted"
        );
        assert!(!rects.is_empty());
        let max_right = rects.iter().map(Rect::right).fold(0., f32::max);
        // A wide, non-floating scrollbar owns the far-right gutter.
        assert!((max_right + 16. - right).abs() < 1.);
        for rect in &rects {
            assert!((rect.height() / rect.width() - 0.5625).abs() < 0.001);
            assert!(rect.right() <= right);
        }
        assert!(s.visible_thumbs.len() < s.count());

        let initial_top = handles[0].top();
        let mut scrolled_input = input(Vec2::new(width, 900.));
        scrolled_input.events = vec![
            egui::Event::PointerMoved(egui::pos2(width / 2., 400.)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: Vec2::new(0., -5_000.),
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let _ = ctx.run_ui(scrolled_input, |ui| {
            presenter_ui::browser(ui, &mut s, &mut native);
        });
        // The scroll area's viewport reflects the new offset on the next frame.
        output = ctx.run_ui(input(Vec2::new(width, 900.)), |ui| {
            presenter_ui::browser(ui, &mut s, &mut native);
        });
        let mut scrolled_edge = vec![];
        for shape in &output.shapes {
            right_edge_rects(&shape.shape, right, &mut scrolled_edge);
        }
        let scrolled_handle = scrolled_edge
            .iter()
            .find(|(rect, fill)| {
                *fill == egui::Color32::from_rgb(70, 210, 175) && (rect.width() - 8.).abs() < 0.1
            })
            .map(|(rect, _)| *rect)
            .expect("scrollbar handle remains visible after scrolling");
        assert!(scrolled_handle.top() > initial_top);
        assert!(!s.visible_thumbs.contains(&0));
    }
}

#[test]
fn floating_windows_can_grow_and_shrink_after_content_is_drawn() {
    for id in ["layout-window", "displays-window", "slide-preview"] {
        let ctx = Context::default();
        let mut s = studio();
        s.layout_open = id == "layout-window";
        s.displays_open = id == "displays-window";
        if id == "slide-preview" {
            s.browser_preview = Some((s.generation, 0));
        }
        let mut native = UiState::default();
        let mut frame = |events| {
            let mut raw = input(Vec2::new(1920., 1200.));
            raw.events = events;
            let _ = ctx.run_ui(raw, |ui| {
                if id == "slide-preview" {
                    presenter_ui::browser(ui, &mut s, &mut native);
                } else {
                    presenter_ui::presenter(ui, &mut s, &mut native);
                }
            });
        };
        for _ in 0..4 {
            frame(vec![]);
        }
        let rect = || ctx.memory(|m| m.area_rect(egui::Id::new(id))).unwrap();
        for delta in [Vec2::new(120., 80.), Vec2::new(-180., -180.)] {
            let before = rect();
            let start = before.right_bottom() - Vec2::splat(2.);
            frame(vec![egui::Event::PointerMoved(start)]);
            frame(vec![egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }]);
            let end = start + delta;
            frame(vec![egui::Event::PointerMoved(end)]);
            frame(vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }]);
            for _ in 0..4 {
                frame(vec![]);
            }
            let after = rect();
            if delta.x > 0. {
                assert!(
                    after.width() > before.width() + 50.,
                    "{id}: {before:?} -> {after:?}"
                );
                assert!(
                    after.height() > before.height() + 30.,
                    "{id}: {before:?} -> {after:?}"
                );
            } else {
                assert!(
                    after.width() < before.width() - 50.,
                    "{id}: {before:?} -> {after:?}"
                );
                assert!(
                    after.height() < before.height() - 50.,
                    "{id}: {before:?} -> {after:?}"
                );
            }
        }
    }
}

#[test]
fn fullscreen_button_trials_both_directions_and_can_revert() {
    let mut s = studio();
    s.execute(Action::ToggleFullscreen);
    assert!(!s.presenter_fullscreen() && !s.presentation_fullscreen());
    assert!(s.settings.presenter_fullscreen);
    s.execute(Action::RevertRoutes);
    assert!(s.presenter_fullscreen());
    s.execute(Action::ToggleFullscreen);
    s.execute(Action::KeepRoutes);
    assert!(!s.settings.presenter_fullscreen);
    s.execute(Action::ToggleFullscreen);
    assert!(s.presenter_fullscreen() && s.presentation_fullscreen());
    s.execute(Action::KeepRoutes);
    assert!(s.settings.presenter_fullscreen && s.settings.presentation_fullscreen);
}

#[test]
fn timer_waits_for_activity_reset_rearms_and_pause_stays_paused() {
    use std::time::Instant;
    let mut s = studio();
    s.blanked = true;
    s.execute(Action::Go(1));
    assert!(s.timer.running(Instant::now(), s.timer_limit()));
    s.blanked = false;
    assert!(s.timer.running(Instant::now(), s.timer_limit()));
    s.execute(Action::ToggleTimer);
    s.presentation_action();
    assert!(!s.timer.running(Instant::now(), s.timer_limit()));
    s.execute(Action::ResetTimer);
    assert!(!s.timer.running(Instant::now(), s.timer_limit()));
    s.settings.zoom_enabled = true;
    s.set_zoom_from_action(deckstride::zoom::View {
        scale: 2.,
        center: [0.5; 2],
    });
    assert!(s.timer.running(Instant::now(), s.timer_limit()));
    s.timer = Default::default();
    s.layout.timer_autostart = false;
    s.presentation_action();
    assert!(!s.timer.running(Instant::now(), s.timer_limit()));
    s.timer = Default::default();
    s.layout.timer_autostart = true;
    s.routes.presentation.clear();
    s.execute(Action::Go(s.requested));
    assert!(!s.timer.running(Instant::now(), s.timer_limit()));
    s.execute(Action::Blank);
    assert!(s.timer.running(Instant::now(), s.timer_limit()));
}

#[test]
fn progress_bar_height_changes_rendered_geometry() {
    let ctx = Context::default();
    let mut s = studio();
    let mut native = UiState::default();
    s.layout.header = false;
    s.layout.footer = false;
    s.layout.slide_progress = true;
    for height in [8., 24., 80., 120.] {
        s.layout.slide_progress_height = height;
        let output = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
            presenter_ui::presenter(ui, &mut s, &mut native)
        });
        fn contains_bar(shape: &egui::Shape, height: f32) -> bool {
            match shape {
                egui::Shape::Rect(r) => {
                    r.rect.width() > 1200. && (r.rect.height() - height).abs() < 1.
                }
                egui::Shape::Vec(shapes) => shapes.iter().any(|s| contains_bar(s, height)),
                _ => false,
            }
        }
        assert!(
            output.shapes.iter().any(|s| contains_bar(&s.shape, height)),
            "Missing bar at height {height}"
        );
        let display = format!("display-{height}");
        let mut view = s.take_presenter_view(&display);
        view.layout = s.layout.clone();
        s.presenter_views.insert(display.clone(), view);
        s.layout.slide_progress_height = 32.; // Shared fallback must not set this display's height.
        let output = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
            presenter_ui::presenter_on_display(ui, &mut s, &mut native, &display)
        });
        assert!(output.shapes.iter().any(|s| contains_bar(&s.shape, height)));
    }
}

#[test]
fn hidden_footer_hides_routine_rolling_buffer_status() {
    fn contains_text(shape: &egui::Shape, needle: &str) -> bool {
        match shape {
            egui::Shape::Text(text) => text.galley.text().contains(needle),
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| contains_text(shape, needle)),
            _ => false,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    s.layout.header = false;
    s.layout.footer = false;
    s.evicted_frames = 1;
    s.prepared.insert(0);
    let mut native = UiState::default();

    let hidden = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
        presenter_ui::presenter(ui, &mut s, &mut native)
    });
    assert!(s.preparation_status().contains("Rolling buffer"));
    assert!(
        !hidden
            .shapes
            .iter()
            .any(|shape| contains_text(&shape.shape, "Rolling buffer"))
    );

    s.layout.footer = true;
    let visible = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
        presenter_ui::presenter(ui, &mut s, &mut native)
    });
    assert!(
        visible
            .shapes
            .iter()
            .any(|shape| contains_text(&shape.shape, "Rolling buffer"))
    );
}

#[test]
fn source_name_and_path_are_in_the_footer_instead_of_the_header() {
    fn contains_exact(shape: &egui::Shape, expected: &str) -> bool {
        match shape {
            egui::Shape::Text(text) => text.galley.text() == expected,
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| contains_exact(shape, expected)),
            _ => false,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    let path = std::path::PathBuf::from("shows").join("conference.pdf");
    s.deck.as_mut().unwrap().path = path.clone();
    s.layout.header = true;
    s.layout.footer = true;
    let mut native = UiState::default();
    let visible = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
        presenter_ui::presenter(ui, &mut s, &mut native)
    });
    for expected in ["conference.pdf", &path.display().to_string()] {
        assert!(
            visible
                .shapes
                .iter()
                .any(|shape| contains_exact(&shape.shape, expected)),
            "missing {expected}"
        );
    }
    assert!(
        !visible
            .shapes
            .iter()
            .any(|shape| contains_exact(&shape.shape, "Demo"))
    );

    s.layout.footer = false;
    native.footer_until = std::time::Instant::now() - std::time::Duration::from_secs(1);
    let hidden = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
        presenter_ui::presenter(ui, &mut s, &mut native)
    });
    assert!(
        !hidden
            .shapes
            .iter()
            .any(|shape| contains_exact(&shape.shape, &path.display().to_string()))
    );
}

#[test]
fn fullscreen_and_routing_changes_preserve_explicit_output_visibility() {
    for blanked in [false, true] {
        for confirm in [false, true] {
            let mut s = studio();
            s.blanked = blanked;
            for _ in 0..2 {
                s.execute(Action::ToggleFullscreen);
                assert_eq!(s.blanked, blanked);
                s.execute(if confirm {
                    Action::KeepRoutes
                } else {
                    Action::RevertRoutes
                });
                assert_eq!(s.blanked, blanked);
            }
            s.begin_display_edit();
            std::mem::swap(
                &mut s.draft_routes.presenter,
                &mut s.draft_routes.presentation,
            );
            s.execute(Action::ApplyRoutes);
            assert_eq!(s.blanked, blanked);
            s.route_trial.as_mut().unwrap().deadline = std::time::Instant::now();
            s.poll();
            assert_eq!(s.blanked, blanked);
            s.set_monitors(vec![s.monitors[0].clone()], 0);
            assert_eq!(s.blanked, blanked);
        }
    }
}

#[test]
fn double_click_notes_opens_editor_and_editor_keeps_unsaved_draft() {
    fn text_pos(shapes: &[egui::epaint::ClippedShape], label: &str) -> Pos2 {
        fn find(s: &egui::Shape, label: &str) -> Option<Pos2> {
            match s {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                egui::Shape::Vec(v) => v.iter().find_map(|s| find(s, label)),
                _ => None,
            }
        }
        shapes
            .iter()
            .find_map(|s| find(&s.shape, label))
            .unwrap_or_else(|| panic!("missing {label}"))
    }
    let ctx = Context::default();
    let mut s = studio();
    s.layout.header = false;
    s.layout.footer = false;
    let mut native = UiState::default();
    let mut time = 0.;
    let mut render = |s: &mut Studio, events| {
        time += 0.04;
        let mut raw = input(Vec2::new(1280., 900.));
        raw.events = events;
        raw.time = Some(time);
        ctx.run_ui(raw, |ui| presenter_ui::presenter(ui, s, &mut native))
    };
    let output = render(&mut s, vec![]);
    let pos = text_pos(&output.shapes, "PRESENTER NOTES · AUTO-FIT") + Vec2::new(0., 60.);
    for _ in 0..2 {
        for pressed in [true, false] {
            render(
                &mut s,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
    assert!(s.actions.iter().any(|a| matches!(a, Action::EditNotes)));
    s.actions.clear();
    s.notes_editor = Some(deckstride::studio::NotesEditor {
        page: 0,
        text: "Saved".into(),
        saved_text: "Saved".into(),
        busy: false,
        error: None,
        snapshot: Some(deckstride::notes::Snapshot {
            path: "demo.notes.toml".into(),
            revision: "test".into(),
            notes: vec!["Saved".into()],
        }),
    });
    for _ in 0..3 {
        render(&mut s, vec![]);
    }
    s.notes_editor.as_mut().unwrap().text = "Unsaved notes".into();
    let output = render(&mut s, vec![]);
    text_pos(&output.shapes, "Save notes");
    text_pos(&output.shapes, "Discard changes and close");
    s.execute(Action::Exit);
    assert!(!s.close_requested);
    assert_eq!(s.notes_editor.as_ref().unwrap().text, "Unsaved notes");
    s.execute(Action::CancelExit);
    let pos = text_pos(&output.shapes, "Save notes");
    for pressed in [true, false] {
        render(
            &mut s,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(s.actions.iter().any(|a| matches!(a, Action::SaveNotes)));
}

#[test]
fn notes_import_review_requires_explicit_save_and_cancel_discards_preview() {
    fn label_pos(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| label_pos(s, label)),
            _ => None,
        }
    }
    for (button, apply) in [("Import and save notes", true), ("Cancel", false)] {
        let ctx = Context::default();
        let mut s = studio();
        let original = s.deck.as_ref().unwrap().notes.clone();
        s.notes_import = Some(deckstride::studio::NotesImport {
            busy: false,
            error: None,
            preview: Some(deckstride::notes::ImportPreview {
                path: "demo.notes.toml".into(),
                revision: Some("revision".into()),
                sections: vec!["New first note".into(), "".into()],
                total: 30,
            }),
        });
        let mut state = UiState::default();
        let mut time = 0.;
        let mut render = |s: &mut Studio, events| {
            time += 0.04;
            let mut raw = input(Vec2::new(900., 700.));
            raw.time = Some(time);
            raw.events = events;
            ctx.run_ui(raw, |ui| presenter_ui::presenter(ui, s, &mut state))
        };
        for _ in 0..3 {
            render(&mut s, vec![]);
        }
        let output = render(&mut s, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| label_pos(&s.shape, button))
            .unwrap();
        assert_eq!(s.deck.as_ref().unwrap().notes, original);
        for pressed in [true, false] {
            render(
                &mut s,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(
            s.actions
                .iter()
                .any(|a| matches!(a, Action::ApplyNotesImport)),
            apply
        );
        assert_eq!(s.notes_import.is_some(), apply);
        assert_eq!(s.deck.as_ref().unwrap().notes, original);
    }
}

#[test]
fn zoom_is_opt_in_resets_on_slide_change_and_does_not_blank_output() {
    let mut s = studio();
    assert!(!s.settings.zoom_enabled);
    s.blanked = false;
    s.set_zoom(deckstride::zoom::View {
        scale: 4.,
        center: [0.3, 0.6],
    });
    assert_eq!(s.zoom.view.scale, 1.);
    s.enable_zoom(true);
    s.set_zoom(deckstride::zoom::View {
        scale: 4.,
        center: [0.3, 0.6],
    });
    let ctx = Context::default();
    let mut state = UiState::default();
    for size in [Vec2::new(1280., 900.), Vec2::new(600., 400.)] {
        let _ = ctx.run_ui(input(size), |ui| {
            presenter_ui::presenter(ui, &mut s, &mut state)
        });
        assert_eq!(s.zoom.view.scale, 4.);
    }
    let epoch = s.zoom.epoch;
    s.execute(Action::Go(1));
    assert_eq!(s.zoom.view.scale, 1.);
    assert!(s.zoom.epoch > epoch);
    assert!(!s.blanked);
    s.set_zoom(deckstride::zoom::View {
        scale: 8.,
        center: [0.5; 2],
    });
    s.zoom_warning = true;
    let _ = ctx.run_ui(input(Vec2::new(900., 700.)), |ui| {
        presenter_ui::presenter(ui, &mut s, &mut state)
    });
    assert!(!s.zoom_warning_dismissed);
    s.enable_zoom(false);
    assert_eq!(s.zoom.view, Default::default());
    assert!(!s.zoom_warning);
    assert!(!s.blanked);
}

#[test]
fn zoom_preview_matches_reference_output_crop_and_tracks_window_shape() {
    use deckstride::{output::OutputViewport, zoom::View};
    let mut s = studio();
    s.settings.zoom_enabled = true;
    s.set_zoom(View {
        scale: 2.,
        center: [0.4, 0.6],
    });
    assert_eq!(s.presentation_viewport(), Some([1920, 1080]));
    let ctx = Context::default();
    let mut native = UiState::default();
    for size in [[1920, 1080], [900, 1200]] {
        s.output_viewport = Some(OutputViewport {
            monitor: "two".into(),
            size,
            fullscreen: s.presentation_fullscreen(),
        });
        assert_eq!(s.presentation_viewport(), Some(size));
        let output = ctx.run_ui(input(Vec2::new(1280., 900.)), |ui| {
            presenter_ui::presenter(ui, &mut s, &mut native);
        });
        let texture = native.textures.values().next().unwrap().id();
        let mut actual = Rect::NOTHING;
        for primitive in ctx.tessellate(output.shapes, output.pixels_per_point) {
            if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive
                && mesh.texture_id == texture
            {
                for vertex in mesh.vertices {
                    actual.extend_with(vertex.uv);
                }
            }
        }
        let output_rect =
            Rect::from_min_size(Pos2::ZERO, Vec2::new(size[0] as f32, size[1] as f32));
        let (_, expected) = presenter_ui::zoom_geometry(output_rect, [64, 36], s.zoom.view, false);
        assert!(
            (actual.min - expected.min).length() < 0.0001,
            "{actual:?} vs {expected:?}"
        );
        assert!((actual.max - expected.max).length() < 0.0001);
    }
    // Stale route/fullscreen reports and minimized windows cannot distort the preview.
    s.output_viewport.as_mut().unwrap().fullscreen = !s.presentation_fullscreen();
    assert_eq!(s.presentation_viewport(), Some([1920, 1080]));
    s.output_viewport.as_mut().unwrap().fullscreen = s.presentation_fullscreen();
    s.output_viewport.as_mut().unwrap().size = [0, 0];
    assert_eq!(s.presentation_viewport(), Some([1920, 1080]));
    s.routes.presentation = vec!["missing".into(), "one".into()];
    s.monitors[0].size = (1280, 1024);
    assert_eq!(s.presentation_viewport(), Some([1280, 1024]));
    s.routes.presentation.clear();
    assert_eq!(s.presentation_viewport(), None);
}

#[test]
fn held_navigation_buttons_accelerate_and_release_without_an_extra_step() {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            egui::Shape::Vec(v) => v.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    s.settings.language = deckstride::localization::Language::En;
    s.layout.footer = true;
    s.blanked = false;
    let mut native = UiState::default();
    let frame = |s: &mut Studio, native: &mut UiState, events| {
        let mut raw = input(Vec2::new(1920., 1080.));
        raw.events = events;
        let output = ctx.run_ui(raw, |ui| presenter_ui::presenter(ui, s, native));
        for action in std::mem::take(&mut s.actions) {
            s.execute(action);
        }
        output
    };
    for _ in 0..3 {
        frame(&mut s, &mut native, vec![]);
    }
    let output = frame(&mut s, &mut native, vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|s| find(&s.shape, "Next"))
        .unwrap();
    frame(
        &mut s,
        &mut native,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ],
    );
    assert_eq!(s.requested, 1);
    native.navigation_hold = Some(deckstride::navigation::Hold::new(
        1,
        std::time::Instant::now() - std::time::Duration::from_secs(6),
    ));
    frame(&mut s, &mut native, vec![]);
    assert_eq!(s.requested, 4); // Batches capped to 10% of this 30-slide deck.
    frame(
        &mut s,
        &mut native,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
    );
    assert_eq!(s.requested, 4);
    assert!(native.navigation_hold.is_none());
    assert!(!s.blanked);
}

#[test]
fn third_monitor_routes_restore_after_repeated_disconnects_without_changing_visibility() {
    for third_is_presenter in [false, true] {
        for blanked in [false, true] {
            let mut s = studio();
            s.monitors.push(Monitor {
                id: "three".into(),
                name: "Third".into(),
                origin: (3840, 0),
                size: (1920, 1080),
            });
            if third_is_presenter {
                s.routes.presenter.push("three".into());
            } else {
                s.routes.presentation.push("three".into());
            }
            s.settings.routes = s.routes.clone();
            s.blanked = blanked;
            let saved = s.settings.routes.clone();
            let all = s.monitors.clone();
            for missing in [2, 1, 2] {
                let connected = all
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != missing)
                    .map(|(_, monitor)| monitor.clone())
                    .collect();
                s.set_monitors(connected, 0);
                assert!(!s.routes.presenter.contains(&all[missing].id));
                assert!(!s.routes.presentation.contains(&all[missing].id));
                assert_eq!(s.settings.routes, saved);
                assert_eq!(s.blanked, blanked);
                s.new_output = false;
                s.dirty_routes = false;
                s.set_monitors(all.clone(), 0);
                assert_eq!(s.routes, saved);
                assert!(s.new_output && s.dirty_routes);
                assert_eq!(s.blanked, blanked);
                assert!(!s.privacy_hold);
            }
        }
    }
}

#[test]
fn one_presenter_confirmation_dismisses_both_presenters_on_three_monitors() {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    for confirming_presenter in 0..2 {
        let mut s = studio();
        s.monitors.push(Monitor {
            id: "three".into(),
            name: "Third".into(),
            origin: (3840, 0),
            size: (1920, 1080),
        });
        s.settings.routes = s.routes.clone();
        s.begin_display_edit();
        s.draft_routes.presenter.push("three".into());
        s.execute(Action::ApplyRoutes);
        let expected = s.routes.clone();
        let mut views = [
            (Context::default(), UiState::default()),
            (Context::default(), UiState::default()),
        ];
        let size = Vec2::new(1280., 720.);
        let mut buttons = vec![];
        for (ctx, native) in &mut views {
            let mut output = ctx.run_ui(input(size), |ui| {
                presenter_ui::presenter(ui, &mut s, native)
            });
            for _ in 0..2 {
                output = ctx.run_ui(input(size), |ui| {
                    presenter_ui::presenter(ui, &mut s, native)
                });
            }
            buttons.push(
                output
                    .shapes
                    .iter()
                    .find_map(|shape| find(&shape.shape, "Keep changes"))
                    .unwrap(),
            );
        }
        let pos = buttons[confirming_presenter];
        let (ctx, native) = &mut views[confirming_presenter];
        for pressed in [true, false] {
            let mut raw = input(size);
            raw.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ];
            let _ = ctx.run_ui(raw, |ui| presenter_ui::presenter(ui, &mut s, native));
        }
        assert_eq!(
            s.actions
                .iter()
                .filter(|action| matches!(action, Action::KeepRoutes))
                .count(),
            1
        );
        for action in std::mem::take(&mut s.actions) {
            s.execute(action);
        }
        assert!(s.route_trial.is_none());
        assert_eq!(s.settings.routes, expected);
        assert!(!s.blanked);
        // Neither presenter needs another click; both consume the same confirmed state.
        for (ctx, native) in &mut views {
            let output = ctx.run_ui(input(size), |ui| {
                presenter_ui::presenter(ui, &mut s, native)
            });
            assert!(
                output
                    .shapes
                    .iter()
                    .all(|shape| find(&shape.shape, "Keep changes").is_none())
            );
        }
        s.execute(Action::RevertRoutes); // An obsolete second dialog cannot undo confirmation.
        assert_eq!(s.routes, expected);
    }
}

#[test]
fn presenter_layouts_are_display_scoped_and_save_actions_keep_the_origin() {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    let mut s = studio();
    s.routes.presenter = vec!["one".into(), "two".into()];
    s.routes.presentation.clear();
    s.settings.routes = s.routes.clone();
    let main = s.take_presenter_view("one");
    let main_layout = main.layout.clone();
    s.presenter_views.insert("one".into(), main);
    let mut second = s.take_presenter_view("two");
    second.layout_open = true;
    second.layout.notes.show = false;
    second.layout.current.size = 180.;
    second.layout.clock_position = deckstride::settings::TimerPosition::LeftOfSlides;
    s.presenter_views.insert("two".into(), second);
    s.layout.timer_mode = deckstride::settings::TimerMode::Countdown;
    s.layout.countdown_seconds = 99;
    let contexts = [Context::default(), Context::default()];
    let mut natives = [UiState::default(), UiState::default()];
    let size = Vec2::new(1400., 1000.);
    // Only the second presenter's Layout panel is open.
    let main_output = contexts[0].run_ui(input(size), |ui| {
        presenter_ui::presenter_on_display(ui, &mut s, &mut natives[0], "one")
    });
    assert!(
        main_output
            .shapes
            .iter()
            .all(|shape| find(&shape.shape, "Save layout · All projects").is_none())
    );
    let mut output = contexts[1].run_ui(input(size), |ui| {
        presenter_ui::presenter_on_display(ui, &mut s, &mut natives[1], "two")
    });
    for _ in 0..2 {
        output = contexts[1].run_ui(input(size), |ui| {
            presenter_ui::presenter_on_display(ui, &mut s, &mut natives[1], "two")
        });
    }
    let pos = output
        .shapes
        .iter()
        .find_map(|shape| find(&shape.shape, "Save layout · All projects"))
        .unwrap();
    for pressed in [true, false] {
        let mut raw = input(size);
        raw.events = vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ];
        let _ = contexts[1].run_ui(raw, |ui| {
            presenter_ui::presenter_on_display(ui, &mut s, &mut natives[1], "two")
        });
    }
    assert!(
        s.actions
            .iter()
            .any(|a| matches!(a, Action::SaveDisplayLayout { display, .. } if display == "two"))
    );
    // Another presenter renders before the queued save: it cannot steal the save's scope.
    let _ = contexts[0].run_ui(input(size), |ui| {
        presenter_ui::presenter_on_display(ui, &mut s, &mut natives[0], "one")
    });
    for action in std::mem::take(&mut s.actions) {
        s.execute(action);
    }
    assert_eq!(s.presenter_views["one"].layout, main_layout);
    assert!(!s.settings.for_display("two", "demo").notes.show);
    assert!(s.settings.for_display("one", "demo").notes.show);
    assert_eq!(s.settings.layout.countdown_seconds, 99);
    assert_eq!(s.timer_limit(), Some(std::time::Duration::from_secs(99)));
    assert!(!s.settings.display_layouts.contains_key("one"));
    let mut project_layout = s.presenter_views["two"].layout.clone();
    project_layout.notes.show = true;
    s.execute(Action::SaveDisplayLayout {
        display: "two".into(),
        project: true,
        layout: project_layout.clone(),
    });
    assert!(s.settings.for_display("two", "demo").notes.show);
    assert!(!s.settings.for_display("two", "other").notes.show);
    s.execute(Action::UseDisplayDefault("two".into()));
    assert!(!s.presenter_views["two"].layout.notes.show);
    let all = s.monitors.clone();
    s.set_monitors(vec![all[0].clone()], 0);
    s.set_monitors(all, 0);
    assert!(!s.presenter_views["two"].layout.notes.show);
    // Restart/source change discards transient views but keeps the display profiles.
    let restored = s.settings.for_display("two", "demo");
    s.presenter_views.clear();
    assert_eq!(s.take_presenter_view("two").layout, restored);
}

#[test]
fn preferences_window_stays_on_the_presenter_that_opened_it() {
    fn contains(shape: &egui::Shape, label: &str) -> bool {
        match shape {
            egui::Shape::Text(text) => text.galley.text() == label,
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| contains(shape, label)),
            _ => false,
        }
    }
    let mut s = studio();
    s.routes.presenter = vec!["one".into(), "two".into()];
    s.routes.presentation.clear();
    let mut first = s.take_presenter_view("one");
    first.preferences_open = true;
    s.presenter_views.insert("one".into(), first);
    let second = s.take_presenter_view("two");
    s.presenter_views.insert("two".into(), second);
    let contexts = [Context::default(), Context::default()];
    let mut natives = [UiState::default(), UiState::default()];
    let size = Vec2::new(1280., 900.);
    let mut render = |index: usize, display: &str| {
        let mut output = contexts[index].run_ui(input(size), |ui| {
            presenter_ui::presenter_on_display(ui, &mut s, &mut natives[index], display)
        });
        for _ in 0..2 {
            output = contexts[index].run_ui(input(size), |ui| {
                presenter_ui::presenter_on_display(ui, &mut s, &mut natives[index], display)
            });
        }
        output
            .shapes
            .iter()
            .any(|shape| contains(&shape.shape, "Appearance"))
    };
    assert!(render(0, "one"));
    assert!(!render(1, "two"));
}

#[test]
fn preferences_show_the_running_executable_path() {
    fn find(shape: &egui::Shape, text: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(label) if label.galley.text() == text => {
                Some(label.pos + label.galley.size() * 0.5)
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, text)),
            _ => None,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    s.preferences_open = true;
    let expected = s.executable_path().display().to_string();
    let mut ui = UiState::default();
    let mut output = ctx.run_ui(input(Vec2::new(1280., 900.)), |root| {
        presenter_ui::presenter(root, &mut s, &mut ui)
    });
    for _ in 0..2 {
        output = ctx.run_ui(input(Vec2::new(1280., 900.)), |root| {
            presenter_ui::presenter(root, &mut s, &mut ui)
        });
    }
    let storage = output
        .shapes
        .iter()
        .find_map(|shape| find(&shape.shape, "Storage"))
        .expect("Preferences should expose a Storage tab");
    for pressed in [true, false] {
        let mut raw = input(Vec2::new(1280., 900.));
        raw.events = vec![
            egui::Event::PointerMoved(storage),
            egui::Event::PointerButton {
                pos: storage,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ];
        output = ctx.run_ui(raw, |root| presenter_ui::presenter(root, &mut s, &mut ui));
    }
    assert!(
        output
            .shapes
            .iter()
            .any(|shape| find(&shape.shape, "Application executable").is_some())
    );
    assert!(
        output
            .shapes
            .iter()
            .any(|shape| find(&shape.shape, &expected).is_some())
    );
}

#[test]
fn any_presenter_can_edit_and_save_another_presenter_layout() {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
            _ => None,
        }
    }
    let mut s = studio();
    s.routes.presenter = vec!["one".into(), "two".into()];
    s.routes.presentation.clear();
    let mut first = s.take_presenter_view("one");
    let first_layout = first.layout.clone();
    first.layout_open = true;
    first.layout_target = Some("two".into());
    s.presenter_views.insert("one".into(), first);
    let second = s.take_presenter_view("two");
    assert!(second.layout.notes.show);
    s.presenter_views.insert("two".into(), second);

    let ctx = Context::default();
    let mut native = UiState::default();
    let size = Vec2::new(1400., 1000.);
    let mut initial = ctx.run_ui(input(size), |ui| {
        presenter_ui::presenter_on_display(ui, &mut s, &mut native, "one")
    });
    for _ in 0..2 {
        initial = ctx.run_ui(input(size), |ui| {
            presenter_ui::presenter_on_display(ui, &mut s, &mut native, "one")
        });
    }
    for label in ["Edit layout for", "Display 2 · Secondary", "Slide focus"] {
        assert!(
            initial
                .shapes
                .iter()
                .any(|shape| find(&shape.shape, label).is_some()),
            "missing layout target control: {label}"
        );
    }

    let mut click = |label: &str| {
        let mut output = ctx.run_ui(input(size), |ui| {
            presenter_ui::presenter_on_display(ui, &mut s, &mut native, "one")
        });
        for _ in 0..2 {
            output = ctx.run_ui(input(size), |ui| {
                presenter_ui::presenter_on_display(ui, &mut s, &mut native, "one")
            });
        }
        let position = output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, label))
            .unwrap();
        for pressed in [true, false] {
            let mut raw = input(size);
            raw.events = vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ];
            let _ = ctx.run_ui(raw, |ui| {
                presenter_ui::presenter_on_display(ui, &mut s, &mut native, "one")
            });
        }
    };
    click("Slide focus");
    click("Save layout · All projects");
    drop(click);

    assert_eq!(s.presenter_views["one"].layout, first_layout);
    assert!(!s.presenter_views["two"].layout.notes.show);
    assert!(s.presenter_views_dirty);
    assert!(s.actions.iter().any(
        |action| matches!(action, Action::SaveDisplayLayout { display, .. } if display == "two")
    ));
}

#[test]
fn explicit_swap_during_show_preserves_visibility_and_uses_reversible_trial() {
    for blanked in [false, true] {
        let mut s = studio();
        s.execute(Action::Go(0));
        s.blanked = blanked;
        let original = s.routes.clone();
        s.execute(Action::SwapRoutes);
        assert!(s.route_trial.is_some());
        assert_eq!(s.routes.presenter, original.presentation);
        assert_eq!(s.routes.presentation, original.presenter);
        assert_eq!(s.blanked, blanked);
        let swapped = s.routes.clone();
        s.execute(Action::SwapRoutes);
        assert_eq!(s.routes, swapped);
        s.execute(Action::RevertRoutes);
        assert_eq!(s.routes, original);
        assert_eq!(s.blanked, blanked);
    }
}

#[test]
fn screen_overrides_survive_global_commands_swap_and_hotplug_and_require_confirmation() {
    use deckstride::settings::ScreenMode::{AlwaysFullscreen, AlwaysWindowed};
    for blanked in [false, true] {
        let mut s = studio();
        s.blanked = blanked;
        s.begin_display_edit();
        s.draft_screen_modes = [
            ("one".into(), AlwaysWindowed),
            ("two".into(), AlwaysFullscreen),
        ]
        .into();
        s.execute(Action::ApplyRoutes);
        assert!(!s.screen_fullscreen("one", true));
        assert!(s.screen_fullscreen("two", false));
        assert!(s.settings.screen_modes.is_empty());
        s.execute(Action::RevertRoutes);
        assert!(s.screen_fullscreen("one", true));
        s.begin_display_edit();
        s.draft_screen_modes = [
            ("one".into(), AlwaysWindowed),
            ("two".into(), AlwaysFullscreen),
        ]
        .into();
        s.execute(Action::ApplyRoutes);
        s.execute(Action::KeepRoutes);
        let saved = s.settings.screen_modes.clone();
        for global in [false, true] {
            s.execute(Action::ToggleFullscreen);
            assert_eq!(s.presenter_fullscreen(), global);
            assert_eq!(s.presentation_fullscreen(), global);
            assert!(!s.screen_fullscreen("one", global));
            assert!(s.screen_fullscreen("two", global));
            assert_eq!(s.screen_fullscreen("other", global), global);
            assert_eq!(s.blanked, blanked);
            s.execute(Action::KeepRoutes);
            assert_eq!(s.settings.screen_modes, saved);
        }
        s.execute(Action::SwapRoutes);
        assert!(!s.screen_fullscreen("one", s.presentation_fullscreen()));
        assert!(s.screen_fullscreen("two", s.presenter_fullscreen()));
        s.execute(Action::RevertRoutes);
        s.begin_display_edit();
        s.draft_screen_modes.clear();
        s.execute(Action::ApplyRoutes);
        s.set_monitors(vec![s.monitors[0].clone()], 0);
        assert!(s.route_trial.is_none());
        assert_eq!(s.settings.screen_modes, saved);
        assert!(!s.screen_fullscreen("one", true));
        assert_eq!(s.blanked, blanked);
    }
}

#[test]
fn displays_dropdown_edits_only_selected_screen_until_applied() {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    s.settings.language = deckstride::localization::Language::En;
    s.begin_display_edit();
    let mut native = UiState::default();
    let mut frame = |s: &mut Studio, events| {
        let mut raw = input(Vec2::new(1920., 1080.));
        raw.events = events;
        ctx.run_ui(raw, |ui| presenter_ui::presenter(ui, s, &mut native))
    };
    for label in ["Follow global", "Always windowed"] {
        let mut output = frame(&mut s, vec![]);
        for _ in 0..3 {
            output = frame(&mut s, vec![]);
        }
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, label))
            .unwrap_or_else(|| panic!("Missing {label}"));
        for pressed in [true, false] {
            frame(
                &mut s,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
    }
    assert_eq!(
        s.draft_screen_modes.get("one"),
        Some(&deckstride::settings::ScreenMode::AlwaysWindowed)
    );
    assert!(!s.draft_screen_modes.contains_key("two"));
    assert!(s.settings.screen_modes.is_empty());
    assert!(s.screen_fullscreen("one", true));
    s.execute(Action::ApplyRoutes);
    assert!(!s.screen_fullscreen("one", true));
}

#[test]
fn output_viewport_accepts_screen_override_instead_of_global_mode() {
    let mut s = studio();
    s.settings.screen_modes.insert(
        "two".into(),
        deckstride::settings::ScreenMode::AlwaysWindowed,
    );
    s.output_viewport = Some(deckstride::output::OutputViewport {
        monitor: "two".into(),
        fullscreen: false,
        size: [800, 600],
    });
    assert_eq!(s.presentation_viewport(), Some([800, 600]));
    s.output_viewport.as_mut().unwrap().fullscreen = true;
    assert_eq!(s.presentation_viewport(), Some([1920, 1080]));
}

#[test]
fn quit_confirmation_requires_explicit_confirmation_and_protects_notes() {
    let mut s = studio();
    s.execute(Action::ConfirmExit);
    assert!(!s.close_requested);
    s.execute(Action::Exit);
    assert!(s.quit_confirmation && !s.close_requested && !s.blanked);
    s.execute(Action::CancelExit);
    s.execute(Action::ConfirmExit);
    assert!(!s.quit_confirmation && !s.close_requested);
    s.notes_editor = Some(deckstride::studio::NotesEditor {
        page: 0,
        text: "Unsaved".into(),
        saved_text: "Saved".into(),
        busy: false,
        error: None,
        snapshot: None,
    });
    s.execute(Action::Exit);
    s.execute(Action::ConfirmExit);
    assert!(s.quit_confirmation && !s.close_requested);
    assert_eq!(s.notes_editor.as_ref().unwrap().text, "Unsaved");
    s.notes_editor.as_mut().unwrap().text = "Saved".into();
    s.notes_editor.as_mut().unwrap().busy = true;
    s.execute(Action::ConfirmExit);
    assert!(!s.close_requested);
    s.notes_editor = None;
    s.notes_import = Some(deckstride::studio::NotesImport {
        preview: None,
        busy: true,
        error: None,
    });
    s.execute(Action::ConfirmExit);
    assert!(!s.close_requested);
    s.notes_import = None;
    s.execute(Action::ConfirmExit);
    assert!(s.close_requested && !s.quit_confirmation);
}

#[test]
fn quit_modal_buttons_work_in_presenter_and_browser() {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    for browser in [false, true] {
        for label in ["Cancel", "Quit presentation"] {
            let ctx = Context::default();
            let mut s = studio();
            s.settings.language = deckstride::localization::Language::En;
            s.execute(Action::Exit);
            let mut native = UiState::default();
            let mut frame = |s: &mut Studio, events| {
                let mut raw = input(Vec2::new(1920., 1080.));
                raw.events = events;
                ctx.run_ui(raw, |ui| {
                    if browser {
                        presenter_ui::browser(ui, s, &mut native);
                    } else {
                        presenter_ui::presenter(ui, s, &mut native);
                    }
                })
            };
            let mut output = frame(&mut s, vec![]);
            for _ in 0..3 {
                output = frame(&mut s, vec![]);
            }
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| find(&shape.shape, label))
                .unwrap();
            for pressed in [true, false] {
                frame(
                    &mut s,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
            for action in std::mem::take(&mut s.actions) {
                s.execute(action);
            }
            assert!(!s.quit_confirmation);
            assert_eq!(s.close_requested, label == "Quit presentation");
            assert!(!s.blanked);
        }
    }
}

#[test]
fn single_screen_offer_is_startup_only_and_never_rearmed_by_disconnects() {
    for initial_count in [0, 1, 2] {
        for accept in [false, true] {
            let mut s = studio();
            let one = s.monitors[0].clone();
            s.observe_startup_displays(initial_count, (initial_count == 1).then_some("one"));
            s.set_monitors(vec![one.clone()], 0);
            s.initialized = false;
            assert!(!s.startup_single_offer());
            s.initialized = true;
            assert_eq!(s.startup_single_offer(), initial_count == 1);
            s.execute(Action::StartupSingleScreen(accept));
            assert!(!s.startup_single_offer());
            if initial_count == 1 && accept {
                assert!(s.is_single_screen("one"));
                assert!(s.settings.single_screens.is_empty());
                assert!(s.routes.presentation.is_empty());
                s.execute(Action::KeepRoutes);
                assert_eq!(s.settings.single_screens, ["one"]);
            } else {
                assert!(!s.is_single_screen("one"));
            }
            s.set_monitors(vec![], 0);
            s.observe_startup_displays(1, Some("one"));
            s.set_monitors(vec![one], 0);
            assert!(!s.startup_single_offer());
        }
    }
    // A pending offer is withdrawn when the startup topology changes, too.
    let mut s = studio();
    let one = s.monitors[0].clone();
    s.observe_startup_displays(1, Some("one"));
    s.set_monitors(vec![one.clone()], 0);
    assert!(s.startup_single_offer());
    s.set_monitors(vec![], 0);
    s.set_monitors(vec![one], 0);
    assert!(!s.startup_single_offer());
}

#[test]
fn single_screen_trials_preserve_roles_privacy_and_visibility() {
    for blanked in [false, true] {
        let mut s = studio();
        s.blanked = blanked;
        s.settings.routes = s.routes.clone();
        s.begin_display_edit();
        s.draft_single_screens = vec!["one".into()];
        s.execute(Action::ApplyRoutes);
        assert!(s.is_single_screen("one"));
        assert!(!s.is_single_screen("two"));
        assert_eq!(s.routes.presentation, ["two"]);
        assert_eq!(s.blanked, blanked);
        s.execute(Action::RevertRoutes);
        assert!(!s.is_single_screen("one"));
        s.begin_display_edit();
        s.draft_single_screens = vec!["one".into()];
        s.execute(Action::ApplyRoutes);
        s.execute(Action::KeepRoutes);
        let routes = s.routes.clone();
        s.execute(Action::SwapRoutes); // Single screen is not a group-swap destination.
        assert_eq!(s.routes, routes);
        s.set_monitors(vec![s.monitors[1].clone()], 0);
        assert!(s.routes.presenter.is_empty());
        assert!(!s.is_single_screen("two"));
        assert!(!s.startup_single_offer());
        assert_eq!(s.blanked, blanked);
    }
}

#[test]
fn single_screen_renders_only_slide_reuses_texture_and_keeps_geometry_under_menus() {
    fn text(shape: &egui::Shape, labels: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => labels.push(t.galley.text().to_owned()),
            egui::Shape::Vec(shapes) => {
                for s in shapes {
                    text(s, labels);
                }
            }
            _ => {}
        }
    }
    fn image_rect(shape: &egui::Shape) -> Option<Rect> {
        match shape {
            egui::Shape::Mesh(mesh) if matches!(mesh.texture_id, egui::TextureId::Managed(id) if id > 0) => {
                Some(mesh.calc_bounds())
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(image_rect),
            _ => None,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    s.settings.language = deckstride::localization::Language::En;
    s.settings.single_screens = vec!["one".into()];
    let original = s.settings.for_display("one", &s.project());
    let mut native = UiState::default();
    let mut render = |s: &mut Studio, events| {
        let mut raw = input(Vec2::new(1280., 720.));
        raw.events = events;
        ctx.run_ui(raw, |ui| {
            presenter_ui::presenter_on_display(ui, s, &mut native, "one")
        })
    };
    let first = render(&mut s, vec![]);
    let mut labels = vec![];
    for shape in &first.shapes {
        text(&shape.shape, &mut labels);
    }
    assert!(
        labels.is_empty(),
        "Single screen must hide notes/status/controls: {labels:?}"
    );
    let slide = first
        .shapes
        .iter()
        .find_map(|s| image_rect(&s.shape))
        .unwrap();
    assert_eq!(slide.size(), Vec2::new(1280., 720.));
    assert_eq!(s.presenter_views["one"].layout, original);
    let second = render(&mut s, vec![]);
    assert!(
        second.textures_delta.set.is_empty(),
        "Idle repaint must reuse the slide texture"
    );
    let revealed = render(&mut s, vec![egui::Event::PointerMoved(Pos2::new(600., 1.))]);
    assert_eq!(
        revealed.shapes.iter().find_map(|s| image_rect(&s.shape)),
        Some(slide)
    );
    labels.clear();
    for shape in &revealed.shapes {
        text(&shape.shape, &mut labels);
    }
    assert!(labels.iter().any(|label| label == "Menu"));
    assert!(!labels.iter().any(|label| label.contains("A long note")));
    s.execute(Action::Go(8)); // Not prepared: retain the last ready image.
    let pending = render(&mut s, vec![]);
    assert_eq!(
        pending.shapes.iter().find_map(|s| image_rect(&s.shape)),
        Some(slide)
    );
    assert!(
        pending
            .textures_delta
            .set
            .iter()
            .all(|(id, _)| *id == egui::TextureId::Managed(0)),
        "Pending navigation must not upload a new slide texture"
    );
    s.blanked = true;
    let blank = render(&mut s, vec![]);
    assert!(
        blank
            .shapes
            .iter()
            .find_map(|s| image_rect(&s.shape))
            .is_none()
    );
    s.blanked = false;
    s.generation += 1;
    let changed = render(&mut s, vec![]);
    assert!(
        changed
            .shapes
            .iter()
            .find_map(|s| image_rect(&s.shape))
            .is_none()
    );
}

#[test]
fn displays_can_select_single_screen_without_changing_audience_role() {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    s.settings.language = deckstride::localization::Language::En;
    s.begin_display_edit();
    let mut native = UiState::default();
    let mut frame = |s: &mut Studio, events| {
        let mut raw = input(Vec2::new(1920., 1080.));
        raw.events = events;
        ctx.run_ui(raw, |ui| presenter_ui::presenter(ui, s, &mut native))
    };
    let mut output = frame(&mut s, vec![]);
    for _ in 0..3 {
        output = frame(&mut s, vec![]);
    }
    let pos = output
        .shapes
        .iter()
        .find_map(|shape| find(&shape.shape, "Single screen"))
        .unwrap();
    for pressed in [true, false] {
        frame(
            &mut s,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
    assert_eq!(s.draft_single_screens, ["one"]);
    assert!(!s.is_single_screen("one"));
    s.execute(Action::ApplyRoutes);
    assert!(s.is_single_screen("one"));
    assert_eq!(s.routes.presentation, ["two"]);
    s.execute(Action::KeepRoutes);
    s.begin_display_edit();
    s.draft_single_screens.clear();
    s.execute(Action::ApplyRoutes);
    assert!(!s.is_single_screen("one"));
    s.execute(Action::RevertRoutes);
    assert!(s.is_single_screen("one"));
}

#[test]
fn startup_single_prompt_explains_presenter_intent_and_saved_mode_does_not_reoffer() {
    let ctx = Context::default();
    let mut s = studio();
    s.settings.language = deckstride::localization::Language::En;
    let one = s.monitors[0].clone();
    s.observe_startup_displays(1, Some("one"));
    s.set_monitors(vec![one], 0);
    let mut native = UiState::default();
    fn has_text(shape: &egui::Shape, text: &str) -> bool {
        match shape {
            egui::Shape::Text(t) => t.galley.text() == text,
            egui::Shape::Vec(shapes) => shapes.iter().any(|s| has_text(s, text)),
            _ => false,
        }
    }
    let mut result = ctx.run_ui(input(Vec2::new(1280., 720.)), |ui| {
        presenter_ui::presenter(ui, &mut s, &mut native)
    });
    for _ in 0..3 {
        result = ctx.run_ui(input(Vec2::new(1280., 720.)), |ui| {
            presenter_ui::presenter(ui, &mut s, &mut native)
        });
    }
    assert!(result.shapes.iter().any(|shape| has_text(
        &shape.shape,
        "Deckstride is intended for a presenter setup with separate Presenter and Slides screens."
    )));
    s.settings.single_screens = vec!["one".into()];
    assert!(!s.startup_single_offer());
    s.poll();
    assert!(s.startup_single_screen.is_none());
}

#[test]
fn single_screen_bottom_controls_reveal_and_hide_without_pinning_layout() {
    fn contains(shape: &egui::Shape, label: &str) -> bool {
        match shape {
            egui::Shape::Text(t) => t.galley.text() == label,
            egui::Shape::Vec(shapes) => shapes.iter().any(|s| contains(s, label)),
            _ => false,
        }
    }
    let ctx = Context::default();
    let mut s = studio();
    s.settings.language = deckstride::localization::Language::En;
    s.settings.single_screens = vec!["one".into()];
    let mut native = UiState::default();
    let render = |s: &mut Studio, native: &mut UiState, pos| {
        let mut raw = input(Vec2::new(1280., 720.));
        raw.events = vec![egui::Event::PointerMoved(pos)];
        ctx.run_ui(raw, |ui| {
            presenter_ui::presenter_on_display(ui, s, native, "one")
        })
    };
    let revealed = render(&mut s, &mut native, Pos2::new(640., 719.));
    assert!(
        revealed
            .shapes
            .iter()
            .any(|shape| contains(&shape.shape, "Next"))
    );
    native.header_until = std::time::Instant::now();
    native.footer_until = std::time::Instant::now();
    let hidden = render(&mut s, &mut native, Pos2::new(640., 360.));
    assert!(
        !hidden
            .shapes
            .iter()
            .any(|shape| contains(&shape.shape, "Next") || contains(&shape.shape, "Menu"))
    );
    // The original Presenter preferences have not been rewritten to hide its bars.
    assert!(s.presenter_views["one"].layout.header);
    assert!(s.presenter_views["one"].layout.footer);
}
