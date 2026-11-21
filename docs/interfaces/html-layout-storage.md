# HTML concept layout storage — version 1

Applies only to `docs/design/presenter-menu.html`, not the native application.
The page uses browser localStorage key `deckstride.menu-layouts.v1`. Storage is
scoped by the browser to the page origin; file-URL handling is browser-dependent.
It is not the native global configuration or a file saved alongside a PDF.

The JSON envelope contains `version: 1`, `global` (null or a layout) and `projects`
(an object mapping the fixed mock project IDs `ideas` and `workshop` to layouts).
Each layout contains `previous`, `current`, `next`, `notes`, `elapsed` and `clock`.
Each widget has a boolean `show` and an integer `size`. Every widget, including current, can be hidden.
Preview sizes are relative width weights: previous/next 30–160, current 60–200.
Notes size is a height percentage 12–65; elapsed/clock sizes are CSS pixels 16–72.
Layouts also carry `chrome: {"header": true, "footer": true}` with boolean pinned
visibility. Old version-1 records omitting `chrome` load with both visible; explicit
invalid types are rejected. Temporary edge reveals are never saved. Hidden bars
reveal as overlays near the corresponding edge, and can be pinned from the right-click
menu or their own button. Hiding them releases layout space; clocks inside a hidden
header are hidden with it, without losing their individual visibility preferences.
No document contents, notes, screen assignments or elapsed time are persisted.

Load precedence: project override, global default, built-in balanced layout.
Edits and presets only affect the live view until explicitly saved. Project save
replaces that project's entire layout. All-project save replaces the global default
and preserves all project overrides. “Use all-projects default” removes the current
project override and loads the global or built-in default. Changing demo projects
or reloading discards unsaved edits. The existing Reset demo action reloads the page;
it does not erase saved layouts.

Example (a complete all-project default without overrides):

```json
{"version":1,"global":{"previous":{"show":true,"size":60},"current":{"show":true,"size":130},"next":{"show":true,"size":75},"notes":{"show":true,"size":28},"elapsed":{"show":true,"size":28},"clock":{"show":true,"size":28},"chrome":{"header":true,"footer":true}},"projects":{}}
```

Reject unsupported versions, malformed envelopes, missing widgets, wrong types,
non-integer/out-of-range sizes. Limit input to 20,000
JavaScript string code units. Ignore unknown fields/project IDs when normalizing.
Failed reads load built-in defaults with a visible status. Failed saves/removals
leave the current view and existing storage untouched; do not report success.
Unsupported/corrupt records are not overwritten by save; removing this specific
browser storage key allows recovery. No migration from other versions exists.

The native implementation must define real project identity and per-user settings
storage separately; the mock's two project IDs are not a native compatibility contract.

## Validation

`tests/browser/layout-storage.js` is an explicitly invoked browser regression harness.
Append it as a script before `</body>` in a temporary copy of the concept HTML and
open that copy in an isolated Firefox profile. It displays a PASS/FAIL report and
restores the original storage record. Covers round trips, project/global precedence,
override removal, unsaved changes, unsupported/corrupt records and failed storage.
It is not a native-application or physical-display acceptance test.

Version 1 now accepts `current.show: false`; previously saved layouts remain valid.
With all slide previews hidden, visible notes expand into the released space.

`tests/browser/chrome-visibility.js` additionally checks saved bar visibility, legacy
record defaults, edge reveal, pointer/focus retention and right-click recovery. Append
it to a temporary HTML copy and wait for its asynchronous PASS/FAIL report.
