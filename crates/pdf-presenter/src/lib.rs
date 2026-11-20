//! Native PDF and picture-folder presentation, isolated rendering and shared state.
//! Page indices are zero-based; blanking preserves the selected page.
use std::num::NonZeroUsize;

/// Navigation and audience-output state for one nonempty document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presentation {
    page_count: NonZeroUsize,
    page: usize,
    blanked: bool,
}

impl Presentation {
    /// Start at the first page, with audience output initially blanked.
    pub fn new(page_count: NonZeroUsize) -> Self {
        Self {
            page_count,
            page: 0,
            blanked: true,
        }
    }

    /// Selected document page, regardless of output blanking.
    pub fn page(&self) -> usize {
        self.page
    }

    /// Page visible to the audience, or `None` while blanked.
    pub fn audience_page(&self) -> Option<usize> {
        (!self.blanked).then_some(self.page)
    }

    /// Show or hide audience content without changing navigation.
    pub fn set_blanked(&mut self, blanked: bool) {
        self.blanked = blanked;
    }

    /// Select a valid page. Invalid requests leave the state unchanged.
    pub fn go_to(&mut self, page: usize) -> bool {
        if page >= self.page_count.get() {
            return false;
        }
        self.page = page;
        true
    }

    /// Advance without wrapping at the end of the document.
    pub fn advance(&mut self) {
        self.page = (self.page + 1).min(self.page_count.get() - 1);
    }

    /// Go back without wrapping at the beginning of the document.
    pub fn previous(&mut self) {
        self.page = self.page.saturating_sub(1);
    }
}

/// Native desktop proof of concept.
pub mod desktop;
/// Software mock-slide drawing and controls.
pub mod paint;
/// Mock presentation and monitor selection.
pub mod poc;

/// Documents and prepared frames.
pub mod document;
/// Native persistent configuration.
pub mod settings;
/// Isolated background processes.
pub mod worker;

/// Native application entry points.
pub mod native;
/// Isolated output and power leases.
pub mod output;
/// Native presenter UI.
pub mod presenter_ui;
/// Shared native application state.
pub mod studio;

/// Presenter interface language.
pub mod localization;
/// Standalone presenter palettes.
pub mod theme;

mod artwork;

/// Monotonic elapsed/countdown timer.
pub mod timer;

/// Bounded vector pointer and presentation annotations.
pub mod tools;

/// Editable presentation notes sidecars.
pub mod notes;

pub mod zoom;
