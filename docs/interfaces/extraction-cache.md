# Extraction cache design requirements

Status: authorized implementation direction, not an implemented file format.
There is no serializer or compatibility promise yet. Before writing caches, specify
and test a versioned `extraction.json` schema with representative fixtures here.

The user accepts pre-rendered page images stored alongside the PDF and an
`extraction.json` describing source SHA and extraction. Use a separate per-document
cache directory beside the source so unrelated PDFs and existing files cannot collide.
Preserve the PDF. Handle read-only locations with a writable application-cache fallback
and show where extraction is stored; do not make sidecar write access a prerequisite
for presenting.

The eventual manifest must identify its schema version, source content SHA (prefer
SHA-256), source byte size, renderer/version, rendering settings and page count.
Page entries need source page index, artifact path, dimensions, rotation/crop/notes
interpretation, image format and readiness/integrity information. Define units,
numbering, validation, omissions and invalidation explicitly before implementation.
Source filename, size or modification time alone is not sufficient cache identity.
Render-resolution and backend changes must not accidentally reuse incompatible images.

Display a newly rendered first page while source hashing/extraction continues.
Do not trust old cache entries until identity and settings validation completes.
Write artifacts and manifest updates atomically, publish only complete page entries,
and tolerate interrupted/partial extraction. Reject malformed manifests, unsupported
versions, unsafe paths and corrupt/missing artifacts; regenerate affected pages
without silently treating stale content as current. Bound disk, decode, RAM and GPU
usage. Priority changes must cancel or supersede obsolete background jobs.

Cache contents may include document content and speaker notes. Keep them local and
out of Git. Future cleanup must only remove owned cache artifacts, never source PDFs
or unrelated neighboring files. Tests must cover invalidation, interrupted extraction,
read-only locations, collisions, corruption and safe regeneration.
