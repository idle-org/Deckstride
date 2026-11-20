use crate::localization::tr;
// Native controls for the approved presenter layout and safe slide browser.
use crate::{
    settings::{Layout, TimerPosition},
    studio::{Action, Studio},
};
use egui::{Color32, Context, FontId, Pos2, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
/// Composite a fitted slide over independent page and padding colors in every view.
pub fn paint_slide(
    painter: &egui::Painter,
    rect: Rect,
    texture: egui::TextureId,
    dimensions: [u32; 2],
    colors: crate::settings::SlideColors,
) {
    let [width, height] = dimensions.map(|v| v as f32);
    let scale = (rect.width() / width).min(rect.height() / height);
    let target = Rect::from_center_size(rect.center(), Vec2::new(width * scale, height * scale));
    let [r, g, b] = colors.fit_padding;
    painter.rect_filled(rect, 0., Color32::from_rgb(r, g, b));
    let [r, g, b] = colors.page_background;
    painter.rect_filled(target, 0., Color32::from_rgb(r, g, b));
    painter.image(
        texture,
        target,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
        Color32::WHITE,
    );
}
const TEAL: Color32 = Color32::from_rgb(103, 226, 205);
pub struct UiState {
    pub textures: HashMap<(u64, usize, u32), TextureHandle>,
    pub header_until: Instant,
    pub footer_until: Instant,
    note_cache: Option<(String, Vec2, Arc<egui::Galley>)>,
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            textures: HashMap::new(),
            header_until: Instant::now(),
            footer_until: Instant::now(),
            note_cache: None,
        }
    }
}
impl UiState {
    pub fn image(&mut self, ui: &mut Ui, s: &Studio, page: usize, rect: Rect) {
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
            paint_slide(
                ui.painter(),
                rect,
                tex.id(),
                [frame.info.width, frame.info.height],
                s.settings.slide_colors,
            );
        } else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if page < s.count() {
                    tr(&locale, "Preparing slide…")
                } else {
                    tr(&locale, "No slide")
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
fn timer_height(s: &Studio, position: TimerPosition) -> f32 {
    [
        (&s.layout.elapsed, s.layout.elapsed_position),
        (&s.layout.clock, s.layout.clock_position),
    ]
    .into_iter()
    .filter(|(w, p)| w.show && *p == position)
    .map(|(w, _)| w.size + 32.)
    .fold(0., f32::max)
}
fn timers(ui: &mut Ui, s: &Studio, position: TimerPosition) {
    let locale = ui.ctx().clone();
    let secs = s.started.elapsed().as_secs();
    for (w, p, label, value) in [
        (
            &s.layout.elapsed,
            s.layout.elapsed_position,
            "Elapsed time",
            format!("{:02}:{:02}", secs / 60, secs % 60),
        ),
        (
            &s.layout.clock,
            s.layout.clock_position,
            "Time · HH:MM",
            chrono::Local::now().format("%H:%M").to_string(),
        ),
    ] {
        if w.show && p == position {
            ui.add_space(16.);
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.small(tr(&locale, label));
                    ui.label(egui::RichText::new(value).size(w.size));
                });
            });
        }
    }
}
pub fn presenter(ui: &mut Ui, s: &mut Studio, native: &mut UiState) {
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
    let header = s.layout.header || native.header_until > now || s.menu_open;
    let footer = s.layout.footer || native.footer_until > now;
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
        ui.checkbox(&mut s.layout.header, tr(&locale, "Keep header visible"));
        ui.checkbox(&mut s.layout.footer, tr(&locale, "Keep footer visible"));
        if ui.button(tr(&locale, "Show both")).clicked() {
            s.layout.header = true;
            s.layout.footer = true;
            ui.close();
        }
        if ui.button(tr(&locale, "Layout…")).clicked() {
            s.layout_open = true;
            ui.close();
        }
    });
    let header_height = (timer_height(s, TimerPosition::Header) + 20.).max(64.);
    let mut body = screen.shrink(16.);
    if s.layout.header {
        body.min.y += header_height;
    }
    if s.layout.footer {
        body.max.y -= 90.;
    }
    if s.layout.slide_progress {
        let rect = Rect::from_min_max(Pos2::new(body.left(), body.bottom() - 24.), body.max);
        body.max.y -= 32.;
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
                    .text(format!(
                        "{} {current} / {}",
                        tr(&locale, "Slide"),
                        s.count()
                    )),
            );
        });
    }
    for position in [TimerPosition::AboveSlides, TimerPosition::BelowNotes] {
        let height = timer_height(s, position);
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
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.horizontal(|ui| timers(ui, s, position));
            });
        }
    }
    let preview_visible = s.layout.previous.show || s.layout.current.show || s.layout.next.show;
    let notes_h = if s.layout.notes.show {
        if preview_visible {
            body.height() * s.layout.notes.size / 100.
        } else {
            body.height()
        }
    } else {
        0.
    };
    let preview = Rect::from_min_max(
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
    if preview_visible {
        let items = [
            (
                tr(&locale, "PREVIOUS"),
                s.requested.checked_sub(1),
                s.layout.previous.clone(),
            ),
            (
                tr(&locale, "CURRENT"),
                Some(s.requested),
                s.layout.current.clone(),
            ),
            (
                tr(&locale, "NEXT"),
                (s.requested + 1 < s.count()).then_some(s.requested + 1),
                s.layout.next.clone(),
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
                native.image(
                    ui,
                    s,
                    page,
                    Rect::from_min_max(rect.min + Vec2::new(0., 24.), rect.max),
                );
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
    if s.layout.notes.show {
        let rect = Rect::from_min_max(Pos2::new(body.left(), body.bottom() - notes_h), body.max);
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
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(10.)), |ui| {
            ui.horizontal(|ui| {
                if bar_visibility_button(ui, s.layout.header, true).clicked() {
                    s.layout.header = !s.layout.header;
                    native.header_until = now;
                }
                let trigger = ui.button(tr(&locale, "Menu"));
                let mut menu_open = if trigger.clicked() {
                    !s.menu_open
                } else {
                    s.menu_open
                };
                // Settings controls and nested selectors must not dismiss their parent menu.
                egui::Popup::menu(&trigger)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .open_bool(&mut menu_open)
                    .show(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(520.)
                            .show(ui, |ui| menu(ui, s));
                    });
                s.menu_open = menu_open;
                ui.label(
                    s.deck
                        .as_ref()
                        .map_or("PDF Presenter", |d| d.title.as_str()),
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
                    s.layout_open = !s.layout_open;
                }
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
                            tr(&locale, "Show presentation")
                        } else {
                            tr(&locale, "Blank presentation")
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
                timers(ui, s, TimerPosition::Header);
            });
        });
    }
    if footer {
        let rect = Rect::from_min_size(
            Pos2::new(screen.left(), screen.bottom() - 90.),
            Vec2::new(screen.width(), 90.),
        );
        ui.painter().rect_filled(rect, 0., ui.visuals().window_fill);
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(8.)), |ui| {
            ui.horizontal(|ui| {
                if bar_visibility_button(ui, s.layout.footer, false).clicked() {
                    s.layout.footer = !s.layout.footer;
                    native.footer_until = now;
                }
                button(
                    ui,
                    tr(&locale, "Previous"),
                    Action::Go(s.requested.saturating_sub(1)),
                    &mut s.actions,
                );
                button(
                    ui,
                    tr(&locale, "Next"),
                    Action::Go(s.requested + 1),
                    &mut s.actions,
                );
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
    if !footer && (s.loading || s.notes_loading || (s.deck.is_some() && !s.fully_buffered())) {
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
    if s.layout_open {
        let mut open = true;
        egui::Window::new(tr(&locale, "Your presenter view"))
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
                                    s.layout = Layout::default();
                                }
                                if ui.button(tr(&locale, "Slide focus")).clicked() {
                                    s.layout = Layout::default();
                                    s.layout.previous.show = false;
                                    s.layout.next.show = false;
                                    s.layout.notes.show = false;
                                    s.layout.elapsed.show = false;
                                    s.layout.clock.show = false;
                                }
                                if ui.button(tr(&locale, "Notes first")).clicked() {
                                    s.layout.notes.show = true;
                                    s.layout.notes.size = 60.;
                                    s.layout.previous.show = false;
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
                                &mut s.layout.previous,
                                30.,
                                160.,
                                "Relative size",
                            );
                            widget(
                                ui,
                                tr(&locale, "Current slide"),
                                &mut s.layout.current,
                                60.,
                                200.,
                                "Relative size",
                            );
                            widget(
                                ui,
                                tr(&locale, "Next slide"),
                                &mut s.layout.next,
                                30.,
                                160.,
                                "Relative size",
                            );
                            ui.separator();
                            widget(
                                ui,
                                tr(&locale, "Notes"),
                                &mut s.layout.notes,
                                12.,
                                65.,
                                "Space (%)",
                            );
                        });
                        layout_section(ui, tr(&locale, "Timers and progress"), |ui| {
                            widget(
                                ui,
                                tr(&locale, "Elapsed time"),
                                &mut s.layout.elapsed,
                                16.,
                                72.,
                                "Text size",
                            );
                            ui.add_enabled_ui(s.layout.elapsed.show, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(&locale, "Elapsed time position"));
                                    timer_position(
                                        ui,
                                        "elapsed-position",
                                        &mut s.layout.elapsed_position,
                                    );
                                });
                            });
                            ui.separator();
                            widget(
                                ui,
                                tr(&locale, "Time · HH:MM"),
                                &mut s.layout.clock,
                                16.,
                                72.,
                                "Text size",
                            );
                            ui.add_enabled_ui(s.layout.clock.show, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(&locale, "Clock position"));
                                    timer_position(
                                        ui,
                                        "clock-position",
                                        &mut s.layout.clock_position,
                                    );
                                });
                            });
                            ui.separator();
                            ui.checkbox(
                                &mut s.layout.slide_progress,
                                tr(&locale, "Slide progress bar"),
                            );
                        });
                        layout_section(ui, tr(&locale, "Header and footer"), |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(
                                    &mut s.layout.header,
                                    tr(&locale, "Keep header visible"),
                                );
                                ui.checkbox(
                                    &mut s.layout.footer,
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
                            Action::SaveProject,
                            &mut s.actions,
                        );
                    });
                    button(
                        ui,
                        tr(&locale, "Save layout · All projects"),
                        Action::SaveGlobal,
                        &mut s.actions,
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.add_enabled_ui(s.deck.is_some(), |ui| {
                        button(
                            ui,
                            tr(&locale, "Use all-projects default"),
                            Action::UseGlobal,
                            &mut s.actions,
                        );
                    });
                    ui.small(tr(&locale, "Project layouts override the default."));
                });
            });
        s.layout_open = open;
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
                                        tr(&locale, "Presentation"),
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
                    tr(&locale, "Presentation fullscreen"),
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
                    "Applying a setup blanks output. Show when destinations are verified.",
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
}
fn browser_preview_settings(ui: &mut Ui, s: &mut Studio) {
    let locale = ui.ctx().clone();
    egui::Frame::group(ui.style())
        .inner_margin(12.)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong(tr(&locale, "Preview size"));
            ui.small(tr(&locale, "Fit more slides or inspect larger previews."));
            ui.spacing_mut().slider_width = (ui.available_width() - 4.).max(80.);
            ui.visuals_mut().slider_trailing_fill = true;
            ui.add(
                egui::Slider::new(&mut s.settings.thumbnail_size, 100. ..=400.)
                    .show_value(false)
                    .text(""),
            )
            .on_hover_text(format!(
                "{}: {:.0}",
                tr(&locale, "Preview size"),
                s.settings.thumbnail_size
            ));
            ui.horizontal(|ui| {
                ui.small(tr(&locale, "Compact"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(tr(&locale, "Large"));
                });
            });
        });
}
fn browser_screen_settings(ui: &mut Ui, s: &mut Studio) {
    let locale = ui.ctx().clone();
    egui::Frame::group(ui.style())
        .inner_margin(12.)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong(tr(&locale, "Show on screen"));
            ui.small(tr(&locale, "Choose where this slide browser opens."));
            let selected = s
                .settings
                .browser_monitor
                .as_ref()
                .and_then(|id| s.monitors.iter().find(|m| &m.id == id))
                .map(|m| m.name.as_str())
                .unwrap_or(tr(&locale, "Presenter screen"));
            egui::ComboBox::from_id_salt("browser-monitor")
                .width(ui.available_width())
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
        .max_height(((footer_rect.top() - ui.cursor().top()) * 0.5).max(60.))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Frame::group(ui.style())
                .inner_margin(12.)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing = Vec2::new(16., 10.);
                    ui.spacing_mut().button_padding = Vec2::new(12., 7.);
                    ui.spacing_mut().interact_size.y = 30.;
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
                                .min_size(Vec2::new(160., 38.)),
                            )
                            .clicked()
                        {
                            s.actions.push(Action::Go(s.browser_selected));
                        }
                        ui.checkbox(&mut s.change_on_click, tr(&locale, "Change on click"));
                    });
                    ui.horizontal_wrapped(|ui| {
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
                        );
                        ui.label(tr(
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
    ui.add_space(8.);
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
    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
    let width = (browser_width - ui.spacing().scroll.allocated_width()).max(1.);
    let gap = 16.;
    let cols = ((width + gap) / (s.settings.thumbnail_size + gap))
        .floor()
        .max(1.) as usize;
    let size = ((width - gap * (cols - 1) as f32) / cols as f32).max(1.);
    let rows = s.count().div_ceil(cols);
    s.visible_thumbs.clear();
    egui::ScrollArea::vertical()
        .max_width(browser_width)
        .max_height((footer_rect.top() - ui.cursor().top() - 8.).max(0.))
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show_rows(ui, size * 0.5625 + 50., rows, |ui, range| {
            ui.spacing_mut().item_spacing.x = gap;
            for row in range {
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
                            native.image(ui, s, page, rect);
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
            native.image(ui, s, page, rect);
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
    ui.separator();
    button(
        ui,
        tr(&locale, "All slides"),
        Action::OpenBrowser,
        &mut s.actions,
    );
    if ui.button(tr(&locale, "Displays")).clicked() {
        s.begin_display_edit();
        ui.close();
    }
    if ui.button(tr(&locale, "Layout")).clicked() {
        s.layout_open = true;
        ui.close();
    }
    ui.menu_button(tr(&locale, "Appearance"), |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_min_width(250.);
            let mut changed = false;
            ui.label(tr(&locale, "Theme"));
            ui.menu_button(s.settings.theme.name().to_owned(), |ui| {
                for theme in crate::theme::Theme::ALL {
                    let name = theme.name();
                    changed |= ui
                        .selectable_value(&mut s.settings.theme, theme.clone(), name)
                        .changed();
                }
            });
            ui.label(tr(&locale, "Language"));
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
            if ui.checkbox(&mut s.settings.auto_show,tr(&locale,"Show presentation automatically when opening a document")).changed() {s.save_preferences();}
            ui.separator();
            ui.label(tr(&locale,"RAM budget per process (presenter + output each use this allowance):"));
            if ui.add(egui::Slider::new(&mut s.settings.buffer_mib,256..=16384).text("MiB")).changed() {s.remember_buffer_budget();}
            ui.small(tr(&locale,"Raise only within available RAM; OS paging can cause delays. Screen rendering is capped at 4096 px."));
            ui.separator();
            ui.label(s.preparation_status());
            ui.collapsing(tr(&locale,"Storage locations"), |ui| {
                ui.label(format!("{}: {}",if s.settings.language.french(){"Diapositives préparées"}else{"Processed slides"},s.cache_path.display()));
                ui.label(format!("Configuration: {}",s.settings_path.display()));
            });
        });
    });
    ui.separator();
    if ui.button(tr(&locale, "Reset timer")).clicked() {
        s.started = Instant::now();
        ui.close();
    }
    button(
        ui,
        tr(&locale, "Leave fullscreen / safe blank"),
        Action::Windowed,
        &mut s.actions,
    );
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
            if ui
                .button("Swap presenter and presentation displays…")
                .clicked()
            {
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
