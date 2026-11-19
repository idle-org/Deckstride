use pdf_presenter::{
    paint,
    poc::{Action, Display, Model, Options, SLIDES, window_rect},
};

fn displays() -> Vec<Display> {
    vec![
        Display {
            name: "Left".into(),
            origin: (-1920, 0),
            size: (1920, 1080),
        },
        Display {
            name: "Main".into(),
            origin: (0, 0),
            size: (2560, 1440),
        },
        Display {
            name: "Portrait".into(),
            origin: (2560, -200),
            size: (1080, 1920),
        },
    ]
}
fn model() -> Model {
    Model::new(displays(), 1, None, None, false).unwrap()
}

#[test]
fn defaults_use_primary_and_a_distinct_secondary() {
    let m = model();
    assert_eq!((m.presenter, m.audience), (1, 0));
    assert!(m.fullscreen && m.presenter_fullscreen);
    assert!(!m.blanked); // Explicit mock-output launch; real PDF activation remains separate.
    let one = Model::new(vec![displays()[0].clone()], 0, None, None, false).unwrap();
    assert!(one.fullscreen && one.presenter_fullscreen);
    assert_eq!((one.presenter, one.audience), (0, 0));
    let windowed = Model::new(displays(), 0, Some(2), Some(1), true).unwrap();
    assert!(!windowed.fullscreen && !windowed.presenter_fullscreen);
    let one_windowed = Model::new(vec![displays()[0].clone()], 0, None, None, true).unwrap();
    assert!(!one_windowed.fullscreen && !one_windowed.presenter_fullscreen);
    assert_eq!(
        Model::new(displays(), 99, None, None, false)
            .unwrap()
            .presenter,
        2
    );
}

#[test]
fn invalid_display_assignments_do_not_start_a_show() {
    assert!(Model::new(vec![], 0, None, None, false).is_err());
    assert!(Model::new(displays(), 0, Some(4), None, false).is_err());
    assert!(Model::new(displays(), 0, None, Some(4), false).is_err());
    assert!(Model::new(displays(), 0, Some(1), Some(1), false).is_err());
}

#[test]
fn controls_bound_navigation_and_keep_display_roles_separate() {
    let mut m = model();
    m.apply(Action::Previous);
    assert_eq!(m.page, 0);
    for _ in 0..100 {
        m.apply(Action::Next);
    }
    assert_eq!(m.page, SLIDES - 1);
    m.apply(Action::Previous);
    assert_eq!(m.page, SLIDES - 2);
    m.apply(Action::Blank);
    assert!(m.blanked);
    m.apply(Action::Blank);
    assert!(!m.blanked);
    m.apply(Action::Swap);
    assert_eq!((m.presenter, m.audience), (0, 1));
    for action in [Action::PresenterDisplay, Action::AudienceDisplay] {
        for _ in 0..12 {
            assert!(m.apply(action));
            assert_ne!(m.presenter, m.audience);
        }
    }
    m.apply(Action::Fullscreen);
    assert!(!m.fullscreen);
    m.apply(Action::PresenterFullscreen);
    assert!(!m.presenter_fullscreen);
    m.apply(Action::PresenterFullscreen);
    assert!(m.presenter_fullscreen);
    m.apply(Action::Escape);
    assert!(m.blanked && !m.fullscreen && !m.presenter_fullscreen);
}

#[test]
fn topology_loss_keeps_output_blanked_until_explicit_refresh() {
    let mut m = model();
    assert!(!m.topology_changed(&displays()));
    assert!(m.topology_changed(&[]));
    assert!(m.disconnected && m.blanked && !m.fullscreen);
    m.apply(Action::Blank);
    m.apply(Action::Fullscreen);
    assert!(m.blanked && !m.fullscreen);
    let mut one = Model::new(vec![displays()[0].clone()], 0, None, None, false).unwrap();
    one.apply(Action::PresenterDisplay);
    one.apply(Action::AudienceDisplay);
    assert_eq!((one.presenter, one.audience), (0, 0));
}

#[test]
fn window_geometry_handles_negative_coordinates_and_rehearsal() {
    assert_eq!(
        window_rect(&displays()[0], false, false),
        (-1896, 48, 1000, 700)
    );
    assert_eq!(
        window_rect(&displays()[0], true, true),
        (-936, 48, 896, 700)
    );
    assert_eq!(
        window_rect(
            &Display {
                name: "tiny".into(),
                origin: (0, 0),
                size: (1, 1)
            },
            false,
            true
        ),
        (24, 48, 160, 120)
    );
}

#[test]
fn arguments_are_validated_without_a_desktop() {
    let parse = |args: &[&str]| Options::parse(args.iter().map(|s| s.to_string()));
    let opts = parse(&["--presenter", "2", "--audience", "3", "--windowed"]).unwrap();
    assert_eq!(
        (opts.presenter, opts.audience, opts.windowed),
        (Some(1), Some(2), true)
    );
    for args in [
        vec!["--presenter"],
        vec!["--audience", "0"],
        vec!["--presenter", "x"],
        vec!["--invalid"],
    ] {
        assert!(parse(&args).is_err());
    }
    assert!(parse(&["--list-monitors"]).unwrap().list);
    for help in ["--help", "-h", "--version"] {
        assert!(parse(&[help]).unwrap().help);
    }
    let smoke = parse(&["--smoke-test"]).unwrap();
    assert!(smoke.smoke && smoke.windowed);
}

#[test]
fn presenter_controls_hit_correctly_at_multiple_scales() {
    for (w, h) in [(960, 640), (1920, 1280), (1920, 1080), (480, 320)] {
        let scale = (w as f64 / 960.).min(h as f64 / 640.);
        let ox = (w as f64 - 960. * scale) / 2.;
        let oy = (h as f64 - 640. * scale) / 2.;
        for b in &paint::BUTTONS {
            assert_eq!(
                paint::hit_test(w, h, ox + (b.x + 20.) * scale, oy + (b.y + 20.) * scale),
                Some(b.action)
            );
        }
        assert_eq!(paint::hit_test(w, h, 0., 0.), None);
    }
}

#[test]
fn software_frames_change_slides_and_blank_without_presenter_content() {
    let mut m = model();
    let mut audience = vec![0; 960 * 640];
    let mut presenter = audience.clone();
    paint::draw(&mut audience, 960, 640, &m, true, "private controls");
    paint::draw(&mut presenter, 960, 640, &m, false, "Presenter status");
    assert_ne!(audience, presenter);
    let first = audience.clone();
    m.apply(Action::Next);
    paint::draw(&mut audience, 960, 640, &m, true, "");
    assert_ne!(audience, first);
    m.apply(Action::Blank);
    paint::draw(&mut audience, 960, 640, &m, true, "");
    assert!(audience.iter().all(|&p| p == 0));
    for state in 0..4 {
        match state {
            0 => m.page = SLIDES - 1,
            1 => m.disconnected = true,
            2 => {
                m.disconnected = false;
                m.displays.truncate(1);
            }
            _ => m.blanked = false,
        }
        paint::draw(&mut presenter, 960, 640, &m, false, "status");
        assert!(presenter.iter().any(|&p| p != 0));
    }
    let mut small = vec![0; 1];
    paint::draw(&mut small, 1, 1, &m, false, "");
    paint::draw(&mut small, 0, 0, &m, false, "");
    assert_ne!(small[0], 0);
}
