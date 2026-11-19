# Implementation task list

0.1.12 implements the approved native workflow; see [implementation evidence and
remaining acceptance](native-implementation.md). The historical tasks below include
broader production hardening and release gates. Unchecked items may be partially
implemented; they are not claims that the native app is absent. Physical acceptance
is deliberately separate from code completion.

Implemented portions: T02 native/mirrored roles and independent output; T05 leases;
T06 bundled renderer/codecs (Hayro replaces PDFium); T07 bounded pipes; T08 real PDFs;
T08a folders; T09 priorities; T10 worker deadlines; T11/T12 verified atomic caches;
T13 bounded decoded cache; T14 software composition; T16 approved presenter/browser
workflow; T17 asynchronous annotation/sidecar notes; T18 last-frame crash hold and
blanked output-host restart. T01 full latency instrumentation, T15 disk quota,
T17 explicit notes-region crop, T18 reconnect, platform acceptance and clean-machine
release checks remain open. Do not mark their broader acceptance from this code alone.

## P0 — Instrumentation and two-display proof

- [ ] T01: Add monotonic startup/input/frame markers, bounded logs and a benchmark
  recipe. Establish reference machines/decks and baseline Pympress timings with the
  user. Record refresh rates, OS/GPU/backend versions and cold/warm conditions.
- [ ] T02: Build a native presenter shell and separate minimal audience process.
  No PDF dependency on shell startup. Implement monitor enumeration/identification,
  explicit presenter and audience selection, swapping, fullscreen and emergency blank.
  Default both roles to fullscreen; retain explicit windowed rehearsal.
  Support several monitors per role (e.g. presenter 1+2, presentation 3, or vice versa).
  One role per monitor; shared slide/blank/navigation state across mirrored views.
  Preserve surviving assignments and isolate each output's rendering/backpressure.
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
- [ ] T08a: Add picture-folder sources with natural filename ordering and progressive
  worker decoding. Finalize the [folder contract](../interfaces/picture-folders.md)
  with fixtures for numeric names, ties, unsupported/corrupt images and slow storage.
  Share the PDF slide pipeline, cancellation, caches and portable codec requirements.
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
  Let the speaker independently show/hide and resize previous/current/next previews, notes,
  elapsed time and local HH:MM time. Hidden widgets reclaim space.
  Header/footer can be hidden independently and recovered by edge hover or right-click;
  persist pinned visibility with project/global layouts, not temporary reveals.
  Provide explicit project and all-project saves; project layouts override the global
  default, with an action to remove the current project override. Keep complete
  notes fitted, and persist layout visibility/sizes in the global settings contract.
  Review the [interactive layout concept](../design/presenter-menu.html) before implementation.
  Add digit-buffer + Enter direct slide navigation (1-based), with visible pending
  number, Backspace editing, Escape cancellation and range checks; preserve blanking.
  Add a separate windowed all-slides browser with adjustable thumbnail size and
  Change on click disabled by default. Selection and live slide are separate; provide
  explicit Show selected. Place it on a chosen monitor or presenter by default,
  falling back to presenter if missing. Bound/virtualize thumbnail loading.
- [ ] T17: Extract available notes asynchronously and support explicit notes-page
  interpretation/cropping. Load image sidecar `.txt` notes and global `notes.json` or
  `notes.toml`; test precedence, invalid files and missing notes. Always word-wrap and
  size the complete notes to fit their pane without clipping or requiring scrolling.
  Test long notes, long unbroken words, Unicode, resize and mixed DPI; keep layout
  responsive and bundle required fonts. Validate note-free fallback; never send notes
  to audience.
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
