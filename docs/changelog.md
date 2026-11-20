# Changelog

## Unreleased

### 0.1.64

- Keep previous/next shortcuts active in the presenter and All slides, even when a
  menu control consumed the key; notes editing and modified shortcuts keep priority.
  Restore a wider high-contrast scrollbar at the far right of All slides. Label
  absent thumbnail frames accurately as preparing, queued, cached on disk or failed
  instead of claiming every scrolled-to slide is preparing. Linux checks pass with
  86.38% whole-application coverage; the release binary is rebuilt.

### 0.1.63

- Aim double-click fit-width zoom at the clicked page position instead of always
  centering the page. Clamp the crop at page edges as a best effort; a second
  double-click still resets to the whole page. Keep shared presenter/output zoom.
  Linux checks pass with 86.42% whole-application coverage; the release binary
  is rebuilt. Physical pointer/display acceptance remains separate.

### 0.1.62

- Keep the default timer at zero when a deck's initial slide is merely displayed.
  Start it on the first navigation, zoom or output visibility action. Rename Restart
  to Reset: it now returns to zero and rearms this action trigger, while explicit
  Start still begins immediately and Pause cannot be undone by later activity.
  Linux checks pass with 86.54% whole-application coverage; the release binary
  is rebuilt. Physical presentation timing remains a separate acceptance check.

### 0.1.61

- Keep Displays, Layout and fullscreen as header buttons and remove their Menu
  duplicates. Add direct Start/Pause and Restart timer header buttons; remove
  timer controls from Menu while retaining them in Layout's timer settings.
  Wrap controls on narrow windows and reserve a separate row for header clocks.

### 0.1.60

- Match the zoom-enabled current preview to the first connected presentation
  output's aspect ratio. Fit-width, wheel zoom and panning now show the same crop
  in that preview and output, rather than fitting independently to the preview pane.
  Report actual output-window dimensions for windowed/resized output; use monitor
  dimensions until reported. No image copies or PDF rendering are added.
  Linux checks pass with 86.35% coverage, including actual preview-mesh crop
  comparisons and portrait-page fit-width regression checks. Physical display
  acceptance remains separate.

### 0.1.59

- Increase the Menu dropdown height from 520 to 720 logical pixels so Quit stays
  accessible without scrolling on taller screens. Limit it to the space below
  its button and retain scrolling on smaller windows.

### 0.1.58

- Double-click an unzoomed current preview or presentation output to fit the page
  width to that viewport (within the existing 8× limit). Double-click while zoomed
  still resets to the whole page. Reuse the buffered texture and shared zoom state;
  update English/French gesture hints and the guide. Linux checks pass, including
  pointer-gesture regression tests and 86.22% whole-application coverage.

### 0.1.57

- Fix tiny clock/timer text in the header, above slides and below notes: use the
  reserved row height and account for labels/padding. Put large header timers,
  or timers in narrow windows, on a separate row below the menu controls.
  Regression checks verify rendered text size and separation from slides.
  Linux workspace tests, 13 Python tests, formatting and warnings-denied
  Clippy/rustdoc pass; release binary rebuilt. Coverage measurement failed twice
  on corrupt LLVM worker profiles after the tests passed.

### 0.1.56

- Move the presenter header fullscreen button after Menu, Displays, All slides
  and Layout, keeping the hide-header icon at the far left.

### 0.1.55

- Let zoom expand portrait/landscape pages into each screen without stretching,
  instead of trapping them inside the original fitted area. Save an opt-in
  “Keep fitted page area when zooming” preference. Keep buffered texture reuse,
  bounded drawing, pointer anchoring and shared zoom/center across outputs.
  Linux regression checks cover aspect ratio, screen fill, edges, settings and
  pixel output without new texture uploads. Physical multiscreen testing remains open.

### 0.1.54

- Add globally saved, default-off page zoom (1×–8×), wheel/pinch zoom, drag panning
  and double-click reset from the presenter/current preview or any output. Mirror
  the crop with lightweight ordered messages; reset on slide changes and preserve
  visibility. Reuse buffered textures without new rendering, file I/O or image
  allocations. Explain sharpness/CPU tradeoffs and warn on repeated slow frames.
  Bundle a licensed renderer patch for bounded scanline sampling and parallel crop
  bands (four threads maximum). Local software-drawing medians: ~5 ms at 1080p,
  ~16 ms at 4K; physical multicreen/OS acceptance remains separate.
  Linux checks cover input, pixels, opt-in migration and disk-free output buffering.

