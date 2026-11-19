<h1 align="center">PDF Presenter</h1>
<p align="center"><img src="crates/pdf-presenter/assets/app-icon.png" width="160" height="160" alt="PDF Presenter icon"></p>
<p align="center">A standalone beffroi tool for multiscreen PDF presentations.</p>

<p align="center">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/version.svg" alt="Current version">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/test.svg" alt="Test status">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/coverage.svg" alt="Line coverage">
</p>

**Dual-screen proof of concept:** native presenter controls and fullscreen mock
slides on the secondary display. With one display it opens two windowed rehearsal
views. PDF loading, notes, sleep inhibition and production fault isolation are not
implemented yet.

```sh
cargo run --locked -p pdf-presenter
# Or choose display numbers explicitly:
cargo run --locked -p pdf-presenter -- --presenter 1 --audience 2
```

**S** swaps displays, **B** blanks, **F** toggles audience fullscreen, **Escape**
returns to safe windowed mode, and **Q** quits.
[Run the dual-screen test](docs/development/dual-screen-poc.md).

The goal remains a fast, resilient Pympress-style presenter on Windows, macOS and
Linux, with selectable displays and portable executables bundling all non-system
dependencies. This first proof needs physical testing on each OS.

Builds independently; no sibling beffroi-tools or Canvas Pilot checkout is required.

[Development and CI](docs/development/README.md) · [Implementation plan](docs/development/plan.md) ·
[Agent guide](AGENTS.md) · [Changelog](docs/changelog.md) ·
[beffroi-tools](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools)
