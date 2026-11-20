use crate::localization::tr;
// Native controls for the approved presenter layout and safe slide browser.
use crate::{
    settings::{Layout, TimerMode, TimerPosition},
    studio::{Action, SlideLoadState, Studio},
    tools::Tool,
};
use egui::{Color32, Context, FontId, Pos2, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
const BROWSER_SCROLL_HANDLE: Color32 = Color32::from_rgb(70, 210, 175);
/// Composite a fitted slide over independent page and padding colors in every view.
pub fn paint_slide(
    painter: &egui::Painter,
    rect: Rect,
    texture: egui::TextureId,
    dimensions: [u32; 2],
    colors: crate::settings::SlideColors,
) {
    paint_slide_zoomed(
        painter,
        rect,
        texture,
        dimensions,
        colors,
        Default::default(),
    );
}
pub fn fitted_slide(rect: Rect, dimensions: [u32; 2]) -> Rect {
    let [width, height] = dimensions.map(|v| v.max(1) as f32);
    let scale = (rect.width() / width).min(rect.height() / height);
    Rect::from_center_size(rect.center(), Vec2::new(width * scale, height * scale))
}
/// Crop UVs, never enlarge geometry or allocate another image for zoom.
pub fn paint_slide_zoomed(
    painter: &egui::Painter,
    rect: Rect,
    texture: egui::TextureId,
    dimensions: [u32; 2],
    colors: crate::settings::SlideColors,
    view: crate::zoom::View,
) {
    paint_slide_adapted(
        painter,
        rect,
        texture,
        dimensions,
        colors,
        crate::zoom::Sync {
            enabled: true,
            view,
            keep_fitted_area: true,
            ..Default::default()
        },
    );
}
/// Visible page rectangle and texture crop; geometry never exceeds the viewport.
pub fn zoom_geometry(
    rect: Rect,
    dimensions: [u32; 2],
    view: crate::zoom::View,
    keep_fitted_area: bool,
) -> (Rect, Rect) {
    let view = view.normalized();
    let fitted = fitted_slide(rect, dimensions);
    if keep_fitted_area || view.scale == 1. {
        return (fitted, view.uv());
    }
    let enlarged = fitted.size() * view.scale;
    let size = enlarged.min(rect.size());
    let span = size / enlarged;
    let center =
        Vec2::new(view.center[0], view.center[1]).clamp(span * 0.5, Vec2::splat(1.) - span * 0.5);
    (
        Rect::from_center_size(rect.center(), size),
        Rect::from_center_size(center.to_pos2(), span),
    )
}
/// Adapt to the viewport using the existing buffered texture, with no rerender.
pub fn paint_slide_adapted(
    painter: &egui::Painter,
    rect: Rect,
    texture: egui::TextureId,
    dimensions: [u32; 2],
    colors: crate::settings::SlideColors,
    zoom: crate::zoom::Sync,
) {
    let view = zoom.normalized().view;
    let (target, uv) = zoom_geometry(rect, dimensions, view, zoom.keep_fitted_area);
    let [r, g, b] = colors.fit_padding;
    painter.rect_filled(rect, 0., Color32::from_rgb(r, g, b));
    let [r, g, b] = colors.page_background;
    painter.rect_filled(target, 0., Color32::from_rgb(r, g, b));
    // Separate clip bands keep large moving crops parallel in the bounded CPU
    // renderer pool. Every band reads the same texture; no image is duplicated.
    let pixels = target.area() * painter.ctx().pixels_per_point().powi(2);
    let bands = if view.scale > 1. && pixels > 1_000_000. {
        4
    } else {
        1
    };
    for band in 0..bands {
        let a = band as f32 / bands as f32;
        let b = (band + 1) as f32 / bands as f32;
        let rect = Rect::from_min_max(
            Pos2::new(target.left(), target.top() + target.height() * a),
            Pos2::new(target.right(), target.top() + target.height() * b),
        );
        let crop = Rect::from_min_max(
            Pos2::new(uv.left(), uv.top() + uv.height() * a),
            Pos2::new(uv.right(), uv.top() + uv.height() * b),
        );
        painter
            .with_clip_rect(painter.clip_rect().intersect(rect))
            .image(texture, rect, crop, Color32::WHITE);
    }
}
/// Shared presenter/output input in normalized slide coordinates; no file I/O.
pub fn zoom_gestures(
    ui: &mut Ui,
    target: Rect,
    view: crate::zoom::View,
) -> (crate::zoom::View, egui::Response) {
    zoom_gestures_in(
        ui,
        target,
        [
            target.width().max(1.) as u32,
            target.height().max(1.) as u32,
        ],
        view,
        true,
    )
}
/// Screen-adapted gestures anchor the actual visible crop, including page padding.
pub fn zoom_gestures_in(
    ui: &mut Ui,
    rect: Rect,
    dimensions: [u32; 2],
    view: crate::zoom::View,
    keep_fitted_area: bool,
) -> (crate::zoom::View, egui::Response) {
    let (target, uv) = zoom_geometry(rect, dimensions, view, keep_fitted_area);
    let response = ui.interact(target, ui.id().with("page-zoom"), Sense::click_and_drag());
    let mut next = view;
    if response.hovered() {
        let scroll = ui.input_mut(|i| {
            let scroll = i.smooth_scroll_delta.y;
            i.smooth_scroll_delta = Vec2::ZERO;
            scroll
        });
        let pinch = ui.input(|i| i.zoom_delta());
        if scroll != 0. || pinch != 1. {
            next.scale =
                (view.scale * (scroll * 0.002).clamp(-0.7, 0.7).exp() * pinch).clamp(1., 8.);
            let pos = response.hover_pos().unwrap_or(target.center());
            let source = uv.min + (pos - target.min) / target.size() * uv.size();
            let (new_target, new_uv) = zoom_geometry(rect, dimensions, next, keep_fitted_area);
            let center = source - (pos - new_target.center()) / new_target.size() * new_uv.size();
            let half = new_uv.size() * 0.5;
            let center = center.to_vec2().clamp(half, Vec2::splat(1.) - half);
            next.center = [center.x, center.y];
        }
    }
    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = ui.input(|i| i.pointer.delta()) / target.size() * uv.size();
        let half = uv.size() * 0.5;
        let center = (uv.center().to_vec2() - delta).clamp(half, Vec2::splat(1.) - half);
        next.center = [center.x, center.y];
    }
    if response.double_clicked() {
        next = Default::default();
        if view.normalized().scale == 1. {
            let source = response.hover_pos().map_or(Pos2::new(0.5, 0.5), |pos| {
                uv.min + (pos - target.min) / target.size() * uv.size()
            });
            let fitted = fitted_slide(rect, dimensions);
            if fitted.width() > 0. {
                next.scale = (rect.width() / fitted.width()).clamp(1., 8.);
                let (_, fitted_uv) = zoom_geometry(rect, dimensions, next, keep_fitted_area);
                let half = fitted_uv.size() * 0.5;
                let center = source.to_vec2().clamp(half, Vec2::splat(1.) - half);
                next.center = [center.x, center.y];
            }
        }
    }
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(if response.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    }
    (next, response)
}
const TEAL: Color32 = Color32::from_rgb(103, 226, 205);
pub struct UiState {
    pub textures: HashMap<(u64, usize, u32), TextureHandle>,
    pub header_until: Instant,
    pub footer_until: Instant,
    pub navigation_hold: Option<crate::navigation::Hold>,
    note_cache: Option<(String, Vec2, Arc<egui::Galley>)>,
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            textures: HashMap::new(),
            header_until: Instant::now(),
            footer_until: Instant::now(),
            note_cache: None,
            navigation_hold: None,
        }
    }
}
impl UiState {
    pub fn image(&mut self, ui: &mut Ui, s: &Studio, page: usize, rect: Rect) {
        self.image_view(ui, s, page, rect, Default::default(), false);
    }
    fn browser_image(&mut self, ui: &mut Ui, s: &Studio, page: usize, rect: Rect) {
        self.image_view(ui, s, page, rect, Default::default(), true);
    }
    fn image_view(
        &mut self,
        ui: &mut Ui,
        s: &Studio,
        page: usize,
        rect: Rect,
        view: crate::zoom::View,
        browser: bool,
    ) {
        let locale = ui.ctx().clone();
        ui.painter()
            .rect_filled(rect, 6., Color32::from_rgb(18, 28, 31));
        if let Some(frame) = s.frame(page) {
            let key = (s.generation, page, frame.info.width.max(frame.info.height));
            let tex = self.textures.entry(key).or_insert_with(|| {
                ui.ctx().load_texture(
                    format!("slide-{}-{}-{}", key.0, key.1, key.2),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [frame.info.width as usize, frame.info.height as usize],
                        &frame.rgba,
                    ),
                    TextureOptions::LINEAR,
                )
            });
            paint_slide_adapted(
                ui.painter(),
                rect,
                tex.id(),
                [frame.info.width, frame.info.height],
                s.settings.slide_colors,
                crate::zoom::Sync { view, ..s.zoom },
            );
        } else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if page >= s.count() {
                    tr(&locale, "No slide")
                } else {
                    tr(
                        &locale,
                        match (browser, s.slide_load_state(page)) {
                            (true, SlideLoadState::Preparing) => "Loading into memory…",
                            (true, SlideLoadState::Cached) => "On disk · loads nearby",
                            (true, SlideLoadState::Queued) => "Not loaded · waiting",
                            (true, SlideLoadState::Failed) => "Could not load slide",
                            (true, SlideLoadState::Ready) => "Slide ready",
                            (false, SlideLoadState::Ready) => "Slide ready",
                            (false, SlideLoadState::Preparing) => "Preparing slide…",
                            (false, SlideLoadState::Cached) => "Cached · loads when nearby",
                            (false, SlideLoadState::Queued) => "Queued for preparation",
                            (false, SlideLoadState::Failed) => "Could not load slide",
                        },
                    )
                },
                FontId::proportional(18.),
                Color32::GRAY,
            );
        }
    }
    fn trim(&mut self, s: &Studio) {
        self.textures.retain(|(generation, page, edge), _| {
            *generation == s.generation
                && (page.abs_diff(s.requested) <= 1
                    || (s.browser_open && s.visible_thumbs.contains(page)))
                && s.frames
                    .values()
                    .any(|f| f.info.page == *page && f.info.width.max(f.info.height) == *edge)
        });
    }
}
pub fn theme(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(16, 21, 23);
    visuals.window_fill = Color32::from_rgb(25, 37, 40);
    visuals.selection.bg_fill = Color32::from_rgb(37, 90, 77);
    visuals.selection.stroke.color = TEAL;
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.spacing.button_padding = egui::vec2(10., 7.);
        style.spacing.interact_size = egui::vec2(30., 32.);
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(15.));
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.));
    });
}
fn fullscreen_button(ui: &mut Ui, s: &mut Studio) {
    let label = if s.presenter_fullscreen() && s.presentation_fullscreen() {
        "Exit fullscreen"
    } else {
        "Enter fullscreen"
    };
    if ui
        .add_enabled(
            s.route_trial.is_none(),
            egui::Button::new(tr(ui.ctx(), label)),
        )
        .clicked()
    {
        s.actions.push(Action::ToggleFullscreen);
    }
}

