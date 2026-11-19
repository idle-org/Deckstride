# Manual preview releases

The standalone `rust_realease` tool (sibling checkout `../rust_realease`) consumes
root `rust-release.toml` and `Cross.toml`. Jenkins still only validates/publishes
badges. The release configuration selects Linux x86_64 GNU and Windows x86_64 GNU.
macOS remains an implementation target, but is not advertised as a prepared release
artifact until a linked native build and runtime acceptance are available.

Build local artifacts without a token or contacting Gitea:

```sh
CROSS_CONTAINER_ENGINE=podman ../rust_realease/target/release/rust_realease . \
  --package pdf-presenter --compile-only
```

Use the installed `rust_realease` command instead if preferred. Cross must have
access to its configured images. Outputs live under `dist/release-*/`, including
raw executables, binstall `.tgz` archives, a Linux portable `.tgz`, a Windows
portable `.zip`, and `SHA256SUMS`. The binaries themselves need no companion assets.
Portable bundles add the license, quickstart and optional Python Linux launcher.
Wayland shells resolve icons through the matching installed desktop entry;
X11 and Windows also receive the embedded native window icon directly.
The Windows PE icon is embedded at build time using winresource/windres.

Keep a clean committed checkout for reusable builds. Private local test PDFs are
ignored and never included in archives. Dependency notices are embedded (`--licenses`)
and regenerated with `python3 scripts/notices.py` after dependency changes.

Before publication, run all checks in [CI](ci.md), inspect bundle contents and
checksums, and test extracted binaries on clean Windows/Linux machines. Physical
multiscreen, sleep and performance acceptance remains in [the plan](plan.md).
Version 0.1.17 is prepared as a **preview**, not a production-readiness claim.

Publication is a separate explicit action: create local `v0.1.17` at the reviewed
commit, push that commit/tag, then run:

```sh
rust_realease . --package pdf-presenter --prerelease \
  --message-file docs/development/release-notes/v0.1.17.md \
  --token-file ../gitea-token.txt
```

`--dry-run` previews without remote mutation. Before tagging, its missing-tag
prerequisite is expected. Do not use automatic tag/push options as part of local
preparation. Never put credentials in configuration, artifacts or Git.

## Local validation for 0.1.17

Formatting, warnings-denied Clippy/rustdoc, 47 Rust tests (including native X11
icon-property checks), 12 Python tests, desktop-entry syntax validation and
macOS aarch64 compile-check pass. Whole-application line coverage is 84.05%.
The native test now tolerates BadWindow when unrelated desktop windows disappear
during enumeration; other X11 errors still fail. No production code is excluded.
Windows Explorer/taskbar appearance and Linux Wayland shell integration still
need native user confirmation.
