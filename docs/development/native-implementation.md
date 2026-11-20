# Native implementation — 0.1.12 and 0.1.13

The HTML approval is implemented in the native executable. This record supersedes
starting-stack proposals in `implementation.md`; historical acceptance budgets still
apply. The [user guide](../user-guide.md) describes actual controls and limitations.

## Implemented

- Real PDF and picture-folder loading, natural filename ordering, source preservation,
  separate annotation/sidecar/global notes extraction after a usable frame exists.
- Presenter current/previous/next previews, fitted notes, elapsed/HH:MM widgets,
  independent visibility/sizes, project/global saves and hidden-bar recovery.
- Digit+Enter jumps; independent windowed, virtualized slide browser with preview
  sizing, selected/current distinction, explicit Show selected and optional click-live.
- N-monitor role groups, mirroring, group swap, fullscreen defaults, standard-location
  global preferences, missing-monitor fallback and runtime operator-screen privacy hold.
- Independent presentation process with acknowledged frame submission, blanking,
  neighbour preloading, heartbeat supervision and bounded restarts preserving output visibility. Last frame
  survives presenter failure. Slow work is isolated from both native event loops.
- Same-executable workers for rendering, notes, dialogs, config and frame reads;
  bounded pipes and deadlines, source generations, foreground supersession, corruption
  checks and atomic cache/settings writes. Frame memory and image decodes are bounded.
- Process-scoped keepawake leases; X11 screensaver/DPMS fallback with explicit partial
  support reporting. Idle scheduling avoids unconditional frame rendering.

## Implementation choices

Hayro 0.9 is bundled as the PDF renderer instead of the initially proposed PDFium.
Image 0.25 provides the enabled codecs. No external renderer, fonts download or
runtime extraction/install step is needed. egui 0.34 controls are composed through
`egui_software_backend`/softbuffer, using cached meshes/textures and damage handling.
The minimal output process only receives raw frames and display/blank state, never
operator notes or menus. This establishes a software rendering baseline; it does not
claim GPU-level throughput at every resolution.

0.1.13 replaces neighbour-only caching with whole-deck preparation and RAM residency.
A cold 512-pixel preview precedes screen-sized frames; warm verified full-size cache
hits skip that preview. Two workers fill the complete deck, and the independent
output acknowledges its own complete RAM buffer before READY. Small-screen outputs
share the largest screen-sized variant (4096-pixel cap). Progress, cache reuse, RAM
usage, approximate ETA and memory-pressure pause are explicit in the presenter.
No application slide-file reads, decoding or rasterization occur on prepared navigation.
OS paging can still occur; configuring enough available physical RAM remains necessary.

The default decoded budget is 15% of available RAM per process, with a soft
512 MiB floor constrained to 20% of available RAM (64–16384 MiB operational bounds),
adjustable in Menu. Insufficient capacity uses rolling eviction of non-active slides
and never reports the entire deck as RAM-ready. Worker/texture memory is
additional. See [cache](../interfaces/extraction-cache.md),
[settings](../interfaces/settings.md) and [private IPC](../interfaces/process-protocol.md).

The original single-process mock remains available through `--display-proof` for
window-management comparison. It is not the default native presenter.

## Validation evidence, 2026-11-19

Local Linux/X11, kernel 7.3.1-1-cachyos, Intel i5-8500, rustc 1.100.0. Checks include
real synthetic PDF pages/annotations and picture folders, notes precedence and errors,
settings round trips/invalid versions, checksum corruption/regeneration, stale/failed
loads, a deliberately stuck worker terminated and replaced, native presenter/browser
smoke, and an independent output host displaying two frames, blanking and exiting.
Headless egui checks cover widget visibility/sizing, notes, menus and virtualized grids.
A regression prevents operator-screen loss from moving notes onto a previous output.

Formatting, warnings-denied Clippy/rustdoc, workspace tests and Python helper tests
pass (33 Rust tests and 10 Python tests). Whole-application line coverage is 84.30%,
above the 75% floor. No production code is excluded. Xvfb is unavailable here,
so native checks used the available X11 session; the existing CI wrapper still requires
Xvfb. These tests do not establish physical monitor placement or projector acceptance.

