# Managed blank picture v1

One optional asset lives at `blank-picture.rgba` next to settings.toml. Settings v1
`blank_picture` selects it; missing/false means solid black. It is independent of
document caches and their cleanup. Reset disables the asset, retaining at most
one bounded file; the next successful import atomically replaces it.

The v1 binary representation uses the existing frame envelope: unsigned
little-endian u32 width, u32 height, row-major unpremultiplied RGBA8 pixels, then
32 raw SHA256 bytes covering the header and pixels. Width and height must be
1–2048; length must be exactly 8 + width × height × 4 + 32 bytes. Maximum file
size is 16 MiB + 40 bytes. FrameInfo.sha256 is lowercase hex of that same digest.
Invalid dimensions, lengths or checksum are rejected; there is no permissive
interpretation of other versions. No source filename or operator notes are stored.

A killable worker reads at most 64 MiB of source data, with image decoder limits
of 256 MiB allocation and 32768 pixels per source edge. Images are proportionally
resized to fit 2048 × 2048 without upscaling; animated sources use a still frame.
Import failure preserves the existing copy/selection. The original is read-only.

The presenter and output each retain at most 16 MiB of decoded blank pixels,
separately from slide-cache budgets; native output textures are bounded per
window. Loading is asynchronous and the output compares the returned digest,
dimensions and path with its request to reject stale replacement reads.
A missing/corrupt asset stays black; no slide is substituted. Output fits the
whole image on black, including transparency, without zoom or annotations.
Tests cover normalized import/load round trips, deleted sources, corruption,
failed replacement, worker import/reset and native blank/show acknowledgements.
