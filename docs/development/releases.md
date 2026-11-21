# Manual releases

The standalone `rust_realease` tool (sibling checkout `../rust_realease`) consumes
root `rust-release.toml` and `Cross.toml`. Jenkins still only validates/publishes
badges. The release configuration selects Linux x86_64 GNU, Linux ARM64 musl,
Windows x86_64 GNU, and both Apple Silicon and Intel macOS 13+ targets. Releases are
ordinary releases below 1.0; do not pass `--prerelease` during publication.

Build local artifacts without a token or contacting Gitea:

```sh
CROSS_CONTAINER_ENGINE=podman ../rust_realease/target/release/rust_realease . \
  --package pdf-presenter --compile-only
```

Use the installed `rust_realease` command instead if preferred. Cross must have
access to its configured images. Outputs live under `dist/` (the tool prints the final release directory), including
raw executables, binstall `.tgz` archives, portable bundles, `SHA256SUMS` and an
artifact manifest. Windows portable bundles use ZIP; other targets use `.tgz`.
The binaries themselves need no companion assets. Portable bundles add the license,
quickstart and optional Python Linux launcher.
Wayland shells resolve icons through the matching installed desktop entry;
X11 and Windows also receive the embedded native window icon directly.
The Windows PE icon is embedded at build time using winresource/windres.

Keep a clean committed checkout for reusable builds. Private local test PDFs are
ignored and never included in archives. Dependency notices are embedded (`--licenses`)
and regenerated with `python3 scripts/notices.py` after dependency changes.

Before publication, run all checks in [CI](ci.md), inspect bundle contents and
checksums, and test extracted binaries on available native platforms. Cross-built
macOS artifacts require the generated release-page workaround and native Mac testing
when available. Physical multiscreen, sleep and performance acceptance remains in
[the plan](plan.md).

Publication is a separate explicit action: create the version tag at the reviewed
commit, push that commit/tag, then run:

```sh
rust_realease . --package pdf-presenter \
  --message-file docs/development/release-notes/vVERSION.md \
  --token-file ../gitea-token.txt
```

`--dry-run` previews without remote mutation. Before tagging, its missing-tag
prerequisite is expected. Do not use automatic tag/push options as part of local
preparation. Never put credentials in configuration, artifacts or Git.

## Local validation for 0.1.33

Built Linux x86_64 GNU and Windows x86_64 GNU from commit `6bba0c4` with
`CROSS_CONTAINER_ENGINE=podman rust_realease . --package pdf-presenter --compile-only`.
Artifacts are in `dist/release-oTFW9Q/`. The installed tool is 0.1.6-rc.2: it builds
raw binaries and binstall archives but does not implement the configured portable
bundles. Those two bundles were assembled from the same binaries using exactly the
file list, names and archive roots in `rust-release.toml`; SHA256SUMS covers all six
artifacts. This older tool does not emit an artifact manifest.

All six checksums and all archive member lists were verified. The extracted Linux
portable executable passed native smoke on Wayland and X11, embedded notices were
checked, and the optional launcher was installed in temporary XDG directories and
passed desktop-file-validate. Temporary extracted files were removed. Windows PE
imports list only system DLLs; Linux ELF imports libc/libm/libgcc_s/the loader.
Source validation for the included changes is recorded under 0.1.33 in the
[changelog](../changelog.md); full local Linux coverage was 86.38%.

The dependency-notice regeneration produced no changes. Private untracked PDFs
were preserved and excluded from all archives. No tag, push or remote release was
created. Windows runtime, Niri placement, clean-machine and physical multi-monitor
acceptance remain pending. See [0.1.33 release notes](release-notes/v0.1.33.md).

## Local validation for 0.1.17

Formatting, warnings-denied Clippy/rustdoc, 47 Rust tests (including native X11
icon-property checks), 12 Python tests, desktop-entry syntax validation and
macOS aarch64 compile-check pass. Whole-application line coverage is 84.05%.
The native test now tolerates BadWindow when unrelated desktop windows disappear
during enumeration; other X11 errors still fail. No production code is excluded.
Windows Explorer/taskbar appearance and Linux Wayland shell integration still
need native user confirmation.

`rust_realease --compile-only` successfully built and packaged both targets from
commit `5d12b85`. All seven checksummed files (six packages/binaries plus the
artifact manifest) verify. Inspection confirms seven Windows ICON resources plus
the GROUP_ICON and version resources, and only system DLL imports. Linux ELF imports
are libc/libm/libgcc_s/the loader. The extracted Linux portable bundle passed its
native presenter/browser smoke test and optional launcher installation in temporary
XDG directories; the resulting desktop entry passed desktop-file-validate.
No tag, push or Gitea release was created. Windows native appearance still needs user
testing; import inspection alone does not establish clean-machine acceptance.
