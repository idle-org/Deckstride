<h1 align="center">PDF Presenter</h1>
<p align="center"><img src="crates/pdf-presenter/assets/app-icon.png" width="160" height="160" alt="PDF Presenter icon"></p>
<p align="center">A standalone beffroi tool for multiscreen PDF presentations.</p>

<p align="center">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/version.svg" alt="Current version">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/test.svg" alt="Test status">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/coverage.svg" alt="Line coverage">
</p>

**Repository foundation — not yet a usable PDF presenter.** The goal is a blazingly fast Pympress-style PDF presenter: selectable presenter
and audience displays on Windows, macOS and Linux, progressive rendering with
reusable image caches, and current/next previews with notes when extractable.
Reliable dual-screen operation on all three OSes is a hard requirement. Releases
will be portable executables with all non-system dependencies bundled. It is independent of those projects.

The initial Rust core tests page navigation and audience blanking. PDF loading,
rendering, presenter UI and display selection are planned. Build status badges
become available after the first Jenkins publication.

```sh
cargo run --locked -p pdf-presenter
cargo test --locked --workspace
```

Builds independently; no sibling beffroi-tools or Canvas Pilot checkout is required.

[Development and CI](docs/development/README.md) · [Implementation plan](docs/development/plan.md) ·
[Agent guide](AGENTS.md) · [Changelog](docs/changelog.md) ·
[beffroi-tools](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools)
