# Presentation notes sidecars — version 1

Implemented in 0.1.52. A PDF `presentationX.pdf` uses
`presentationX.notes.toml` in the same directory. Image folder `Slides/` uses
`Slides.notes.toml` beside that folder. Paths follow the canonical source location.
Opening a presentation never creates or writes a sidecar. The PDF, image files,
legacy image `.txt` sidecars and folder-global notes are always left untouched.

```toml
version = 1

[notes]
"1" = "Opening remarks"
"2" = "" # Explicitly suppress embedded notes on this page.
"3" = """
First point
Second point
"""
```

PDF keys are one-based decimal page numbers. Image-folder keys are exact image
filenames, matching the natural-ordered frozen slide list (e.g. `"slide_02.png"`).
A named sidecar entry wins over PDF annotation text, or over the image folder's
existing `.txt`/global notes. Missing entries fall back to extracted notes; explicit
empty strings override them. Keys for other pages/files are ignored by display but
preserved on save. Unknown top-level fields and unsupported versions are rejected.
Malformed/unreadable sidecars produce a presenter warning and display falls back
to extracted notes; the editor refuses to overwrite the invalid file.

Limits: 10000 entries, 256 KiB UTF-8 per note, 8 MiB combined note text, 16 MiB
serialized file. Reads and writes run in a dedicated supervised worker with a
30-second deadline, independently of rendering. Regular files only: symlink or
non-file sidecars are refused. JSON worker messages retain their 16 MiB bound;
pathological escape-heavy notes can exceed that transport bound and report an error.

Double-clicking the notes pane opens the current slide's editor and prepares the
sidecar. If absent, all extracted/resolved notes (including empty slides) are written
using an atomically persisted temporary file with no-clobber creation. If already
present, preparation only reads it. The editor captures the slide index; subsequent
navigation does not retarget a draft. Save modifies that page's key, preserving all
other entries and showing saved text in the auto-fit pane after acknowledgement.
TOML formatting/comments are regenerated on save; text values round-trip unchanged.

Creation and saves sync a temporary file in the sidecar directory before atomic
persistence. Saves compare the sidecar's SHA-256 with the editor revision and check
again just before replacement; detected external edits/removal fail without writing.
This is optimistic conflict detection, not a lock against an external writer racing
the final rename. Changed source SHA/slide identity is rejected; reopen the source.
Read-only directories, invalid files, conflicts and worker failures retain the draft
with explicit retry/reload/discard choices. Reload explicitly discards the draft.
A file created by opening the editor remains even if editing is canceled.

The editor disables presentation keyboard shortcuts while open. Opening another
source is held until it closes; normal application exit is held while saving or
while a draft is dirty. Explicit Discard changes and close is available. Force-kill
or OS shutdown can still lose unsaved drafts. Editor state is transient and never
sent to presentation outputs or persisted in global/project layout settings.

See [example](examples/presentation.notes.toml), `tests/notes.rs`, the pointer
interaction tests in `tests/presenter_ui.rs`, and the private worker protocol.
