//! Built-in palettes adapted from Canvas Pilot (MIT); no sibling runtime dependency.
use egui::Color32;
const BG: Color32 = Color32::from_rgb(17, 23, 32);
const PANEL: Color32 = Color32::from_rgb(27, 35, 48);
const FIELD: Color32 = Color32::from_rgb(18, 26, 37);
const LINE: Color32 = Color32::from_rgb(53, 68, 88);
const TEXT: Color32 = Color32::from_rgb(237, 243, 252);
const MUTED: Color32 = Color32::from_rgb(165, 181, 201);
const CYAN: Color32 = Color32::from_rgb(102, 219, 239);
const AMBER: Color32 = Color32::from_rgb(246, 197, 119);
const PURPLE: Color32 = Color32::from_rgb(102, 85, 217);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Midra,
    HighContrast,
    FoxifiedDark,
    CorporateLight,
    Bluecore,
    DupontMonsanto,
    EdenGreen,
    FoxifiedLight,
    Hci,
    IronDocks,
    Peaceframe,
    Rebel,
    SiriusMind,
    Solarized,
    SonsOfBanard,
    StormPilot,
    TessierAshpol,
    Ustf,
    VerdantRebel,
    WeicatIndustrial,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Color32,
    pub banner: Color32,
    pub grid: Color32,
    pub layer: Color32,
    pub pattern: [Color32; 2],
    pub bg: Color32,
    pub panel: Color32,
    pub field: Color32,
    pub line: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub cyan: Color32,
    pub amber: Color32,
    pub purple: Color32,
}

