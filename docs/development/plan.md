# Implementation and acceptance plan

## Product requirements — agreed 2026-11-19

The goal is a blazingly fast PDF presenter in the spirit of Pympress. Reliability
of the two display roles and presentation speed take precedence over matching the
Canvas Pilot technology stack. This is the implementation brief, not a claim that
these capabilities already exist.

1. **Dual-screen operation is a hard requirement on Windows, macOS and Linux.**
   The operator explicitly selects the display for the presenter view and the display
   for the audience presentation. Both roles must work together, including fullscreen
   audience output. This is foundational work, not a later enhancement or an optional
   platform feature. The user will test iteratively until it works. Do not declare
   the application ready with an unresolved display-placement failure on any of the
   three operating systems.
2. **Presentation speed is the primary performance goal.** Show a usable first page
   as soon as possible and prepare the rest progressively. Pre-rendering pages to
   images and keeping them beside the PDF with an `extraction.json` containing source
   SHA and extraction metadata is explicitly acceptable. Any practical implementation
   technique may be considered for speed, including native platform code, background
   workers, renderer subprocesses, disk caches, decoded-image caches, texture preloading
   and lower-resolution previews followed by sharper images. Benchmark choices;
   avoid committing to a stack solely for consistency with another tool.
3. **Presenter mode must show the current and next slides.** Include notes when they
   can be extracted reliably; current/next previews are the required fallback when
   notes are absent or unsupported. Notes extraction must not delay presentation or
   leak notes onto the audience output.

The hard reliability requirement is an acceptance gate, not an assertion that an
OS, compositor, GPU or disconnected monitor cannot fail. Handle failures explicitly,
keep presenter content private, and make output recovery possible without restarting
or losing the selected slide. Do not silently demote an affected OS to unsupported.

## Completed foundation — 0.1.0

Independent workspace, source license, app/repository icon, agent instructions,
Jenkins pipeline, badge helpers and tests. The presentation core has bounded
zero-based navigation and audience blanking, with output initially blanked.
The executable only identifies the project's current status. No PDF renderer or
native windows have been implemented; no physical-display acceptance is recorded.

## Implementation order

1. Prove two independently assigned windows on Windows, macOS and Linux first.
   Enumerate displays, identify them visibly, select presenter/audience destinations,
   swap roles, and enter/leave audience fullscreen. Rust is the foundation; egui is
   a candidate, not a constraint. Evaluate platform APIs or another host if needed.
   On Linux, test X11 and Wayland separately: investigate placement/fullscreen
   constraints and provide an explicit operator placement flow where needed. A
   limitation must be resolved and tested, not hidden behind automatic placement.
2. Integrate a real PDF backend into those two windows. Render the requested page
   first, display it immediately when ready, then prepare next/previous pages ahead
   of more distant pages. Prioritize interactive jumps over background extraction,
   cancel obsolete work, and keep UI/input handling independent of rendering.
   Evaluate backend licensing, packaging, rotation/crop fidelity and all three OSes.
3. Add reusable extraction and bounded memory/GPU caches. Warm presentation should
   use prepared images/textures rather than synchronously re-rendering the PDF on
   each slide change. Avoid decode, disk I/O and texture-upload stalls on the input
   path. Use bounded parallelism and measured prefetch, not unbounded whole-deck RAM.
   Start displaying without waiting for the complete deck or its source hash; only
   reuse persistent extraction after validating its identity and render settings.
   See [cache design requirements](../interfaces/extraction-cache.md).
4. Finish presenter controls: current/next slide previews, keyboard navigation,
   direct selection and blank/unblank. Extract available notes asynchronously; use
   current/next-only mode if unavailable. Support explicit notes-page layout/cropping
   when needed; do not assume every PDF carries structured speaker notes.
5. Iterate on real-machine display and performance failures with the user until the
   acceptance gates below pass. Package each OS with the chosen renderer dependencies,
   app artwork, license notices and reproducible build instructions. FreeBSD remains
   the CI host, not a substitute for Windows/macOS/Linux desktop testing.

## Display acceptance gate

Record OS/version, window backend (including Linux X11/Wayland), GPU, monitor setup,
app commit, reproduction, result and remaining defects. The user will provide
physical testing; automated tests and headless CI do not establish this acceptance.

For each of Windows, macOS and Linux, verify explicit presenter and audience display
selection, both role assignments, swap, fullscreen exit/re-entry, focus and keyboard
navigation while both windows are active. Cover mixed DPI, differing resolutions,
portrait orientation, negative desktop coordinates and primary-display changes.
Exercise disconnect/reconnect, sleep/wake and missing saved displays. Never place
notes or operator controls on the audience display as a fallback. Keep selection
and blanking stable; report unavailable output and offer reassignment. With only one
available display, offer rehearsal and explain that dual-screen output is unavailable.
No unresolved hard-requirement failure is accepted as a finished implementation.

## Speed acceptance gate

Use fixed synthetic and user-approved decks, with simple/vector-heavy/image-heavy
pages, rotations and mixed sizes, plus large decks. Compare equivalent workloads
against Pympress on the same machine, resolution and cache conditions when available.
Record hardware, app/backend versions, deck SHA, display resolution, page count,
cold versus warm cache and test method. Do not infer speed from language choice.

Measure time to first visible page, time to presentation-ready dual windows, and
input-to-visible slide latency (median, p95 and worst case). Include rapid next/back,
uncached random jumps, notes extraction and background cache work. Record peak RAM,
GPU memory when measurable, disk-cache size and extraction time. Distinguish actual
visible-output timing from CPU submission timing. Establish numerical budgets from
initial measurements and user testing; none have been agreed yet. No claims of
"blazingly fast" acceptance without recorded measurements and user validation.

## Scope after the core requirements

Multiple audience outputs beyond the required pair of presenter/audience roles,
independent decks per display, remote control, annotations, embedded media and
transitions are later scope. Native PPTX playback is not implied. No network service
or presentation-switcher integration is required. Preserve source PDFs and never
show a wrong or stale slide to achieve a lower latency measurement.
