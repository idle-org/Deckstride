use pdf_presenter::Presentation;
use std::num::NonZeroUsize;

#[test]
fn navigation_stops_at_document_boundaries() {
    let mut show = Presentation::new(NonZeroUsize::new(3).unwrap());
    show.previous();
    assert_eq!(show.page(), 0);
    show.advance();
    assert_eq!(show.page(), 1);
    show.advance();
    show.advance();
    assert_eq!(show.page(), 2);
    show.previous();
    assert_eq!(show.page(), 1);
    let before = show.clone();
    assert!(!show.go_to(3));
    assert!(!show.go_to(usize::MAX));
    assert_eq!(show, before);
    assert!(show.go_to(0));
    assert_eq!(show.page(), 0);
}

#[test]
fn blanking_preserves_navigation_and_starts_safe() {
    let mut show = Presentation::new(NonZeroUsize::new(2).unwrap());
    assert_eq!(show.audience_page(), None);
    show.set_blanked(false);
    assert_eq!(show.audience_page(), Some(0));
    show.set_blanked(true);
    show.advance();
    assert_eq!(show.audience_page(), None);
    show.set_blanked(false);
    assert_eq!(show.audience_page(), Some(1));
}

#[test]
fn single_page_and_maximum_count_stay_in_bounds() {
    let mut show = Presentation::new(NonZeroUsize::new(1).unwrap());
    show.advance();
    show.previous();
    assert_eq!(show.page(), 0);
    let mut largest = Presentation::new(NonZeroUsize::new(usize::MAX).unwrap());
    assert!(largest.go_to(usize::MAX - 1));
    largest.advance();
    assert_eq!(largest.page(), usize::MAX - 1);
}
