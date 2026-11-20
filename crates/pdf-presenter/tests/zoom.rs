use egui::{Context, Pos2, RawInput, Rect, TextureOptions, Vec2};
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use pdf_presenter::{
    output::{HostState, coalesce},
    settings::Settings,
    zoom::{PerformanceWatch, Sync, View},
};
use std::time::{Duration, Instant};
#[test]
fn crop_anchor_pan_and_limits() {
    let start = View::default();
    let zoom = start.zoom_at(2., [0.25, 0.75]);
    assert_eq!(zoom.center, [0.375, 0.625]);
    assert_eq!(zoom.uv().size(), Vec2::splat(0.5));
    let panned = zoom.pan([0.1, -0.1]);
    assert!((panned.center[0] - 0.325).abs() < 0.00001);
    let edge = panned.pan([100., -100.]);
    assert_eq!(edge.center, [0.25, 0.75]);
    assert_eq!(edge.zoom_at(1., [0., 0.]), start);
    assert_eq!(start.zoom_at(100., [0.5; 2]).scale, 8.);
    assert_eq!(
        View {
            scale: f32::NAN,
            center: [f32::INFINITY, 0.]
        }
        .normalized(),
        start
    );
    assert_eq!(start.pan([f32::NAN, f32::INFINITY]), start);
}
#[test]
fn synchronization_rejects_stale_gestures_and_coalesces_without_losing_frames() {
    let mut presenter = Sync::default();
    presenter.set(true, View::default());
    let mut host = presenter;
    host.gesture(host.view.zoom_at(3., [0.5; 2]));
    let old = host;
    host.gesture(host.view.pan([0.1, 0.]));
    assert!(presenter.accept_output(host));
    assert!(!presenter.accept_output(old));
    host.accept_presenter(old);
    assert_eq!(host, presenter);
    presenter.set(true, View::default());
    assert!(!presenter.accept_output(host));
    host.accept_presenter(presenter);
    assert_eq!(host.view, View::default());
    presenter.set(false, host.view.zoom_at(4., [0.5; 2]));
    assert_eq!(presenter.view, View::default());
    let mut pending = Some(HostState {
        generation: 4,
        page: 2,
        sequence: 12,
        monitors: vec!["screen".into()],
        zoom: host,
        ..Default::default()
    });
    host.gesture(host.view.zoom_at(4., [0.5; 2]));
    coalesce(
        &mut pending,
        HostState {
            zoom_only: true,
            zoom: host,
            generation: 4,
            page: 2,
            ..Default::default()
        },
    );
    let state = pending.as_ref().unwrap();
    assert_eq!(state.monitors, vec!["screen"]);
    assert_eq!(state.sequence, 12);
    assert!(!state.zoom_only);
    assert_eq!(state.zoom, host);
    coalesce(
        &mut pending,
        HostState {
            zoom_only: true,
            generation: 3,
            ..Default::default()
        },
    );
    assert_eq!(pending.as_ref().unwrap().zoom, host);
    coalesce(
        &mut pending,
        HostState {
            quit: true,
            ..Default::default()
        },
    );
    assert!(pending.unwrap().quit);
}
#[test]
fn settings_default_opt_out_roundtrip_and_warning_threshold() {
    assert!(!Settings::default().zoom_enabled);
    let old: Settings = toml::from_str("version=1").unwrap();
    assert!(!old.zoom_enabled);
    assert!(!old.zoom_keep_fitted_area);
    assert!(!Settings::default().zoom_keep_fitted_area);
    let enabled = Settings {
        zoom_enabled: true,
        zoom_keep_fitted_area: true,
        ..Default::default()
    };
    let restored: Settings = toml::from_str(&toml::to_string(&enabled).unwrap()).unwrap();
    assert!(restored.zoom_enabled);
    assert!(restored.zoom_keep_fitted_area);
    let mut watch = PerformanceWatch::default();
    assert!(!watch.observe(false, Duration::from_secs(1)));
    assert!(!watch.observe(true, Duration::from_millis(40)));
    assert!(!watch.observe(true, Duration::from_millis(40)));
    assert!(watch.observe(true, Duration::from_millis(40)));
    assert!(!watch.observe(true, Duration::from_millis(20)));
    assert!(!watch.observe(true, Duration::from_millis(40)));
}
#[test]
fn zoom_paints_existing_texture_crop_with_constant_geometry_and_no_new_uploads() {
    let ctx = Context::default();
    let mut rgba = vec![0; 64 * 32 * 4];
    for y in 0..32 {
        for x in 0..64 {
            let color = if x < 32 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            };
            rgba[(y * 64 + x) * 4..(y * 64 + x + 1) * 4].copy_from_slice(&color);
        }
    }
    let texture = ctx.load_texture(
        "slide",
        egui::ColorImage::from_rgba_unmultiplied([64, 32], &rgba),
        TextureOptions::NEAREST,
    );
    let mut renderer = EguiSoftwareRender::new(ColorFieldOrder::Bgra);
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(200.));
    let mut pixels = vec![0u32; 200 * 200];
    for (index, view) in [
        View::default(),
        View {
            scale: 2.,
            center: [0.25, 0.5],
        },
        View {
            scale: 2.,
            center: [0.75, 0.5],
        },
    ]
    .into_iter()
    .enumerate()
    {
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ui| {
                pdf_presenter::presenter_ui::paint_slide_zoomed(
                    ui.painter(),
                    rect,
                    texture.id(),
                    [64, 32],
                    Default::default(),
                    view,
                );
            },
        );
        if index > 0 {
            assert!(
                output.textures_delta.set.is_empty(),
                "zoom uploaded another image"
            );
        }
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        for primitive in &primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive
                && mesh.texture_id == texture.id()
            {
                assert!(
                    mesh.vertices.iter().all(|v| rect.contains(v.pos)),
                    "zoom enlarged draw geometry"
                );
            }
        }
        renderer.render(
            &mut BufferMutRef::new(bytemuck::cast_slice_mut(&mut pixels), 200, 200),
            &primitives,
            &output.textures_delta,
            output.pixels_per_point,
        );
        assert_eq!(pixels[5 * 200 + 5] & 0xffffff, 0); // Padding unchanged.
        if index > 0 {
            let expected = if index == 1 { 0xff0000 } else { 0x0000ff };
            assert_eq!(pixels[100 * 200 + 30] & 0xffffff, expected);
            assert_eq!(pixels[100 * 200 + 170] & 0xffffff, expected);
        }
    }
}
#[test]
#[ignore = "Manual Linux software drawing measurement; not a timing-sensitive CI assertion"]
fn buffered_zoom_drawing_measurement() {
    for [width, height] in [[1920usize, 1080usize], [3840, 2160]] {
        let ctx = Context::default();
        let rgba: Vec<u8> = (0..1920 * 1080)
            .flat_map(|p| [(p % 256) as u8, ((p / 1920) % 256) as u8, 128, 255])
            .collect();
        let texture = ctx.load_texture(
            "benchmark",
            egui::ColorImage::from_rgba_unmultiplied([1920, 1080], &rgba),
            TextureOptions::LINEAR,
        );
        let mut renderer = EguiSoftwareRender::new(ColorFieldOrder::Bgra);
        let mut pixels = vec![0u32; width * height];
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        for scale in [1., 4., 8.] {
            let mut times = Vec::new();
            for i in 0..12 {
                let start = Instant::now();
                let view = View {
                    scale,
                    center: [0.4 + i as f32 * 0.01, 0.5],
                }
                .normalized();
                let output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(rect),
                        ..Default::default()
                    },
                    |ui| {
                        pdf_presenter::presenter_ui::paint_slide_adapted(
                            ui.painter(),
                            rect,
                            texture.id(),
                            [210, 297],
                            Default::default(),
                            Sync {
                                enabled: true,
                                view,
                                ..Default::default()
                            },
                        );
                    },
                );
                let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
                renderer.render(
                    &mut BufferMutRef::new(bytemuck::cast_slice_mut(&mut pixels), width, height),
                    &primitives,
                    &output.textures_delta,
                    output.pixels_per_point,
                );
                if i > 1 {
                    times.push(start.elapsed().as_secs_f64() * 1000.);
                }
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{width}x{height} {scale}x: median {:.2} ms, max {:.2} ms",
                times[times.len() / 2],
                times.last().unwrap()
            );
        }
    }
}

