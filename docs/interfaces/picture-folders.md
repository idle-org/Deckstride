# Picture folders and notes — planned input contract

Status: requirements and initial implementation choices, not an implemented format.
Finalize schemas, examples and validation tests before shipping folder loading.

## Slides

Read supported pictures from the selected folder without changing source files.
Initially use its immediate files, not recursive traversal. Natural filename ordering
is mandatory: compare digit runs numerically (`slide_2` before `slide_10`), without
integer overflow. Use deterministic, locale-independent text comparison and an exact
filename tie-break for equal numeric values (`slide_1`, `slide_01`). Specify case and
Unicode rules and supported bundled codecs in the final contract and fixtures.

Freeze the ordered file list before showing its first slide; decode images
progressively in bounded workers. Folder scans must not block input. Failed imports
retain the working deck; corrupt/missing images must never silently renumber slides.
Preserve filenames and source bytes. Define source identity/cache invalidation for
folder images independently of the PDF extraction manifest before implementing caches.

## Notes

Support UTF-8 matching-stem sidecars: `slide_01.png` uses `slide_01.txt`.
Also support one folder-level `notes.json` or `notes.toml`, keyed by exact image
filename rather than slide number, so natural sorting does not change associations.
Initial choice: a present sidecar overrides the global entry, including an empty
sidecar intentionally clearing that slide's notes. Missing entries mean no notes.
If both global formats exist, require an explicit selection instead of silently
combining conflicting files. Malformed notes produce a presenter-only diagnostic;
notes parsing must not prevent showing a valid slide. Finalize the global schema,
size/encoding validation and ambiguous matching-stem behavior with examples/tests.

Always fit the complete current note inside the available presenter notes area using
wrapping and automatic font sizing, updating on content, resize and DPI changes.
Do not silently truncate, clip or require scrolling. Keep layout work bounded and
responsive. Notes and their errors must never reach the audience output.

## Acceptance

Test natural ordering (including leading zeros and very long digit runs), Unicode
names, both global formats, sidecar precedence, empty/missing/invalid notes, corrupt
images and read-only/slow folders. Test short/long notes, unbroken strings and pane
resize/DPI changes. Verify first-image latency and navigation while remaining images
and notes load, and confirm original files remain unchanged.