fn tool_button(ui: &mut Ui, toolbox: &mut crate::tools::Toolbox, tool: Tool, label: &str) {
    if ui.selectable_label(toolbox.tool == tool, label).clicked() {
        toolbox.select(tool);
    }
}

fn toolbox_window(ctx: &Context, s: &mut Studio) {
    if !s.toolbox.open {
        return;
    }
    let mut open = true;
    let before = (s.toolbox.overlay.pointer, s.toolbox.overlay.marks.len());
    egui::Window::new(tr(ctx, "Tools"))
        .id(egui::Id::new("presentation-tools"))
        .open(&mut open)
        .default_pos(ctx.content_rect().right_top() + Vec2::new(-330., 72.))
        .default_width(300.)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                tool_button(ui, &mut s.toolbox, Tool::Off, tr(ctx, "Off"));
                tool_button(ui, &mut s.toolbox, Tool::Pointer, tr(ctx, "Pointer"));
                tool_button(ui, &mut s.toolbox, Tool::Arrow, tr(ctx, "Arrow"));
                tool_button(
                    ui,
                    &mut s.toolbox,
                    Tool::Highlighter,
                    tr(ctx, "Highlighter"),
                );
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(tr(ctx, "Color"));
                for color in [
                    [244, 67, 54],
                    [255, 214, 0],
                    [0, 210, 255],
                    [255, 255, 255],
                ] {
                    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.), Sense::click());
                    ui.painter().circle_filled(
                        rect.center(),
                        9.,
                        Color32::from_rgb(color[0], color[1], color[2]),
                    );
                    if color == s.toolbox.color {
                        ui.painter().circle_stroke(
                            rect.center(),
                            12.,
                            egui::Stroke::new(2_f32, ui.visuals().hyperlink_color),
                        );
                    }
                    if response.clicked() {
                        s.toolbox.color = color;
                    }
                }
            });
            ui.add(egui::Slider::new(&mut s.toolbox.size, 4. ..=32.).text(tr(ctx, "Size")));
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!s.toolbox.overlay.marks.is_empty(), egui::Button::new(tr(ctx, "Undo")))
                    .clicked()
                {
                    s.toolbox.undo();
                }
                if ui
                    .add_enabled(
                        s.toolbox.overlay.pointer.is_some() || !s.toolbox.overlay.marks.is_empty(),
                        egui::Button::new(tr(ctx, "Clear")),
                    )
                    .clicked()
                {
                    s.toolbox.clear();
                }
            });
            ui.small(tr(
                ctx,
                "Pointer follows the current slide. Drag to place arrows or highlights. Up to 64 vector marks; slide images are never rerendered.",
            ));
        });
    s.toolbox.open = open;
    if (s.toolbox.overlay.pointer, s.toolbox.overlay.marks.len()) != before {
        s.new_overlay = true;
    }
}

fn slide_tools(ui: &mut Ui, s: &mut Studio, area: Rect, dimensions: [u32; 2]) {
    let (target, uv) = zoom_geometry(area, dimensions, s.zoom.view, s.zoom.keep_fitted_area);
    let sense = match s.toolbox.tool {
        Tool::Arrow | Tool::Highlighter => Sense::click_and_drag(),
        Tool::Pointer => Sense::hover(),
        Tool::Off => Sense::hover(),
    };
    let response = ui.interact(target, ui.id().with("slide-tools"), sense);
    let mut changed = false;
    match s.toolbox.tool {
        Tool::Off => changed |= s.toolbox.set_pointer(None),
        Tool::Pointer => {
            let point = response
                .hover_pos()
                .map(|point| crate::tools::screen_to_page(target, uv, point));
            changed |= s.toolbox.set_pointer(point);
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        }
        Tool::Arrow | Tool::Highlighter => {
            changed |= s.toolbox.set_pointer(None);
            if response.drag_started()
                && let Some(point) = response.interact_pointer_pos()
            {
                changed |= s
                    .toolbox
                    .begin_mark(crate::tools::screen_to_page(target, uv, point));
            }
            if response.dragged()
                && let Some(point) = response.interact_pointer_pos()
            {
                changed |= s
                    .toolbox
                    .update_mark(crate::tools::screen_to_page(target, uv, point));
            }
            if response.drag_stopped() {
                s.toolbox.end_mark();
            }
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        }
    }
    crate::tools::paint(ui.painter(), target, uv, &s.toolbox.overlay);
    if changed {
        s.presentation_action();
        s.new_overlay = true;
    }
}

fn navigation_button(
    ui: &mut Ui,
    s: &mut Studio,
    native: &mut UiState,
    label: &str,
    direction: i32,
) {
    let response = ui.button(label);
    let now = Instant::now();
    let active = response.is_pointer_button_down_on() && response.hovered();
    let owned = native
        .navigation_hold
        .is_some_and(|h| h.direction == direction);
    if active {
        if !owned {
            native.navigation_hold = Some(crate::navigation::Hold::new(direction, now));
            s.actions.push(Action::Navigate {
                delta: direction,
                repeated: false,
            });
        } else if let Some(delta) = native
            .navigation_hold
            .as_mut()
            .unwrap()
            .tick(now, s.count())
        {
            s.actions.push(Action::Navigate {
                delta,
                repeated: true,
            });
        }
    } else if owned {
        native.navigation_hold = None;
        s.actions.push(Action::FinishNavigation);
    } else if response.clicked() {
        s.actions.push(Action::Navigate {
            delta: direction,
            repeated: false,
        });
    }
}
fn button(ui: &mut Ui, label: &str, action: Action, actions: &mut Vec<Action>) {
    if ui.button(label).clicked() {
        actions.push(action);
    }
}
// Paint the bar/chevron directly: visibility controls must not depend on font glyphs.
fn bar_visibility_button(ui: &mut Ui, pinned: bool, header: bool) -> egui::Response {
    let label = tr(
        ui.ctx(),
        match (header, pinned) {
            (true, true) => "Auto-hide header",
            (false, true) => "Auto-hide footer",
            (true, false) => "Keep header visible",
            (false, false) => "Keep footer visible",
        },
    )
    .to_owned();
    let response = ui.add(egui::Button::new("").min_size(Vec2::splat(28.)));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
    });
    let center = response.rect.center();
    let stroke = egui::Stroke::new(2_f32, ui.style().interact(&response).fg_stroke.color);
    let edge = if header { -7. } else { 7. };
    ui.painter().line_segment(
        [center + Vec2::new(-7., edge), center + Vec2::new(7., edge)],
        stroke,
    );
    let direction = if header == pinned { -1. } else { 1. };
    ui.painter().add(egui::Shape::line(
        vec![
            center + Vec2::new(-5., -2. * direction),
            center + Vec2::new(0., 3. * direction),
            center + Vec2::new(5., -2. * direction),
        ],
        stroke,
    ));
    response.on_hover_text(label)
}
fn layout_section(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(12.)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong(title);
            ui.add_space(4.);
            content(ui);
        });
}
fn widget(
    ui: &mut Ui,
    label: &str,
    w: &mut crate::settings::Widget,
    min: f32,
    max: f32,
    size_label: &str,
) {
    let locale = ui.ctx().clone();
    ui.spacing_mut().slider_width = (ui.available_width() - 155.).clamp(60., 160.);
    ui.horizontal_wrapped(|ui| {
        ui.add_sized([150., 30.], egui::Checkbox::new(&mut w.show, label));
        ui.add_enabled(
            w.show,
            egui::Slider::new(&mut w.size, min..=max).text(tr(&locale, size_label)),
        );
    });
}
fn timer_buttons(ui: &mut Ui, s: &mut Studio) {
    let locale = ui.ctx().clone();
    let running = s.timer.running(Instant::now(), s.timer_limit());
    if ui
        .button(tr(
            &locale,
            if running {
                "Pause timer"
            } else {
                "Start timer"
            },
        ))
        .clicked()
    {
        s.execute(Action::ToggleTimer);
    }
    if ui.button(tr(&locale, "Reset timer")).clicked() {
        s.execute(Action::ResetTimer);
    }
}
fn timer_controls(ui: &mut Ui, s: &mut Studio) {
    let locale = ui.ctx().clone();
    ui.horizontal_wrapped(|ui| timer_buttons(ui, s));
    let running = s.timer.running(Instant::now(), s.timer_limit());
    let state = if running {
        "Running"
    } else if s.timer.waiting_for_action && s.layout.timer_autostart {
        "Waiting for first action"
    } else if s.timer_limit().is_some() && s.timer.seconds(Instant::now(), s.timer_limit()) == 0 {
        "Time is up"
    } else {
        "Stopped"
    };
    ui.small(tr(&locale, state));
}
fn timer_settings(ui: &mut Ui, s: &mut Studio) {
    let locale = ui.ctx().clone();
    let before = (
        s.layout.timer_mode,
        s.layout.countdown_seconds,
        s.layout.timer_autostart,
    );
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(
            &mut s.layout.timer_mode,
            TimerMode::Elapsed,
            tr(&locale, "Elapsed time"),
        );
        ui.selectable_value(
            &mut s.layout.timer_mode,
            TimerMode::Countdown,
            tr(&locale, "Countdown"),
        );
    });
    if s.layout.timer_mode == TimerMode::Countdown {
        let mut minutes = s.layout.countdown_seconds / 60;
        let mut seconds = s.layout.countdown_seconds % 60;
        ui.horizontal_wrapped(|ui| {
            ui.label(tr(&locale, "Duration"));
            ui.add(
                egui::DragValue::new(&mut minutes)
                    .range(0..=1440)
                    .suffix(" min"),
            );
            ui.add(
                egui::DragValue::new(&mut seconds)
                    .range(0..=59)
                    .suffix(" s"),
            );
        });
        s.layout.countdown_seconds = (minutes * 60 + seconds).clamp(1, 86400);
    }
    ui.checkbox(
        &mut s.layout.timer_autostart,
        tr(&locale, "Start on first action"),
    );
    ui.small(tr(
        &locale,
        "Start begins immediately. Reset waits for the next presentation action.",
    ));
    if before
        != (
            s.layout.timer_mode,
            s.layout.countdown_seconds,
            s.layout.timer_autostart,
        )
    {
        s.timer = crate::timer::Timer::default();
    }
    timer_controls(ui, s);
}

