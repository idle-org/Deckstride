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
## Disk retention — 0.1.70

Menu → Preferences → Disk cache controls total size (default 10240 MiB / 10 GiB)
and lifetime (default 30 days), with **Expunge unused cache now** to remove all
eligible artifacts regardless of age/size. One day is 86400 seconds. Age is measured
from filesystem modification time (render/write time), not access time; warm reads
do not extend lifetime. Future timestamps do not expire until their age reaches
the limit. Expired files are removed first in oldest-first order, then older files
are removed until total bytes are within the target. Ties sort by path.

Since 0.1.71, a dedicated killable worker runs automatic maintenance once per launch,
after settings and any initial document/notes loading and slide preparation finish.
Rolling-buffer decks need all slides prepared on disk, not resident in RAM. With no
document, cleanup runs after startup settings load. Incomplete/failed slide preparation
defers automatic cleanup; manual expunge remains available. Later document opens and
option changes do not schedule another pass. New limits apply on the next startup;
manual expunge is immediate. Cleanup never blocks first-page display or UI input.
Only one cleanup request is admitted per presenter; its deadline is 30 seconds.
Errors/timeouts appear in the Disk cache section without automatic retries.
A manual request also consumes any pending startup pass, avoiding duplicate work.
Manual cleanup reports removed files/bytes, remaining bytes and retained candidates.
Counts are a scan snapshot; concurrent rendering can change actual usage.

Current presenter projects are excluded. Live document workers also hold shared OS
locks on `<project-key>/.lease-v1`; cleanup takes a nonblocking exclusive lock before
deleting each file. Locks release on process exit, including crashes. This protects
workers opening/rendering decks across instances. Lease files are empty, contain no
persistent fields, are never unlinked, and support old caches by lazy creation.
Older application versions do not honor these locks: avoid concurrent mixed-version
use of one cache. Active decks can exceed the quota or lifetime; limits are eventual
targets, not a hard write cap. Manual expunge also retains active projects. To clear
the current deck, close the application and delete `slides-v1`, or reopen the app
without a deck and use manual expunge.

The scan is fixed to project/source-hash directories, never follows discovered
symlinks, and caps traversal at 200000 entries before deletion. It deletes only known
page/edge `hayro09` and `hayro09-straight` RGBA files and extraction manifests;
unknown files and temporary files are retained. Total bytes count all regular files
at the artifact level, including unknown files, but exclude filesystem overhead and
empty lease files. Unknown files or protected decks can prevent reaching the target.
Metadata is rechecked under the project lock before deletion. Empty source-hash
directories are removed when possible; project/lease directories remain. Image and
manifest deletion is individually recoverable: missing artifacts regenerate.
Source documents, notes sidecars and settings are outside cleanup scope. A full or
unwritable disk still reports an explicit error; a memory-only fallback is pending.