#[test]
fn wheel_drag_and_double_click_operate_in_slide_coordinates() {
    let ctx = Context::default();
    let target = Rect::from_min_max(Pos2::new(40., 40.), Pos2::new(440., 240.));
    let mut view = View::default();
    let mut time = 0.;
    let mut render = |events: Vec<egui::Event>| {
        time += 0.04;
        let _ = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 300.))),
                events,
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                view = pdf_presenter::presenter_ui::zoom_gestures(ui, target, view).0;
            },
        );
        view
    };
    let center = target.center();
    render(vec![egui::Event::PointerMoved(center)]);
    render(vec![egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        phase: egui::TouchPhase::Move,
        delta: Vec2::new(0., 300.),
        modifiers: egui::Modifiers::NONE,
    }]);
    let mut zoom = View::default();
    for _ in 0..30 {
        zoom = render(vec![]);
    }
    assert!(zoom.scale > 1.2);
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    render(vec![click(center, true)]);
    let moved = center + Vec2::new(40., 10.);
    let panned = render(vec![egui::Event::PointerMoved(moved)]);
    assert!(panned.center[0] < zoom.center[0]);
    render(vec![click(moved, false)]);
    for _ in 0..20 {
        render(vec![]);
    }
    for _ in 0..2 {
        render(vec![click(moved, true)]);
        zoom = render(vec![click(moved, false)]);
    }
    assert_eq!(zoom, View::default());
}

