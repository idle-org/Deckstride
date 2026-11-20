//! Bounded, resolution-independent presentation annotations.
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};

pub const MAX_MARKS: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Off,
    Pointer,
    Arrow,
    Highlighter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkKind {
    Arrow,
    Highlighter,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mark {
    pub kind: MarkKind,
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub color: [u8; 3],
    pub width: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Overlay {
    #[serde(default)]
    pub pointer: Option<[f32; 2]>,
    #[serde(default)]
    pub pointer_color: [u8; 3],
    #[serde(default)]
    pub pointer_size: f32,
    #[serde(default)]
    pub marks: Vec<Mark>,
}

impl Overlay {
    pub fn normalized(mut self) -> Self {
        self.pointer = self.pointer.and_then(normalized_point);
        self.pointer_size = finite_clamp(self.pointer_size, 4., 48., 12.);
        self.marks.truncate(MAX_MARKS);
        self.marks.retain_mut(|mark| {
            let Some(from) = normalized_point(mark.from) else {
                return false;
            };
            let Some(to) = normalized_point(mark.to) else {
                return false;
            };
            mark.from = from;
            mark.to = to;
            mark.width = finite_clamp(mark.width, 2., 48., 8.);
            true
        });
        self
    }
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

fn normalized_point(point: [f32; 2]) -> Option<[f32; 2]> {
    point
        .iter()
        .all(|value| value.is_finite())
        .then(|| point.map(|value| value.clamp(0., 1.)))
}

pub struct Toolbox {
    pub open: bool,
    pub tool: Tool,
    pub color: [u8; 3],
    pub size: f32,
    pub overlay: Overlay,
    drag_mark: Option<usize>,
}

impl Default for Toolbox {
    fn default() -> Self {
        Self {
            open: false,
            tool: Tool::Off,
            color: [244, 67, 54],
            size: 10.,
            overlay: Overlay {
                pointer_color: [244, 67, 54],
                pointer_size: 10.,
                ..Default::default()
            },
            drag_mark: None,
        }
    }
}

impl Toolbox {
    pub fn select(&mut self, tool: Tool) -> bool {
        self.tool = tool;
        self.drag_mark = None;
        tool != Tool::Pointer && self.overlay.pointer.take().is_some()
    }

    pub fn set_pointer(&mut self, point: Option<[f32; 2]>) -> bool {
        let point = point.and_then(normalized_point);
        let changed = self.overlay.pointer != point
            || self.overlay.pointer_color != self.color
            || self.overlay.pointer_size != self.size;
        self.overlay.pointer = point;
        self.overlay.pointer_color = self.color;
        self.overlay.pointer_size = self.size;
        changed
    }

    pub fn begin_mark(&mut self, point: [f32; 2]) -> bool {
        let kind = match self.tool {
            Tool::Arrow => MarkKind::Arrow,
            Tool::Highlighter => MarkKind::Highlighter,
            _ => return false,
        };
        if self.overlay.marks.len() == MAX_MARKS {
            self.overlay.marks.remove(0);
        }
        self.overlay.marks.push(Mark {
            kind,
            from: point,
            to: point,
            color: self.color,
            width: self.size,
        });
        self.drag_mark = Some(self.overlay.marks.len() - 1);
        true
    }

    pub fn update_mark(&mut self, point: [f32; 2]) -> bool {
        let Some(mark) = self
            .drag_mark
            .and_then(|index| self.overlay.marks.get_mut(index))
        else {
            return false;
        };
        let Some(point) = normalized_point(point) else {
            return false;
        };
        if mark.to == point {
            return false;
        }
        mark.to = point;
        true
    }

    pub fn end_mark(&mut self) {
        self.drag_mark = None;
    }

    pub fn undo(&mut self) -> bool {
        self.drag_mark = None;
        self.overlay.marks.pop().is_some()
    }

    pub fn clear(&mut self) -> bool {
        self.drag_mark = None;
        let changed = self.overlay.pointer.take().is_some() || !self.overlay.marks.is_empty();
        self.overlay.marks.clear();
        changed
    }
}

pub fn screen_to_page(target: Rect, uv: Rect, point: Pos2) -> [f32; 2] {
    let local = (point - target.min) / target.size().max(Vec2::splat(1.));
    let page = uv.min + local * uv.size();
    [page.x.clamp(0., 1.), page.y.clamp(0., 1.)]
}

fn page_to_screen(target: Rect, uv: Rect, point: [f32; 2]) -> Option<Pos2> {
    let point = Pos2::new(point[0], point[1]);
    uv.contains(point).then(|| {
        target.min + (point - uv.min) / uv.size().max(Vec2::splat(f32::EPSILON)) * target.size()
    })
}

pub fn paint(painter: &egui::Painter, target: Rect, uv: Rect, overlay: &Overlay) {
    paint_scaled(painter, target, uv, overlay, 1.);
}

/// Scale fixed-width marks so a smaller Presenter preview resembles the Slides view.
pub fn paint_scaled(
    painter: &egui::Painter,
    target: Rect,
    uv: Rect,
    overlay: &Overlay,
    scale: f32,
) {
    let scale = finite_clamp(scale, 0.05, 4., 1.);
    for mark in overlay.marks.iter().take(MAX_MARKS) {
        let (Some(from), Some(to)) = (
            page_to_screen(target, uv, mark.from),
            page_to_screen(target, uv, mark.to),
        ) else {
            continue;
        };
        let color = match mark.kind {
            MarkKind::Arrow => Color32::from_rgb(mark.color[0], mark.color[1], mark.color[2]),
            MarkKind::Highlighter => {
                Color32::from_rgba_unmultiplied(mark.color[0], mark.color[1], mark.color[2], 105)
            }
        };
        let width = mark.width * scale;
        if mark.kind == MarkKind::Arrow
            && let Some((shaft_end, head)) = arrow_geometry(from, to, mark.width, scale)
        {
            painter.line_segment([from, shaft_end], Stroke::new(width, color));
            painter.add(egui::Shape::convex_polygon(
                head.to_vec(),
                color,
                Stroke::NONE,
            ));
        } else {
            painter.line_segment([from, to], Stroke::new(width, color));
        }
    }
    if let Some(point) = overlay
        .pointer
        .and_then(|point| page_to_screen(target, uv, point))
    {
        let color = Color32::from_rgb(
            overlay.pointer_color[0],
            overlay.pointer_color[1],
            overlay.pointer_color[2],
        );
        painter.circle_filled(point, overlay.pointer_size * scale, color);
        painter.circle_stroke(
            point,
            (overlay.pointer_size + 2.) * scale,
            Stroke::new(2. * scale, Color32::WHITE),
        );
    }
}

pub fn preview_paint_scale(target: Rect, slides_target: Rect) -> f32 {
    let ratio = (target.width() / slides_target.width().max(f32::EPSILON))
        .min(target.height() / slides_target.height().max(f32::EPSILON));
    finite_clamp(ratio, 0.05, 4., 1.)
}

/// A filled head avoids the three overlapping thick strokes that deform large arrows.
fn arrow_geometry(from: Pos2, to: Pos2, width: f32, scale: f32) -> Option<(Pos2, [Pos2; 3])> {
    let vector = to - from;
    let distance = vector.length();
    if distance <= 2. * scale {
        return None;
    }
    let direction = vector / distance;
    let normal = Vec2::new(-direction.y, direction.x);
    let width = width * scale;
    let head_length = (width * 1.8)
        .clamp(10. * scale, 56. * scale)
        .min(distance * 0.45);
    let head_half_width = (width * 0.9)
        .clamp(6. * scale, 30. * scale)
        .min(head_length * 0.75);
    let base = to - direction * head_length;
    let shaft_end = base + direction * (width * 0.12).min(head_length * 0.12);
    Some((
        shaft_end,
        [
            to,
            base + normal * head_half_width,
            base - normal * head_half_width,
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_arrow_has_a_bounded_filled_head_and_shortened_shaft() {
        let from = Pos2::new(0., 0.);
        let to = Pos2::new(0., 200.);
        let (shaft_end, head) = arrow_geometry(from, to, 48., 1.).unwrap();
        assert!(shaft_end.y < to.y);
        assert_eq!(head[0], to);
        assert!(head[1].distance(head[2]) <= 60.);
        assert!(to.distance(head[1]) < 70.);

        let (_, short_head) = arrow_geometry(from, Pos2::new(20., 0.), 48., 1.).unwrap();
        assert!(short_head[0].distance(short_head[1]) < 15.);
    }

    #[test]
    fn preview_scale_keeps_arrow_head_proportional() {
        let slides = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600., 900.));
        let preview = Rect::from_min_size(Pos2::ZERO, Vec2::new(400., 225.));
        assert_eq!(preview_paint_scale(preview, slides), 0.25);

        let (_, full) = arrow_geometry(Pos2::ZERO, Pos2::new(400., 0.), 32., 1.).unwrap();
        let (_, small) = arrow_geometry(Pos2::ZERO, Pos2::new(100., 0.), 32., 0.25).unwrap();
        assert!((full[1].distance(full[2]) * 0.25 - small[1].distance(small[2])).abs() < 0.01);
    }
}
