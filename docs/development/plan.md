# Implementation and acceptance plan

## Completed foundation — 0.1.0

Independent workspace, source license, app/repository icon, agent instructions,
Jenkins pipeline, badge helpers and tests. The presentation core has bounded
zero-based navigation and audience blanking, with output initially blanked.
The executable only identifies the project's current status.

## Next implementation

1. Validate a PDF renderer and native multiwindow host in a small vertical slice.
   Rust/egui is the initial direction for consistency with Canvas Pilot. Evaluate
   PDF backend licensing, deployment, text/vector fidelity, page rotation,
   password handling, cancellation and availability on Windows, Linux and macOS.
   FreeBSD is the existing Jenkins worker, not a promised desktop platform.
2. Load a PDF read-only, render a page, and replace the current document only after
   successful loading. Bound image dimensions, prefetch and cache memory. Add small,
   synthetic PDF fixtures and real rendering tests.
3. Build an operator window with current/next previews, slide selection, elapsed
   timer and keyboard controls. Keep presenter notes private; PDF notes layouts
   require an explicit interpretation choice rather than guessing.
4. Add one audience window on an explicitly chosen display, aspect-preserving fit,
   fullscreen and blank/unblank. Prove operator/audience isolation. Then extend to
   multiple audience displays, initially showing the same slide.
5. Handle display hotplug, mixed DPI and orientation, failed renderer work, changing
   documents and orderly shutdown. Persist display preferences only after their
   contract and fallback rules are documented.
6. Package each validated platform with app icons and required renderer dependencies,
   licenses and reproducible build instructions. Add a user manual when usable
   workflows exist; reserve only the pdf-presenter book namespace.

## Decisions before extending scope

Independent content per display versus mirrored audience outputs, notes layout,
remote control, annotations, embedded media and transitions need separate scope.
PDF is the input target; native PowerPoint/PPTX playback is not implied. No external
network service or presentation-switcher integration is required for the first app.

## Acceptance still required

Unit tests verify navigation, invalid page rejection and blanking. They do not
verify rendered pixels or window placement. Before calling the app usable, run a
real PDF through the chosen backend and inspect rotated/cropped/mixed-size pages.
On every supported OS, exercise two and three displays, mixed DPI, fullscreen,
keyboard focus, output activation, blanking, monitor unplug/replug and load failure.
Record OS/backend versions and observed results. Confirm no operator content ever
appears on an audience output and no source PDF is modified.