Release builds for Linux x86_64 and Windows x86_64 GNU succeed. macOS aarch64 passes
cross-target `cargo check`; this is not a macOS linked binary or runtime test.
Windows import inspection lists only OS DLLs, with no separate PDF/codec/runtime DLL.
Linux ELF imports libc, libm, libgcc_s and the loader; X11/Wayland client libraries are
loaded as platform services. Offline clean-machine checks remain a release gate.
`--licenses` embeds dependency/font notices generated by `scripts/notices.py`.

A single release-worker timing sample on a local 3-page PDF (114985 bytes,
SHA-256 `349a1307cef3f475c28ab269d9ddbebec8d88b1a2dfbab6815e876b027afc88b`):

| Operation | Worker round trip |
| --- | ---: |
| Open/identify | 5.1 ms |
| First page, 1280 edge, empty cache | 53.2 ms |
| First page refinement, 2560 edge | 114.8 ms |
| Next page, 1280 edge | 33.3 ms |
| Warm cached first page, 1280 edge | 38.7 ms |

This includes pipe transfer to the measurement process, not optical display latency.
One sample under concurrent build load is diagnostic evidence, not a p95/worst-case
benchmark or comparison against Pympress. The source PDF remains untracked and unchanged.

## Remaining acceptance and hardening

- User physical Windows/macOS/Linux X11/Wayland matrix: routing, swap, mixed DPI,
  negative coordinates, mirror groups, browser destination, hotplug and compositor
  policies. Identical monitor names currently use occurrence-based identity.
- Repeated cold/warm startup and optical input-to-visible latency, complex/large PDF
  fidelity, low memory/slow/full disk, maximum-resolution/multi-output stress.
- Idle-timeout, AC/battery, blank-state and crash-release sleep checks on each OS.
- Automatic reconnect to an orphan output host; Windows worker memory job limits;
  per-output process isolation where one display driver can stall a shared host.
- Unwritable-cache memory fallback. Since 0.1.70 disk cache maintenance supports
  size/age limits and manual expunge, preserving active decks; failures retain output.
- Explicit slide/notes-region cropping for notes-page PDFs, broader bundled CJK font
  coverage and pathological notes-layout latency. Current PDF notes are annotation
  Contents, not inferred speaker text embedded as page graphics.
- Clean-machine portable package, platform minimum-version, signing/notarization and
  macOS native build/runtime validation. No new binaries have been published remotely.

These remain visible release gates; native workflow implementation is not a claim
that the hard multiscreen/performance requirements have already been accepted.

## 0.1.13 preparation regression evidence

New tests prepare eight slides, reopen the same persistent cache without rewriting
artifacts or generating an unnecessary 512-pixel preview, then delete both source
and processed files before random navigation. Both the presenter-state test and the
independent native output-host test continue from resident frames. Additional tests
cover resolution-generation invalidation, output acknowledgment readiness, budget
pause/resume without eviction and backward-compatible settings defaults. This is
application I/O evidence, not an optical latency or OS-page-locking claim.

Validation for 0.1.13: 37 Rust tests (including native window/output tests), 10 Python
helper tests, formatting and warnings-denied Clippy/rustdoc pass. The whole-application
coverage is 84.42% (75% floor), with no production exclusions. Linux and Windows release
artifacts are rebuilt, and macOS aarch64 is compile-checked. The native presenter was
visually inspected with the local PDF and showed all three slides buffered in RAM.
Navigating the previous buffered show while a new load fails is also regression-tested.


## 0.1.14 presenter workflow

Menu is an anchored dropdown with Appearance and Preferences submenus. Displays uses
numbered screen cards and direct role selection. The independent windowed slide
browser requests topmost placement/focus; reopening restores/focuses it. Output is
enabled by default on successful document open, with a globally saved opt-out.
Explicit emergency blanking still applies, and NO OUTPUT DISPLAYED is explicit even
with a hidden footer. Appearance contains all 20 Canvas Pilot built-in palettes and
System/English/French. Additional languages and external packs remain unimplemented;
worker/OS diagnostics are preserved verbatim. This repository remains standalone.

Validation on the available Linux/X11 desktop: real pointer clicks opened All slides,
its Close control, the compact dropdown and display cards. A separate release run
loaded persisted French/Corporate Light preferences and auto_show=false, showing the
red output indicator and translated warning. The test harness had to raise its app
before synthetic pointer delivery; input sent to another foreground window was not
an application failure. Temporary preferences/cache and owned test processes were
cleaned up. This is local GUI evidence, not acceptance of the multi-OS physical matrix.

