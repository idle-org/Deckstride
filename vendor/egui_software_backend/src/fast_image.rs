//! PDF Presenter patch: bounded scanline sampling for cropped, untinted image quads.
//! Keep general meshes, fonts, wrapping, full images and tinting on the upstream path.
use crate::{BufferMutRef, EguiTexture};
use alloc::vec::Vec;
use egui::{Color32, Mesh, Rect, TextureFilter, TextureWrapMode, Vec2};

pub(crate) fn draw(
    buffer: &mut BufferMutRef<'_>,
    clip: &Rect,
    mesh: &Mesh,
    offset: Vec2,
    texture: &EguiTexture,
) -> bool {
    if mesh.texture_id == egui::TextureId::Managed(0)
        || mesh.vertices.len() != 4
        || mesh.indices.len() != 6
        || texture.options.wrap_mode != TextureWrapMode::ClampToEdge
        || mesh
            .vertices
            .iter()
            .any(|v| v.color != Color32::WHITE || !v.pos.is_finite() || !v.uv.is_finite())
    {
        return false;
    }
    let mut rect = Rect::NOTHING;
    let mut uv = Rect::NOTHING;
    for v in &mesh.vertices {
        rect.extend_with(v.pos);
        uv.extend_with(v.uv);
    }
    if rect.width() <= 0.
        || rect.height() <= 0.
        || uv.width() <= 0.
        || uv.height() <= 0.
        || uv.min.x < 0.
        || uv.min.y < 0.
        || uv.max.x > 1.
        || uv.max.y > 1.
        || (uv.width() >= 0.99999 && uv.height() >= 0.99999)
    {
        return false;
    }
    let mut corners = [0usize; 4];
    let mut seen = 0u8;
    for (i, v) in mesh.vertices.iter().enumerate() {
        if (v.pos.x != rect.min.x && v.pos.x != rect.max.x)
            || (v.pos.y != rect.min.y && v.pos.y != rect.max.y)
        {
            return false;
        }
        let x = usize::from(v.pos.x == rect.max.x);
        let y = usize::from(v.pos.y == rect.max.y);
        let c = x + y * 2;
        if seen & (1 << c) != 0
            || v.uv.x != if x == 0 { uv.min.x } else { uv.max.x }
            || v.uv.y != if y == 0 { uv.min.y } else { uv.max.y }
        {
            return false;
        }
        seen |= 1 << c;
        corners[i] = c;
    }
    let mut triangles = [[0usize; 3]; 2];
    for (t, indices) in triangles.iter_mut().zip(mesh.indices.chunks_exact(3)) {
        for (c, i) in t.iter_mut().zip(indices) {
            let Some(corner) = corners.get(*i as usize) else {
                return false;
            };
            *c = *corner;
        }
        t.sort_unstable();
    }
    triangles.sort_unstable();
    if triangles != [[0, 1, 2], [1, 2, 3]] && triangles != [[0, 1, 3], [0, 2, 3]] {
        return false;
    }
    rect = rect.translate(offset);
    let visible = rect.intersect(*clip);
    if !visible.is_positive() {
        return true;
    }
    let bound = |v: f32, max: usize| (v - 0.5).ceil().clamp(0., max as f32) as usize;
    let x0 = bound(visible.min.x, buffer.width);
    let x1 = bound(visible.max.x, buffer.width);
    let y0 = bound(visible.min.y, buffer.height);
    let y1 = bound(visible.max.y, buffer.height);
    if x0 >= x1 || y0 >= y1 {
        return true;
    }
    let step = uv.size() / rect.size();
    let linear = texture.options.magnification != TextureFilter::Nearest;
    let axis = |value: f32, size: usize| {
        if !linear {
            let i = (value * size as f32).floor().clamp(0., (size - 1) as f32) as usize;
            return (i, i, 0u32);
        }
        let p = value * size as f32 - 0.5;
        let lo = p.floor();
        let a = lo.clamp(0., (size - 1) as f32) as usize;
        let b = (lo + 1.).clamp(0., (size - 1) as f32) as usize;
        (a, b, ((p - lo) * 256.).round().clamp(0., 256.) as u32)
    };
    // O(viewport width) scratch, reused across every scanline; no image allocation.
    let columns: Vec<_> = (x0..x1)
        .map(|x| {
            axis(
                uv.min.x + (x as f32 + 0.5 - rect.min.x) * step.x,
                texture.width,
            )
        })
        .collect();
    let mut row0 = alloc::vec![[0u8;4]; columns.len()];
    let mut row1 = row0.clone();
    let mut cached0 = usize::MAX;
    let mut cached1 = usize::MAX;
    let horizontal = |row: &mut [[u8; 4]], source_y: usize| {
        let source = &texture.data[source_y * texture.width..(source_y + 1) * texture.width];
        for (dst, &(a, b, w)) in row.iter_mut().zip(&columns) {
            let a = source[a];
            let b = source[b];
            for c in 0..4 {
                dst[c] = ((u32::from(a[c]) * (256 - w) + u32::from(b[c]) * w + 128) >> 8) as u8;
            }
        }
    };
    for y in y0..y1 {
        let (ya, yb, wy) = axis(
            uv.min.y + (y as f32 + 0.5 - rect.min.y) * step.y,
            texture.height,
        );
        if ya == cached1 {
            core::mem::swap(&mut row0, &mut row1);
            core::mem::swap(&mut cached0, &mut cached1);
        }
        if ya != cached0 {
            horizontal(&mut row0, ya);
            cached0 = ya;
        }
        if yb != cached1 {
            horizontal(&mut row1, yb);
            cached1 = yb;
        }
        let destination = buffer.get_mut_span(x0, x1, y);
        if texture.opaque {
            // Contiguous arithmetic: LLVM vectorizes this hot loop. Enlarged rows
            // reuse horizontally filtered samples instead of sampling four texels
            // and recalculating X coordinates for every screen pixel.
            let weight = wy as u16;
            for ((dst, a), b) in destination.iter_mut().zip(&row0).zip(&row1) {
                for c in 0..4 {
                    dst[c] = ((u16::from(a[c]) * (256 - weight) + u16::from(b[c]) * weight + 128)
                        >> 8) as u8;
                }
            }
        } else {
            for ((dst, a), b) in destination.iter_mut().zip(&row0).zip(&row1) {
                let mut src = [0u8; 4];
                for c in 0..4 {
                    src[c] =
                        ((u32::from(a[c]) * (256 - wy) + u32::from(b[c]) * wy + 128) >> 8) as u8;
                }
                if src[3] == 255 {
                    *dst = src;
                } else if src[3] != 0 {
                    let alpha = 255 - u32::from(src[3]);
                    for c in 0..4 {
                        dst[c] = (u32::from(src[c]) + (u32::from(dst[c]) * alpha + 127) / 255)
                            .min(255) as u8;
                    }
                }
            }
        }
    }
    true
}
