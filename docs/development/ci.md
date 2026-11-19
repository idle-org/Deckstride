# CI and Jenkins setup

Create a Jenkins Multibranch Pipeline for `idle-org/pdf-presenter`, using the root
`Jenkinsfile`, SCM credentials and the existing `rust_no_docker` worker. Repository
upload does not create a Jenkins job. Configure branch discovery and a Gitea webhook
in Jenkins/Gitea when connecting the job.

The worker needs Rust, rustup, uv, POSIX shell/awk/sed and a writable
`/mnt/assets/coverage`. Every Jenkins Python invocation uses
`uv run --no-project --python ">=3.11" python`, including initial/final badges and
coverage. uv selects a compatible installed Python or downloads one where supported;
no system `python3` alias or project virtual environment is required. On workers
without a supported uv-managed Python build (including FreeBSD), provision Python
3.11+ for uv to discover.

Native GUI CI additionally requires Xvfb and ordinary X11
client libraries (on FreeBSD install `xorg-vfbserver` and the required X11 packages). The pipeline installs cargo-llvm-cov 0.9.1 and the rustfmt,
Clippy and llvm-tools-preview components. It builds and tests without a sibling
checkout or PDF renderer. `scripts/gui_ci.py` starts an isolated Xvfb server for
tests/coverage, waits at most 10 seconds for readiness and always cleans it up.
It enables the native smoke integration test. `uv` does not supply Xvfb: an
administrator must provision the Jenkins worker (`apt-get install xvfb` on
Debian/Ubuntu, or `pkg install xorg-vfbserver` on FreeBSD) and expose it on PATH.
A failure at `command -v Xvfb` means that prerequisite is missing, not a Python
version error. Missing Xvfb fails with provisioning guidance; GUI
production code is never excluded from coverage.

```sh
uv run --no-project --python ">=3.11" python -m unittest discover -s tests -v
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo build --locked --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
PDF_PRESENTER_GUI_TESTS=1 uv run --no-project --python ">=3.11" python scripts/ci.py coverage
```

Coverage measures the entire Rust workspace with a 75% floor. Add all future
production GUI and renderer code to this same gate. Badge helpers are derived from
Canvas Pilot and have tests for invalid/stale reports and isolated publication.

Cleanup archives exactly `coverage/version.svg`, `coverage/test.svg` and
`coverage/coverage.svg`. Only main builds without CHANGE_ID publish badges under
`/mnt/assets/coverage/idle-org/pdf-presenter/main/`, including honest failed-test
or unavailable-coverage status. No shared asset root is synchronized or deleted.
There is no release job or binary upload in this pipeline.
Separate local packaging uses [rust_realease](releases.md).
The Jenkinsfile still requires a real Jenkins run; local tests cannot validate
worker configuration or prove the remote pipeline has run.


The coverage command above needs a usable desktop session. On a headless POSIX host,
run `uv run --no-project --python ">=3.11" python scripts/gui_ci.py python scripts/ci.py coverage` instead. To include
native smoke in local workspace tests, set `PDF_PRESENTER_GUI_TESTS=1`. Without it,
the ordinary tests remain headless and explicitly skip native-window smoke. CI sets
it through the virtual-display wrapper for both tests and coverage. Xvfb tests cannot
validate actual monitor routing, compositor policy, macOS Spaces or Windows desktop
behavior. The wrapper starts no network listener and alters no physical display.