fn timer_position(ui: &mut Ui, id: &str, position: &mut TimerPosition) {
    let locale = ui.ctx().clone();
    egui::ComboBox::from_id_salt(id)
        .selected_text(tr(&locale, position.label()))
        .show_ui(ui, |ui| {
            for choice in TimerPosition::ALL {
                ui.selectable_value(position, choice, tr(&locale, choice.label()));
            }
        });
}
fn timer_height(layout: &Layout, position: TimerPosition) -> f32 {
    [
        (&layout.elapsed, layout.elapsed_position),
        (&layout.clock, layout.clock_position),
    ]
    .into_iter()
    .filter(|(w, p)| w.show && *p == position)
    .map(|(w, _)| w.size * 1.3 + 36.)
    .fold(0., f32::max)
}
fn timers(ui: &mut Ui, s: &Studio, layout: &Layout, position: TimerPosition) {
    let locale = ui.ctx().clone();
    let secs = s.timer.seconds(Instant::now(), s.timer_limit());
    let entries = [
        (
            &layout.elapsed,
            layout.elapsed_position,
            if s.layout.timer_mode == TimerMode::Countdown {
                "Countdown"
            } else {
                "Elapsed time"
            },
            format!("{:02}:{:02}", secs / 60, secs % 60),
        ),
        (
            &layout.clock,
            layout.clock_position,
            "Time · HH:MM",
            chrono::Local::now().format("%H:%M").to_string(),
        ),
    ];
    let count = entries
        .iter()
        .filter(|(w, p, _, _)| w.show && *p == position)
        .count();
    if count == 0 {
        return;
    }
    let stacked = matches!(
        position,
        TimerPosition::LeftOfNotes
            | TimerPosition::RightOfNotes
            | TimerPosition::LeftOfSlides
            | TimerPosition::RightOfSlides
    );
    let available = ui.available_size();
    let width = if stacked {
        available.x
    } else {
        (available.x - (12. + ui.spacing().item_spacing.x) * count.saturating_sub(1) as f32)
            / count as f32
    }
    .max(1.);
    let height = if stacked {
        (available.y - (12. + ui.spacing().item_spacing.y) * count.saturating_sub(1) as f32)
            / count as f32
    } else {
        available.y
    }
    .max(1.);
    for (index, (w, _, label, value)) in entries
        .into_iter()
        .filter(|(w, p, _, _)| w.show && *p == position)
        .enumerate()
    {
        if index > 0 {
            ui.add_space(12.);
        }
        let desired = ui.painter().layout_no_wrap(
            value.clone(),
            FontId::proportional(w.size),
            ui.visuals().text_color(),
        );
        let size = if stacked {
            Vec2::new(width, height)
        } else {
            Vec2::new(
                (desired.size().x + 24.).min(width),
                (w.size * 1.3 + 36.).min(height),
            )
        };
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 6., ui.visuals().extreme_bg_color);
        painter.text(
            rect.min + Vec2::splat(8.),
            egui::Align2::LEFT_TOP,
            tr(&locale, label),
            FontId::proportional(12.),
            ui.visuals().weak_text_color(),
        );
        let area = Rect::from_min_max(rect.min + Vec2::new(8., 28.), rect.max - Vec2::splat(8.));
        if area.is_positive() {
            let scale = (area.width() / desired.size().x)
                .min(area.height() / desired.size().y)
                .min(1.);
            painter.text(
                area.center(),
                egui::Align2::CENTER_CENTER,
                value,
                FontId::proportional((w.size * scale).max(1.)),
                ui.visuals().text_color(),
            );
        }
    }
}
fn side_timers(
    ui: &mut Ui,
    s: &Studio,
    layout: &Layout,
    rect: &mut Rect,
    left: TimerPosition,
    right: TimerPosition,
) {
    let original = *rect;
    for (position, is_left) in [(left, true), (right, false)] {
        let size = timer_height(layout, position);
        if size == 0. || !original.is_positive() {
            continue;
        }
        let width = ((size - 36.) / 1.3 * 3.6 + 24.).min(original.width() * 0.32);
        let column = if is_left {
            Rect::from_min_max(
                original.min,
                Pos2::new(original.left() + width, original.bottom()),
            )
        } else {
            Rect::from_min_max(
                Pos2::new(original.right() - width, original.top()),
                original.max,
            )
        };
        if is_left {
            rect.min.x = column.right() + 8.;
        } else {
            rect.max.x = column.left() - 8.;
        }
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(column)
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(column));
                timers(ui, s, layout, position);
            },
        );
    }
}

