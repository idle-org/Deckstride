# PDF Presenter

Standalone Rust application foundation for multiscreen PDF presentation. Read
`AGENT_DOCUMENTATION.md` and `docs/development/plan.md` before implementing behavior.
The native implementation is described in `docs/development/native-implementation.md`.
The original display proof remains available through `--display-proof` for comparison.
Physical display, performance, sleep and clean-machine acceptance remain open.

## Structure and intended architecture

- `crates/pdf-presenter/src/lib.rs`: tested, UI-independent presentation state.
- `src/main.rs` in that crate: CLI/native entry point. `native.rs` owns windows, `presenter_ui.rs` controls, `studio.rs` shared state,
  `document.rs` rendering, `worker.rs` supervised jobs, `output.rs` independent host/leases,
  `settings.rs` persistence. `desktop.rs`/`poc.rs`/`paint.rs` retain the display diagnostic.
- `crates/pdf-presenter/tests/`: meaningful behavior tests, including boundary cases.
- `crates/pdf-presenter/assets/`: generated app artwork and provenance.
- `scripts/`, `tests/`, `Jenkinsfile`: standalone validation and badge publication.
- `docs/development/`: architecture decisions, implementation plan and CI setup.
- `docs/interfaces/`: persistent-format contracts when formats are introduced.

Keep the build standalone; do not depend on sibling repositories. Crate count,
compilation time and package size are not priorities. Add dependencies or split
workspace crates where runtime resilience, performance or maintainability benefits. Reliable
selectable presenter/audience displays on Windows, macOS and Linux are a HARD
requirement. Prove this first and iterate with the user on physical testing until
it works. Presentation speed is the primary performance goal: prioritize first-page
display, progressive preparation and measured cold/warm slide latency. Pre-rendered
images and a source-SHA extraction.json sidecar cache are authorized; define its
contract before implementation. Presenter mode requires current/next previews and
notes when extractable, with a current/next-only fallback. Both windows default to
fullscreen. Also support picture folders in natural filename order, matching `.txt`
notes or global `notes.json`/`notes.toml`. Always fit the full presenter notes to the
available pane using wrapping and font sizing; recompute on resize/DPI/content changes.
Folder/notes loading is implemented in the native application. Read the plan for acceptance
gates. Rust/egui is a candidate, not a constraint: native APIs, renderer processes or
another window host are acceptable when they satisfy reliability and speed. Keep PDF
rendering, document state and display/window management separate without speculative
abstractions. Do not copy Pympress or PowerPoint code/assets without license review.

Implementation follows `docs/development/tasks.md` and the decisions in
`docs/development/implementation.md`. Fast startup and responsive controls are hard
gates. Keep PDF/native blocking work in killable worker processes; never perform
blocking disk, renderer, portal or IPC waits on UI/input paths. Bound queues, memory,
retries, cancellation and shutdown. Keep output independent of presenter failures.
Automatically inhibit idle display/system sleep throughout an armed presentation,
including blank/paused states; report partial support and release owned leases at end.
Native APIs and measured OS tweaks are authorized; prefer session-scoped reversible
changes with recovery tests. Never claim immunity to OS/hardware faults.

Portable distribution is a hard release gate: one launchable executable per target,
all required non-system dependencies bundled, no runtime downloads/installers or
user-installed PDF backend/toolchain. Re-exec the same binary for internal process
roles. Prefer static linkage; embed and automatically extract unavoidable runtime
assets safely, testing cold startup and read-only executable locations. See the
implementation choices for target packaging and clean-machine acceptance.

## Presentation safety

Open source PDFs read-only. Never rewrite a document as a side effect of presenting.
Failed loads retain the working document. Bound renderer memory/cache usage; handle
malformed, encrypted and large PDFs explicitly. Keep slow rendering off the UI thread.
Audience outputs must never show operator notes, dialogs, paths or controls. Show presentation output automatically on startup/document open by default, with a
saved opt-out. Preserve manual blanking during navigation; topology changes and host
failure recovery still blank explicitly and must show an unmistakable operator warning.
Display disconnects must not move operator content onto an audience output.
Never claim physical multiscreen acceptance from headless unit tests.

## Validation and publication

Run the commands in `docs/development/ci.md`: formatting, Clippy and rustdoc with
warnings denied, workspace tests, Python helper tests, and whole-application coverage
with a 75% floor. Include future GUI/renderer code; do not hide production files from
coverage. Exercise physical displays separately on each claimed platform.

CI uses `rust_no_docker`; it archives exactly three badge SVGs and only main/non-PR
builds publish to `/mnt/assets/coverage/idle-org/pdf-presenter/main/`. No automatic
releases, binaries or manual publication. Jenkins job creation is external setup.
Never modify another tool's badges or the shared manuals root.

Document each persistent representation under `docs/interfaces/` with versions,
fields, units, validation and round trips. Schema changes need matching examples,
tests and migrations or clear unsupported-version errors. Native versioned settings/cache contracts are under `docs/interfaces/`.

## Working conventions

Inspect Git status first and preserve unrelated edits. Keep README concise; detailed
plans and acceptance records belong in docs. Update `docs/changelog.md` for each
subject and bump the workspace patch once per completed subject (initial bootstrap
is 0.1.0); keep Cargo.lock synchronized. Keep credentials, PDFs supplied for private
shows, generated binaries and caches out of Git. Every completed task changing files
ends with a descriptive commit; stage only task-owned changes. Report validation,
limitations and the commit hash. Push only when requested.
