use crate::localization::tr;
// Native controls for the approved presenter layout and safe slide browser.
use crate::{
    settings::Layout,
    studio::{Action, Studio},
};
use egui::{Color32, Context, FontId, Pos2, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
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
            let scale = (rect.width() / frame.info.width as f32)
                .min(rect.height() / frame.info.height as f32);
            let size = Vec2::new(
                frame.info.width as f32 * scale,
                frame.info.height as f32 * scale,
            );
            ui.painter().image(
                tex.id(),
                Rect::from_center_size(rect.center(), size),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                Color32::WHITE,
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
fn widget(ui: &mut Ui, label: &str, w: &mut crate::settings::Widget, min: f32, max: f32) {
    let locale = ui.ctx().clone();
    ui.checkbox(&mut w.show, label);
    ui.add_enabled(
        w.show,
        egui::Slider::new(&mut w.size, min..=max).text(tr(&locale, "Size")),
    );
    ui.separator();
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
    let mut body = screen.shrink(16.);
    if s.layout.header {
        body.min.y += 64.;
    }
    if s.layout.footer {
        body.max.y -= 90.;
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
        for (label, page, w) in items {
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
        let rect = Rect::from_min_size(screen.min, Vec2::new(screen.width(), 64.));
        ui.painter().rect_filled(rect, 0., ui.visuals().window_fill);
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(10.)), |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(if s.layout.header {
                        tr(&locale, "Hide")
                    } else {
                        tr(&locale, "Pin")
                    })
                    .clicked()
                {
                    s.layout.header = !s.layout.header;
                    native.header_until = now;
                }
                let trigger = ui.button(tr(&locale, "Menu"));
                let mut menu_open = if trigger.clicked() {
                    !s.menu_open
                } else {
                    s.menu_open
                };
                egui::Popup::menu(&trigger)
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
                    s.draft_routes = s.routes.clone();
                    s.displays_open = true;
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
                if s.layout.elapsed.show {
                    let secs = s.started.elapsed().as_secs();
                    ui.label(
                        egui::RichText::new(format!("{:02}:{:02}", secs / 60, secs % 60))
                            .size(s.layout.elapsed.size),
                    );
                }
                if s.layout.clock.show {
                    ui.label(
                        egui::RichText::new(chrono::Local::now().format("%H:%M").to_string())
                            .size(s.layout.clock.size),
                    );
                }
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
                if ui
                    .button(if s.layout.footer {
                        tr(&locale, "Hide")
                    } else {
                        tr(&locale, "Pin")
                    })
                    .clicked()
                {
                    s.layout.footer = !s.layout.footer;
                    native.footer_until = now;
                }
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
                    egui::ProgressBar::new(prepared as f32 / (2 * s.count()) as f32)
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
    if s.layout_open {
        let mut open = true;
        egui::Window::new(tr(&locale, "Your presenter view"))
            .id(egui::Id::new("layout-window"))
            .open(&mut open)
            .default_width(330.)
            .vscroll(true)
            .show(&ctx, |ui| {
                ui.horizontal(|ui| {
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
                ui.checkbox(&mut s.layout.header, tr(&locale, "Keep header visible"));
                ui.checkbox(&mut s.layout.footer, tr(&locale, "Keep footer visible"));
                ui.small(tr(
                    &locale,
                    "Hidden bars reveal near edges. Right-click to restore.",
                ));
                widget(
                    ui,
                    tr(&locale, "Previous slide"),
                    &mut s.layout.previous,
                    30.,
                    160.,
                );
                widget(
                    ui,
                    tr(&locale, "Current slide"),
                    &mut s.layout.current,
                    60.,
                    200.,
                );
                widget(ui, tr(&locale, "Next slide"), &mut s.layout.next, 30., 160.);
                widget(ui, tr(&locale, "Notes"), &mut s.layout.notes, 12., 65.);
                widget(
                    ui,
                    tr(&locale, "Elapsed time"),
                    &mut s.layout.elapsed,
                    16.,
                    72.,
                );
                widget(
                    ui,
                    tr(&locale, "Time · HH:MM"),
                    &mut s.layout.clock,
                    16.,
                    72.,
                );
                button(
                    ui,
                    tr(&locale, "Save layout · This project"),
                    Action::SaveProject,
                    &mut s.actions,
                );
                button(
                    ui,
                    tr(&locale, "Save layout · All projects"),
                    Action::SaveGlobal,
                    &mut s.actions,
                );
                button(
                    ui,
                    tr(&locale, "Use all-projects default"),
                    Action::UseGlobal,
                    &mut s.actions,
                );
                ui.small(tr(&locale, "Project layouts override the default."));
            });
        s.layout_open = open;
    }
    if s.displays_open {
        let mut open = true;
        egui::Window::new(tr(&locale, "Displays"))
            .id(egui::Id::new("displays-window"))
            .open(&mut open)
            .default_width(600.)
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
                            ui.set_min_width(540.);
                            ui.horizontal(|ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::new(64., 40.), Sense::hover());
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
                                    FontId::proportional(22.),
                                    ui.visuals().text_color(),
                                );
                                ui.vertical(|ui| {
                                    ui.strong(&m.name);
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
                                ui.horizontal(|ui| {
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
                    &mut s.settings.presenter_fullscreen,
                    tr(&locale, "Presenter fullscreen"),
                );
                ui.checkbox(
                    &mut s.settings.presentation_fullscreen,
                    tr(&locale, "Presentation fullscreen"),
                );
                button(
                    ui,
                    tr(&locale, "Apply and remember"),
                    Action::ApplyRoutes,
                    &mut s.actions,
                );
                ui.small(tr(
                    &locale,
                    "Applying a setup blanks output. Show when destinations are verified.",
                ));
            });
        s.displays_open &= open;
    }
}
pub fn browser(ui: &mut Ui, s: &mut Studio, native: &mut UiState) {
    let locale = ui.ctx().clone();
    ui.painter()
        .rect_filled(ui.max_rect(), 0., ui.visuals().panel_fill);
    native.trim(s);
    if s.count() == 0 {
        ui.label(tr(
            &locale,
            "Open a PDF or picture folder to browse slides.",
        ));
    }
    ui.horizontal_wrapped(|ui| {
        ui.heading(tr(&locale, "All slides"));
        button(
            ui,
            tr(&locale, "Close"),
            Action::CloseBrowser,
            &mut s.actions,
        );
        ui.add(
            egui::Slider::new(&mut s.settings.thumbnail_size, 100. ..=400.)
                .text(tr(&locale, "Preview size")),
        );
        ui.checkbox(&mut s.change_on_click, tr(&locale, "Change on click"));
        egui::ComboBox::from_id_salt("browser-monitor")
            .selected_text(
                s.settings
                    .browser_monitor
                    .as_deref()
                    .unwrap_or(tr(&locale, "Presenter screen")),
            )
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
        button(
            ui,
            tr(&locale, "Show selected"),
            Action::Go(s.browser_selected),
            &mut s.actions,
        );
    });
    ui.label(format!(
        "{} {} · {} {} · {}",
        if s.settings.language.french() {
            "Sélection"
        } else {
            "Selected"
        },
        s.browser_selected + 1,
        if s.settings.language.french() {
            "Actuelle"
        } else {
            "Current"
        },
        s.requested + 1,
        if s.change_on_click {
            tr(&locale, "Click changes presentation")
        } else {
            tr(&locale, "Click only selects — browse safely")
        }
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
    let size = s.settings.thumbnail_size;
    let cols = (ui.available_width() / (size + 16.)).floor().max(1.) as usize;
    let rows = s.count().div_ceil(cols);
    s.visible_thumbs.clear();
    egui::ScrollArea::vertical().show_rows(ui, size * 0.5625 + 50., rows, |ui, range| {
        for row in range {
            ui.horizontal(|ui| {
                for col in 0..cols {
                    let page = row * cols + col;
                    if page >= s.count() {
                        break;
                    }
                    s.visible_thumbs.insert(page);
                    ui.vertical(|ui| {
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(size, size * 0.5625), Sense::click());
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
        s.draft_routes = s.routes.clone();
        s.displays_open = true;
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
            egui::ComboBox::from_id_salt("theme")
                .selected_text(s.settings.theme.name())
                .show_ui(ui, |ui| {
                    for theme in crate::theme::Theme::ALL {
                        let name = theme.name();
                        changed |= ui
                            .selectable_value(&mut s.settings.theme, theme.clone(), name)
                            .changed();
                    }
                });
            ui.label(tr(&locale, "Language"));
            egui::ComboBox::from_id_salt("language")
                .selected_text(s.settings.language.name())
                .show_ui(ui, |ui| {
                    for language in crate::localization::Language::ALL {
                        changed |= ui
                            .selectable_value(&mut s.settings.language, language, language.name())
                            .changed();
                    }
                });
            if changed {
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
