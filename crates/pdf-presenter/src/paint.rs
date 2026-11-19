//! Small embedded bitmap-font renderer; no GPU, PDF or font-file dependency.
use crate::poc::{Action, Model, SLIDES};
use font8x8::{BASIC_FONTS, UnicodeFonts};

/// Reference dimensions for the presenter layout.
pub const WIDTH: f64 = 960.0;
/// Reference dimensions for the presenter layout.
pub const HEIGHT: f64 = 640.0;
const BACKGROUND: u32 = 0x101923;
const WHITE: u32 = 0xeaf3f7;
const TEAL: u32 = 0x00dcc2;
const MUTED: u32 = 0x9eb3c3;

/// Clickable presenter control, in reference-layout coordinates.
pub struct Button {
    /// Left coordinate.
    pub x: f64,
    /// Top coordinate.
    pub y: f64,
    /// Label and keyboard shortcut.
    pub label: &'static str,
    /// Control action.
    pub action: Action,
}

/// Controls shared by hit testing and drawing.
pub const BUTTONS: [Button; 8] = [
    Button {
        x: 24.,
        y: 388.,
        label: "< PREV",
        action: Action::Previous,
    },
    Button {
        x: 256.,
        y: 388.,
        label: "NEXT >",
        action: Action::Next,
    },
    Button {
        x: 488.,
        y: 388.,
        label: "B  BLANK",
        action: Action::Blank,
    },
    Button {
        x: 720.,
        y: 388.,
        label: "S  SWAP",
        action: Action::Swap,
    },
    Button {
        x: 24.,
        y: 448.,
        label: "P  PRESENTER",
        action: Action::PresenterDisplay,
    },
    Button {
        x: 256.,
        y: 448.,
        label: "A  AUDIENCE",
        action: Action::AudienceDisplay,
    },
    Button {
        x: 488.,
        y: 448.,
        label: "F  OUTPUT FS",
        action: Action::Fullscreen,
    },
    Button {
        x: 720.,
        y: 448.,
        label: "V  PRES. FS",
        action: Action::PresenterFullscreen,
    },
];

/// Convert physical coordinates into a presenter action.
pub fn hit_test(width: u32, height: u32, x: f64, y: f64) -> Option<Action> {
    let (scale, ox, oy) = transform(width, height);
    let x = (x - ox) / scale;
    let y = (y - oy) / scale;
    BUTTONS
        .iter()
        .find(|b| x >= b.x && x < b.x + 216. && y >= b.y && y < b.y + 44.)
        .map(|b| b.action)
}

fn transform(width: u32, height: u32) -> (f64, f64, f64) {
    let scale = (f64::from(width) / WIDTH)
        .min(f64::from(height) / HEIGHT)
        .max(0.001);
    (
        scale,
        (f64::from(width) - WIDTH * scale) / 2.,
        (f64::from(height) - HEIGHT * scale) / 2.,
    )
}

struct Canvas<'a> {
    pixels: &'a mut [u32],
    width: usize,
    height: usize,
    scale: f64,
    ox: f64,
    oy: f64,
}

