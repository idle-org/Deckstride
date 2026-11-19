# Slide navigation and mirrored displays concept

Implemented for review in [the HTML mockup](presenter-menu.html), not the native app.

- Type a 1-based slide number, then Enter to show it. The pending number is visible;
  Backspace edits, Escape cancels, invalid numbers do not navigate. Text fields,
  selectors, modifiers and input composition are excluded. Blanking is preserved.
- All slides opens one reusable, resizable popup, requesting 1000×700 and never
  fullscreen. The browser may choose a tab instead or block the popup; report blocking.
  The popup closes with its parent. It contains mock slide art, not the reference
  image's copyrighted slides. Current and selected slides have separate markers.
- Clicking a thumbnail selects only by default. Show selected explicitly navigates.
  Change on click enables immediate navigation; turning it on does not navigate
  the existing selection. Every new popup starts with this option off. Number+Enter
  remains an explicit navigation action in either window. Preview size is adjustable.
- The popup monitor preference defaults to the first assigned presenter monitor.
  An unavailable selected monitor falls back to presenter without erasing preference.
  The HTML selector is simulated; physical placement remains a native acceptance task.
- Displays assigns each connected screen one of Presenter, Presentation or Unused.
  Multiple screens may share either role. At least one presenter is required; no
  presentation screens is valid rehearsal. Swap exchanges entire role groups.
  Apply commits together; Cancel leaves assignments untouched. Missing displays are
  filtered while surviving assignments remain; fallback stays blanked for review.
- The mock retains browser size/monitor preferences and screen groups in memory only.
  Layout-save buttons still save widget layout only. The native settings schema must
  define stable monitor IDs, role arrays and browser preferences before persistence.

Native requirements: synchronized navigation/blanking across outputs, shared prepared
frames with bounded per-output work, no duplicate PDF rendering for identical frames,
no output stalled by another screen, and controls/notes kept off presentation screens.
Thumbnail preparation must be asynchronous and virtualized for large decks. Test
window placement and hotplug with mirrored roles on Windows, macOS, X11 and Wayland.

Validation harness: append `tests/browser/slide-browser.js` as a script to a temporary
copy of the HTML and run in an isolated Firefox profile allowing popups. Wait for the
PASS/FAIL report after popup open/close; a one-shot screenshot can capture too early.
The 24 checks cover buffered navigation, safe browsing, thumbnail sizing, role groups,
missing monitors and reopening. Run `tests/browser/layout-storage.js` separately for
24 existing layout/persistence checks. Neither test proves native monitor placement.
