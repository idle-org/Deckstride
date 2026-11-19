# Implementation choices

Status: implementation direction selected 2026-11-19; no desktop/PDF implementation
exists yet. The [task list](tasks.md) owns execution order and evidence. Choices below
are engineering decisions to validate, not measured claims about an existing app.

## Optimization priorities

Correct dual-screen output, fast startup, bounded input latency and resilience are
release requirements. Dependency count, compilation time and binary/package size
are not optimization goals. Add crates, native code and bundled dependencies when
they improve runtime behavior. Use separate workspace crates where they establish
useful ownership or testing boundaries; process boundaries matter more than crate
count. Do not delay the first visible frame for an update check, network request,
full-deck scan, full hash, notes extraction, cache cleanup or renderer warmup.

## Portable distribution is a hard requirement

Ship one launchable executable per supported OS/architecture, carrying every required
non-system dependency. No installed Rust/Python/Node, system PDFium, codec package,
package-manager step, network access or first-run download may be required. Native OS
libraries, window services and graphics drivers are platform prerequisites and their
minimum supported versions must be documented. Build tools do not belong on user
machines. Copy-and-run from an arbitrary directory must work without installation.

Prefer statically linked PDFium and other runtime libraries. Where static linking is
not viable on a target, embed the required library/assets in the executable and
extract automatically to a versioned writable user-cache directory using atomic
writes and integrity validation. Load only the explicit extracted path, never an
uncontrolled current-directory library. A read-only executable directory must work.
Extraction must not scan/download dependencies or delay shell startup; benchmark
first launch with an empty cache as well as subsequent launches. Report an unwritable
cache as an actionable failure. Do not claim strict in-memory single-file execution
if embedded runtime materialization is used.

Bundle required fonts, icons and native-library notices/resources; document optional
system-font substitutions and test a minimal machine. Preserve all license obligations.
Use the same executable's internal audience/worker modes for process isolation, via
an absolute executable path and private IPC bootstrap token. Internal modes are not
a user setup step. Workers must run on offline clean machines with the same bundle.
macOS may additionally ship a conventional signed .app wrapper for normal desktop
launching; it must contain everything and require no installer. A wrapper does not
justify fetching or depending on separately installed libraries.

## Chosen starting stack

| Area | Choice | Reason and proof required |
| --- | --- | --- |
| Native windows | Rust + winit, with native platform adapters | Explicit window/display lifecycle; prove both display roles on each OS before building the rest. |
| Presenter controls | egui through egui-winit and egui-wgpu | Practical UI; retain direct ownership of the event loop and window management. |
| Audience drawing | Minimal wgpu texture compositor in its own process | No UI, PDF interpreter, notes or file picker on the audience path; independent from presenter failures. |
| GPU fallback | softbuffer CPU image presentation | Predecoded slide buffers remain usable without successful GPU initialization; validate separately on every OS. |
| PDF rasterization | Bundled, pinned PDFium through pdfium-render, isolated workers | Consistent backend across OSes, replaceable workers and no runtime dependency downloads. |
| Work scheduling | Bounded process pool, bounded channels, explicit priorities/deadlines | Never let background extraction starve an interactive slide request. |
| Durable page cache | Lossless PNG baseline, versioned JSON manifest, SHA-256 source identity | Portable artifacts; benchmark LZ4/raw BGRA alternatives before choosing faster storage. |
| Hot cache | Decoded CPU buffers plus GPU textures for current/next/previous | Prepared navigation performs no PDF rendering, image decode or file reads. |
| Platform integration | windows bindings, objc2/IOKit bindings, Linux D-Bus via zbus/ashpd | Direct control and observable errors; version-pin a compatible set during implementation. |
| Measurement | tracing with bounded asynchronous logs and monotonic timestamps | Separate startup, queue, render, decode, upload and visible-frame measurements. |

Exact compatible crate versions and PDFium binary checksums belong in lockfiles and
packaging manifests when introduced. More dependencies are acceptable; do not select
an old or incompatible release merely to satisfy this document. Build PDFium for each
promised target or use verified pinned artifacts with notices. Keep JS/XFA features
disabled unless a demonstrated PDF requirement justifies them.

