<p align="center">
  <img src="crates/pdf-presenter/assets/app-icon.png" width="96" height="96" alt="PDF Presenter icon">
</p>
<h1 align="center">PDF Presenter</h1>
<p align="center"><strong>Your slides on the big screen. Your notes and controls with you.</strong></p>

<p align="center">
  <a href="docs/changelog.md"><img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/version.svg" alt="Version"></a>
  <a href="docs/development/ci.md"><img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/test.svg" alt="Tests"></a>
  <a href="docs/development/ci.md"><img src="https://assets.dandaedre.com/coverage/idle-org/pdf-presenter/main/coverage.svg" alt="Coverage"></a>
</p>

<p align="center">
  <a href="https://proxy.dandaedre.com/gitea/idle-org/pdf-presenter/releases"><strong>Download a release</strong></a> ·
  <a href="docs/user-guide.md#install-and-update">Install and update</a> ·
  <a href="docs/user-guide.md">User guide</a> ·
  <a href="https://proxy.dandaedre.com/gitea/idle-org/pdf-presenter/issues">Report an issue</a>
</p>

PDF Presenter is a desktop presenter for **PDFs and picture folders**. Show slides
fullscreen on your presentation screens while keeping previews, notes, timers and
navigation on your presenter screen. Rehearse on one monitor, or mirror either view
across several displays.

## Download, extract, present

Start with the [release downloads](https://proxy.dandaedre.com/gitea/idle-org/pdf-presenter/releases).
Choose a **portable** package for your computer; no Rust toolchain or separate PDF
renderer is needed.

| Your computer | Download | Run after extracting |
| --- | --- | --- |
| Windows x86_64 | `pdf-presenter-portable-x86_64-pc-windows-gnu-v….zip` | `pdf-presenter.exe` |
| Linux x86_64 | `pdf-presenter-portable-x86_64-unknown-linux-gnu-v….tgz` | `./pdf-presenter` |

Releases are currently **previews**. Windows binaries are unsigned; macOS downloads
are not yet available. Test your display setup before using it for a live event.
[Installation, updates and optional Linux app-menu integration →](docs/user-guide.md#install-and-update)

## Set up your presentation

1. **Open your slides.** Choose Menu → Open PDF or Open picture folder, or drop your
   file/folder onto the presenter. Pictures follow natural filename order.
2. **Choose your screens.** In Displays, assign Presenter or Presentation to each
   monitor. Try the arrangement, then keep it or let it revert automatically.
3. **Make room for what matters.** In Layout, resize or hide previews and notes,
   place the timers, and optionally enable slide progress. Save for this project
   or all projects.
4. **Present.** Use arrows or Space to advance. Type a slide number and press Enter
   to jump. All slides opens a thumbnail browser; B blanks or restores the output.

Opening a document shows the presentation by default. After changing displays,
use **Show presentation** to restore output. Hide the bars for more space; move the
pointer near an edge or right-click to recover them.

Prepared slides are cached for reuse. The presenter shows loading progress, RAM use
and an ETA when available; decks larger than the RAM budget use a rolling buffer.
Notes can come from the PDF or picture-folder sidecars. Themes and English/French
controls are available in Menu.

[**Read the user guide →**](docs/user-guide.md)

---

[Development and source builds](docs/development/README.md) ·
[Changelog](docs/changelog.md) · [MIT license](LICENSE) ·
[More beffroi tools](https://proxy.dandaedre.com/gitea/Beffroi/beffroi-tools)
