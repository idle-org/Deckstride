<h1 align="center">PDF Presenter</h1>
<p align="center"><img src="crates/pdf-presenter/assets/app-icon.png" width="160" height="160" alt="PDF Presenter icon"></p>
<p align="center">A standalone beffroi tool for multiscreen PDF presentations.</p>

<p align="center">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/version.svg" alt="Current version">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/test.svg" alt="Test status">
  <img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/coverage.svg" alt="Line coverage">
</p>

**Native presenter:** open PDFs or naturally ordered picture folders, with configurable
presenter previews/notes, a safe slide browser and mirrored display roles. Outputs
show automatically when a document opens (saved opt-out in Menu). Both roles
default to fullscreen; a single monitor uses presenter-only rehearsal.

```sh
cargo run --locked --release -p pdf-presenter -- --open /path/to/slides.pdf
# Rehearse in a window:
cargo run --locked --release -p pdf-presenter -- --windowed /path/to/pictures
```

Use **Displays** to assign screens, **Layout** to resize/hide widgets and save layouts,
and **All slides** to browse. Type a slide number then Enter to jump; B blanks.
Right-click or approach an edge to recover hidden menus.
[User guide](docs/user-guide.md) · [Native implementation and acceptance](docs/development/native-implementation.md).

Windows/macOS/Linux physical display, sleep and performance acceptance remains open.
PDF rendering and codecs are bundled; no external PDF backend or runtime download.

Builds independently; no sibling beffroi-tools or Canvas Pilot checkout is required.

[Development and CI](docs/development/README.md) · [Implementation plan](docs/development/plan.md) ·
[Agent guide](AGENTS.md) · [Changelog](docs/changelog.md) ·
[beffroi-tools](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools)
