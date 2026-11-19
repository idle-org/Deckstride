# Private same-executable process protocol

0.1.12 uses anonymous stdin/stdout pipes, with no listening ports or public endpoint.
Roles are `--worker`, `--presentation-host`, `--keep-awake`; only the parent-created
pipe handles connect them. They must use the same executable build. This is an
internal unversioned wire protocol, not a stable external API; mixed builds are
unsupported. Persistent disk representations have independent version contracts.

Messages are little-endian u32 byte length followed by UTF-8 JSON (16 MiB maximum).
Worker replies declare `bytes`; frame pixels follow as raw RGBA8, at most 64 MiB,
validated against returned dimensions. No payload is allowed without frame metadata.
Open/render carry generation epochs; supervisor completions retain generation and
job tags so stale results cannot replace a newer document. Notes are requested
separately. Settings, file pickers and cached-frame reads also run in workers.

Supervisors bound command/result queues, kill/reap on deadlines, and recreate failed
workers on the next job. Foreground/background deadlines are 30 s, settings/cache
reads 10 s, native chooser 300 s. Linux workers die with the parent; Unix workers
attempt a 3 GiB address-space limit. Windows memory job limits remain outstanding.
No worker I/O or process wait is performed in a rendering/input callback.

Presentation HostState contains sequence, page, verified frame paths, neighbour
preloads, monitor IDs, fullscreen and blank flags. It contains no notes or operator
UI text. ACKs report only frames submitted successfully to all active output windows;
this is not optical proof that a projector displayed a frame. Heartbeats detect a
stopped host after 8 s; at most three starts, recovery blanked. A presenter crash
leaves the independent host displaying its last frame. Normal exit sends quit and
allows bounded shutdown outside the UI loop. Reconnection to an orphan is pending.

## Whole-deck buffering in 0.1.13

Preview requests first check the verified target-resolution artifact, falling back to
512 only on a miss. Replies identify the actual requested render edge and cache-hit
status. The two workers prepare the remaining deck concurrently with foreground
navigation able to supersede outstanding work.

HostState now carries a document/resolution generation, decoded RAM budget and the
complete available target-frame preload list. The host retains the whole prepared
deck, acknowledges the number buffered for that generation, and retires only temporary
previews or obsolete generations/resolutions. READY requires both presenter and host
counts to reach the deck total. Stale acknowledgments cannot mark another resolution
ready. The bounded JSON envelope and raw-frame limits remain unchanged. This protocol
still requires the same executable build at both ends.
