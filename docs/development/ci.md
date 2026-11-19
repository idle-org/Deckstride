# CI and Jenkins setup

Create a Jenkins Multibranch Pipeline for `idle-org/pdf-presenter`, using the root
`Jenkinsfile`, SCM credentials and the existing `rust_no_docker` worker. Repository
upload does not create a Jenkins job. Configure branch discovery and a Gitea webhook
in Jenkins/Gitea when connecting the job.

The worker needs Rust, rustup, Python 3.11+, POSIX shell/awk/sed and a writable
`/mnt/assets/coverage`. The pipeline installs cargo-llvm-cov 0.9.1 and the rustfmt,
Clippy and llvm-tools-preview components. It builds and tests without a sibling
checkout, display server or PDF renderer.

```sh
python3 -m unittest discover -s tests -v
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo build --locked --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
python3 scripts/ci.py coverage
```

Coverage measures the entire Rust workspace with a 75% floor. Add all future
production GUI and renderer code to this same gate. Badge helpers are derived from
Canvas Pilot and have tests for invalid/stale reports and isolated publication.

Cleanup archives exactly `coverage/version.svg`, `coverage/test.svg` and
`coverage/coverage.svg`. Only main builds without CHANGE_ID publish badges under
`/mnt/assets/coverage/idle-org/pdf-presenter/main/`, including honest failed-test
or unavailable-coverage status. No shared asset root is synchronized or deleted.
There is no manual publisher, release job or binary upload in this pipeline.
The Jenkinsfile still requires a real Jenkins run; local tests cannot validate
worker configuration or prove the remote pipeline has run.
