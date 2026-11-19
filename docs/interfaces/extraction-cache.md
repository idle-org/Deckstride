# Extraction cache v1

Implemented in 0.1.12. All artifacts live under the per-user application cache
`slides-v1/<project-key>/<source-sha256>/`; no files are written beside source PDFs
or images. This deliberately supports read-only show folders and portable launch.
Project identity is the SHA-256 of canonical-path OS bytes. PDF identity hashes the
complete PDF; each folder slide hashes its own image bytes. Source hashing happens
in a worker before any old artifact is trusted. Folder deck SHA identifies ordered
filenames only and is never used as an individual image content hash.

Each completed artifact has `<zero-based-page>-<requested-edge>-extraction.json`:
`version` 1, `source` canonical source path, `sha256` source content digest,
`renderer` `hayro-0.9/image-0.25`, `page`, `edge` (maximum pixel dimension), `width`,
`height` (actual pixels), `artifact` (basename). Example: `examples/extraction-v1.json`.
The artifact key includes page, requested edge and renderer generation (`hayro09`).
Changing rendering semantics requires a new cache namespace/key. Notes are omitted.
PDF page crop/rotation is interpreted by Hayro; no presenter-note-region crop applies.

The raw `.rgba` artifact contains width and height as little-endian u32, straight
RGBA8 pixels in row-major order, then 32 bytes SHA-256 over header+pixels. Limits:
1–4096 pixels per dimension, exactly width×height×4 bytes, 64 MiB pixel ceiling.
A reader validates dimensions, length and checksum before displaying. Missing,
partial or corrupt raw files regenerate from the verified source. Artifact and
manifest writes are individually atomic; a crash may leave a usable artifact without
its informational manifest. The manifest is not consumed for loading and cannot
redirect a file read. Unsupported manifest versions are not imported. Cache files
are disposable, not an interchange format or project representation.

First render uses a 1280-pixel edge, then current-slide refinement at 2560;
neighbours use 1280 and visible browser thumbnails 320. No eager whole-deck render.
Decoded presenter cache targets 128 MiB with current/adjacent slides pinned;
output retains at most six decoded frames. Disk eviction/quota remains outstanding.
A cache write failure reports an error and holds existing output. Source bytes
remain unchanged, including on malformed inputs and regeneration.