### 0.1.53

- Import UTF-8 TXT notes separated by hyphen lines or Markdown notes separated by
  level-one headings, via Menu or presenter drag-and-drop. Review before converting
  to a complete presentation notes sidecar; preserve remaining notes/source files,
  detect external conflicts and keep processing off the presentation thread.
  Document examples and mapping in the README. Linux regression coverage includes
  parsers, worker flow, natural ordering, atomic saves and conflict handling.

### 0.1.52

- Load presentation-named notes sidecars before falling back to PDF/folder notes.
  Double-click notes to create a full-deck sidecar if absent and edit the captured
  slide in a resizable editor. Save atomically through an isolated worker, preserve
  source files and other notes, detect external conflicts and retain failed drafts.

### 0.1.51

- Preserve shown/manually blanked output through fullscreen and routing trials,
  confirmation, rollback, hotplug and host recovery. Separate explicit output
  blank requests from diagnostic status, retaining presenter privacy protections.

### 0.1.50

- Compact All slides controls into two short rows on wide windows. Move explanatory
  text to tooltips, remove nested settings cards and reduce padding/button heights;
  keep wrapping and separately scrolling controls for small windows.

### 0.1.49

- Add monotonic elapsed/countdown timing with Start/Pause, Restart, configurable
  duration and first-output autostart. Preserve pauses across slide changes; reset
  on successful source loads and stop countdown at zero. Save timer preferences
  with layouts, and add an 8–120 pixel slide progress bar height control.

### 0.1.48

- Add independent left/right of notes/slides timer positions and raise clock/elapsed
  text size to 320 logical pixels. Reserve side columns, stack shared-side timers,
  fit large timer text to its panel and refit notes in the remaining space.

### 0.1.47

- Escape in the presenter dismisses transient UI without leaving fullscreen or
  blanking output. Add explicit Enter/Exit fullscreen controls in the header and
  menu, using the existing reversible display trial. Apply `--windowed` only at startup.

### 0.1.46

- Choose unsaved RAM budgets from 15% of available memory, with a soft 512 MiB
  floor limited to 20% and a 64 MiB operational minimum for constrained systems.
  Preserve explicit saved budgets and apply the lower limit to both processes.

### 0.1.45

- Distinguish Theme and Language section headings with smaller, bold, muted text
  and separate their selectable submenus with spacing and a divider.

### 0.1.44

- Size Layout scrolling from its resized window rather than the desktop. Remove
  Displays’ forced content width, wrap roles/names, and constrain floating windows
  to the viewport. Bound the browser toolbar separately so short windows retain
  thumbnail space and the Close footer.

### 0.1.43

- Reorganize Layout into padded preset, slide/notes, timer/progress and bar sections.
  Pair timer positions with their controls, explain size units and keep save actions
  outside the settings scroll area. Disable project actions until a document is open.

### 0.1.42

- Remove the redundant All slides heading from the browser content and move Close
  into a fixed bottom-right footer, outside the scrolling thumbnail grid.

### 0.1.41

- Redesign All slides controls with labeled settings cards, a filled Compact/Large
  slider, readable monitor names and a prominent Show selected button. Stack cards
  in narrow windows and explain safe/live browsing alongside its controls.

### 0.1.40

- Draw header/footer visibility icons directly instead of relying on arrow glyphs.
  Use explicit Auto-hide header/footer tooltips and accessible names, switching to
  Keep header/footer visible when the bar is already auto-hidden.

### 0.1.39

- Rename the All slides screen selector to “Show on screen” (French: “Afficher
  sur l’écran”).

### 0.1.38

- Keep the All slides scrollbar at the right edge with a stable gutter. Fit
  thumbnail columns to the available window width, using Preview size as the
  preferred density while retaining slide aspect ratios and virtualized rows.

### 0.1.37

