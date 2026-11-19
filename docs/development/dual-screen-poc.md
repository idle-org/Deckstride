# Dual-screen proof of concept — 0.1.5

This is a runnable native-window test with mock slides. It opens the presenter on
the primary/default display and the audience on a distinct secondary display.
Both windows start fullscreen by default; `--windowed` disables fullscreen for both. Nothing is loaded from a PDF. Launching this specific test
explicitly activates mock output; the production document model still starts blanked.

## Run and test

Build with `cargo build --locked --release`, then run `target/release/pdf-presenter`
(or `pdf-presenter.exe` on Windows). The executable contains the mock data and bitmap
font. It needs only normal OS desktop libraries; no PDF renderer, font download,
configuration file or sibling repository is required. Each OS/architecture needs
its own build; a Linux executable is not a Windows/macOS executable.

```sh
cargo run --locked -p pdf-presenter
# Inspect display numbering without opening the two windows:
cargo run --locked -p pdf-presenter -- --list-monitors
# Explicit selection (one-based display numbers from the listing):
cargo run --locked -p pdf-presenter -- --presenter 1 --audience 2
# Keep both windows decorated for troubleshooting:
cargo run --locked -p pdf-presenter -- --windowed
```

With one detected display, fullscreen views overlap. Use `--windowed` to open
both views side-by-side for rehearsal, or Escape to leave fullscreen and blank output.
That is not a successful two-display acceptance test. With multiple displays, choosing
the same one for both roles is rejected. When the backend has no primary-display
concept (notably Wayland), the first enumerated display is the default; use explicit
selection and check the result physically.

| Control | Result |
| --- | --- |
| Right/Down/Page Down/Space | Next mock slide |
| Left/Up/Page Up | Previous mock slide |
| B or Blank button | Toggle black audience output |
| S or Swap button | Exchange presenter/audience display assignments |
| P / A or respective buttons | Cycle presenter / audience display, keeping them distinct when possible |
| F | Toggle audience fullscreen |
| V | Toggle presenter fullscreen on its selected display |
| Escape | Blank audience and return both windows to decorated mode |
| R | Re-enumerate displays and restore default assignment, keeping output blanked; B resumes |
| Q / close presenter | Exit both windows |
| Close audience | Same safe action as Escape; it does not terminate the presenter |

Buttons are clickable in the presenter only. Keyboard controls work with either
window focused. The presenter shows mock current/next slides and requested display
numbers; the audience has no operator controls or notes. A changed monitor name,
position or resolution suspends/hides output until refresh. Polling happens every
750 ms; this does not guarantee the OS cannot relocate a window before detection.

## Physical acceptance to perform

On Windows, macOS and Linux, connect two displays in extended-desktop mode. Confirm
both views start fullscreen, presenter on primary and audience on secondary. Advance,
blank/unblank, swap roles, then explicitly select each destination. Verify focus,
Escape, return to fullscreen, and exit. Repeat with mixed DPI and portrait/negative
coordinates. Unplug/replug the secondary display and test R to recover. Record OS,
window backend, monitor layout, chosen numbers, observed placement and failures.

Wayland does not generally allow arbitrary window positioning. This proof requests
selected-monitor fullscreen; V also allows testing that route for the presenter.
Windowed placement may still require compositor-assisted dragging. A compositor that
ignores the requested destination is an open hard-requirement defect, not acceptance.
macOS fullscreen/Spaces behavior must also be tested on hardware.

## Deliberate limits of this proof

The narrow scope is two native windows and display control. It uses winit and
softbuffer, and both windows currently share one event loop/process. There is no
PDF rendering, worker isolation, GPU fallback, notes extraction, persistence, sleep
inhibition or production recovery watchdog. The [implementation choices](implementation.md)
remain the target architecture; this test is not a claim to have completed it.
The bitmap UI is test instrumentation, not the final presenter design. There is a
40-megapixel software-surface allocation limit; drawing failure exits with an error.

## Automated evidence

Tests cover monitor assignment/invalid selection, navigation bounds, blanking,
topology invalidation, negative-coordinate geometry, scaled pointer hit testing and
software frames. `PDF_PRESENTER_GUI_TESTS=1 cargo test --locked --workspace` also opens
real native windows, exercises mock controls/fullscreen transitions and checks frames
were presented to both surfaces. `--smoke-test` exposes that short automatic check.
A virtual display proves native rendering/lifecycle only, not physical placement.

Initial local evidence: Linux/X11, one DisplayPort monitor at 1920x1080. Both native
windows rendered, fullscreen transitions completed, and window screenshots were
visually inspected. Actual two-monitor placement, Windows/macOS runtime behavior and
Wayland runtime behavior remain unverified. See CI for virtual-display requirements.

Validation for this change: 14 Rust tests (including the desktop smoke), 10 Python
helper tests, formatting, Clippy and rustdoc passed. Whole-application coverage was
89.13%, with no GUI exclusions. Compile checks passed for x86_64 Windows GNU,
aarch64 macOS and x86_64 FreeBSD; these are not native execution evidence. The Linux
release executable was built and inspected for linked runtime dependencies. It uses
normal system C/runtime and desktop libraries; compatibility with older distributions
has not been established. The Xvfb wrapper has lifecycle/error tests; an actual Xvfb
run was unavailable locally, so Jenkins still needs its new Xvfb prerequisite installed.
The Windows proof retains a console for display-list and startup diagnostics.


## Linux transition follow-up — 0.1.4

The user reports smooth Windows swap/fullscreen operation and functioning but messy
Linux transitions. The X11 path now applies only changed window assignments, exits
fullscreen, allows 120 ms for the window manager to restore geometry, then moves and
resizes. Fullscreen entry follows another 120 ms later. These are event-loop deadlines,
not sleeps: slide and blank controls remain responsive, and a newer request replaces
pending work. This is a small settling-time workaround to retest, not compositor
acknowledgement or a guarantee for every window manager. Windows and Wayland placement
are unchanged. No new dependencies, window framework or system settings are involved.
