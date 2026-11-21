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

- `zoom_keep_fitted_area`: boolean, default false, added in 0.1.55. Missing fields
  migrate to screen-adapting zoom: preserve aspect ratio while expanding into the
  viewport, cropping overflow. True retains the original fitted page area. This
  global preference does not change cached render resolution or enable zoom.
- `zoom_enabled`: boolean, default false, added in 0.1.54. Global opt-in for
  presenter/current-slide and output zoom gestures. Missing fields in older v1
  files retain normal non-zoom behavior. Menu → Preferences saves this setting
  immediately. Crop/scale, gesture serials and warning state are transient and are
  never saved in global/project layouts; each newly opened slide starts fitted.
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
  Since 0.1.51, display topology/fullscreen changes and host recovery preserve
  the current output visibility; only explicit blank controls change it.
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
- Since 0.1.49, `layout.slide_progress_height`: finite logical pixels 8–120,
  default 24. Height is capped to 25% of available body height on small windows.
- Since 0.1.49, `layout.timer_mode`: `elapsed` (default) or `countdown`;
  `layout.countdown_seconds`: integer 1–86400, default 1200;
  `layout.timer_autostart`: boolean, default true. These preferences and bar size
  round-trip through both global and project layouts. Missing fields use defaults.
  Older v1 binaries ignore these additive fields and lose them on resave.
  Running/paused state, monotonic timestamps and elapsed time are transient,
  never saved. Successful source loads reset the timer to its waiting state;
  failed loads preserve the current timer. Mode/duration/autostart edits reset it.
  Automatic start uses the first explicit navigation, zoom or output-visibility
  action; passively displaying the initial output frame does not start timing.
  Pause disables pending autostart; later actions never resume a paused timer. Reset
  returns to the configured initial value and arms autostart for the next navigation,
  zoom or visibility action. Countdown uses ceiling seconds, stops at zero and never wraps; explicit
  Start after expiry restarts the configured duration immediately.
- `projects`: at most 4096 SHA-256 canonical-path keys mapping to full layouts.
  Path identity uses OS encoded bytes, not source content. Moving files or switching
  platforms may therefore select a different layout. These remain the shared timer
  settings and the legacy visual fallback for displays without profiles.
- `display_layouts`: optional map added in 0.1.76; keys are the same monitor IDs
  used by routes (`native-name#occurrence`), not presenter indexes or primary status.
  At most 128 profiles; keys are nonempty UTF-8 strings of at most 1024 bytes.
  Each profile contains `layout` (all-project visual default) and `projects`
  (canonical-path SHA keys to visual overrides). At most 4096 display/project
  overrides in total. All nested Layout fields use the existing units, ranges and
  validation above, including finite sizes. Missing maps are empty; old v1 files
  require no rewrite/migration. Older binaries ignore these additive profiles and
  discard them on resave.
  Resolution is display/project override → display default → legacy project/global
  layout if there is no display profile. The first save creates a profile whose
  default is copied from the legacy global layout; Save all projects replaces that
  display default, Save this project stores only that display's project override,
  and Use all-project layout removes only that display's project override.
  The three timer-behavior fields (`timer_mode`, `countdown_seconds`,
  `timer_autostart`) in display layouts are compatibility copies, not independent
  timers. Their authoritative values remain the shared legacy project/global
  layout; scoped saves also save those shared timer fields. Timer visibility, size
  and position are independent per display. Use all-project layout only resets the
  current display's visuals and leaves shared timer behavior unchanged.
  Runtime edits are retained by display ID across ordinary window recreation and
  hotplug, but only explicit Save persists them. Successful source/settings loads
  resolve saved profiles afresh; failed source loads preserve current edits.
  Session view state is bounded to 128 retained displays, pruning disconnected
  entries when that limit is reached. Menus/panel visibility are per presenter and
  transient. Reordering monitors/roles does not copy a layout to another ID;
  indistinguishable native names still share the existing route identifier limits.
- `routes.presenter` and `routes.presentation`: arrays of monitor identifiers
  (`native-name#occurrence`), disjoint and unique, at most 32 entries combined.
  Saved preferences are not overwritten by temporary missing-monitor fallback.
- `single_screens`: optional array added in 0.1.89, default empty. Up to 32 unique
  monitor IDs, each a nonempty UTF-8 string of at most 1024 bytes; no units.
  A marker is active only while its monitor is in `routes.presenter`; those
  windows retain operator-process ownership but use the slide-first single-screen
  view. Audience routes never use this marker to reveal operator controls.
  Displays presents this combination as the separate Single screen role. Choosing
  another role removes that connected monitor's marker. Disconnected markers are
  retained; group swaps leave active Single screen monitors in place.
  Apply previews routes and markers together; Keep changes persists both, while
  Revert, timeout or hotplug discards trial values. Single-screen windows use the
  global `presentation_fullscreen` default and per-screen `screen_modes` overrides.
  Existing presenter layouts are preserved and restored when switching back.
  Old v1 files migrate with an empty array; older binaries ignore this additive
  field and show the ordinary Presenter view on the operator-owned route.
  The startup offer/decision is transient, never serialized. It is eligible only
  on the first native monitor discovery with exactly one screen, after settings
  load, and is suppressed for an already configured Single screen monitor. A
  topology change withdraws it permanently for that run; later disconnects or
  reconnections never create an offer. Round-trip and invalid-boundary tests cover
  the saved array; startup/hotplug tests cover the transient decision.
- `presenter_fullscreen`, `presentation_fullscreen`: booleans, default true.
- `screen_modes`: optional monitor-ID map added in 0.1.87. Values are
  `follow_global`, `always_fullscreen`, or `always_windowed`; unknown values fail
  deserialization. At most 128 entries, with nonempty UTF-8 keys of at most 1024
  bytes. No units. Missing maps/entries follow the role's global fullscreen flag;
  old v1 files need no migration. Forced choices override global fullscreen
  commands and the `--windowed` global startup default. Choices stay attached to
  the monitor ID across role swaps, mirrors, disconnects and reconnects, including
  unused monitors. The All slides auxiliary window remains windowed.
  Apply previews the map in the timed display trial; only Keep changes saves it.
  Revert, timeout and hotplug restore confirmed preferences. The UI removes an
  entry when Follow global is selected; explicit `follow_global` entries also
  round-trip unchanged. Older binaries ignore this additive map and discard it
  on resave. See the example and settings round-trip/boundary tests.
- `browser_monitor`: optional monitor ID; missing/unavailable means presenter screen.
- `thumbnail_size`: logical width in pixels, finite 100–400; default 220.
- `disk_cache.max_mib`: integer 64–1048576 MiB, default 10240 (10 GiB).
- `disk_cache.max_days`: integer 1–3650 days, default 30. Added in 0.1.70;
  missing table/fields in v1 settings use defaults. Invalid limits are rejected.
  Global options persist asynchronously; older binaries ignore these fields. See
  [cache retention](extraction-cache.md) for age, active-deck protection and cleanup.
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

- `blank_picture`: optional boolean (0.1.77), default false. True selects the
  managed [blank picture](blank-picture.md); false uses solid black. Missing or
  invalid assets fall back to black. Existing v1 files migrate by defaulting false.