/// Render a legacy/default view (also used by headless UI callers).
pub fn presenter(ui: &mut Ui, s: &mut Studio, native: &mut UiState) {
    let mut view = crate::studio::PresenterView {
        layout: s.layout.clone(),
        layout_open: s.layout_open,
        layout_target: None,
        menu_open: s.menu_open,
    };
    presenter_view(ui, s, native, &mut view, None);
    view.layout.timer_mode = s.layout.timer_mode;
    view.layout.countdown_seconds = s.layout.countdown_seconds;
    view.layout.timer_autostart = s.layout.timer_autostart;
    s.layout = view.layout;
    s.layout_open = view.layout_open;
    s.menu_open = view.menu_open;
}
/// A native presenter always edits and renders the layout of its own monitor.
pub fn presenter_on_display(ui: &mut Ui, s: &mut Studio, native: &mut UiState, display: &str) {
    let mut view = s.take_presenter_view(display);
    presenter_view(ui, s, native, &mut view, Some(display));
    s.presenter_views.insert(display.into(), view);
}
fn presenter_view(
    ui: &mut Ui,
    s: &mut Studio,
    native: &mut UiState,
    view: &mut crate::studio::PresenterView,
    display: Option<&str>,
) {
    let locale = ui.ctx().clone();
    native.trim(s);
    let ctx = ui.ctx().clone();
    let screen = ui.max_rect();
    ui.painter()
        .rect_filled(screen, 0., ui.visuals().panel_fill);
    let now = Instant::now();
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    if pointer.is_some_and(|p| p.y < screen.top() + 72.) {
        native.header_until = now + Duration::from_millis(700);
    }
    if pointer.is_some_and(|p| p.y > screen.bottom() - 72.) {
        native.footer_until = now + Duration::from_millis(700);
    }
    let header = view.layout.header || native.header_until > now || view.menu_open;
    let footer = view.layout.footer || native.footer_until > now;
    if native.navigation_hold.is_some() && (!footer || !ctx.input(|i| i.focused)) {
        native.navigation_hold = None;
        s.actions.push(Action::FinishNavigation);
    }

    let background = ui.interact(screen, ui.id().with("context"), Sense::click());
    background.context_menu(|ui| {
        if !s.presentation_started
            && s.route_trial.is_none()
            && ui
                .button(if s.settings.language.french() {
                    "Inverser les écrans…"
                } else {
                    "Swap displays…"
                })
                .clicked()
        {
            s.actions.push(Action::SwapRoutes);
            ui.close();
        }
        if s.route_trial.is_some() && ui.button("Revert display change").clicked() {
            s.actions.push(Action::RevertRoutes);
            ui.close();
        }
        ui.checkbox(&mut view.layout.header, tr(&locale, "Keep header visible"));
        ui.checkbox(&mut view.layout.footer, tr(&locale, "Keep footer visible"));
        if ui.button(tr(&locale, "Show both")).clicked() {
            view.layout.header = true;
            view.layout.footer = true;
            ui.close();
        }
        if ui.button(tr(&locale, "Layout…")).clicked() {
            view.layout_open = true;
            ui.close();
        }
    });
    let header_timer_height = timer_height(&view.layout, TimerPosition::Header);
    let separate_header_timers =
        header_timer_height > 100. || (header_timer_height > 0. && screen.width() < 1400.);
    let header_controls_height = if screen.width() < 1100. { 76. } else { 44. };
    let header_height = (header_timer_height
        + 20.
        + if separate_header_timers {
            header_controls_height
        } else {
            0.
        })
    .max(header_controls_height + 20.)
    .min(screen.height() * 0.35);
    let mut body = screen.shrink(16.);
    if view.layout.header {
        body.min.y += header_height;
    }
    if view.layout.footer {
        body.max.y -= 90.;
    }
    if view.layout.slide_progress {
        let height = view
            .layout
            .slide_progress_height
            .min(body.height().max(0.) * 0.25);
        let rect = Rect::from_min_max(Pos2::new(body.left(), body.bottom() - height), body.max);
        body.max.y -= height + 8.;
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            let current = if s.count() == 0 {
                0
            } else {
                s.requested.min(s.count() - 1) + 1
            };
            let fraction = current as f32 / s.count().max(1) as f32;
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_width(rect.width())
                    .desired_height(height)
                    .text(format!(
                        "{} {current} / {}",
                        tr(&locale, "Slide"),
                        s.count()
                    )),
            );
        });
    }
    for position in [TimerPosition::AboveSlides, TimerPosition::BelowNotes] {
        let height = timer_height(&view.layout, position).min(body.height().max(0.) * 0.35);
        if height > 0. {
            let top = if position == TimerPosition::AboveSlides {
                body.top()
            } else {
                body.bottom() - height
            };
            let rect =
                Rect::from_min_size(Pos2::new(body.left(), top), Vec2::new(body.width(), height));
            if position == TimerPosition::AboveSlides {
                body.min.y += height + 8.;
            } else {
                body.max.y -= height + 8.;
            }
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| timers(ui, s, &view.layout, position),
            );
        }
    }
    let preview_visible =
        view.layout.previous.show || view.layout.current.show || view.layout.next.show;
    let notes_h = if view.layout.notes.show {
        if preview_visible {
            body.height() * view.layout.notes.size / 100.
        } else {
            body.height()
        }
    } else {
        0.
    };
    let mut preview = Rect::from_min_max(
        body.min,
        Pos2::new(
            body.max.x,
            body.max.y
                - notes_h
                - if notes_h > 0. && preview_visible {
                    14.
                } else {
                    0.
                },
        ),
    );
    let mut notes_rect =
        Rect::from_min_max(Pos2::new(body.left(), body.bottom() - notes_h), body.max);
    // When a pane is hidden, use the remaining body so enabled timers stay accessible.
    if !preview_visible {
        preview = body;
    }
    if !view.layout.notes.show {
        notes_rect = body;
    }
    side_timers(
        ui,
        s,
        &view.layout,
        &mut preview,
        TimerPosition::LeftOfSlides,
        TimerPosition::RightOfSlides,
    );
    if !preview_visible {
        notes_rect = preview;
    }
    if !view.layout.notes.show {
        notes_rect = preview;
    }
    side_timers(
        ui,
        s,
        &view.layout,
        &mut notes_rect,
        TimerPosition::LeftOfNotes,
        TimerPosition::RightOfNotes,
    );
    if !view.layout.notes.show {
        preview = notes_rect;
    }
    if preview_visible {
        let items = [
            (
                tr(&locale, "PREVIOUS"),
                s.requested.checked_sub(1),
                view.layout.previous.clone(),
            ),
            (
                tr(&locale, "CURRENT"),
                Some(s.requested),
                view.layout.current.clone(),
            ),
            (
                tr(&locale, "NEXT"),
                (s.requested + 1 < s.count()).then_some(s.requested + 1),
                view.layout.next.clone(),
            ),
        ];
        let total: f32 = items
            .iter()
            .filter(|(_, _, w)| w.show)
            .map(|(_, _, w)| w.size)
            .sum();
        let count = items.iter().filter(|(_, _, w)| w.show).count();
        let width = preview.width() - 14. * count.saturating_sub(1) as f32;
        let mut x = preview.left();
        for (index, (label, page, w)) in items.into_iter().enumerate() {
            if !w.show {
                continue;
            }
            let rect = Rect::from_min_size(
                Pos2::new(x, preview.top()),
                Vec2::new(width * w.size / total, preview.height()),
            );
            ui.painter().text(
                rect.min,
                egui::Align2::LEFT_TOP,
                format!(
                    "{label}  {}",
                    page.map_or(String::new(), |p| format!("{} / {}", p + 1, s.count()))
                ),
                FontId::proportional(13.),
                ui.visuals().hyperlink_color,
            );
            if let Some(page) = page {
                let area = Rect::from_min_max(rect.min + Vec2::new(0., 24.), rect.max);
                if index == 1
                    && (s.settings.zoom_enabled
                        || s.toolbox.tool != Tool::Off
                        || !s.toolbox.overlay.marks.is_empty())
                {
                    // Mirror the output aspect, not the independently sized presenter pane.
                    // This makes the shared zoom/center produce the same visible page crop.
                    let area = s
                        .presentation_viewport()
                        .map_or(area, |dimensions| fitted_slide(area, dimensions));
                    native.image_view(ui, s, page, area, s.zoom.view, false);
                    if s.route_trial.is_none()
                        && let Some(frame) = s.frame(page)
                    {
                        let dimensions = [frame.info.width, frame.info.height];
                        if s.toolbox.tool == Tool::Off && s.settings.zoom_enabled {
                            let (view, response) = zoom_gestures_in(
                                ui,
                                area,
                                dimensions,
                                s.zoom.view,
                                s.zoom.keep_fitted_area,
                            );
                            response.on_hover_text(tr(
                                &locale,
                                "Scroll to zoom · drag to pan · double-click to fit width / reset",
                            ));
                            if view != s.zoom.view {
                                s.set_zoom_from_action(view);
                            }
                        }
                        slide_tools(ui, s, area, dimensions);
                    }
                } else {
                    native.image(ui, s, page, area);
                }
            } else if index == 2 && s.count() > 0 && s.requested == s.count() - 1 {
                // Presenter-only decoration: never create a page, frame or navigation target.
                let area = Rect::from_min_max(rect.min + Vec2::new(0., 24.), rect.max);
                let painter = ui.painter().with_clip_rect(area);
                painter.rect_filled(area, 6., ui.visuals().extreme_bg_color);
                let text = painter.layout(
                    tr(&locale, "End of slides").into(),
                    FontId::proportional(24_f32.min(area.width() / 8.).max(1.)),
                    ui.visuals().text_color(),
                    (area.width() - 24.).max(1.),
                );
                painter.galley(
                    area.center() - text.size() / 2.,
                    text,
                    ui.visuals().text_color(),
                );
            }
            x = rect.right() + 14.;
        }
    }
    if view.layout.notes.show {
        let rect = notes_rect;
        let response = ui.interact(rect, ui.id().with("presenter-notes"), Sense::click());
        response
            .clone()
            .on_hover_text(tr(&locale, "Double-click to edit this slide’s notes"));
        if response.double_clicked() && s.count() > 0 {
            s.actions.push(Action::EditNotes);
        }
        ui.painter().rect_filled(rect, 8., ui.visuals().window_fill);
        ui.painter().text(
            rect.min + Vec2::splat(14.),
            egui::Align2::LEFT_TOP,
            tr(&locale, "PRESENTER NOTES · AUTO-FIT"),
            FontId::proportional(12.),
            ui.visuals().hyperlink_color,
        );
        let text = s
            .deck
            .as_ref()
            .and_then(|d| d.notes.get(s.requested))
            .filter(|n| !n.is_empty())
            .map_or(tr(&locale, "No notes for this slide"), String::as_str);
        let area = Rect::from_min_max(rect.min + Vec2::new(16., 40.), rect.max - Vec2::splat(16.));
        if area.is_positive() {
            if native
                .note_cache
                .as_ref()
                .is_none_or(|(old, size, _)| old != text || *size != area.size())
            {
                let mut lo = 0.1;
                let mut hi = 48.;
                let mut galley = ui.fonts_mut(|f| {
                    f.layout(
                        text.into(),
                        FontId::proportional(lo),
                        ui.visuals().text_color(),
                        area.width(),
                    )
                });
                for _ in 0..14 {
                    let mid = (lo + hi) / 2.;
                    let candidate = ui.fonts_mut(|f| {
                        f.layout(
                            text.into(),
                            FontId::proportional(mid),
                            ui.visuals().text_color(),
                            area.width(),
                        )
                    });
                    if candidate.size().y <= area.height() && candidate.size().x <= area.width() {
                        lo = mid;
                        galley = candidate;
                    } else {
                        hi = mid;
                    }
                }
                native.note_cache = Some((text.into(), area.size(), galley));
            }
            if let Some((_, _, galley)) = &native.note_cache {
                ui.painter()
                    .with_clip_rect(area)
                    .galley_with_override_text_color(
                        area.min,
                        galley.clone(),
                        ui.visuals().text_color(),
                    );
            }
        }
    }
    if header {
        let rect = Rect::from_min_size(screen.min, Vec2::new(screen.width(), header_height));
        ui.painter().rect_filled(rect, 0., ui.visuals().window_fill);
        let mut controls = rect.shrink(10.);
        if separate_header_timers {
            controls.max.y = controls.min.y + header_controls_height - 8.;
        }
        ui.scope_builder(egui::UiBuilder::new().max_rect(controls), |ui| {
            ui.with_layout(
                egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    if bar_visibility_button(ui, view.layout.header, true).clicked() {
                        view.layout.header = !view.layout.header;
                        native.header_until = now;
                    }
                    let trigger = ui.button(tr(&locale, "Menu"));
                    let mut menu_open = if trigger.clicked() {
                        !view.menu_open
                    } else {
                        view.menu_open
                    };
                    // Settings controls and nested selectors must not dismiss their parent menu.
                    egui::Popup::menu(&trigger)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .open_bool(&mut menu_open)
                        .show(|ui| {
                            egui::ScrollArea::vertical()
                                .max_height(
                                    (screen.bottom() - trigger.rect.bottom() - 16.)
                                        .clamp(80., 720.),
                                )
                                .show(ui, |ui| menu(ui, s));
                        });
                    view.menu_open = menu_open;
                    ui.add_sized(
                        [100., 20.],
                        egui::Label::new(
                            s.deck
                                .as_ref()
                                .map_or("PDF Presenter", |d| d.title.as_str()),
                        )
                        .truncate(),
                    );
                    if ui.button(tr(&locale, "Displays")).clicked() {
                        s.begin_display_edit();
                    }
                    button(
                        ui,
                        tr(&locale, "All slides"),
                        Action::OpenBrowser,
                        &mut s.actions,
                    );
                    if ui.button(tr(&locale, "Layout")).clicked() {
                        view.layout_open = !view.layout_open;
                    }
                    if ui.button(tr(&locale, "Tools")).clicked() {
                        s.toolbox.open = !s.toolbox.open;
                    }
                    fullscreen_button(ui, s);
                    timer_buttons(ui, s);
                    let inactive = s.blanked || s.routes.presentation.is_empty() || s.count() == 0;
                    let color = if inactive {
                        ui.visuals().error_fg_color
                    } else {
                        ui.visuals().hyperlink_color
                    };
                    let (indicator, _) = ui.allocate_exact_size(Vec2::splat(12.), Sense::hover());
                    ui.painter().circle_filled(indicator.center(), 5., color);
                    if ui
                        .button(
                            egui::RichText::new(if s.blanked {
                                tr(&locale, "Show slides")
                            } else {
                                tr(&locale, "Blank slides")
                            })
                            .color(if s.blanked {
                                ui.visuals().error_fg_color
                            } else {
                                ui.visuals().hyperlink_color
                            }),
                        )
                        .clicked()
                    {
                        s.actions.push(Action::Blank);
                    }
                    if !separate_header_timers {
                        timers(ui, s, &view.layout, TimerPosition::Header);
                    }
                },
            );
        });
        if separate_header_timers {
            let timer_rect = Rect::from_min_max(
                rect.min + Vec2::new(10., 10. + header_controls_height),
                rect.max - Vec2::splat(10.),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(timer_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| timers(ui, s, &view.layout, TimerPosition::Header),
            );
        }
    }
    if footer {
        let rect = Rect::from_min_size(
            Pos2::new(screen.left(), screen.bottom() - 90.),
            Vec2::new(screen.width(), 90.),
        );
        ui.painter().rect_filled(rect, 0., ui.visuals().window_fill);
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(8.)), |ui| {
            ui.horizontal(|ui| {
                if bar_visibility_button(ui, view.layout.footer, false).clicked() {
                    view.layout.footer = !view.layout.footer;
                    native.footer_until = now;
                }
                navigation_button(ui, s, native, tr(&locale, "Previous"), -1);
                navigation_button(ui, s, native, tr(&locale, "Next"), 1);
                ui.label(format!(
                    "{} / {}",
                    if s.count() == 0 { 0 } else { s.requested + 1 },
                    s.count()
                ));
                let unavailable = s.blanked || s.routes.presentation.is_empty() || s.count() == 0;
                ui.colored_label(
                    if unavailable {
                        ui.visuals().error_fg_color
                    } else {
                        ui.visuals().hyperlink_color
                    },
                    if unavailable {
                        tr(&locale, "NO OUTPUT DISPLAYED")
                    } else if s.displayed == Some(s.requested) {
                        tr(&locale, "PRESENTING")
                    } else {
                        tr(&locale, "PREPARING OUTPUT")
                    },
                );
                ui.small(&s.awake_status);
            });
            ui.label(s.preparation_status());
            ui.small(&s.status);
            if s.count() > 0 && !s.fully_buffered() {
                let prepared = s.prepared.len()
                    + if s.routes.presentation.is_empty() {
                        s.prepared.len()
                    } else {
                        s.output_buffered
                    };
                ui.add(
                    egui::ProgressBar::new(if s.evicted_frames > 0 {
                        s.prepared.len() as f32 / s.count() as f32
                    } else {
                        prepared as f32 / (2 * s.count()) as f32
                    })
                    .desired_height(4.),
                );
            }
        });
    }
    let compact_preparation = s.loading
        || s.notes_loading
        || (s.deck.is_some() && !s.fully_buffered() && s.evicted_frames == 0);
    if !footer && compact_preparation {
        egui::Area::new("preparation-progress".into())
            .anchor(egui::Align2::LEFT_BOTTOM, Vec2::new(12., -12.))
            .interactable(false)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width((screen.width() - 24.).max(100.));
                    ui.label(s.preparation_status());
                });
            });
    }
    if !footer && (s.blanked || s.routes.presentation.is_empty() || s.count() == 0) {
        egui::Area::new("output-warning".into())
            .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-12., 76.))
            .interactable(false)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        tr(&locale, "NO OUTPUT DISPLAYED"),
                    );
                });
            });
    }
    if !s.jump.is_empty() {
        egui::Area::new("jump".into())
            .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0., -80.))
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(format!(
                        "{} {} · {}",
                        if s.settings.language.french() {
                            "Diapositive"
                        } else {
                            "Go to slide"
                        },
                        s.jump,
                        if s.settings.language.french() {
                            "Entrée : diffuser · Échap : annuler"
                        } else {
                            "Enter to show · Esc to cancel"
                        }
                    ));
                });
            });
    }
    if let Some(trial) = &s.route_trial {
        let seconds = trial
            .deadline
            .saturating_duration_since(Instant::now())
            .as_secs()
            + 1;
        egui::Modal::new(egui::Id::new("confirm-displays")).show(&ctx, |ui| {
            ui.heading(if s.settings.language.french() {
                "Conserver ces écrans ?"
            } else {
                "Keep these display settings?"
            });
            ui.label(if s.settings.language.french() {
                "Une seule confirmation suffit pour tous les écrans."
            } else {
                "Confirm once on any screen to apply to all displays."
            });
            ui.label(if s.settings.language.french() {
                format!("Retour automatique dans {seconds} s. Échap pour annuler.")
            } else {
                format!("Automatically reverts in {seconds} seconds. Escape to revert.")
            });
            ui.horizontal(|ui| {
                button(
                    ui,
                    if s.settings.language.french() {
                        "Conserver"
                    } else {
                        "Keep changes"
                    },
                    Action::KeepRoutes,
                    &mut s.actions,
                );
                button(
                    ui,
                    if s.settings.language.french() {
                        "Annuler"
                    } else {
                        "Revert now"
                    },
                    Action::RevertRoutes,
                    &mut s.actions,
                );
            });
        });
    }
    if view.layout_open {
        let layout_choices = display.map_or_else(Vec::new, |_| {
            s.routes
                .presenter
                .iter()
                .filter_map(|id| {
                    s.monitors
                        .iter()
                        .position(|monitor| &monitor.id == id)
                        .map(|index| {
                            (
                                id.clone(),
                                format!("Display {} · {}", index + 1, s.monitors[index].name),
                            )
                        })
                })
                .collect::<Vec<_>>()
        });
        let owner_display = display.map(str::to_owned);
        let mut selected_layout_display = view
            .layout_target
            .clone()
            .filter(|selected| layout_choices.iter().any(|(id, _)| id == selected))
            .or_else(|| owner_display.clone());
        let rendered_layout_display = selected_layout_display.clone();
        let editing_remote = rendered_layout_display.is_some()
            && rendered_layout_display.as_deref() != owner_display.as_deref();
        let mut remote_view = editing_remote
            .then(|| s.take_presenter_view(rendered_layout_display.as_deref().unwrap()));
        let remote_layout_before = remote_view.as_ref().map(|remote| remote.layout.clone());
        let layout = remote_view
            .as_mut()
            .map_or(&mut view.layout, |remote| &mut remote.layout);
        let mut open = true;
        egui::Window::new(tr(&locale, "Presenter layout"))
            .id(egui::Id::new("layout-window"))
            .open(&mut open)
            .default_size(Vec2::new(560., 680.).min(ctx.content_rect().size() - Vec2::splat(48.)))
            .min_size(Vec2::new(320., 320.))
            .max_size((ctx.content_rect().size() - Vec2::splat(32.)).max(Vec2::splat(1.)))
            .resizable(true)
            .show(&ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(12., 8.);
                ui.spacing_mut().button_padding = Vec2::new(12., 7.);
                ui.spacing_mut().interact_size.y = 30.;
                ui.visuals_mut().slider_trailing_fill = true;
                if display.is_some() {
                    ui.horizontal(|ui| {
                        ui.strong(if s.settings.language.french() {
                            "Modifier la disposition de"
                        } else {
                            "Edit layout for"
                        });
                        let selected = selected_layout_display
                            .as_ref()
                            .and_then(|selected| {
                                layout_choices
                                    .iter()
                                    .find(|(id, _)| id == selected)
                                    .map(|(_, label)| label.as_str())
                            })
                            .unwrap_or("Presenter");
                        egui::ComboBox::from_id_salt("layout-presenter-display")
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                for (id, label) in &layout_choices {
                                    ui.selectable_value(
                                        &mut selected_layout_display,
                                        Some(id.clone()),
                                        label,
                                    );
                                }
                            });
                    });
                    ui.label(if s.settings.language.french() {
                        "Disposition et sauvegardes propres à cet écran. Le fonctionnement du minuteur reste partagé."
                    } else {
                        "Layout and saves apply to this display. Timer behavior is shared."
                    });
                }
                ui.label(tr(
                    &locale,
                    "Changes appear immediately. Save to reuse this layout.",
                ));
                egui::ScrollArea::vertical()
                    .id_salt("layout-settings-scroll")
                    .max_height(
                        (ui.available_height()
                            - if ui.available_width() < 510. {
                                190.
                            } else {
                                140.
                            })
                        .max(48.),
                    )
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        layout_section(ui, tr(&locale, "Start with a preset"), |ui| {
                            ui.horizontal_wrapped(|ui| {
                                if ui.button(tr(&locale, "Balanced")).clicked() {
                                    *layout = Layout::default();
                                }
                                if ui.button(tr(&locale, "Slide focus")).clicked() {
                                    *layout = Layout::default();
                                    layout.previous.show = false;
                                    layout.next.show = false;
                                    layout.notes.show = false;
                                    layout.elapsed.show = false;
                                    layout.clock.show = false;
                                }
                                if ui.button(tr(&locale, "Notes first")).clicked() {
                                    layout.notes.show = true;
                                    layout.notes.size = 60.;
                                    layout.previous.show = false;
                                }
                            });
                        });
                        layout_section(ui, tr(&locale, "Slides and notes"), |ui| {
                            ui.small(tr(
                                &locale,
                                "Hide anything you do not need to give the rest more room.",
                            ));
                            widget(
                                ui,
                                tr(&locale, "Previous slide"),
                                &mut layout.previous,
                                30.,
                                160.,
                                "Relative size",
                            );
                            widget(
                                ui,
                                tr(&locale, "Current slide"),
                                &mut layout.current,
                                60.,
                                200.,
                                "Relative size",
                            );
                            widget(
                                ui,
                                tr(&locale, "Next slide"),
                                &mut layout.next,
                                30.,
                                160.,
                                "Relative size",
                            );
                            ui.separator();
                            widget(
                                ui,
                                tr(&locale, "Notes"),
                                &mut layout.notes,
                                12.,
                                65.,
                                "Space (%)",
                            );
                        });
                        layout_section(ui, tr(&locale, "Timers and progress"), |ui| {
                            timer_settings(ui, s);
                            ui.separator();
                            widget(
                                ui,
                                tr(&locale, "Elapsed time"),
                                &mut layout.elapsed,
                                16.,
                                320.,
                                "Text size",
                            );
                            ui.add_enabled_ui(layout.elapsed.show, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(&locale, "Elapsed time position"));
                                    timer_position(
                                        ui,
                                        "elapsed-position",
                                        &mut layout.elapsed_position,
                                    );
                                });
                            });
                            ui.separator();
                            widget(
                                ui,
                                tr(&locale, "Time · HH:MM"),
                                &mut layout.clock,
                                16.,
                                320.,
                                "Text size",
                            );
                            ui.add_enabled_ui(layout.clock.show, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(&locale, "Clock position"));
                                    timer_position(
                                        ui,
                                        "clock-position",
                                        &mut layout.clock_position,
                                    );
                                });
                            });
                            ui.separator();
                            ui.checkbox(
                                &mut layout.slide_progress,
                                tr(&locale, "Slide progress bar"),
                            );
                            ui.add_enabled(
                                layout.slide_progress,
                                egui::Slider::new(&mut layout.slide_progress_height, 8. ..=120.)
                                    .text(tr(&locale, "Bar height")),
                            );
                        });
                        layout_section(ui, tr(&locale, "Header and footer"), |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(
                                    &mut layout.header,
                                    tr(&locale, "Keep header visible"),
                                );
                                ui.checkbox(
                                    &mut layout.footer,
                                    tr(&locale, "Keep footer visible"),
                                );
                            });
                            ui.small(tr(
                                &locale,
                                "Hidden bars reveal near edges. Right-click to restore.",
                            ));
                        });
                    });
                ui.separator();
                ui.strong(tr(&locale, "Remember this layout"));
                ui.horizontal_wrapped(|ui| {
                    ui.add_enabled_ui(s.deck.is_some(), |ui| {
                        button(
                            ui,
                            tr(&locale, "Save layout · This project"),
                            rendered_layout_display.as_deref().map_or(Action::SaveProject, |display| Action::SaveDisplayLayout { display: display.into(), project: true, layout: layout.clone() }),
                            &mut s.actions,
                        );
                    });
                    button(
                        ui,
                        tr(&locale, "Save layout · All projects"),
                        rendered_layout_display.as_deref().map_or(Action::SaveGlobal, |display| Action::SaveDisplayLayout { display: display.into(), project: false, layout: layout.clone() }),
                        &mut s.actions,
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.add_enabled_ui(s.deck.is_some(), |ui| {
                        button(
                            ui,
                            tr(&locale, "Use all-projects default"),
                            rendered_layout_display.as_deref().map_or(Action::UseGlobal, |display| Action::UseDisplayDefault(display.into())),
                            &mut s.actions,
                        );
                    });
                    ui.small(tr(&locale, "Project layouts override the default."));
                });
            });
        view.layout_open = open;
        view.layout_target = selected_layout_display;
        if let Some(remote_view) = remote_view {
            if remote_layout_before.as_ref() != Some(&remote_view.layout) {
                s.presenter_views_dirty = true;
            }
            s.presenter_views
                .insert(rendered_layout_display.unwrap(), remote_view);
        }
    }
    if s.displays_open {
        let mut open = true;
        egui::Window::new(tr(&locale, "Displays"))
            .id(egui::Id::new("displays-window"))
            .open(&mut open)
            .default_size(Vec2::new(600., 580.).min(ctx.content_rect().size() - Vec2::splat(48.)))
            .min_size(Vec2::new(320., 240.))
            .max_size((ctx.content_rect().size() - Vec2::splat(32.)).max(Vec2::splat(1.)))
            .resizable(true)
            .vscroll(true)
            .show(&ctx, |ui| {
                ui.label(tr(
                    &locale,
                    "Each screen has one role; repeat a role to mirror it.",
                ));
                for (index, m) in s.monitors.iter().enumerate() {
                    egui::Frame::group(ui.style())
                        .inner_margin(12.)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (slot, _) =
                                    ui.allocate_exact_size(Vec2::splat(64.), Sense::hover());
                                let dimensions =
                                    Vec2::new(m.size.0.max(1) as f32, m.size.1.max(1) as f32);
                                let scale =
                                    (slot.width() / dimensions.x).min(slot.height() / dimensions.y);
                                let rect =
                                    Rect::from_center_size(slot.center(), dimensions * scale);
                                ui.painter()
                                    .rect_filled(rect, 4., ui.visuals().extreme_bg_color);
                                ui.painter().rect_stroke(
                                    rect,
                                    4.,
                                    egui::Stroke::new(2_f32, ui.visuals().hyperlink_color),
                                    egui::StrokeKind::Inside,
                                );
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    (index + 1).to_string(),
                                    FontId::proportional(
                                        22_f32.min(rect.width() * 0.5).min(rect.height() * 0.65),
                                    ),
                                    ui.visuals().text_color(),
                                );
                                ui.vertical(|ui| {
                                    ui.add(egui::Label::new(egui::RichText::new(&m.name).strong()).wrap());
                                    ui.small(format!("{} × {} px", m.size.0, m.size.1));
                                });
                            });
                            ui.add_space(8.);
                            let mut role = if s.draft_routes.presenter.contains(&m.id) {
                                1
                            } else if s.draft_routes.presentation.contains(&m.id) {
                                2
                            } else {
                                0
                            };
                            ui.push_id(&m.id, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    for (i, name) in [
                                        tr(&locale, "Unused"),
                                        tr(&locale, "Presenter"),
                                        tr(&locale, "Slides"),
                                    ]
                                    .iter()
                                    .enumerate()
                                    {
                                        ui.selectable_value(&mut role, i, *name);
                                    }
                                });
                            });
                            s.draft_routes.presenter.retain(|id| id != &m.id);
                            s.draft_routes.presentation.retain(|id| id != &m.id);
                            match role {
                                1 => s.draft_routes.presenter.push(m.id.clone()),
                                2 => s.draft_routes.presentation.push(m.id.clone()),
                                _ => {}
                            }
                        });
                    ui.add_space(8.);
                }
                if ui.button(tr(&locale, "Swap groups")).clicked()
                    && !s.draft_routes.presentation.is_empty()
                {
                    std::mem::swap(
                        &mut s.draft_routes.presenter,
                        &mut s.draft_routes.presentation,
                    );
                }
                ui.checkbox(
                    &mut s.draft_presenter_fullscreen,
                    tr(&locale, "Presenter fullscreen"),
                );
                ui.checkbox(
                    &mut s.draft_presentation_fullscreen,
                    tr(&locale, "Slides fullscreen"),
                );
                button(
                    ui,
                    if s.settings.language.french() {
                        "Appliquer les modifications"
                    } else {
                        "Apply changes"
                    },
                    Action::ApplyRoutes,
                    &mut s.actions,
                );
                ui.small(tr(
                    &locale,
                    "Output visibility is preserved while previewing display changes.",
                ));
                if !s.draft_presenter_fullscreen || !s.draft_presentation_fullscreen {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        tr(&locale, "Window positioning can be unreliable in windowed mode, especially on Linux. Switch back to fullscreen if you experience positioning issues."),
                    );
                }
            });
        s.displays_open &= open;
    }
    toolbox_window(&ctx, s);
    if s.settings.zoom_enabled && s.zoom_warning && !s.zoom_warning_dismissed {
        egui::Window::new(tr(&ctx, "Zoom performance"))
            .collapsible(false).resizable(false).show(&ctx, |ui| {
                ui.set_max_width(420.);
                ui.label(tr(&ctx, "Zoomed frames are taking over 33 ms to draw. Panning may feel slow on this display. Reset or disable zoom to return to normal viewing."));
                if ui.button(tr(&ctx, "Reset zoom")).clicked() { s.set_zoom_from_action(Default::default()); s.zoom_warning_dismissed = true; }
                if ui.button(tr(&ctx, "Disable zoom")).clicked() { s.enable_zoom(false); }
                if ui.button(tr(&ctx, "Keep zoom enabled")).clicked() { s.zoom_warning_dismissed = true; }
            });
    }
    notes_import_ui(&ctx, s);
    notes_editor_ui(&ctx, s);
}
fn notes_import_ui(ctx: &Context, s: &mut Studio) {
    let Some(import) = &s.notes_import else {
        return;
    };
    let mut apply = false;
    let mut close = false;
    egui::Window::new(tr(ctx, "Import notes"))
        .id(egui::Id::new("notes-import"))
        .collapsible(false).resizable(true)
        .default_size(Vec2::new(560., 360.).min(ctx.content_rect().size() - Vec2::splat(48.)))
        .max_size(ctx.content_rect().size() - Vec2::splat(32.))
        .show(ctx, |ui| {
            if import.busy {
                ui.horizontal(|ui| { ui.spinner(); ui.label(tr(ctx, "Processing notes…")); });
            }
            egui::ScrollArea::vertical().max_height((ui.available_height() - 45.).max(60.)).show(ui, |ui| {
                if let Some(preview) = &import.preview {
                    ui.label(format!("{}: {} / {}", tr(ctx, "Slides to update"), preview.sections.len(), preview.total));
                    ui.label(tr(ctx, "Sections replace notes starting at slide 1. Remaining slides keep their notes."));
                    ui.label(tr(ctx, if preview.revision.is_some() { "Update existing notes file" } else { "Create notes file" }));
                    ui.label(preview.path.display().to_string());
                    ui.separator();
                    ui.strong(tr(ctx, "Preview (first 3 slides)"));
                    for (index, note) in preview.sections.iter().enumerate().take(3) {
                        ui.strong(format!("{} {}", tr(ctx, "Slide"), index + 1));
                        let mut excerpt: String = note.chars().take(500).collect();
                        if excerpt.len() < note.len() { excerpt.push('…'); }
                        ui.label(if note.is_empty() { tr(ctx, "Empty note") } else { &excerpt });
                    }
                }
                if let Some(error) = &import.error { ui.colored_label(ui.visuals().error_fg_color, error); }
            });
            ui.separator();
            ui.horizontal(|ui| {
                apply = ui.add_enabled(!import.busy && import.preview.is_some() && import.error.is_none(), egui::Button::new(tr(ctx, "Import and save notes"))).clicked();
                close = ui.add_enabled(!import.busy, egui::Button::new(tr(ctx, "Cancel"))).clicked();
            });
        });
    if apply {
        s.actions.push(Action::ApplyNotesImport);
    }
    if close {
        s.notes_import = None;
    }
}
fn notes_editor_ui(ctx: &Context, s: &mut Studio) {
    let Some(editor) = &mut s.notes_editor else {
        return;
    };
    let mut action = None;
    let mut close = false;
    egui::Window::new(format!(
        "{} · {} {}",
        tr(ctx, "Edit notes"),
        tr(ctx, "Slide"),
        editor.page + 1
    ))
    .id(egui::Id::new("notes-editor"))
    .collapsible(false)
    .resizable(true)
    .default_size(Vec2::new(640., 480.).min(ctx.content_rect().size() - Vec2::splat(48.)))
    .min_size(Vec2::new(300., 220.))
    .max_size(ctx.content_rect().size() - Vec2::splat(32.))
    .show(ctx, |ui| {
        if editor.busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr(
                    ctx,
                    if editor.snapshot.is_some() {
                        "Saving notes…"
                    } else {
                        "Loading all notes and preparing sidecar…"
                    },
                ));
            });
        }
        if let Some(snapshot) = &editor.snapshot {
            ui.label(
                snapshot
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            )
            .on_hover_text(snapshot.path.display().to_string());
            ui.small(tr(
                ctx,
                "The file contains all slides. Save changes only this slide.",
            ));
        }
        if let Some(error) = &editor.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let ready = editor.snapshot.is_some() && !editor.busy;
        egui::ScrollArea::vertical()
            .id_salt("notes-editor-text-scroll")
            .max_height((ui.available_height() - 76.).max(60.))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_enabled(
                    ready,
                    egui::TextEdit::multiline(&mut editor.text)
                        .id(egui::Id::new((
                            "notes-editor-text",
                            s.generation,
                            editor.page,
                        )))
                        .desired_width(f32::INFINITY)
                        .desired_rows(12)
                        .char_limit(crate::notes::MAX_NOTE),
                );
            });
        if editor.text.len() > crate::notes::MAX_NOTE {
            ui.colored_label(ui.visuals().error_fg_color, tr(ctx, "Note exceeds 256 KiB"));
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    ready && editor.text.len() <= crate::notes::MAX_NOTE,
                    egui::Button::new(tr(ctx, "Save notes")),
                )
                .clicked()
            {
                action = Some(Action::SaveNotes);
            }
            let dirty = editor.text != editor.saved_text;
            if ui
                .add_enabled(
                    !editor.busy,
                    egui::Button::new(tr(
                        ctx,
                        if dirty {
                            "Discard changes and close"
                        } else {
                            "Close"
                        },
                    )),
                )
                .clicked()
            {
                close = true;
            }
            if editor.error.is_some()
                && ui
                    .add_enabled(
                        !editor.busy,
                        egui::Button::new(tr(
                            ctx,
                            if editor.snapshot.is_some() {
                                "Reload (discard edits)"
                            } else {
                                "Retry"
                            },
                        )),
                    )
                    .clicked()
            {
                action = Some(Action::ReloadNotes);
            }
        });
    });
    if let Some(action) = action {
        s.actions.push(action);
    }
    if close {
        s.notes_editor = None;
    }
}