41 Rust tests, including native smoke/output and new pointer-dispatch/default-output
regressions, and 10 Python tests pass. Formatting, Clippy/rustdoc with warnings denied,
and macOS aarch64 compile-check pass. Whole-application coverage is 82.73% (75% floor),
without production exclusions. Linux and Windows x86_64 release builds succeed. Windows/macOS runtime validation
remains open.


## 0.1.15 Windows chooser correction

The reported Windows dialog was unowned because rfd ran in a worker without the
presenter's HWND. Repeated Open actions were also admitted into the four-entry worker
queue. The worker now receives the active presenter HWND and uses it as the common
dialog owner, retaining the killable-worker design. A shared outstanding-request guard
prevents queuing, and native completion restores owner input/focus even after timeout.

44 Rust tests and 10 Python tests pass. The new controlled-transport regression sends
100 alternating PDF/folder requests, verifies exactly one admission and no delayed
reopening after cancellation, then tests explicit retry, selection, worker error,
timeout and submission failure. Windows x86_64 release compilation and macOS aarch64
compile-check pass; Windows fullscreen/modal placement requires user runtime retest.
Formatting and warnings-denied Clippy/rustdoc pass. Whole-app coverage is 82.88% with
no production exclusions. An initial coverage run failed to merge a corrupted worker
profile; a fresh full run passed. Linux release is rebuilt for the shared admission fix.


## 0.1.16 output lifecycle correction

The output Close handler previously only blanked, even after the presenter pipe
closed. The native host now observes receiver disconnection and accepts Close/Escape
as exit when orphaned, while retaining crash-time last-frame preservation until then.
Normal presenter exit, event-loop errors and OutputLink destruction all shut down the
owned host. A separate shared process handle permits termination/reaping even when
the supervisor is blocked writing a large state into an unread pipe.

Regression tests fill a fake output's pipe and verify both shutdown and Drop reap it
within two seconds. Native Linux/X11 tests create real output windows, disconnect
the parent pipe, verify the process remains alive, then send WM_DELETE_WINDOW or
Escape and verify clean exit without force-kill. Test guards kill only on failure or
cleanup. These checks do not establish Windows/macOS physical acceptance.

46 Rust tests and 10 Python tests pass, including the native window/output tests.
Formatting, warnings-denied Clippy/rustdoc and macOS aarch64 compilation pass.
Whole-application coverage is 83.97%, with no production exclusions. Linux and Windows
x86_64 release builds succeed. Runtime retesting on Windows remains necessary for
the reported platform behavior.


## 0.1.19 bounded rolling buffers

Supersedes 0.1.13's memory-pressure pause: preparation now evicts distant non-active
frames while retaining processed disk artifacts. Processed-slide progress and RAM
residency are distinct; foreground navigation reloads cached slides through workers.
The displayed/requested pages remain protected, the output mirrors the working set,
and old reads cannot repopulate frames the presenter has already evicted. Output
admission accounts for a last-visible frame held outside its cache. Once preparation
finishes, speculative reloads must improve locality to avoid endless eviction cycles.
Budget increases refill RAM; decreases free non-active frames with in-flight capacity
reserved. Full-deck READY semantics are preserved, while rolling mode explicitly
reports disk reloads, actual resident count, evictions and preparation ETA.

Validation: 49 automated Rust tests, 12 Python tests, formatting and warnings-denied
Clippy/rustdoc pass; whole-app line coverage is 84.34%. New tests cover eviction,
cached reload without artifact rewrites, idle stability, budget reduction/refill,
and pinned output allocations. Native X11 output tests shrink the working set, reload
evicted entries, then delete processed files and verify the refilled buffer can still
navigate entirely from RAM. macOS aarch64 compile-check passes.

The user-authorized local `test_pdf/Ravenous.pdf` (322 pages, 60661865 bytes) was tested
at a 1920-pixel target edge and 256 MiB decoded budget using the release worker.
All 322 pages were processed; the final working set held 22 slides/about 218 MiB,
with 303 evictions. The test asserted the decoded budget throughout processing and
navigation. An evicted slide, middle, final and first page all became available;
observed request-to-presenter-frame times were 326–362 ms under concurrent background
preparation. The complete stress test took 12.29 seconds. These are worker/state
measurements, not optical audience latency or total process RSS. Worker/UI/texture
memory remains additional. The source SHA-256 was unchanged after the test.

