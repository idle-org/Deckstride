# Implementation task list

Production task completion remains pending. The 0.1.3 [mock display proof](dual-screen-poc.md)
implements the two native windows and selection/control portion of T02, in one process;
T02 isolation and T03/T04 physical acceptance remain open. Completed work is in the [plan](plan.md).
The [implementation choices](implementation.md) define the selected direction and
initial budgets. A checkbox means code, relevant tests and evidence exist, not just
that an API was identified. Do not mark physical acceptance from unit tests.

## P0 — Instrumentation and two-display proof

- [ ] T01: Add monotonic startup/input/frame markers, bounded logs and a benchmark
  recipe. Establish reference machines/decks and baseline Pympress timings with the
  user. Record refresh rates, OS/GPU/backend versions and cold/warm conditions.
- [ ] T02: Build a native presenter shell and separate minimal audience process.
  No PDF dependency on shell startup. Implement monitor enumeration/identification,
  explicit presenter and audience selection, swapping, fullscreen and emergency blank.
- [ ] T03: Test both assignments on Windows and macOS, then Linux X11 and Wayland
  (GNOME/KDE). Add native adapters where required. Verify actual output placement;
  an unsupported placement call is a defect, not a completed task.
- [ ] T04: Test mixed DPI, portrait, negative coordinates, differing resolutions,
  focus/keyboard operation, reconnect, primary-display changes, sleep/wake and missing
  displays. Keep operator content private during every failure and recovery.
- [ ] T05: Implement session-scoped keep-awake leases on all three OSes. Cover idle
  system/display sleep and screensaver handling, partial support and error status.
  Hold protection while blanked/paused; release after ending presentation and crashes.

Exit gate: user verifies both roles on all three OSes, with remaining bugs recorded;
no known selection/isolation failure may be called supported. Keep-awake tests must
exceed the configured idle timeout without mouse/keyboard input, with slides visible
and blanked, on AC and battery where available. Test normal sleep resumes after exit.

## P1 — Bounded workers and first real PDF

Depends on T02; display testing continues in parallel with implementation work.

- [ ] T06: Pin compatible GUI/native crates and PDFium builds for Windows/macOS/Linux;
  verify checksums, licenses and architecture support. Statically link non-system
  dependencies where possible; embed any required runtime assets otherwise. Use the
  same executable for internal audience/worker modes, with no runtime downloads. Crate count/build time/package size are not gates.
- [ ] T07: Define/test versioned bounded IPC, frame-buffer ownership and generation
  IDs. Build worker startup, cancellation, exit detection, termination/reaping and
  retry limits. Prevent stdout/log pipe saturation from blocking a child.
- [ ] T08: Render a first PDF page in a worker; display a preview immediately and
  refine to target resolution. Add rotated/cropped/mixed-size synthetic PDF fixtures.
  Confirm source files remain byte-for-byte unchanged and failed loads retain the show.
- [ ] T09: Add foreground/neighbor/background priority queues and bounded worker
  concurrency. Obsolete jobs must not delay the newest request. Isolate slow storage,
  image decoding, notes and cache writes from both native event loops.
- [ ] T10: Add memory/allocation limits, progress deadlines, worker replacement and
  backoff. Inject a worker hang, crash, malformed reply and oversized image. Verify
  presenter navigation and emergency audience blank remain responsive throughout.

Exit gate: real PDF opens on all three OSes; first-page and input timing captured;
renderer failures cannot hang either window or silently replace the working slide.

## P2 — Prepared slides and persistent cache

Depends on T07–T10; build the format contract before writing persistent cache files.

- [ ] T11: Specify extraction.json version, SHA-256 identity, renderer/settings keys,
  per-page artifacts, geometry and readiness/integrity fields. Add versioned examples
  and round-trip/invalid-manifest tests to the interface documentation.
- [ ] T12: Implement progressive lossless extraction, atomic commits, partial-resume,
  read-only-sidecar fallback and corrupt-cache regeneration. Hash in background;
  never display stale unverified pages to make warm startup look faster.
