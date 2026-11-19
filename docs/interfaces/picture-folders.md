# Picture folders and notes

Implemented in 0.1.12. Immediate regular files with PNG, JPG/JPEG, WebP, BMP, GIF,
TIF/TIFF extensions become slides; no recursive traversal or symlink-file inclusion.
Bundled decoders display the first image/frame for animated/multipage formats.
The folder snapshot is frozen before presentation. Reopen to discover changes.
Maximum 10000 slides; input files are limited to 512 MiB, image dimensions to 32768
per side and decoder allocation to 512 MiB. Rendered artifacts are at most 4096 per side.

Natural comparison treats ASCII digit runs numerically without integer overflow,
ignoring leading zeroes for numeric comparison. Other UTF-8 bytes compare with ASCII
case folding, then exact filename breaks ties. There is no locale collation or Unicode
normalization. Use UTF-8 filenames for portable notes keys. Missing/corrupt slides
retain their position and report an error rather than renumbering the deck.

A UTF-8 matching-stem sidecar (`slide_01.png` → `slide_01.txt`) overrides global notes,
even when empty. Multiple images sharing that stem share its sidecar. Otherwise use
one global notes file mapping exact image filenames to strings:

```json
{"slide_01.png": "Opening remarks", "slide_02.jpg": "Questions"}
```

```toml
"slide_01.png" = "Opening remarks"
"slide_02.jpg" = "Questions"
```

When both globals exist, both are ignored with a presenter diagnostic; sidecars still
work. Remove/rename the unwanted global and reopen. Invalid UTF-8, malformed mappings
or unreadable sidecars produce warnings, never a failed image load. Each notes file
is at most 256 KiB and combined notes at most 8 MiB; excess notes are omitted with a
warning. Missing entries mean empty text; unknown global keys are ignored.

Native opening enumerates slides first; notes are extracted separately after a usable
current frame exists. Notes remain local to presenter state. All accepted text wraps
and shrinks to the pane, recalculated on text/area changes. Extremely long notes may
be unreadably small; unknown font glyphs depend on bundled font coverage.
Tests cover ordering, both globals, empty/invalid sidecars, source preservation,
checksum corruption/regeneration, actual image and PDF rendering.