fn browser_preview_settings(ui: &mut Ui, s: &mut Studio) {
    let locale = ui.ctx().clone();
    ui.horizontal(|ui| {
        ui.label(tr(&locale, "Preview size"))
            .on_hover_text(tr(&locale, "Fit more slides or inspect larger previews."));
        ui.spacing_mut().slider_width = ui.available_width().max(40.);
        ui.visuals_mut().slider_trailing_fill = true;
        ui.add(egui::Slider::new(&mut s.settings.thumbnail_size, 100. ..=400.).show_value(false))
            .on_hover_text(format!(
                "{}: {:.0}",
                tr(&locale, "Preview size"),
                s.settings.thumbnail_size
            ));
    });
}
fn browser_screen_settings(ui: &mut Ui, s: &mut Studio) {
    let locale = ui.ctx().clone();
    ui.horizontal(|ui| {
        ui.label(tr(&locale, "Show on screen"))
            .on_hover_text(tr(&locale, "Choose where this slide browser opens."));
        let selected = s
            .settings
            .browser_monitor
            .as_ref()
            .and_then(|id| s.monitors.iter().find(|m| &m.id == id))
            .map(|m| m.name.as_str())
            .unwrap_or(tr(&locale, "Presenter screen"));
        egui::ComboBox::from_id_salt("browser-monitor")
            .width(ui.available_width().max(80.))
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut s.settings.browser_monitor,
                    None,
                    tr(&locale, "Presenter screen"),
                );
                for monitor in &s.monitors {
                    ui.selectable_value(
                        &mut s.settings.browser_monitor,
                        Some(monitor.id.clone()),
                        &monitor.name,
                    );
                }
            });
    });
}
fn paint_browser_scrollbar(ui: &Ui, inner: Rect, content_height: f32, offset: f32) {
    let track = Rect::from_min_max(
        Pos2::new(inner.right() + 4., inner.top()),
        Pos2::new(inner.right() + 16., inner.bottom()),
    );
    if track.height() <= 0. {
        return;
    }
    let viewport = inner.height();
    let handle_height = if content_height <= viewport {
        track.height()
    } else {
        (viewport / content_height * track.height())
            .max(36.)
            .min(track.height())
    };
    let max_offset = (content_height - viewport).max(0.);
    let travel = (track.height() - handle_height).max(0.);
    let top = track.top()
        + if max_offset > 0. {
            offset.clamp(0., max_offset) / max_offset * travel
        } else {
            0.
        };
    let handle = Rect::from_min_max(
        Pos2::new(track.left() + 2., top),
        Pos2::new(track.right() - 2., top + handle_height),
    );
    ui.painter()
        .rect_filled(track, 3., Color32::from_rgb(28, 34, 36));
    ui.painter().rect_filled(handle, 3., BROWSER_SCROLL_HANDLE);
    ui.painter().rect_stroke(
        handle,
        3.,
        egui::Stroke::new(1_f32, ui.visuals().text_color()),
        egui::StrokeKind::Inside,
    );
}
pub fn browser(ui: &mut Ui, s: &mut Studio, native: &mut UiState) {
    let browser_width = ui.available_width();
    let footer_rect = Rect::from_min_max(
        Pos2::new(ui.max_rect().left(), ui.max_rect().bottom() - 44.),
        ui.max_rect().max,
    );
    let locale = ui.ctx().clone();
    ui.painter()
        .rect_filled(ui.max_rect(), 0., ui.visuals().panel_fill);
    if s.change_on_click
        || s.browser_preview
            .is_some_and(|(generation, page)| generation != s.generation || page >= s.count())
    {
        s.browser_preview = None;
    }
    native.trim(s);
    if s.count() == 0 {
        ui.label(tr(
            &locale,
            "Open a PDF or picture folder to browse slides.",
        ));
    }
    egui::ScrollArea::vertical()
        .id_salt("browser-controls-scroll")
        .max_height(((footer_rect.top() - ui.cursor().top()) * 0.3).max(60.))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Frame::group(ui.style())
                .inner_margin(8.)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing = Vec2::new(12., 6.);
                    ui.spacing_mut().button_padding = Vec2::new(8., 3.);
                    ui.spacing_mut().interact_size.y = 24.;
                    if ui.available_width() >= 680. {
                        ui.columns(2, |columns| {
                            browser_preview_settings(&mut columns[0], s);
                            browser_screen_settings(&mut columns[1], s);
                        });
                    } else {
                        browser_preview_settings(ui, s);
                        browser_screen_settings(ui, s);
                    }
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        let accent = ui.visuals().selection.bg_fill;
                        let text = ui.visuals().selection.stroke.color;
                        if ui
                            .add_enabled(
                                s.count() > 0,
                                egui::Button::new(
                                    egui::RichText::new(tr(&locale, "Show selected"))
                                        .strong()
                                        .color(text),
                                )
                                .fill(accent)
                                .min_size(Vec2::new(120., 28.)),
                            )
                            .clicked()
                        {
                            s.actions.push(Action::Go(s.browser_selected));
                        }
                        ui.checkbox(&mut s.change_on_click, tr(&locale, "Change on click"));
                        ui.colored_label(
                            if s.change_on_click {
                                ui.visuals().warn_fg_color
                            } else {
                                ui.visuals().hyperlink_color
                            },
                            tr(
                                &locale,
                                if s.change_on_click {
                                    "Live browsing"
                                } else {
                                    "Safe browsing"
                                },
                            ),
                        ).on_hover_text(tr(
                    &locale,
                    if s.change_on_click {
                        "Click changes presentation"
                    } else {
                        "Click to select. Double-click to preview. Show selected to present."
                    },
                ));
                    });
                });
        });
    ui.add_space(4.);
    ui.label(format!(
        "{} {} · {} {}",
        if s.settings.language.french() {
            "Sélection"
        } else {
            "Selected"
        },
        if s.count() == 0 {
            0
        } else {
            s.browser_selected + 1
        },
        if s.settings.language.french() {
            "Actuelle"
        } else {
            "Current"
        },
        if s.count() == 0 { 0 } else { s.requested + 1 },
    ));
    if !s.jump.is_empty() {
        ui.label(format!(
            "{} {} — {}",
            if s.settings.language.french() {
                "Diapositive"
            } else {
                "Go to slide"
            },
            s.jump,
            if s.settings.language.french() {
                "Entrée : diffuser, Échap : annuler"
            } else {
                "Enter to show, Escape to cancel"
            }
        ));
    }
    // Reserve a stable gutter, so neither thumbnail sizing nor scrollbar placement
    // depends on how much of the last row is populated.
    let mut scroll_style = egui::style::ScrollStyle::solid();
    scroll_style.bar_width = 12.;
    scroll_style.handle_min_length = 36.;
    scroll_style.foreground_color = true;
    ui.spacing_mut().scroll = scroll_style;
    let width = (browser_width - ui.spacing().scroll.allocated_width()).max(1.);
    let gap = 16.;
    let cols = ((width + gap) / (s.settings.thumbnail_size + gap))
        .floor()
        .max(1.) as usize;
    let size = ((width - gap * (cols - 1) as f32) / cols as f32).max(1.);
    let rows = s.count().div_ceil(cols);
    s.visible_thumbs.clear();
    let row_height = size * 0.5625 + 50.;
    let scroll = egui::ScrollArea::vertical()
        .max_width(browser_width)
        .max_height((footer_rect.top() - ui.cursor().top() - 8.).max(0.))
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show_viewport(ui, |ui, viewport| {
            let pitch = row_height + ui.spacing().item_spacing.y;
            ui.set_height((pitch * rows as f32 - ui.spacing().item_spacing.y).max(0.));
            let mut first = (viewport.top() / pitch).floor().max(0.) as usize;
            let last = ((viewport.bottom() / pitch).ceil() as usize + 1).min(rows);
            first = first.min(last);
            let y = ui.max_rect().top() + first as f32 * pitch;
            let bottom = ui.max_rect().top() + last as f32 * pitch;
            let rect = Rect::from_x_y_ranges(ui.max_rect().x_range(), y..=bottom);
            let offset = viewport.top();
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for row in first..last {
                    ui.horizontal(|ui| {
                        for col in 0..cols {
                            let page = row * cols + col;
                            if page >= s.count() {
                                break;
                            }
                            s.visible_thumbs.insert(page);
                            ui.vertical(|ui| {
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(size, size * 0.5625),
                                    Sense::click(),
                                );
                                native.browser_image(ui, s, page, rect);
                                if s.browser_selected == page || s.requested == page {
                                    ui.painter().rect_stroke(
                                        rect,
                                        4.,
                                        egui::Stroke::new(
                                            2_f32,
                                            if s.requested == page {
                                                ui.visuals().hyperlink_color
                                            } else {
                                                Color32::GOLD
                                            },
                                        ),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                if response.double_clicked() && !s.change_on_click {
                                    s.browser_preview = Some((s.generation, page));
                                }
                                if response.clicked() {
                                    s.browser_selected = page;
                                    if s.change_on_click {
                                        s.actions.push(Action::Go(page));
                                    }
                                }
                                ui.label(format!(
                                    "{}{}",
                                    page + 1,
                                    if s.requested == page {
                                        tr(&locale, " · CURRENT")
                                    } else {
                                        ""
                                    }
                                ));
                            });
                        }
                    });
                }
            });
            offset
        });
    paint_browser_scrollbar(ui, scroll.inner_rect, scroll.content_size.y, scroll.inner);
    ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
        ui.separator();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(tr(&locale, "Close")).min_size(Vec2::new(100., 32.)))
                .clicked()
            {
                s.actions.push(Action::CloseBrowser);
            }
        });
    });
    if let Some((_, page)) = s.browser_preview {
        // Keep preview requests in the browser's bounded asynchronous loading path.
        s.visible_thumbs.insert(page);
        let mut open = true;
        let screen = ui.ctx().content_rect().size();
        egui::Window::new(format!(
            "{} · {} / {}",
            tr(&locale, "Slide preview"),
            page + 1,
            s.count()
        ))
        .id(egui::Id::new("slide-preview"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(Vec2::new(800., 500.).min(screen * 0.8))
        .min_size(Vec2::new(240., 180.))
        .max_size((screen - Vec2::splat(32.)).max(Vec2::splat(1.)))
        .show(ui.ctx(), |ui| {
            ui.label(tr(&locale, "Preview only — presentation unchanged"));
            let (rect, _) =
                ui.allocate_exact_size(ui.available_size().max(Vec2::splat(1.)), Sense::hover());
            native.browser_image(ui, s, page, rect);
        });
        if !open {
            s.browser_preview = None;
        }
    }
}

fn menu(ui: &mut Ui, s: &mut Studio) {
    let locale = ui.ctx().clone();
    ui.set_min_width(260.);
    let actions_before = s.actions.len();
    ui.add_enabled_ui(!s.dialog_busy, |ui| {
        button(
            ui,
            tr(&locale, "Open PDF…"),
            Action::Pick(false),
            &mut s.actions,
        );
        button(
            ui,
            tr(&locale, "Open picture folder…"),
            Action::Pick(true),
            &mut s.actions,
        );
    });
    if s.dialog_busy {
        ui.label(tr(&locale, "File chooser already open"));
    }
    ui.add_enabled_ui(
        !s.dialog_busy
            && !s.loading
            && s.deck.is_some()
            && s.notes_editor.is_none()
            && s.notes_import.is_none(),
        |ui| {
            button(
                ui,
                tr(&locale, "Import notes…"),
                Action::PickNotes,
                &mut s.actions,
            );
        },
    );
    ui.separator();
    button(
        ui,
        tr(&locale, "All slides"),
        Action::OpenBrowser,
        &mut s.actions,
    );
    ui.menu_button(tr(&locale, "Appearance"), |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_min_width(250.);
            let mut changed = false;
            ui.label(
                egui::RichText::new(tr(&locale, "Theme"))
                    .small()
                    .strong()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.menu_button(s.settings.theme.name().to_owned(), |ui| {
                for theme in crate::theme::Theme::ALL {
                    let name = theme.name();
                    changed |= ui
                        .selectable_value(&mut s.settings.theme, theme.clone(), name)
                        .changed();
                }
            });
            ui.add_space(6.);
            ui.separator();
            ui.add_space(4.);
            ui.label(
                egui::RichText::new(tr(&locale, "Language"))
                    .small()
                    .strong()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.menu_button(s.settings.language.name(), |ui| {
                for language in crate::localization::Language::ALL {
                    changed |= ui
                        .selectable_value(&mut s.settings.language, language, language.name())
                        .changed();
                }
            });
            ui.separator();
            for (label, color) in [
                (
                    "Page background",
                    &mut s.settings.slide_colors.page_background,
                ),
                (
                    "Fit padding color",
                    &mut s.settings.slide_colors.fit_padding,
                ),
            ] {
                ui.horizontal(|ui| {
                    ui.label(tr(&locale, label));
                    changed |= ui.color_edit_button_srgb(color).changed();
                });
            }
            ui.small(tr(
                &locale,
                "Page background fills transparency; fit padding fills space around the slide.",
            ));
            if changed {
                s.new_output = true;
                s.save_preferences();
                ui.ctx().request_repaint();
            }
        });
    });
    ui.menu_button(tr(&locale,"Preferences"), |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(370.);
            ui.collapsing(if s.settings.language.french() { "Image des diapositives masquées" } else { "Blank slides" }, |ui| {
                ui.label(if s.settings.blank_picture {
                    if s.settings.language.french() { "Image personnalisée sélectionnée" } else { "Custom picture selected" }
                } else if s.settings.language.french() { "Écran noir" } else { "Solid black" });
                if ui.add_enabled(s.initialized && !s.dialog_busy && !s.blank_busy, egui::Button::new(if s.settings.language.french() { "Choisir une image…" } else { "Choose picture…" })).clicked() {
                    s.actions.push(Action::PickBlankPicture);
                }
                if ui.add_enabled(s.initialized && !s.dialog_busy && (s.settings.blank_picture || s.blank_busy), egui::Button::new(if s.settings.language.french() { "Utiliser un écran noir" } else { "Use solid black" })).clicked() {
                    s.actions.push(Action::ClearBlankPicture);
                }
                ui.small(if s.settings.language.french() { "Une copie est conservée et ajustée à tous les écrans Diapositives, sur fond noir." } else { "A copy is saved and fitted to all Slides screens against a black background." });
                if s.blank_busy { ui.spinner(); }
            });
            ui.separator();
            #[cfg(target_os = "windows")]
            ui.group(|ui| {
                ui.strong(if s.settings.language.french() { "Application PDF par défaut" } else { "Default PDF application" });
                ui.label(if s.settings.language.french() { "Enregistre PDF Presenter et son icône pour les fichiers PDF de votre compte Windows." } else { "Register PDF Presenter and its icon for PDF files in your Windows account." });
                if ui.button(if s.settings.language.french() { "Définir comme application PDF par défaut…" } else { "Set as default PDF app…" }).clicked() {
                    s.actions.push(Action::RegisterPdfAssociation);
                }
                ui.small(if s.settings.language.french() { "Windows ouvrira Applications par défaut : sélectionnez PDF Presenter pour .pdf afin de confirmer." } else { "Windows will open Default Apps; select PDF Presenter for .pdf to confirm." });
            });
            #[cfg(target_os = "windows")]
            ui.separator();
            let mut enabled = s.settings.zoom_enabled;
            if ui.checkbox(&mut enabled, tr(&locale, "Enable page zoom")).changed() { s.enable_zoom(enabled); }
            if ui.checkbox(&mut s.settings.zoom_keep_fitted_area, tr(&locale, "Keep fitted page area when zooming")).changed() {
                s.set_zoom(s.zoom.view);
                s.save_preferences();
            }
            ui.small(tr(&locale, "By default, zoom expands the page to fill the screen without stretching. Enable this option to keep the original page-shaped area."));
            ui.small(tr(&locale, "Zoom reuses buffered images: no extra PDF rendering or image memory. Enlarged slides may look softer. Software drawing can slow down on large or multiple displays."));
            ui.separator();
            if ui.checkbox(&mut s.settings.auto_show,tr(&locale,"Show slides automatically when opening a document")).changed() {s.save_preferences();}
            ui.separator();
            ui.label(tr(&locale,"RAM budget per process (presenter + output each use this allowance):"));
            if ui.add(egui::Slider::new(&mut s.settings.buffer_mib,64..=16384).text("MiB")).changed() {s.remember_buffer_budget();}
            ui.small(tr(&locale,"Raise only within available RAM; OS paging can cause delays. Screen rendering is capped at 4096 px."));
            ui.separator();
            ui.collapsing(tr(&locale, "Disk cache"), |ui| {
                let mut changed = false;
                ui.label(tr(&locale, "Maximum total size (MiB)"));
                changed |= ui.add(egui::DragValue::new(&mut s.settings.disk_cache.max_mib).range(64..=1048576).speed(64)).changed();
                ui.label(tr(&locale, "Expire after (days since rendering)"));
                changed |= ui.add(egui::DragValue::new(&mut s.settings.disk_cache.max_days).range(1..=3650)).changed();
                if changed { s.remember_cache_policy(); }
                ui.small(tr(&locale, "Oldest files are removed first. Active decks are protected and may exceed the limit. Automatic cleanup runs once at startup, after loading and preparation finish."));
                if ui.add_enabled(!s.cache_busy && !s.loading && s.initialized, egui::Button::new(tr(&locale, "Expunge unused cache now"))).clicked() { s.clean_cache(true); }
                ui.small(&s.cache_status);
            });
            ui.label(s.preparation_status());
            ui.collapsing(tr(&locale,"Storage locations"), |ui| {
                ui.label(format!("{}: {}",if s.settings.language.french(){"Diapositives préparées"}else{"Processed slides"},s.cache_path.display()));
                ui.label(format!("Configuration: {}",s.settings_path.display()));
            });
        });
    });
    if s.settings.zoom_enabled {
        ui.menu_button(tr(&locale, "Page zoom"), |ui| {
            let mut scale = s.zoom.view.scale;
            if ui
                .add(egui::Slider::new(&mut scale, 1.0..=8.0).text("×"))
                .changed()
            {
                s.set_zoom_from_action(s.zoom.view.zoom_at(scale, [0.5; 2]));
            }
            if ui.button(tr(&locale, "Reset zoom")).clicked() {
                s.set_zoom_from_action(Default::default());
            }
            ui.small(tr(
                &locale,
                "Scroll to zoom · drag to pan · double-click to fit width / reset",
            ));
            ui.small(tr(
                &locale,
                "Use the current slide preview or a Slides screen. Changing slides resets zoom.",
            ));
        });
    }
    ui.separator();
    button(ui, tr(&locale, "Quit"), Action::Exit, &mut s.actions);
    if s.actions.len() != actions_before {
        ui.close();
    }
}

/// Minimal setup controls permitted on output windows, without any presenter data.
pub fn output_display_recovery(
    ui: &mut egui::Ui,
    startup: bool,
    trial: bool,
) -> Option<crate::output::DisplayAction> {
    use crate::output::DisplayAction;
    let mut action = None;
    if startup && !trial {
        ui.interact(
            ui.max_rect(),
            ui.id().with("startup-display-menu"),
            egui::Sense::click(),
        )
        .context_menu(|ui| {
            if ui.button("Swap Presenter and Slides screens…").clicked() {
                action = Some(DisplayAction::Swap);
                ui.close();
            }
        });
    }
    if trial {
        egui::Window::new("Keep these display settings?")
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label("Automatically reverts after 15 seconds. Escape to revert.");
                ui.horizontal(|ui| {
                    if ui.button("Keep changes").clicked() {
                        action = Some(DisplayAction::Keep);
                    }
                    if ui.button("Revert now").clicked() {
                        action = Some(DisplayAction::Revert);
                    }
                });
            });
    }
    action
}
