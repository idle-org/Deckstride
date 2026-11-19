# Changelog

## Unreleased

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
