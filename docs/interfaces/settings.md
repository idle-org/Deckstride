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

- `auto_show`: boolean, default true (0.1.14). Enables output on startup and successful
  document open; false starts/opens blanked. Manual navigation preserves blanking.
  Display topology changes, safe fullscreen exit and host recovery still blank output.
- `theme`: built-in palette enum in snake_case, default `midra` (0.1.14). Values:
  `midra`, `high_contrast`, `foxified_dark`, `corporate_light`, `bluecore`,
  `dupont_monsanto`, `eden_green`, `foxified_light`, `hci`, `iron_docks`, `peaceframe`,
  `rebel`, `sirius_mind`, `solarized`, `sons_of_banard`, `storm_pilot`,
  `tessier_ashpol`, `ustf`, `verdant_rebel`, `weicat_industrial`.
- `language`: `system` (default), `en` or `fr` (0.1.14). Native OS locale selects
  French for `fr*`, English otherwise. Unknown theme/language values are rejected.
  Missing new fields in older v1 files use these defaults; no migration is required.
  Theme, language and automatic-output changes persist asynchronously, globally.
- `layout`: `previous`, `current`, `next`, `notes`, `elapsed`, `clock`, each with
  boolean `show` and finite floating `size`; plus boolean `header`/`footer`.
  Previous/next sizes are relative weights 30–160, current 60–200; notes size
  is percentage 12–65; elapsed/clock sizes are logical pixels 16–72. Defaults are in `settings.rs`.
- `layout.elapsed_position` and `layout.clock_position`: `header` (default),
  `above_slides`, or `below_notes`, added in 0.1.24. Independent presenter-only
  placement presets; unknown values are rejected. Body timer rows reserve space
  and remain visible with hidden bars. Header timers follow header visibility.
- `layout.slide_progress`: boolean, default false, added in 0.1.24. Shows requested
  slide / total and its fraction in a reserved bottom body row; empty decks show
  0 / 0 with zero fill. This is navigation progress, not render/output readiness.
  These three fields round-trip in both global and project layouts. Older v1
  layouts omit them and use defaults without migration.
- `projects`: at most 4096 SHA-256 canonical-path keys mapping to full layouts.
  Path identity uses OS encoded bytes, not source content. Moving files or switching
  platforms may therefore select a different layout. Project overrides win globally.
- `routes.presenter` and `routes.presentation`: arrays of monitor identifiers
  (`native-name#occurrence`), disjoint and unique, at most 32 entries combined.
  Saved preferences are not overwritten by temporary missing-monitor fallback.
- `presenter_fullscreen`, `presentation_fullscreen`: booleans, default true.
- `browser_monitor`: optional monitor ID; missing/unavailable means presenter screen.
- `thumbnail_size`: logical width in pixels, finite 100–400; default 220.
- `buffer_mib`: integer 256–16384, default 1024, added in 0.1.13. Decoded slide RAM
  allowance for each presenter/output process; one MiB = 1048576 bytes. Additional
  worker/texture/UI memory is not included. Older v1 files default to 1024. The menu
  persists changes asynchronously. Since 0.1.19, lowering evicts non-active frames
  while reserving room for outstanding renders; increasing allows RAM refill.

Display assignments and fullscreen choices are previewed in a 15-second transient
trial before persistence. Only Keep changes updates their saved values. Timeout,
Escape, Revert now and hotplug discard the trial. Unrelated preference saves during
a trial serialize the previously confirmed display values. No v1 schema change.

Transient display trials/startup recovery eligibility, reveals, blank/current/selected slides, elapsed time, loaded document,
Change on click and worker state are not persisted. HTML prototype localStorage is
separate and is not automatically imported. See `examples/settings-v1.toml` and
round-trip/invalid-version tests in `tests/settings.rs`.