#[test]
fn optimized_crop_matches_general_renderer_with_alpha_clipping_and_dpi() {
    for dpi in [1., 2.] {
        let ctx = Context::default();
        ctx.set_pixels_per_point(dpi);
        let rgba: Vec<u8> = (0..64 * 32)
            .flat_map(|p| {
                [
                    (p % 64 * 4) as u8,
                    (p / 64 * 8) as u8,
                    80,
                    (40 + p % 64 * 3) as u8,
                ]
            })
            .collect();
        let texture = ctx.load_texture(
            "alpha",
            egui::ColorImage::from_rgba_unmultiplied([64, 32], &rgba),
            TextureOptions::LINEAR,
        );
        let mut fast = EguiSoftwareRender::new(ColorFieldOrder::Bgra);
        let mut reference =
            EguiSoftwareRender::new(ColorFieldOrder::Bgra).with_allow_raster_opt(false);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320., 240.));
        let mut actual = vec![0u32; (320. * dpi) as usize * (240. * dpi) as usize];
        let mut expected = actual.clone();
        for view in [
            View {
                scale: 2.,
                center: [0.25, 0.25],
            },
            View {
                scale: 3.1,
                center: [0.7, 0.4],
            },
        ] {
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    let painter = ui.painter().with_clip_rect(rect.shrink(20.));
                    pdf_presenter::presenter_ui::paint_slide_zoomed(
                        &painter,
                        rect,
                        texture.id(),
                        [64, 32],
                        Default::default(),
                        view,
                    );
                },
            );
            let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
            let width = (320. * output.pixels_per_point) as usize;
            let height = (240. * output.pixels_per_point) as usize;
            // egui applies a requested DPI change on a subsequent frame.
            actual.resize(width * height, 0);
            expected.resize(width * height, 0);
            actual.fill(0);
            expected.fill(0);
            fast.render(
                &mut BufferMutRef::new(bytemuck::cast_slice_mut(&mut actual), width, height),
                &primitives,
                &output.textures_delta,
                output.pixels_per_point,
            );
            reference.render(
                &mut BufferMutRef::new(bytemuck::cast_slice_mut(&mut expected), width, height),
                &primitives,
                &output.textures_delta,
                output.pixels_per_point,
            );
            for (a, b) in actual.iter().zip(&expected) {
                for (a, b) in a.to_le_bytes().iter().zip(b.to_le_bytes()) {
                    assert!(
                        a.abs_diff(b) <= 2,
                        "crop pixel mismatch: {a} vs {b} at DPI {dpi}"
                    );
                }
            }
        }
    }
}

#[test]
fn parallel_crop_bands_have_no_seams_at_fractional_boundaries() {
    let ctx = Context::default();
    let texture = ctx.load_texture(
        "solid",
        egui::ColorImage::from_rgba_unmultiplied([64, 32], &[255, 180, 60, 255].repeat(64 * 32)),
        TextureOptions::LINEAR,
    );
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1931., 1087.));
    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(rect),
            ..Default::default()
        },
        |ui| {
            pdf_presenter::presenter_ui::paint_slide_zoomed(
                ui.painter(),
                rect,
                texture.id(),
                [64, 32],
                Default::default(),
                View {
                    scale: 3.7,
                    center: [0.4, 0.6],
                },
            );
        },
    );
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    assert_eq!(primitives.iter().filter(|p| matches!(&p.primitive, egui::epaint::Primitive::Mesh(m) if m.texture_id == texture.id())).count(),4);
    let mut pixels = vec![0u32; 1931 * 1087];
    EguiSoftwareRender::new(ColorFieldOrder::Bgra).render(
        &mut BufferMutRef::new(bytemuck::cast_slice_mut(&mut pixels), 1931, 1087),
        &primitives,
        &output.textures_delta,
        output.pixels_per_point,
    );
    for y in 62..1025 {
        for x in [50, 965, 1880] {
            assert_eq!(
                pixels[y * 1931 + x] & 0xffffff,
                0xffb43c,
                "band seam at {x},{y}"
            );
        }
    }
}

