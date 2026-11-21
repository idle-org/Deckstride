//! Bounded, resolution-independent crop of an already buffered slide.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub scale: f32,
    pub center: [f32; 2],
}
impl Default for View {
    fn default() -> Self {
        Self {
            scale: 1.,
            center: [0.5; 2],
        }
    }
}
impl View {
    pub fn normalized(self) -> Self {
        if !self.scale.is_finite() || self.center.iter().any(|v| !v.is_finite()) {
            return Self::default();
        }
        let scale = self.scale.clamp(1., 8.);
        let half = 0.5 / scale;
        Self {
            scale,
            center: self.center.map(|v| v.clamp(half, 1. - half)),
        }
    }
    pub fn zoom_at(self, scale: f32, anchor: [f32; 2]) -> Self {
        let old = self.normalized();
        let scale = if scale.is_finite() {
            scale.clamp(1., 8.)
        } else {
            old.scale
        };
        let mut next = Self { scale, ..old };
        for (i, a) in anchor.into_iter().enumerate() {
            if a.is_finite() {
                next.center[i] += (a.clamp(0., 1.) - 0.5) * (1. / old.scale - 1. / scale);
            }
        }
        next.normalized()
    }
    /// Drag displacement as a fraction of the fitted slide viewport.
    pub fn pan(self, delta: [f32; 2]) -> Self {
        let mut next = self.normalized();
        for (i, d) in delta.into_iter().enumerate() {
            if d.is_finite() {
                next.center[i] -= d / next.scale;
            }
        }
        next.normalized()
    }
    pub fn uv(self) -> egui::Rect {
        let v = self.normalized();
        egui::Rect::from_center_size(
            egui::pos2(v.center[0], v.center[1]),
            egui::Vec2::splat(1. / v.scale),
        )
    }
}
/// Presenter epochs override output gestures. Output serials order local gestures
/// within an epoch, allowing bounded channels to coalesce and retry latest state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sync {
    pub enabled: bool,
    #[serde(default)]
    pub keep_fitted_area: bool,
    pub view: View,
    pub epoch: u64,
    pub serial: u64,
}
impl Sync {
    pub fn normalized(mut self) -> Self {
        self.view = if self.enabled {
            self.view.normalized()
        } else {
            View::default()
        };
        self
    }
    pub fn set(&mut self, enabled: bool, view: View) {
        self.enabled = enabled;
        self.view = view;
        self.epoch = self.epoch.wrapping_add(1);
        self.serial = 0;
        *self = self.normalized();
    }
    pub fn gesture(&mut self, view: View) {
        if self.enabled {
            self.view = view.normalized();
            self.serial = self.serial.wrapping_add(1);
        }
    }
    pub fn accept_output(&mut self, other: Self) -> bool {
        if !self.enabled
            || !other.enabled
            || other.epoch != self.epoch
            || other.serial <= self.serial
        {
            return false;
        }
        *self = other.normalized();
        true
    }
    pub fn accept_presenter(&mut self, other: Self) {
        if (other.epoch, other.serial) >= (self.epoch, self.serial) {
            *self = other.normalized();
        }
    }
}
#[derive(Default)]
pub struct PerformanceWatch {
    slow: u8,
}
impl PerformanceWatch {
    /// Three consecutive zoomed frames above a 30 Hz frame budget trigger notice.
    pub fn observe(&mut self, active: bool, elapsed: std::time::Duration) -> bool {
        self.slow = if active && elapsed > std::time::Duration::from_millis(33) {
            self.slow.saturating_add(1)
        } else {
            0
        };
        self.slow >= 3
    }
}