- Double-click a thumbnail in All slides to open a resizable preview without
  changing output. Change on click mode retains live navigation without previews.
  Escape dismisses the preview first; changing documents clears stale previews.

### 0.1.36

- Give All slides a padded toolbar with separate title, preview/display settings,
  and presentation-action rows. Enlarge control spacing and label the browser
  screen selector; rows still wrap in narrower windows.

### 0.1.35

- Hide the previous-slide preview by default; enable it in Layout. Preserve explicit
  visibility choices in saved layouts and verify default/opt-in persistence.

### 0.1.34

- Restrict keepawake to its supported Linux, Windows and macOS targets so the
  FreeBSD CI worker can compile. Other platforms explicitly report sleep protection
  as unavailable while retaining the helper status/parent-pipe lifecycle.
- FreeBSD cross-target Clippy passes for all targets/features; native FreeBSD
  execution awaits Jenkins. Linux checks/native tests and 13 Python tests pass;
  whole-application line coverage is 86.40%.

### 0.1.33

- Prepare local Linux/Windows x86_64 preview artifacts with rust_realease; verify
  all six artifact checksums, portable/binstall archive contents, extracted Linux
  Wayland/X11 smoke, launcher and embedded notices. The installed older tool omits
  portable bundles, so those were assembled from its binaries using the configured
  file list. Windows runtime remains untested; no remote publication performed.

- Associate All slides with a presenter window on Linux: send Wayland's toplevel
  parent hint and X11's transient/dialog hints so tiling desktops can float it.
  Keep the browser separate, resizable and non-modal; refresh its parent after
  presenter recreation. Compositor policies still control placement. Linux tests,
  Clippy/rustdoc, Python checks and X11 native smoke pass; coverage is 86.38%.
  Verified the Wayland parent request before first buffer and actual X11 dialog/
  transient properties. Niri and Windows/macOS runtime testing were not available.

### 0.1.32

- Show a warning beneath the display setup notice whenever either fullscreen
  option is disabled: window positioning can be unreliable, especially on Linux;
  return to fullscreen if affected. Include French text and user-guide guidance.
  Linux checks and native X11 smoke pass; whole-app coverage is 86.70%.
  Windows/macOS runtime checks were not run.

### 0.1.31

- Rename the display setup button to “Apply changes” (French: “Appliquer les
  modifications”). Preserve the 15-second confirmation, automatic revert and
  blanked-output behavior; update the user guide to match. Linux checks and native
  X11 smoke pass; whole-app coverage is 86.78%. Windows/macOS were not tested.

### 0.1.30

- Show “End of slides” in the presenter's Next preview at the last real slide.
  This is a non-interactive placeholder, never a deck page, browser entry or
  audience frame. Navigation still stops at the last real slide. Linux tests,
  native X11 smoke, formatting, Clippy/rustdoc and Python checks pass; whole-app
  coverage is 86.82%. Windows/macOS runtime checks were not run.

### 0.1.29

- Draw numbered screen rectangles in Displays using each monitor's pixel aspect
  ratio, including portrait and ultrawide screens. Keep their slots aligned and
  scale the screen number to fit. Linux tests, native X11 smoke, formatting,
  Clippy/rustdoc and Python checks pass; whole-app coverage is 86.72%.
  Physical multi-monitor and Windows/macOS validation were not run.

### 0.1.28

- Add Shift+Space to move back one slide from the presenter; Space still advances.
  Read current input modifiers so Shift works even before the next repaint.
  Preserve text-input focus, slide boundaries and manual blanking. Linux tests,
  native X11 smoke, formatting, Clippy/rustdoc and Python checks pass; whole-app
  coverage is 86.70%. Windows/macOS runtime checks were not run.

### 0.1.27

- Remove Jenkins's Xvfb prerequisite and native-window test wrapper. Keep native
  smoke available locally and all production code in coverage. Temporarily use a
  60% headless Jenkins floor; local full-GUI runs retain the 75% floor. Coverage
  badges still show the actual measurement and tool/test failures remain failures.
  Headless coverage passes at 65.21%; all 13 Python helper tests pass. Jenkins
  shell blocks pass syntax checks; the remote job has not been rerun.

### 0.1.26

