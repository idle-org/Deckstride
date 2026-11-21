mod common;
use deckstride::{document::Document, presenter_ui::paint_slide, settings::SlideColors};
use egui::{Context, Pos2, RawInput, Rect, TextureOptions, Vec2};
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};

#[test]
fn transparent_pdf_composites_on_white_or_custom_page_with_independent_padding() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("transparent.pdf");
    common::pdf(&path);
    // Same-length replacements preserve the synthetic PDF's xref offsets.
    let source = String::from_utf8(std::fs::read(&path).unwrap())
        .unwrap()
        .replace("640 360 re", "320 180 re");
    std::fs::write(&path, &source).unwrap();
    let doc = Document::open(&path, temp.path().join("cache")).unwrap();
    let frame = doc.render(0, 320).unwrap();
    assert_eq!((frame.info.width, frame.info.height), (320, 180));
    assert_eq!(frame.rgba[(90 * 320 + 240) * 4 + 3], 0);
    let cached = std::fs::read(&frame.info.path).unwrap();
    let ctx = Context::default();
    let texture = ctx.load_texture(
        "pdf",
        egui::ColorImage::from_rgba_unmultiplied([320, 180], &frame.rgba),
        TextureOptions::NEAREST,
    );
    let mut renderer = EguiSoftwareRender::new(ColorFieldOrder::Bgra);
    for colors in [
        SlideColors::default(),
        SlideColors {
            page_background: [20, 60, 100],
            fit_padding: [130, 40, 10],
        },
    ] {
        for size in [[400, 400], [800, 200]] {
            let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(size[0] as f32, size[1] as f32));
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    paint_slide(ui.painter(), rect, texture.id(), [320, 180], colors);
                },
            );
            let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
            let mut pixels = vec![0u32; size[0] * size[1]];
            renderer.render(
                &mut BufferMutRef::new(bytemuck::cast_slice_mut(&mut pixels), size[0], size[1]),
                &primitives,
                &output.textures_delta,
                output.pixels_per_point,
            );
            let rgb = |x, y| pixels[y * size[0] + x] & 0xffffff;
            let packed =
                |c: [u8; 3]| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]);
            assert_eq!(rgb(5, 5), packed(colors.fit_padding));
            assert_eq!(
                rgb(size[0] / 2 + 80, size[1] / 2),
                packed(colors.page_background)
            );
            // The opaque green rectangle in the lower left is unaffected by either color.
            let opaque = rgb(size[0] / 2 - 80, size[1] / 2 + 40);
            assert_ne!(opaque, packed(colors.page_background));
            assert_ne!(opaque, packed(colors.fit_padding));
        }
    }
    assert_eq!(std::fs::read(&frame.info.path).unwrap(), cached);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
}

#[test]
fn translucent_pdf_pixels_use_straight_alpha_and_ignore_legacy_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("alpha.pdf");
    let content = "/GS gs 1 0 0 rg 0 0 100 100 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /ExtGState /ca 0.5 >>".to_owned(),
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend(format!("{} 0 obj\n{object}\nendobj\n", i + 1).bytes());
    }
    let start = bytes.len();
    bytes.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).bytes());
    for offset in &offsets[1..] {
        bytes.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    bytes.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
            offsets.len()
        )
        .bytes(),
    );
    std::fs::write(&path, bytes).unwrap();
    let doc = Document::open(&path, temp.path().join("cache")).unwrap();
    let frame = doc.render(0, 160).unwrap();
    let pixel = &frame.rgba[(80 * 160 + 80) * 4..][..4];
    assert_eq!(&pixel[..3], &[255, 0, 0]);
    assert!((127..=128).contains(&pixel[3]));
    let legacy = frame.info.path.with_file_name("0-160-hayro09.rgba");
    std::fs::rename(&frame.info.path, legacy).unwrap();
    assert!(doc.cached(0, 160).unwrap().is_none());
    assert_eq!(doc.render(0, 160).unwrap().rgba, frame.rgba);
}
