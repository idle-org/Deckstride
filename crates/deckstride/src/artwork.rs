//! Embedded window artwork and desktop identity shared by all native hosts.
use std::sync::OnceLock;
use winit::window::{Icon, Window, WindowAttributes};

pub(crate) fn window_attributes() -> WindowAttributes {
    static ICON: OnceLock<Option<Icon>> = OnceLock::new();
    let icon = ICON.get_or_init(|| {
        let pixels = image::load_from_memory(include_bytes!("../assets/window-icon.png"))
            .ok()?
            .into_rgba8();
        let (width, height) = pixels.dimensions();
        Icon::from_rgba(pixels.into_raw(), width, height).ok()
    });
    let attributes = Window::default_attributes().with_window_icon(icon.clone());
    #[cfg(target_os = "linux")]
    let attributes = {
        use winit::platform::{wayland::WindowAttributesExtWayland, x11::WindowAttributesExtX11};
        let attributes =
            WindowAttributesExtWayland::with_name(attributes, "deckstride", "deckstride");
        WindowAttributesExtX11::with_name(attributes, "deckstride", "deckstride")
    };
    attributes
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_icon_is_available_for_every_window() {
        for _ in 0..2 {
            assert!(super::window_attributes().window_icon.is_some());
        }
    }
}