- Display transparent slide areas on white by default. Appearance now offers
  Page background and Fit padding color, shared by previews, browser and audience
  output and saved globally. Existing cached slides update without rerendering;
  emergency/manual blanking remains black. Correct PDF alpha conversion and regenerate
  older PDF artifacts under a new key to avoid dark translucent edges. Linux tests,
  native X11 smoke, formatting, Clippy/rustdoc and Python checks pass; full local
  whole-app coverage is 86.40%. Physical multi-monitor and Windows/macOS checks
  were not run.

### 0.1.25

- Make the README user-facing with release-first installation, portable download
  choices and a short presentation workflow. Document updates and optional Linux
  launcher integration in the user guide; remove the stale quickstart version.

### 0.1.24

- Separate elapsed time and clock into labeled boxes with independent Header,
  Above slides and Below notes presets, saved in project/global layouts.
- Add an opt-in presenter slide-progress bar (requested slide / total), distinct
  from loading progress and visible with hidden bars. Older layouts retain defaults.
- Cover settings compatibility/round trips, timer placement and navigation progress.

### 0.1.23

- Put compact arrow icons at the far left of both header and footer to hide or pin
  each bar, retaining localized tooltips and existing edge/right-click reveal.

### 0.1.22

- Keep settings menus open while selecting themes/languages or adjusting Preferences.
  Use nested menus for appearance choices so selections stay inside the menu tree.
- Cover theme/language selection, preference toggles, storage expansion and command,
  outside-click and Escape dismissal with pointer interaction regression tests.

### 0.1.21

- Preview display routing/fullscreen changes for 15 seconds before saving. Keep
  confirms; Revert, Escape, timeout or hotplug restores the prior setup.
- Provide confirmation on both presenter/output screens and startup right-click
  swap on either role, disabled after explicit slide navigation/show.
- Keep output blank during recovery, reject stale confirmations, and leave
  unconfirmed assignments out of preference saves. Test rollback and pointer controls.

### 0.1.20

- Restrict routine agent validation to Linux. Windows/macOS builds and cross-target
  checks run only on explicit request or during requested release preparation.

### 0.1.19

- Continue preparing oversized decks by evicting distant frames, protecting active
  slides and retaining processed disk artifacts. Reload evicted slides on demand.
- Mirror the rolling working set in the output process; account for its pinned
  last frame, discard obsolete reads and free distant frames before replacement.
- Report prepared versus RAM-resident counts, evictions and ETA; reserve READY
  for fully resident decks. Avoid idle eviction loops and refill when budget grows.
- Regression-test pressure, cached reload, idle stability and budget changes;
  exercise the authorized 322-page Ravenous.pdf with a 256 MiB decoded budget.

### 0.1.18

- Use uv-selected Python 3.11+ throughout Jenkins, including badge initialization,
  cleanup, helper tests and nested coverage execution. Launcher tests reuse that interpreter.
- Explain missing Xvfb with worker provisioning instructions; uv manages Python
  and cannot replace the native X server required by GUI tests. Keep GUI coverage mandatory.

### 0.1.17

- Embed application icons in all native windows and the Windows executable.
  Set Linux X11/Wayland identity and provide optional per-user desktop integration.
- Prepare rust_realease configuration for Linux/Windows executables, binstall
  archives and portable bundles, with checksums and preview release instructions.
  No tag, push or remote release is performed by this preparation.

### 0.1.16

- Ensure presenter shutdown can terminate/reap its output process even when the
  supervisor is blocked writing to a full IPC pipe. Run cleanup on event-loop errors
  and destructor paths as well as normal exit.
- Detect loss of the presenter connection. Preserve the last audience frame, but
  allow the orphaned output to exit through its window Close control or Escape.
- Add blocked-pipe shutdown/drop regression coverage and native Linux/X11 orphan
  Close/Escape tests. Windows/macOS runtime acceptance remains pending.


### 0.1.15

- Give Windows PDF/folder dialogs the active presenter window as their owner, so
  they remain above its fullscreen view. Restore owner input/focus on completion,
  including worker failure/timeout; temporarily lower a topmost slide browser.
