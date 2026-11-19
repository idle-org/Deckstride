# Native settings, version 1

`settings.toml` is stored in `$XDG_CONFIG_HOME/pdf-presenter` (otherwise
`~/.config/pdf-presenter`) on Linux. `directories::ProjectDirs` with qualifier `com`,
organization `Beffroi`, application `PdfPresenter` supplies other platforms' standard
per-user config/cache directories. The UI reports the actual configuration path.
There is no separate project file or source-side settings write.

The top-level `version = 1` is mandatory semantically (omitted defaults to 1).
Unknown versions fail clearly; malformed settings fall back to in-memory defaults
with an error, without rewriting the bad file. Missing files use defaults. Reads are
bounded to 4 MiB. Writes validate then atomically replace using a same-directory
flushed temporary file. Unknown fields are ignored and not preserved on round trip.

- `layout`: `previous`, `current`, `next`, `notes`, `elapsed`, `clock`, each with
  boolean `show` and finite floating `size`; plus boolean `header`/`footer`.
  Previous/next sizes are relative weights 30–160, current 60–200; notes size
  is percentage 12–65; elapsed/clock sizes are logical pixels 16–72. Defaults are in `settings.rs`.
- `projects`: at most 4096 SHA-256 canonical-path keys mapping to full layouts.
  Path identity uses OS encoded bytes, not source content. Moving files or switching
  platforms may therefore select a different layout. Project overrides win globally.
- `routes.presenter` and `routes.presentation`: arrays of monitor identifiers
  (`native-name#occurrence`), disjoint and unique, at most 32 entries combined.
  Saved preferences are not overwritten by temporary missing-monitor fallback.
- `presenter_fullscreen`, `presentation_fullscreen`: booleans, default true.
- `browser_monitor`: optional monitor ID; missing/unavailable means presenter screen.
- `thumbnail_size`: logical width in pixels, finite 100–400; default 220.

Transient reveals, blank/current/selected slides, elapsed time, loaded document,
Change on click and worker state are not persisted. HTML prototype localStorage is
separate and is not automatically imported. See `examples/settings-v1.toml` and
round-trip/invalid-version tests in `tests/settings.rs`.
