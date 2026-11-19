# PDF Presenter

Standalone Rust application foundation for multiscreen PDF presentation. Read
`AGENT_DOCUMENTATION.md` and `docs/development/plan.md` before implementing behavior.
The current executable reports bootstrap status; do not describe it as a PDF viewer.

## Structure and intended architecture

- `crates/pdf-presenter/src/lib.rs`: tested, UI-independent presentation state.
- `src/main.rs` in that crate: executable entry point; future desktop host.
- `crates/pdf-presenter/tests/`: meaningful behavior tests, including boundary cases.
- `crates/pdf-presenter/assets/`: generated app artwork and provenance.
- `scripts/`, `tests/`, `Jenkinsfile`: standalone validation and badge publication.
- `docs/development/`: architecture decisions, implementation plan and CI setup.
- `docs/interfaces/`: persistent-format contracts when formats are introduced.

Keep a single-member workspace. Do not depend on sibling repositories. Rust/egui
is the starting desktop direction, following Canvas Pilot; validate multiwindow
behavior and renderer/platform support before choosing dependencies. Keep PDF
rendering, document state and display/window management separate without speculative
abstractions. Do not copy Pympress or PowerPoint code/assets without license review.

## Presentation safety

Open source PDFs read-only. Never rewrite a document as a side effect of presenting.
Failed loads retain the working document. Bound renderer memory/cache usage; handle
malformed, encrypted and large PDFs explicitly. Keep slow rendering off the UI thread.
Audience outputs must never show operator notes, dialogs, paths or controls. Require
explicit output activation, start blanked, and preserve blanking during navigation.
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
tests and migrations or clear unsupported-version errors. There is no saved format yet.

## Working conventions

Inspect Git status first and preserve unrelated edits. Keep README concise; detailed
plans and acceptance records belong in docs. Update `docs/changelog.md` for each
subject and bump the workspace patch once per completed subject (initial bootstrap
is 0.1.0); keep Cargo.lock synchronized. Keep credentials, PDFs supplied for private
shows, generated binaries and caches out of Git. Every completed task changing files
ends with a descriptive commit; stage only task-owned changes. Report validation,
limitations and the commit hash. Push only when requested.