impl Canvas<'_> {
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32) {
        let left = (self.ox + x * self.scale).max(0.) as usize;
        let top = (self.oy + y * self.scale).max(0.) as usize;
        let right = ((self.ox + (x + w) * self.scale).ceil().max(0.) as usize).min(self.width);
        let bottom = ((self.oy + (y + h) * self.scale).ceil().max(0.) as usize).min(self.height);
        for row in top.min(self.height)..bottom {
            self.pixels[row * self.width + left.min(self.width)
                ..row * self.width + right.max(left.min(self.width))]
                .fill(color);
        }
    }
    fn text(&mut self, x: f64, y: f64, text: &str, size: f64, color: u32) {
        for (index, character) in text.chars().enumerate() {
            let glyph = BASIC_FONTS.get(character).unwrap_or([0; 8]);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        self.rect(
                            x + (index * 8 + col) as f64 * size,
                            y + row as f64 * size,
                            size,
                            size,
                            color,
                        );
                    }
                }
            }
        }
    }
    fn slide(&mut self, x: f64, y: f64, w: f64, page: usize) {
        let h = w * 9. / 16.;
        let color = [0x123c51, 0x41285c, 0x215346, 0x633c26][page % 4];
        self.rect(x, y, w, h, color);
        self.rect(x, y, w, 5., TEAL);
        self.text(x + w * 0.07, y + h * 0.2, "MOCK SLIDE", w / 120., WHITE);
        self.text(
            x + w * 0.07,
            y + h * 0.48,
            &format!("{:02} / {SLIDES}", page + 1),
            w / 140.,
            TEAL,
        );
        self.text(
            x + w * 0.07,
            y + h * 0.82,
            "DUAL SCREEN TEST",
            w / 240.,
            MUTED,
        );
    }
}

/// Draw one complete frame into the software surface. Audience frames contain no controls.
pub fn draw(
    pixels: &mut [u32],
    width: u32,
    height: u32,
    model: &Model,
    audience: bool,
    status: &str,
) {
    let expected = width as usize * height as usize;
    if pixels.len() != expected || width == 0 || height == 0 {
        return;
    }
    pixels.fill(if audience { 0 } else { BACKGROUND });
    let (scale, ox, oy) = if audience {
        let scale = (f64::from(width) / WIDTH).min(f64::from(height) / 540.);
        (
            scale,
            (f64::from(width) - WIDTH * scale) / 2.,
            (f64::from(height) - 540. * scale) / 2.,
        )
    } else {
        transform(width, height)
    };
    let mut c = Canvas {
        pixels,
        width: width as usize,
        height: height as usize,
        scale,
        ox,
        oy,
    };
    if audience {
        if !model.blanked && !model.disconnected {
            c.slide(0., 0., WIDTH, model.page);
        }
        return;
    }
    c.text(24., 22., "PDF PRESENTER / DISPLAY PROOF", 2.5, WHITE);
    c.text(
        24.,
        58.,
        &format!(
            "PRESENTER {}   AUDIENCE {}   {} DISPLAY(S)",
            model.presenter + 1,
            model.audience + 1,
            model.displays.len()
        ),
        1.5,
        TEAL,
    );
    c.text(24., 91., "CURRENT", 1.5, MUTED);
    c.text(492., 91., "NEXT", 1.5, MUTED);
    c.slide(24., 115., 444., model.page);
    if model.page + 1 < SLIDES {
        c.slide(492., 115., 444., model.page + 1);
    } else {
        c.text(524., 220., "END OF DECK", 2., MUTED);
    }
    for b in &BUTTONS {
        c.rect(b.x, b.y, 216., 44., 0x263b4b);
        c.text(b.x + 12., b.y + 14., b.label, 1.5, WHITE);
    }
    let message = if model.disconnected {
        "DISPLAY CHANGE: OUTPUT HIDDEN. R TO REFRESH."
    } else if model.displays.len() == 1 {
        "ONE DISPLAY: WINDOWED REHEARSAL ONLY"
    } else if model.blanked {
        "AUDIENCE BLANKED - B TO SHOW MOCK SLIDE"
    } else {
        "MOCK OUTPUT ACTIVE - VERIFY ITS PHYSICAL DISPLAY"
    };
    c.text(24., 514., message, 1.5, TEAL);
    c.text(
        24.,
        545.,
        "ARROWS/SPACE: SLIDES   ESC: SAFE WINDOWED MODE",
        1.5,
        MUTED,
    );
    c.text(
        24.,
        571.,
        "R: REFRESH DISPLAYS   Q: QUIT   NO PDF / NO SLEEP GUARD",
        1.3,
        MUTED,
    );
    c.text(
        24.,
        601.,
        &status.chars().take(76).collect::<String>(),
        1.2,
        MUTED,
    );
}
