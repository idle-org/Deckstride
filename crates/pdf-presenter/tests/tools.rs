use egui::{Pos2, Rect, Vec2};
use pdf_presenter::{
    output::{HostState, coalesce},
    tools::{MAX_MARKS, Mark, MarkKind, Overlay, Toolbox, screen_to_page},
};

#[test]
fn overlay_input_is_bounded_and_normalized() {
    let marks = (0..MAX_MARKS + 10)
        .map(|_| Mark {
            kind: MarkKind::Arrow,
            from: [-1., 0.25],
            to: [2., 0.75],
            color: [255, 0, 0],
            width: 500.,
        })
        .collect();
    let overlay = Overlay {
        pointer: Some([f32::NAN, 0.]),
        pointer_size: f32::INFINITY,
        marks,
        ..Default::default()
    }
    .normalized();
    assert_eq!(overlay.marks.len(), MAX_MARKS);
    assert_eq!(overlay.marks[0].from, [0., 0.25]);
    assert_eq!(overlay.marks[0].to, [1., 0.75]);
    assert_eq!(overlay.marks[0].width, 48.);
    assert_eq!(overlay.pointer, None);
    assert_eq!(overlay.pointer_size, 12.);
}

#[test]
fn toolbox_discards_oldest_mark_and_maps_visible_crop() {
    let mut tools = Toolbox::default();
    tools.select(pdf_presenter::tools::Tool::Arrow);
    for index in 0..MAX_MARKS + 1 {
        assert!(tools.begin_mark([index as f32 / 100., 0.5]));
        tools.end_mark();
    }
    assert_eq!(tools.overlay.marks.len(), MAX_MARKS);
    assert_eq!(tools.overlay.marks[0].from[0], 0.01);

    let target = Rect::from_min_size(Pos2::new(10., 20.), Vec2::new(200., 100.));
    let uv = Rect::from_min_max(Pos2::new(0.25, 0.25), Pos2::new(0.75, 0.75));
    assert_eq!(screen_to_page(target, uv, target.center()), [0.5, 0.5]);
}

#[test]
fn overlay_update_coalesces_without_losing_full_output_state() {
    let mut pending = Some(HostState {
        generation: 9,
        page: 3,
        sequence: 22,
        monitors: vec!["projector".into()],
        ..Default::default()
    });
    let overlay = Overlay {
        pointer: Some([0.2, 0.7]),
        pointer_size: 14.,
        ..Default::default()
    };
    coalesce(
        &mut pending,
        HostState {
            generation: 9,
            page: 3,
            overlay_only: true,
            overlay: overlay.clone(),
            ..Default::default()
        },
    );
    let pending = pending.unwrap();
    assert_eq!(pending.sequence, 22);
    assert_eq!(pending.monitors, vec!["projector"]);
    assert!(!pending.overlay_only);
    assert_eq!(pending.overlay, overlay.clone().normalized());

    let mut lightweight = Some(HostState {
        generation: 9,
        page: 3,
        overlay_only: true,
        overlay: overlay.clone(),
        ..Default::default()
    });
    coalesce(
        &mut lightweight,
        HostState {
            generation: 9,
            page: 3,
            zoom_only: true,
            zoom: pdf_presenter::zoom::Sync {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let lightweight = lightweight.unwrap();
    assert!(lightweight.overlay_only && lightweight.zoom_only);
    assert_eq!(lightweight.overlay, overlay.normalized());
}