- Permit only one outstanding chooser request across all presenter windows. Disable
  Open controls while it is active; repeated clicks cannot queue more dialogs.
- Test a 100-request burst, cancellation, selection, failures, retry and owner IPC.
  Windows compilation passes; native Windows modal placement still needs user retest.


### 0.1.14

- Replace the floating Menu with an anchored dropdown; present display assignments
  as numbered cards with direct role choices.
- Raise/focus the windowed slide browser over fullscreen presenter windows and
  restore an existing browser when opened again; add an explicit Close control.
- Show output by default on document open, with a saved opt-out. Distinguish disabled
  output using a colored indicator and NO OUTPUT DISPLAYED, including with hidden bars.
- Add Canvas Pilot’s 20 built-in palettes and System/English/French interface and
  progress preferences. Persist globally with backward-compatible settings defaults.
  Detailed diagnostics remain verbatim; other languages/external packs are not bundled.
- Retain emergency blanking, topology privacy protection and existing RAM preparation.


### 0.1.13

- Prepare every slide at the largest active screen resolution (4096-pixel edge cap),
  using two supervised workers and a quick cold preview. Reopen verified processed
  frames without rerendering or decoding; avoid redundant preview renders on cache hits.
- Retain the complete prepared deck in presenter and output RAM. READY requires both
  buffers; preparation pauses explicitly at the configurable RAM budget instead of
  evicting slides and incurring later disk reads. Resolution/host changes reset readiness.
- Show preparation stage, slide counts, cache reuse, RAM use and estimated time left;
  retain a compact notification when menus are hidden. Keep errors visible separately.
- Test warm-cache reuse, budget pause/resume, resolution changes and navigation after
  deleting source/cache files, including eight slides in the independent output host.


### 0.1.12

- Implement the approved native presenter workflow: independently sized/hidden widgets,
  fitted notes, project/global layout saves, recoverable header/footer, digit+Enter
  jumps, windowed virtualized slide browser and saved mirrored display groups.
- Load real PDFs and naturally ordered image folders with annotation/sidecar/global
  notes, progressive rendering and SHA-256 verified atomic raw-frame caches.
- Bundle Hayro, image codecs and UI fonts; supervise same-executable worker processes,
  preserve independent presentation output, recover host failures blanked and request
  process-scoped sleep protection including an X11 screensaver/DPMS fallback.
- Add native user and format documentation plus real rendering, UI, persistence,
  malformed-input and stuck-worker recovery tests. Physical multi-OS acceptance and
  further performance/resource hardening are tracked in the native implementation record.


### 0.1.11

- Add independently hideable presenter header/footer to the HTML concept, with
  edge-hover reveal and right-click controls to restore/pin either bar. Hidden bars
  release space; transient reveals overlay the content and remain usable on hover.
- Save pinned bar visibility per project/globally. Older layouts default to both
  visible; temporary reveals never change saved preferences. Native UI remains pending.
  Validation: JavaScript syntax and 67 Firefox checks (19 bar-visibility/recovery,
  24 layout/persistence and 24 slide-browser/display-group regressions).

### 0.1.10

- Add number + Enter slide jumps and a separate windowed thumbnail browser to the
  HTML concept. Preview size is adjustable; clicks select without changing output
  unless Change on click is enabled. Explicit Show selected changes the slide.
- Allow several simulated monitors per presenter/presentation role, with group swap
  and missing-monitor fallback. The slide browser has a simulated monitor preference,
  defaulting to presenter; actual monitor routing and native implementation are pending.
  Validation: JavaScript syntax, real Firefox popup inspection, 24 navigation/popup/
  mirrored-display checks and 24 existing layout/persistence regression checks.

### 0.1.9

- Allow hiding the current-slide preview in the HTML concept and saved layouts.
  Hiding all previews expands visible notes; presentation status remains visible
  in the footer regardless of preview visibility. Existing saved layouts still load.
  Validation: JavaScript syntax and 24 Firefox browser checks, including hidden-current
  persistence, all-preview space recovery and continued navigation/output status.

### 0.1.8

