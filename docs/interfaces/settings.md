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

- `slide_colors.page_background` and `slide_colors.fit_padding`: opaque sRGB
  arrays of exactly three integers, each 0–255, added in 0.1.26. Defaults are
  `[255, 255, 255]` (white beneath transparent slide pixels) and `[0, 0, 0]`
  (black around the aspect-fitted slide). Missing tables/fields in older v1 files
  use these defaults; malformed lengths/out-of-range channels fail deserialization.
  Both are global, round-trip unchanged, persist asynchronously and apply immediately
  to all previews and audience windows. They do not alter source or cached pixels,
  project layouts, or manual/emergency black blanking.
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
  is percentage 12–65; elapsed/clock sizes are logical pixels 16–320. Defaults are in `settings.rs`. Since 0.1.35, the previous-slide widget defaults
  to hidden; an explicitly saved `previous.show` value is preserved.
- `layout.elapsed_position` and `layout.clock_position`: `header` (default),
  `above_slides`, or `below_notes`, added in 0.1.24; `left_of_notes`,
  `right_of_notes`, `left_of_slides`, `right_of_slides` added in 0.1.48. Independent presenter-only
  placement presets; unknown values are rejected. Body timer rows reserve space
  and remain visible with hidden bars. Header timers follow header visibility. Side timers reserve up to 32% of their
  pane per side, stack if sharing a side, and fit their text to available space.
  If the target pane is hidden, side timers use the remaining body. Notes refit
  to the reduced pane. Existing layouts retain their sizes/positions; older binaries
  reject the new positions and sizes over 72.
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
- `buffer_mib`: integer 64–16384, adaptive default, added in 0.1.13. Decoded slide RAM
  allowance for each presenter/output process; one MiB = 1048576 bytes. Additional
  worker/texture/UI memory is not included. Since 0.1.46, missing budgets use 15% of available physical RAM measured once at
  process startup, raised toward 512 MiB only within a 20% ceiling. The operational
  minimum is 64 MiB, maximum 16384 MiB; failed memory detection uses 256 MiB.
  Explicit saved budgets (including the old 1024 MiB default) remain unchanged.
  Saving preferences persists the chosen numeric budget; it is not continuously
  retuned. Older binaries reject budgets below their former 256 MiB minimum. The menu
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
