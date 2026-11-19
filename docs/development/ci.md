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

Jenkins runs headless tests and coverage without Xvfb. Native-window smoke tests
remain available locally but are not enabled in Jenkins; egui rendering, document,
worker and other headless tests still run. The pipeline installs cargo-llvm-cov 0.9.1
and the rustfmt, Clippy and llvm-tools-preview components.

The temporary Jenkins coverage floor is **60%**, selected explicitly by
`PDF_PRESENTER_HEADLESS_CI=1` and `headless-line-floor` in `coverage.toml`. The normal
local full-GUI floor remains **75%**. Both measure all production code, including
GUI code; tool/test failures still fail the build and badges report the actual
measurement. Restore full native testing and the 75% Jenkins floor when a display
server becomes available. Headless coverage measured 65.21% for 0.1.26; this is a
snapshot, not physical-display acceptance.

```sh
uv run --no-project --python ">=3.11" python -m unittest discover -s tests -v
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo build --locked --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
PDF_PRESENTER_GUI_TESTS=1 uv run --no-project --python ">=3.11" python scripts/ci.py coverage
```

Coverage measures the entire Rust workspace with the selected floor. Add all future
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


The coverage command above needs a usable desktop session. On a headless POSIX host with Xvfb installed,
run `uv run --no-project --python ">=3.11" python scripts/gui_ci.py python scripts/ci.py coverage` instead. To include
native smoke in local workspace tests, set `PDF_PRESENTER_GUI_TESTS=1`. Without it,
the ordinary tests remain headless and explicitly skip native-window smoke. CI uses
neither the flag nor the virtual-display wrapper. Xvfb tests cannot
validate actual monitor routing, compositor policy, macOS Spaces or Windows desktop
behavior. The wrapper starts no network listener and alters no physical display.