Reproduce explicitly with an authorized local deck (never a CI/private fixture upload):

```sh
PDF_PRESENTER_STRESS_PDF=/path/to/Ravenous.pdf cargo test --locked --release \
  --test preparation real_pdf_exceeds_budget_and_keeps_navigating -- --ignored --nocapture
```

Linux and Windows x86_64 release builds both succeed.
The test uses temporary processed files and cleans them up. Real Windows/macOS
multiscreen/runtime acceptance remains separate from these regression checks.


## 0.1.21 reversible display setup

Display edits now stage both routes and fullscreen choices in a 15-second trial.
The presenter owns a monotonic deadline and restores the prior setup on timeout,
Revert, Escape or hotplug. Only Keep updates saved preferences; unrelated saves
cannot persist candidate display values. Since 0.1.51, output visibility is preserved through trials and afterward.
Navigation is held during trials; explicit blank/show actions remain available.

Startup right-click swap is available from either role until explicit navigation
or Show presentation. Opening a source alone does not end startup recovery. Pending
trials expose minimal Keep/Revert on outputs even after startup, without transferring
notes or operator controls. Output commands carry the current trial ID; stale and
late confirmations cannot commit another trial. Orphan outputs clear setup controls
and retain ordinary Close/Escape behavior.

Validation: 53 Rust tests and 12 Python tests pass, including actual egui pointer
clicks for the output context menu and Keep/Revert, hidden-bar modal rendering,
timeout/late confirmation, stale tokens, confirmed fullscreen settings and hotplug
rollback. The Linux release build succeeds. Linux native window/output smoke, formatting and warnings-denied
Clippy/rustdoc pass. Whole-app line coverage is 84.57%. These tests do not establish
physical dual-monitor acceptance; the user should retest swapping/recovery on their
setup. Windows/macOS builds and checks were not run, per the platform-build policy.

## Settings menu interaction — 0.1.22

Settings menus close on outside clicks rather than every click. Theme and language
choices use nested menus instead of independent ComboBox popups, so their input
remains part of the parent menu tree. Commands still explicitly close the menu;
Escape dismisses it. Pointer regression coverage exercises both choice menus,
Preferences controls and the three dismissal paths. Physical desktop acceptance
remains separate from these egui input tests.

## Linux browser window hints — 0.1.33

All slides is a non-modal, resizable auxiliary toplevel. Before its first rendered
buffer it sends `xdg_toplevel.set_parent` on Wayland, or `WM_TRANSIENT_FOR` plus
`_NET_WM_WINDOW_TYPE_DIALOG` on X11. A presenter on the browser's selected monitor
is preferred, otherwise the first presenter; an audience window is never the parent.
The association is refreshed if presenter windows are recreated or reassigned.
Winit 0.30.13 exposes the Wayland toplevel but not a portable transient-parent API;
short-lived typed Wayland wrappers borrow Winit's live handles without event dispatch,
roundtrips, proxy destruction or display ownership. Existing transitive Wayland crates
are direct Linux-only dependencies for this operation. Parent hints do not guarantee
compositor placement. Niri-specific runtime acceptance remains a separate check.

Validation for 0.1.33: a native Wayland smoke run on the available KWin session
passed. The protocol regression verifies that All slides sends `set_parent` to
the presenter's toplevel before attaching its first buffer, retains unlimited
maximum size, and does not parent the presenter. X11 smoke passed; external
`xprop` inspection of the owned test windows confirmed the browser's dialog type
and transient owner pointing to the same process's presenter. Full Linux tests,
formatting, warnings-denied Clippy/rustdoc and 13 Python tests pass; whole-app
X11 coverage is 86.38%. Niri is unavailable here, so this verifies hint delivery,
not Niri placement or physical multi-monitor acceptance. Windows/macOS were not
built or runtime-tested for this Linux-only change.


## Buffered page zoom — 0.1.54

Opt-in zoom crops UVs of the current cached texture, with bounded normalized panning
and no oversized draw rectangle. All views share crop state via small coalesced IPC;
page/epoch/serial checks prevent stale gestures from undoing navigation or reset.
The output host updates its mirrors locally and retries the latest crop at 30 Hz
until echoed. Full slide-buffer manifests are not resent for zoom-only updates.
Tests cover anchor math, bounds, pointer gestures, settings migration, software
pixels/unchanged texture uploads and host buffer preservation after deleting source
and cache files. The manual software-drawing measurement is run with:

```sh
cargo test --release --test zoom buffered_zoom_drawing_measurement -- --ignored --nocapture
```

This synthetic measurement excludes OS presentation/compositor and physical mirror
acceptance. Validate real pointer/trackpad input and simultaneous displays on each OS;
only Linux development checks were run for this subject.


The software renderer is bundled with a narrow cropped-quad optimization; see
[`vendor/egui_software_backend/PDF-PRESENTER-PATCH.md`](../../vendor/egui_software_backend/PDF-PRESENTER-PATCH.md)
for provenance and the precise changes. Fixed-point bilinear scanlines reuse two
filtered rows and bounded coordinate tables. Large crops use four disjoint bands
on a lazily initialized pool capped at four CPU threads per native process. This
keeps image buffers shared, avoids PDF jobs and bounds additional scratch memory
by viewport width. Font/tinted/general geometry keeps the original raster path.


Local release-mode measurement (2026-11-20, Intel Core i5-8500, six logical CPUs,
renderer pool capped at four): 1920×1080 source image, changing crop center every
frame, two warm-up frames then ten timed frames per case. Median software drawing:

| Output size | Fitted/static 1× | Panning at 4× | Panning at 8× |
| --- | ---: | ---: | ---: |
| 1920×1080 | 0.79 ms | 4.83 ms | 4.29 ms |
| 3840×2160 | 4.11 ms | 16.07 ms | 15.76 ms |

Maximum measured moving-frame time was 17.02 ms. The 1× case benefits from an
unchanged cached primitive; moving crops deliberately invalidate that cache.
These numbers include egui tessellation and software buffer drawing, exclude native
surface/compositor/monitor latency, and are not a guaranteed display frame rate.
The in-app watcher measures the actual native window draw/presentation call.


## Screen-adapting zoom — 0.1.55

The default now grows the fitted page by the zoom factor into the entire viewport,
then derives a clipped texture rectangle. Geometry never exceeds the viewport and
UVs remain within the original image. No image resampling jobs, uploads or larger
image buffers are added. The saved `zoom_keep_fitted_area` opt-in retains v0.1.54
geometry. Each output computes its own aspect-correct crop from shared scale/center;
input uses the actual visible UV span for anchoring and panning. Tests exercise
portrait and landscape pages/screens, bounded edge panning and actual software pixels.
The manual drawing benchmark now uses A4 page proportions on widescreen outputs.

Linux validation: formatting, warnings-denied Clippy/rustdoc, the full workspace
coverage suite (including native window tests), and 13 Python helper tests pass.
Whole-application line coverage is 86.25%. The release binary is rebuilt. On the
same local i5-8500, the A4-proportioned crop benchmark reports moving-frame medians
of 5.18/4.70 ms at 1080p (4×/8×) and 15.94/15.50 ms at 4K, maximum 16.87 ms.
The synthetic texture remains 1920×1080; page metadata supplies A4 proportions to
exercise viewport expansion. These are software-drawing measurements, not physical
display latency. Windows/macOS were not built or runtime-tested for this change.


## Escape and held navigation — 0.1.72

Browser Escape dismisses the preview/browser even when a control consumes keyboard
input; it preserves audience visibility. Audience Escape can revert a display trial
or close an orphan, but cannot blank a connected show. Synthetic focus-transfer key
presses are ignored. B and explicit output Close retain their blanking behavior.

A constant-size monotonic hold state ignores OS repeat frequency. Native keys tick
on the existing event loop; held mouse buttons redraw only when a repeat is due.
Single steps start after 400 ms at 100 ms intervals, then batches of five/ten at
160 ms intervals after two/five seconds (capped to 10% of the deck, minimum one).
No missed ticks are replayed. Key release, focus loss or modifier changes cancel
keyboard holds. Mouse release, leaving the button or hiding its controls cancels
mouse holds. Skipped uncached destinations defer new rendering until input settles
for 180 ms or ends; existing workers are not restarted on each repeat. Release can
cancel one obsolete foreground job so the final destination gets priority. The
last ready audience frame remains visible; RAM-ready destinations can show directly.
