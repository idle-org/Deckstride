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
The artifact key includes page, requested edge and renderer generation. Since
0.1.26, PDF artifacts use `hayro09-straight`: Hayro premultiplied output is converted
to straight RGBA before caching. Older `hayro09` PDF artifacts are regenerated to
correct dark translucent edges; image artifacts retain `hayro09`. The v1 binary
format is unchanged. Page/padding display colors are composed after loading and
are neither baked into artifacts nor part of the cache key.
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

As of 0.1.13, a cold first frame uses a 512-pixel edge, then all slides are prepared
at the maximum pixel edge of connected presenter/presentation screens (512–4096).
Smaller screens share the largest variant. An exact verified target-size cache hit
bypasses preview rendering. Resolution changes use a distinct existing/new artifact
key. The disk schema remains v1; prior artifacts at matching resolutions remain usable.

When the deck fits, the entire decoded deck is retained in presenter and output RAM.
As of 0.1.19, reaching `buffer_mib` evicts distant non-active frames and continues
processing the rest of the deck. Requested/displayed slides are protected; foreground
navigation takes priority. Render reservations still bound presenter admissions.
Processed-slide progress is independent of RAM residency: an evicted slide remains
prepared on disk and reloads through a worker on demand. Once all slides are processed,
background prefetch improves locality without endlessly exchanging distant slides.
Increasing the budget refills RAM; lowering it retires non-active frames immediately,
allowing for in-flight reservations. A budget too small for protected frames and render
reservations still reports pressure rather than discarding active content.

The output host mirrors the presenter working set, counts its last displayed frame
if retained outside the cache, and frees distant entries before admitting replacements.
Stale reads for evicted entries are discarded. READY requires the whole deck to be
resident in both processes; rolling-buffer mode explicitly permits background disk
reads and is never described as fully RAM-buffered. OS paging and software composition
are outside this application-level no-file-read guarantee. The disk schema remains v1.
Disk eviction/quota remains outstanding; full disk reports an explicit error.