#[test]
fn portrait_zoom_fills_screen_without_stretching_or_out_of_bounds_pixels() {
    use pdf_presenter::presenter_ui::zoom_geometry;
    for size in [
        Vec2::new(1920., 1080.),
        Vec2::new(1080., 1920.),
        Vec2::new(640., 480.),
    ] {
        let rect = Rect::from_min_size(Pos2::new(12., 18.), size);
        for dimensions in [[210, 297], [297, 210]] {
            let (fitted, _) = zoom_geometry(rect, dimensions, View::default(), false);
            for scale in [1., 1.5, 3., 8.] {
                for center in [[0.5; 2], [0.; 2], [1.; 2]] {
                    let view = View { scale, center };
                    let (target, uv) = zoom_geometry(rect, dimensions, view, false);
                    assert!(rect.contains_rect(target));
                    assert!(Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)).contains_rect(uv));
                    let x_scale = target.width() / (uv.width() * dimensions[0] as f32);
                    let y_scale = target.height() / (uv.height() * dimensions[1] as f32);
                    assert!((x_scale - y_scale).abs() < 0.001, "page was stretched");
                    if scale >= 3. {
                        assert_eq!(target, rect);
                    }
                    assert_eq!(zoom_geometry(rect, dimensions, view, true).0, fitted);
                }
            }
        }
    }
}

#[test]
fn adapted_portrait_pixels_fill_padding_without_uploading_another_texture() {
    let ctx = Context::default();
    let texture = ctx.load_texture(
        "portrait",
        egui::ColorImage::filled([21, 30], egui::Color32::RED),
        TextureOptions::LINEAR,
    );
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(320., 180.));
    let mut renderer = EguiSoftwareRender::new(ColorFieldOrder::Bgra);
    let mut pixels = vec![0u32; 320 * 180];
    for (index, (scale, keep_fitted_area)) in [(1., false), (3., false), (3., true)]
        .into_iter()
        .enumerate()
    {
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ui| {
                pdf_presenter::presenter_ui::paint_slide_adapted(
                    ui.painter(),
                    rect,
                    texture.id(),
                    [21, 30],
                    Default::default(),
                    Sync {
                        enabled: true,
                        keep_fitted_area,
                        view: View {
                            scale,
                            center: [0.5; 2],
                        },
                        ..Default::default()
                    },
                );
            },
        );
        if index > 0 {
            assert!(output.textures_delta.set.is_empty());
        }
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        renderer.render(
            &mut BufferMutRef::new(bytemuck::cast_slice_mut(&mut pixels), 320, 180),
            &primitives,
            &output.textures_delta,
            output.pixels_per_point,
        );
        let edge = pixels[90 * 320 + 4] & 0xffffff;
        let center = pixels[90 * 320 + 160] & 0xffffff;
        if index == 1 {
            assert_eq!(edge, center);
        } else {
            assert_ne!(edge, center);
        }
    }
}
#[test]
fn portrait_screen_adapting_gestures_pan_visible_area_and_reset() {
    let ctx = Context::default();
    let target = Rect::from_min_max(Pos2::new(40., 40.), Pos2::new(440., 240.));
    let mut view = View::default();
    let mut time = 0.;
    let mut render = |events: Vec<egui::Event>| {
        time += 0.04;
        let _ = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 300.))),
                events,
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                view = pdf_presenter::presenter_ui::zoom_gestures_in(
                    ui,
                    target,
                    [210, 297],
                    view,
                    false,
                )
                .0;
            },
        );
        view
    };
    let center = target.center();
    render(vec![egui::Event::PointerMoved(center)]);
    render(vec![egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        phase: egui::TouchPhase::Move,
        delta: Vec2::new(0., 300.),
        modifiers: egui::Modifiers::NONE,
    }]);
    let mut zoom = View::default();
    for _ in 0..30 {
        zoom = render(vec![]);
    }
    assert!(zoom.scale > 1.2);
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    render(vec![click(center, true)]);
    let moved = center + Vec2::new(40., 10.);
    let panned = render(vec![egui::Event::PointerMoved(moved)]);
    assert!(panned.center[1] < zoom.center[1]);
    assert_eq!(
        panned.center[0], 0.5,
        "page width still fits; horizontal panning should stop"
    );
    render(vec![click(moved, false)]);
    for _ in 0..20 {
        render(vec![]);
    }
    for _ in 0..2 {
        render(vec![click(moved, true)]);
        zoom = render(vec![click(moved, false)]);
    }
    assert_eq!(zoom, View::default());
}
