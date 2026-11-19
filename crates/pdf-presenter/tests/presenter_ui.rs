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