- [ ] T13: Implement bounded decoded-frame/GPU-texture caches with visible and neighbor
  pages pinned. Prepare textures before a slide advance, pause background work under
  pressure, and benchmark PNG versus LZ4/raw BGRA and mapped/shared buffers.
- [ ] T14: Add GPU initialization/device/surface failure handling and a validated
  softbuffer fallback. Inject failures and verify display assignment survives recovery;
  log visible interruptions and cold CPU fallback startup time.
- [ ] T15: Measure rapid next/back, random jumps, large decks, background extraction,
  low memory, slow/full disk and missing cache artifacts. Verify bounded resource use,
  no busy loops, and cache cleanup touches only owned files.

Exit gate: prepared navigation and blanking meet the initial budgets on reference
hardware. Record cold first page and warm cache-validation costs honestly. Do not
exclude large/complex deck outliers from results.

## P3 — Presenter workflow and host recovery

Depends on usable PDF output and acknowledged displayed-slide state.

- [ ] T16: Show current/next previews, page selection and keyboard controls. Distinguish
  requested/preparing/displayed slides and blank state; keep every dialog on presenter.
- [ ] T17: Extract available notes asynchronously and support explicit notes-page
  interpretation/cropping. Validate note-free fallback; never send notes to audience.
- [ ] T18: Preserve audience output and sleep leases when presenter crashes. Reconnect
  to the actual displayed state. Detect audience event-loop hangs, restart blanked,
  verify destination, then restore prepared content only when safe.
- [ ] T19: Define settings/recovery contracts before persisting display preferences.
  Restore state without silently activating a different monitor; handle changed
  topology and stale device identities explicitly.
- [ ] T20: Test shutdown/cancel during PDF parsing, hashing, disk writes, portal calls,
  worker failure and GPU recovery. No indefinite joins; reap processes and release
  owned resources within bounded shutdown paths.

Exit gate: ordinary presenter work remains responsive under injected faults. Audience
never displays controls, notes or another document due to a late reply or restart.

## P4 — Performance, resilience and release acceptance

- [ ] T21: Run at least an eight-hour presentation soak, repeated open/close and
  thousands of slide changes, plus long paused/blanked sessions exceeding sleep
  timeouts. Track memory/handles, child processes, cache growth and power-lease cleanup.
- [ ] T22: Run fault matrix: stuck PDF calls, unavailable portal, dead D-Bus connection,
  GPU loss, full disk, corrupt cache, unplug/replug, suspend/resume, presenter crash,
  audience crash and saturation of IPC/log queues. Record recovery time and output.
- [ ] T23: Compare cold/warm startup and visible slide latency against Pympress on
  the same corpus/hardware. Fix misses against proposed budgets; review changes with
  the user instead of quietly weakening the requirement.
- [ ] T24: Benchmark targeted OS/scheduler/I/O/GPU tweaks only for reproduced issues.
  Record before/after numbers, lifetime, privileges, restoration and failed experiments.
  Keep useful changes scoped to the presentation and test abnormal-exit restoration.
- [ ] T25: Package runnable artifacts for all three OSes, include icon/backend notices,
  verify offline startup on a clean machine and document supported OS/backend versions.
  Require a portable executable per OS/architecture with every non-system dependency
  bundled: no runtime installer or separately installed PDFium/fonts/toolchain. Audit
  dynamic dependencies on each target. Test empty-cache first run, read-only executable
  directory, arbitrary paths (spaces/non-ASCII), unavailable extraction cache and copied
  binary launch. Verify workers find the executable/assets without relying on cwd.
  Include signing/notarization and bundled-library loading checks for macOS; test an
  optional .app wrapper as well as the documented portable launch path.
  Keep the FreeBSD CI helper/core gates while adding target builds and native checks;
  inability to run the GUI on FreeBSD must not remove production coverage silently.
- [ ] T26: Write the user workflow and acceptance record. User retests every unresolved
  display, responsiveness and sleep defect. No readiness claim until required OS
  matrices, latency evidence and resilience results are recorded.

Optional later work remains in the plan; it must not displace these release blockers.