winit documents platform differences in window positioning and fullscreen. Treat its
API as an implementation tool, not proof that all compositors honor placement. On
Wayland validate selected-output fullscreen and presenter assignment separately; a
manual placement/reassignment flow must actually meet the user workflow on tested
compositors. Do not silently mark the hard requirement complete after testing only
X11. [winit window API](https://docs.rs/winit/latest/winit/window/struct.Window.html)

PDFium calls cannot safely be parallelized merely by adding threads: pdfium-render's
thread-safe mode serializes access. Use one PDFium instance per worker process and
one PDFium call at a time per worker. Progressive PDFium rendering may yield useful
partial work, but cannot replace a process deadline for all parsing/render failures.
[pdfium-render](https://docs.rs/pdfium-render/latest/pdfium_render/),
[PDFium progressive API](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/fpdf_progressive.h)

## Process ownership and fault containment

Start with one workspace package producing a single executable with presenter,
audience-host and PDF-worker modes; split crates when implementation benefits. Do not
build an abstract plugin framework. The presenter opens immediately; PDF/GPU workers
start asynchronously. The audience host owns its native event loop, slide buffers,
output assignment, local emergency blank/escape controls and presentation sleep guard.

The audience host owns the last successfully displayed page and acknowledges its
generation/page/quality to the presenter. Distinguish requested, prepared and displayed
state. The presenter must not label an unacknowledged slide as already on air. Include
document generation and request sequence on every reply; discard stale results.
If the presenter exits or stalls, the audience holds the last confirmed slide or
operator-selected blank state and keeps its emergency controls and inhibition alive.
Reconnecting the presenter must read audience state before issuing new commands.

The presenter supervises audience and worker processes using nonblocking IPC and
bounded queues. A health response must reflect actual event-loop progress; a heartbeat
thread alone cannot prove the UI is responsive. Never interpret a long PDF task as a
healthy worker merely because its IPC thread still answers. Track job progress and
deadlines separately. An unresponsive audience process is restarted with a safe blank
surface, then restored only after its display identity and cached frame are verified.
A restart may create a visible interruption; record it as a failure, not seamless
recovery. A full GPU/compositor/OS hang cannot be guaranteed recoverable by this app.

Control messages use a versioned, length-bounded local IPC protocol. Transfer large
frames through bounded shared-memory slots or mapped temporary artifacts, not JSON
base64 or control queues. Validate dimensions, stride, byte length, format and slot
lifetime before use. Acknowledge ownership before reusing buffers. Never allow a
slow reader to block presentation control. Define and test the protocol before use.

Renderer crashes, deadlocks and memory exhaustion kill only the worker. Limit process
memory where available (Windows Job Objects, Unix resource limits where applicable),
validate allocation sizes everywhere, cap the pool by available memory and CPU, and
reserve capacity for foreground work. Start with one foreground worker and at most
two background workers; tune from measurements. On cancellation ask politely first,
then terminate/reap an overdue worker and replace it. Do not attempt to kill a Rust
thread stuck inside a native PDF call. Limit retries per page/document, back off and
surface persistent failure in the presenter; avoid restart storms.

Use separate foreground and background queues. Priority order: emergency controls,
requested slide, next slide, previous slide, other previews, notes, distant slides.
Reserve upload/decode budgets so a large texture cannot monopolize a frame. Coalesce
obsolete navigation requests but preserve emergency blanking and the final requested
page. Keep the existing audience frame while preparation runs; explicitly show the
pending request in the presenter. Never show an unrelated placeholder as a slide.

## Startup and image pipeline

Show the presenter shell without synchronously initializing PDFium or enumerating
files. Start monitor discovery/power services with bounded operations. Initialize
GPU work without blocking the shell; exercise a CPU-first startup path if GPU startup
violates budget. softbuffer offers direct software image display; its suitability
for this app's resolution and latency is still to be measured.
[softbuffer documentation](https://docs.rs/softbuffer/latest/softbuffer/)

Render the requested page at a useful preview resolution first, then refine it to
the target display resolution in the background. Replace the image atomically, keep
aspect ratio and crop/rotation fixed, and never delay a ready preview for a full-deck
operation. Notes and thumbnail generation have lower priority. Large source reads,
slow disks and network-mounted PDFs must not stall either window: isolate I/O with
workers, offer cancellation and copy to local storage progressively when helpful.
Opening some PDFs may require seeking or a complete read; first-page speed remains
a measured goal, not a promise that every damaged or remote PDF opens instantly.

Persist lossless prepared pages and reuse them only after source/render-settings
validation. Full source hashing runs independently; until complete, render the current
source rather than display unverified stale cache. Immutable memory-mapped blobs or
shared buffers may reduce copies after measurement. Keep bounded CPU and GPU LRUs;
pin the visible slide and immediate neighbors. Background extraction pauses under
memory pressure or foreground demand. Persist cache entries atomically, resume partial
extractions and regenerate corrupt entries. See the [cache requirements](../interfaces/extraction-cache.md).

wgpu surfaces require handling acquisition/configuration and device failure. Do not
spin/retry forever or recreate a GPU device on every frame. Attempt bounded recovery,
then switch to the validated CPU output path. A blocked driver call may require
restarting the audience host; process isolation reduces the blast radius, not kernel
failure risk. [wgpu surface API](https://docs.rs/wgpu/latest/wgpu/struct.Surface.html)

## Keep-awake policy

Acquire display-idle and system-idle inhibition automatically when presentation is
armed, before activating audience output. Hold it through blanking, paused navigation,
notes reading and recovery; a deliberately black slide is still an active presentation.
Release on explicit End Presentation or host exit. The audience host owns the live
lease so a presenter crash does not drop it. Rehearsal may opt in separately. Show
acquired/partial/unavailable status only in the presenter and reacquire after wake,
session/backend restart or lost connection. Every system call/IPC operation needs a
bounded asynchronous path; a broken power service must not freeze the presentation.

| OS | Implementation | Validation and limits |
| --- | --- | --- |
| Windows | Dedicated owner thread calls SetThreadExecutionState with ES_CONTINUOUS, ES_DISPLAY_REQUIRED and ES_SYSTEM_REQUIRED; clear on the same thread. Handle SC_SCREENSAVE/SC_MONITORPOWER in native window integration where allowed during the session. | Execution-state requests do not themselves stop screensavers. Verify both idle display/system sleep and screensaver behavior; managed security policy or explicit sleep may override. |
| macOS | IOPMAssertionCreateWithName display/system idle-sleep assertions, scoped ownership and explicit release. Observe sleep/wake; add a scoped NSProcessInfo activity for App Nap behavior if measured necessary. | Test visible and blanked output and battery/AC. Forced sleep, lid closure, thermal or low-battery intervention cannot be promised away. |
| Linux | XDG desktop portal Inhibit with idle+suspend flags through an async request. If unavailable, use supported session ScreenSaver inhibition for idle and logind sleep inhibition for suspend. Track each lease separately. | Portal backends vary. A logind sleep lock alone is not proof of screen-idle inhibition. Validate GNOME/KDE on Wayland/X11; add compositor/native adapters only where a tested gap requires them. |

These API choices follow [Microsoft execution-state documentation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate),
[Windows system commands](https://learn.microsoft.com/en-us/windows/win32/menurc/wm-syscommand),
[Apple sleep assertions](https://developer.apple.com/library/archive/qa/qa1340/_index.html),
[XDG Inhibit](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Inhibit.html),
[session idle inhibition](https://specifications.freedesktop.org/idle-inhibit/latest/),
and [logind inhibitors](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html).

Prefer process/session-scoped requests over permanent OS changes. Additional tuning
is authorized when useful: benchmark foreground scheduling, background I/O priority,
GPU adapter selection and compositor-specific integration. Keep any configuration
change explicit, reversible, recorded and limited to the presentation. Avoid realtime
scheduling that can starve input or the compositor. Do not blindly disable system
services, security locks, updates or global power settings: that is not evidence of
resilience. A further invasive tweak needs a concrete reproduced problem and a tested
restore path, not a speculative performance claim.

## Initial performance and recovery budgets

These are proposed engineering gates to measure on declared reference hardware and
representative local decks, not user-approved measurements or universal PDF guarantees.
Record p50/p95/worst, cold/warm OS cache, app cache state, resolution and deck identity.

| Operation | Initial target |
| --- | --- |
| Launch to interactive presenter shell | p95 <= 500 ms; renderer/cache scans are excluded from the critical path, not from reported end-to-end startup. |
| Cold local PDF to usable first page | p95 <= 2 s on the baseline corpus; report outliers and renderer failures separately. |
| Warm prepared adjacent-slide navigation | p95 <= 50 ms input-to-visible at 60 Hz; also report input-to-submit and refresh rate. |
| Blank/unblank with prepared surface | p95 <= 50 ms even under render/cache stress. |
| UI event work | No application-controlled blocking I/O or waits; aim for <= 4 ms handler slices, report stalls > 50 ms. |
| Stuck foreground render | Progress deadline initially 2 s; cancel/restart independently, keep controls live; complexity may require a visible longer render attempt. |
| Dead audience host | Detect missing event-loop progress within 1 s under stress; attempt blanked recovery within 2 s on reference hardware. |
| End presentation | Release sleep leases promptly; no wait for full-deck extraction or cache completion. |

Do not improve reported latency by hiding dropped requests, stale slides, failed opens
or recovery interruptions. When a target fails, retain the evidence and fix the cause
or explicitly review the target with the user. No silent relaxation. Validate long
soak behavior and fault injection as seriously as the fast path.