- Add explicit project/all-project layout saves to the HTML concept using browser
  storage. Project overrides take precedence; a reset-to-default action removes only
  the current override. Two demo projects allow reviewing scope and reload behavior.
  Invalid/unavailable storage reports a failure without interrupting layout editing.
  Native settings remain unimplemented. Validation: JavaScript syntax, Firefox
  visual inspection and 17 browser checks for scope, restoration and storage errors.

### 0.1.7

- Extend the HTML presenter concept with a live layout panel: independently hide/show
  and resize previous/next previews, notes, elapsed time and a local HH:MM clock.
  Current-slide size is adjustable; hidden widgets reclaim space. Include balanced,
  slide-focus and notes-first presets, with complete notes automatically fitted.
  Native layout and global persistence remain planned; this is a session-only mockup.
  Validation: JavaScript syntax, Firefox visual inspection and 20 browser checks
  covering widget sizing/visibility, recovered space, presets and long-note fitting.

### 0.1.6

- Add a standalone interactive HTML presenter-menu concept for review: current/next
  slides, auto-fitting notes, display assignment/swap, simulated missing screens and
  settings preview. Use “Presentation” for the viewer-facing output in the concept.
  No native UI or global configuration changes; choices last only for the page session.
  Validation: JavaScript syntax check, Firefox visual inspection and 10 browser
  interaction checks covering navigation, assignments, fallback, menus and note fitting.

### 0.1.5

- Default both presenter and audience windows to fullscreen on every platform;
  `--windowed` keeps both decorated, including single-display rehearsal.
- Record planned picture-folder slides in natural filename order, sidecar/global
  JSON/TOML notes, and complete notes fitted to the available presenter pane.
  Folder loading and notes layout remain unimplemented in this display proof.
- Validation: 16 Rust tests including native smoke, formatting, Clippy/rustdoc,
  10 Python tests, 88.92% whole-application coverage and Linux release build.
  Physical dual-screen startup verification remains pending.

### 0.1.4

- Narrow Linux/X11 fix for mock-window swap/fullscreen transitions: stop resetting
  unaffected windows and stage exit, movement and fullscreen entry without blocking.
  New requests replace pending transitions. Windows and Wayland paths are unchanged.
  Validation: 16 Rust tests including native smoke, Clippy/rustdoc, Windows compile
  check and 89.06% whole-application coverage; Linux physical swap retest is pending.
- User reports Windows swap/fullscreen working smoothly; Linux works but transitions
  were messy. This is partial user evidence, not full platform acceptance.


### 0.1.3

- Added a native dual-screen proof: presenter/current-next controls, fullscreen mock
  audience slides, explicit monitor selection, swapping, blanking and safe windowed
  recovery. One-display rehearsal is clearly identified; no PDF or sleep guard yet.
- Added software rendering and monitor/control tests, real native-window smoke and
  isolated Xvfb CI. Linux/X11 single-display rendering was inspected; physical dual
  display acceptance and Windows/macOS/Wayland runtime testing remain pending.
  Validation: 14 Rust/10 Python tests, lint/docs and 89.13% whole-app coverage;
  Windows, macOS and FreeBSD compile checks passed. CI requires Xvfb.


### 0.1.2

- Added an ordered implementation task list and concrete runtime choices: isolated
  PDF workers/audience host, progressive image caches, bounded recovery, native sleep
  inhibition and measured latency budgets. Dependency/build/package size constraints
  do not take precedence over fast startup and resilience. Require portable executables
  with bundled non-system dependencies and offline clean-machine tests. Choices are researched;
  desktop implementation and physical acceptance remain pending.

### 0.1.1

- Established selectable dual-screen operation on Windows/macOS/Linux as a hard
  acceptance gate and presentation speed as the primary performance goal. Prioritized
  progressive rendering, reusable source-SHA image caches and current/next previews
  with extractable notes. Added physical-display and speed validation requirements;
  cache schema and application behavior remain unimplemented.

### 0.1.0

- Prepared the standalone PDF Presenter repository with a tested navigation/blanking
  core, an explicit bootstrap executable, agent guidance and implementation plan.
- Added Jenkins validation, whole-workspace coverage and isolated three-badge
  publication, plus generated application/repository artwork. PDF rendering and
  multiscreen GUI are future work; no physical-output acceptance is claimed.
