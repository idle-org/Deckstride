//! Presentation state shared by future operator and audience windows.
//!
//! This bootstrap does not load or render PDFs. Page indices are zero-based;
//! blanking hides audience content without losing the selected page.
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