impl Theme {
    pub const ALL: [Self; 20] = [
        Self::Midra,
        Self::HighContrast,
        Self::FoxifiedDark,
        Self::CorporateLight,
        Self::Bluecore,
        Self::DupontMonsanto,
        Self::EdenGreen,
        Self::FoxifiedLight,
        Self::Hci,
        Self::IronDocks,
        Self::Peaceframe,
        Self::Rebel,
        Self::SiriusMind,
        Self::Solarized,
        Self::SonsOfBanard,
        Self::StormPilot,
        Self::TessierAshpol,
        Self::Ustf,
        Self::VerdantRebel,
        Self::WeicatIndustrial,
    ];
    pub fn name(&self) -> &'static str {
        match self {
            Self::Midra => "Midra Dark",
            Self::HighContrast => "High Contrast",
            Self::FoxifiedDark => "Foxified Dark",
            Self::CorporateLight => "Corporate Light",
            Self::Bluecore => "Bluecore",
            Self::DupontMonsanto => "Dupont Monsanto",
            Self::EdenGreen => "Eden Green",
            Self::FoxifiedLight => "Foxified Light",
            Self::Hci => "HCI",
            Self::IronDocks => "Iron Docks",
            Self::Peaceframe => "Peaceframe",
            Self::Rebel => "Rebel",
            Self::SiriusMind => "Sirius Mind",
            Self::Solarized => "Solarized",
            Self::SonsOfBanard => "Sons Of Banard",
            Self::StormPilot => "Storm Pilot",
            Self::TessierAshpol => "Tessier Ashpol",
            Self::Ustf => "USTF",
            Self::VerdantRebel => "Verdant Rebel",
            Self::WeicatIndustrial => "Weicat Industrial",
        }
    }
    pub fn is_light(&self) -> bool {
        matches!(
            self,
            Self::CorporateLight
                | Self::FoxifiedLight
                | Self::Hci
                | Self::Peaceframe
                | Self::Solarized
        )
    }
    pub fn palette(&self) -> Palette {
        let rgb = |hex: u32| Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);
        match self {
            Self::HighContrast => Palette {
                canvas: rgb(0x000000),
                banner: rgb(0x101010),
                grid: rgb(0x777777),
                layer: rgb(0x101010),
                pattern: [rgb(0x000000), rgb(0xffffff)],
                bg: rgb(0x000000),
                panel: rgb(0x000000),
                field: rgb(0x101010),
                line: rgb(0xaaaaaa),
                text: rgb(0xffffff),
                muted: rgb(0xdddddd),
                cyan: rgb(0x00ffff),
                amber: rgb(0xffff00),
                purple: rgb(0x202080),
            },
            Self::Bluecore => sotv_palette(
                [
                    0x1a222d, 0x232f3a, 0x111922, 0x2b3948, 0xd0e4ff, 0xaaccee, 0x3399ff, 0x4488cc,
                ],
                false,
            ),
            Self::DupontMonsanto => sotv_palette(
                [
                    0x1f2c1a, 0x2f3d26, 0x141c10, 0x3d4f2e, 0xeaffcc, 0xd5e6aa, 0x697513, 0xbbdd44,
                ],
                false,
            ),
            Self::EdenGreen => sotv_palette(
                [
                    0x1c2b1a, 0x263f25, 0x152017, 0x2f4a2b, 0xcceebb, 0xbbddaa, 0x6da436, 0x66aa55,
                ],
                false,
            ),
            Self::FoxifiedLight => sotv_palette(
                [
                    0xfff8f2, 0xfff0e6, 0xffe5d0, 0xffd6b3, 0x5a2e0c, 0xaa6033, 0xff8330, 0xcc7744,
                ],
                true,
            ),
            Self::Hci => sotv_palette(
                [
                    0xf2f9f6, 0xe3f0eb, 0xd0e4dd, 0xc6e8de, 0x144d40, 0x226658, 0x33ccaa, 0x66ccb3,
                ],
                true,
            ),
            Self::IronDocks => sotv_palette(
                [
                    0x1e1e1e, 0x2b2b2b, 0x121212, 0x3a3a3a, 0xe0e0e0, 0xcccccc, 0xe17100, 0xaaaaaa,
                ],
                false,
            ),
            Self::Peaceframe => sotv_palette(
                [
                    0xe8f3fb, 0xd4e8f5, 0xbcd9eb, 0xcbe2f2, 0x1c3f55, 0x345f77, 0x3399cc, 0x77aac6,
                ],
                true,
            ),
            Self::Rebel => sotv_palette(
                [
                    0x1a1a1a, 0x2a2a2a, 0x111111, 0x444444, 0xf5f5f5, 0xaaaaaa, 0x555555, 0x666666,
                ],
                false,
            ),
            Self::SiriusMind => sotv_palette(
                [
                    0x0e1a2b, 0x162538, 0x0a121d, 0x1f2f47, 0xd0e6ff, 0xaaccee, 0x3d77b1, 0x4477aa,
                ],
                false,
            ),
            Self::Solarized => sotv_palette(
                [
                    0xfdf6e3, 0xfefcea, 0xeee8d5, 0xeee8d5, 0x586e75, 0xa0b1b7, 0x516a73, 0xcccccc,
                ],
                true,
            ),
            Self::SonsOfBanard => sotv_palette(
                [
                    0x1e1c18, 0x2c2a26, 0x121110, 0x3a3832, 0xf0d9b5, 0xe0c8a0, 0xbd7c20, 0xcc9966,
                ],
                false,
            ),
            Self::StormPilot => sotv_palette(
                [
                    0x101022, 0x1b1b33, 0x080818, 0x2b2b4a, 0xccddff, 0xaabbee, 0x5d5d82, 0x5577aa,
                ],
                false,
            ),
            Self::TessierAshpol => sotv_palette(
                [
                    0x12171b, 0x1a2025, 0x0e1215, 0x202830, 0xccf2ff, 0xb0d8e6, 0x4e8ca0, 0x77cce0,
                ],
                false,
            ),
            Self::Ustf => sotv_palette(
                [
                    0x0f1b2b, 0x162536, 0x0a121f, 0x1e2e44, 0xe0e8f0, 0xf2f2f2, 0xff4444, 0x446688,
                ],
                false,
            ),
            Self::VerdantRebel => sotv_palette(
                [
                    0x1b2a1b, 0x273a27, 0x141d14, 0x314f31, 0xccffd9, 0xaaffcc, 0x00aa72, 0x66dd99,
                ],
                false,
            ),
            Self::WeicatIndustrial => sotv_palette(
                [
                    0x1c1a14, 0x2a261d, 0x0f0e0a, 0x3a341e, 0xfff5cc, 0xffeb99, 0x8f7511, 0xe6b800,
                ],
                false,
            ),
            Self::Midra => Palette {
                canvas: rgb(0x142d40),
                banner: rgb(0x264556),
                grid: rgb(0x27475a),
                layer: rgb(0x173b50),
                pattern: [rgb(0x264b5d), rgb(0x1a3344)],
                bg: BG,
                panel: PANEL,
                field: FIELD,
                line: LINE,
                text: TEXT,
                muted: MUTED,
                cyan: CYAN,
                amber: AMBER,
                purple: PURPLE,
            },
            Self::FoxifiedDark => Palette {
                canvas: rgb(0x281a12),
                banner: rgb(0x503322),
                grid: rgb(0x62432e),
                layer: rgb(0x43291d),
                pattern: [rgb(0x503322), rgb(0x362318)],
                bg: rgb(0x2e1f16),
                panel: rgb(0x3b261b),
                field: rgb(0x1e1309),
                line: rgb(0xaa6033),
                text: rgb(0xffcc99),
                muted: rgb(0xffd8b1),
                cyan: rgb(0xff6600),
                amber: AMBER,
                purple: rgb(0x854018),
            },
            Self::CorporateLight => Palette {
                canvas: rgb(0xffffff),
                banner: rgb(0xe5e9f0),
                grid: rgb(0xd8dee6),
                layer: rgb(0xe9eef3),
                pattern: [rgb(0xe5e9f0), rgb(0xf5f7fa)],
                bg: rgb(0xf4f5f7),
                panel: Color32::WHITE,
                field: rgb(0xf0f1f3),
                line: rgb(0xbbbbbb),
                text: rgb(0x222222),
                muted: rgb(0x52616f),
                cyan: rgb(0x216783),
                amber: rgb(0x815600),
                purple: rgb(0xc5dce8),
            },
        }
    }
    pub fn apply(&self, ctx: &egui::Context) {
        let applied = egui::Id::new("deckstride-theme");
        if ctx.data(|data| data.get_temp::<Self>(applied)) == Some(*self) {
            return;
        }
        let p = self.palette();
        let mut v = if self.is_light() {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        v.override_text_color = Some(p.text);
        v.panel_fill = p.bg;
        v.window_fill = p.panel;
        v.extreme_bg_color = p.field;
        v.faint_bg_color = p.bg;
        v.hyperlink_color = p.cyan;
        v.warn_fg_color = p.amber;
        v.error_fg_color = if self.is_light() {
            Color32::from_rgb(165, 30, 40)
        } else {
            Color32::LIGHT_RED
        };
        v.window_stroke = egui::Stroke::new(1_f32, p.line);
        v.selection.bg_fill = p.purple;
        v.selection.stroke.color = p.text;
        for (w, fill) in [
            (&mut v.widgets.noninteractive, p.panel),
            (&mut v.widgets.inactive, p.field),
            (&mut v.widgets.hovered, p.panel.lerp_to_gamma(p.text, 0.08)),
            (&mut v.widgets.active, p.purple),
            (&mut v.widgets.open, p.panel),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = egui::Stroke::new(
                if *self == Self::HighContrast {
                    2_f32
                } else {
                    1_f32
                },
                p.line,
            );
            w.fg_stroke = egui::Stroke::new(1_f32, p.text);
        }
        ctx.set_visuals(v);
        ctx.data_mut(|data| data.insert_temp(applied, *self));
    }
}
fn sotv_palette(colors: [u32; 8], light: bool) -> Palette {
    let [bg, panel, field, layer, text, subtle, accent, line] =
        colors.map(|hex| Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8));
    let mix = |a: Color32, b: Color32| {
        Color32::from_rgb(
            ((a.r() as u16 + b.r() as u16) / 2) as u8,
            ((a.g() as u16 + b.g() as u16) / 2) as u8,
            ((a.b() as u16 + b.b() as u16) / 2) as u8,
        )
    };
    Palette {
        bg,
        panel,
        field,
        layer,
        text,
        muted: if light { mix(subtle, text) } else { subtle },
        cyan: if light { mix(accent, text) } else { accent },
        amber: if light {
            Color32::from_rgb(129, 86, 0)
        } else {
            AMBER
        },
        purple: mix(accent, panel),
        line,
        canvas: bg,
        banner: panel,
        grid: mix(line, bg),
        pattern: [layer, panel],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_theme_application_keeps_the_existing_style() {
        let ctx = egui::Context::default();
        Theme::Midra.apply(&ctx);
        let first = ctx.global_style();
        Theme::Midra.apply(&ctx);
        assert!(std::sync::Arc::ptr_eq(&first, &ctx.global_style()));
        Theme::CorporateLight.apply(&ctx);
        assert!(!std::sync::Arc::ptr_eq(&first, &ctx.global_style()));
    }
}
