use egui::{Context, Pos2, RawInput, Rect, Vec2};
use pdf_presenter::{
    document::{DeckInfo, Frame, FrameInfo},
    presenter_ui::{self, UiState},
    studio::{Action, Monitor, Studio},
};
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
        for panel in 0..4 {
            s.menu_open = panel == 1;
            s.layout_open = panel == 2;
            s.displays_open = panel == 3;
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
    s.execute(Action::Windowed);
    assert!(s.blanked);
    assert!(!s.settings.presenter_fullscreen);
    s.draft_routes.presenter.clear();
    s.execute(Action::ApplyRoutes);
    assert!(s.status.contains("presenter"));
    s.draft_routes = s.routes.clone();
    s.execute(Action::ApplyRoutes);
    assert!(s.blanked);
    s.execute(Action::Refresh);
    assert!(s.dirty_routes);
    s.execute(Action::Exit);
    assert!(s.close_requested);
}

#[test]
fn lost_operator_screen_never_becomes_notes_on_previous_public_output() {
    let mut s = studio();
    s.settings.routes = s.routes.clone();
    s.set_monitors(vec![s.monitors[1].clone()], 0);
    assert!(s.routes.presenter.is_empty());
    assert!(s.privacy_hold);
    assert!(s.blanked);
    let monitors = studio().monitors;
    s.set_monitors(monitors, 0);
    assert!(!s.privacy_hold);
    assert_eq!(s.routes.presenter, vec!["one"]);
}

#[test]
fn palettes_and_languages_apply_to_presenter_browser_and_popups() {
    use pdf_presenter::{
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
fn startup_output_is_enabled_but_disconnect_still_blanks() {
    let mut s = Studio::new(true, None);
    assert!(!s.blanked);
    let monitors = studio().monitors;
    s.set_monitors(monitors.clone(), 0);
    assert!(!s.blanked);
    s.set_monitors(vec![monitors[0].clone()], 0);
    assert!(s.blanked);
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
    assert!(s.blanked);
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
    use pdf_presenter::output::DisplayAction;
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
    assert!(s.blanked);
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
        ("Swap presenter and presentation displays…", false),
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
        use pdf_presenter::output::DisplayAction;
        assert!(matches!(
            (label, action),
            ("Keep changes", Some(DisplayAction::Keep))
                | ("Revert now", Some(DisplayAction::Revert))
                | (
                    "Swap presenter and presentation displays…",
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
fn nested_settings_remain_interactive_and_menu_still_dismisses() {
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
    s.settings.language = pdf_presenter::localization::Language::En;
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
    click(&mut s, "Appearance");
    click(&mut s, "Midra Dark");
    assert!(
        s.menu_open,
        "Opening a theme selector must keep its parents open"
    );
    click(&mut s, "High Contrast");
    assert_eq!(s.settings.theme, pdf_presenter::theme::Theme::HighContrast);
    assert!(s.menu_open);
    click(&mut s, "English");
    click(&mut s, "System / Système");
    assert_eq!(
        s.settings.language,
        pdf_presenter::localization::Language::System
    );
    assert!(s.menu_open);
    click(&mut s, "Preferences");
    let auto_show = s.settings.auto_show;
    click(
        &mut s,
        "Show presentation automatically when opening a document",
    );
    assert_eq!(s.settings.auto_show, !auto_show);
    assert!(s.menu_open);
    click(&mut s, "Storage locations");
    assert!(s.menu_open);
    click(&mut s, "Reset timer");
    assert!(!s.menu_open, "Commands should dismiss the dropdown");
    click(&mut s, "Menu");
    click(&mut s, "Demo");
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
    use pdf_presenter::settings::TimerPosition;
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
    assert!(text_rect(&output.shapes, "Slide 1 / 30").is_none());
    s.layout.slide_progress = true;
    for page in [0, 14, 29] {
        s.requested = page;
        let output = render(&mut s);
        let progress = text_rect(&output.shapes, &format!("Slide {} / 30", page + 1)).unwrap();
        assert!(progress.top() > text_rect(&output.shapes, "Time · HH:MM").unwrap().bottom());
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
    s.settings.language = pdf_presenter::localization::Language::En;
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
