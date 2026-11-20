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
stopped host after 8 s; at most three starts, recovery preserves the presenter’s output visibility. A presenter crash
leaves the independent host displaying its last frame. Normal exit sends quit and
allows bounded shutdown outside the UI loop. Reconnection to an orphan is pending.

## Whole-deck buffering in 0.1.13

Preview requests first check the verified target-resolution artifact, falling back to
512 only on a miss. Replies identify the actual requested render edge and cache-hit
status. The two workers prepare the remaining deck concurrently with foreground
navigation able to supersede outstanding work.

HostState now carries a document/resolution generation, decoded RAM budget and the
currently resident target-frame preload list. Since 0.1.19 the host mirrors this
working set, retires evicted entries and discards stale read completions. It can
free distant cached frames to admit a replacement while protecting the displayed
and requested frames. Its acknowledgment counts actual resident entries for that
generation, rather than all previously processed slides. READY requires both presenter and host
counts to reach the deck total. Stale acknowledgments cannot mark another resolution
ready. The bounded JSON envelope and raw-frame limits remain unchanged. This protocol
still requires the same executable build at both ends.


## File chooser ownership and admission, 0.1.15

`Pick` carries `folder` and optional `owner` (a signed native Windows HWND integer;
null on other platforms). This identifier belongs to the parent presenter process,
is never dereferenced as worker memory, and is passed to the Windows common dialog
as its owner. Only the active presenter (or first available presenter) supplies it;
an audience window is never the owner. Windows requests without an owner fail clearly.
The protocol remains private and requires matching executable builds.

The shared Studio admits exactly one outstanding chooser job, including across
mirrored presenters and alternating PDF/folder requests. Repeated Open actions are
ignored and the corresponding menu controls are disabled until a completion arrives.
Success, cancellation, worker error and the existing 300-second deadline all release
the guard; submission failure never sets it. The native host re-enables presenter
input and restores owner focus after completion, including a worker killed while the
common dialog had disabled its owner. Existing slide-browser windows temporarily
lose topmost status while the chooser is active. Rendering/output stay independent.


## Output shutdown and lost presenter, 0.1.16

No wire fields change. EOF/failed input drops the host command sender; the native
host detects receiver disconnection instead of treating it as an empty queue forever.
An established output keeps its last frame after unexpected connection loss, while
Close or Escape requests exit on the next event-loop tick. A disconnected host with
no windows exits immediately. B still blanks. With a live presenter, output Close or
Escape retains the existing emergency-blank behavior.

Normal presenter shutdown signals the supervisor and allows 500 ms for graceful
completion, then terminates/reaps the owned child through a separate shared process
handle. The handle is never locked during pipe writes, so a blocked writer cannot
prevent termination. A further one-second manager completion wait is bounded.
Event-loop errors and OutputLink destruction use this cleanup path as well. OS process
termination/reaping still relies on OS responsiveness; these bounds are not a promise
against kernel/driver failure. Unexpected presenter death deliberately does not use
parent-death termination for the audience, to preserve the last usable frame.


## Display setup recovery, 0.1.21

HostState adds default-false `setup_controls` and optional `route_trial` (u64 trial
identifier). Startup controls allow an output-side right-click swap only before the
first explicit navigation/show action; a pending trial always exposes minimal
Keep/Revert controls. Output ACKs can carry `display_action` (`Swap`, `Keep`, `Revert`)
and `route_trial`. The parent validates startup eligibility and trial identity;
stale confirmations cannot commit a later trial. The parent owns the 15-second
monotonic deadline and does not persist routes/fullscreen flags until confirmed.
A late Keep is treated as rollback. Host EOF clears recovery controls so orphan
Escape/Close remains available. Notes, source paths and operator layout data are
still absent from this control payload. Private same-build IPC only.

## Slide background colors, 0.1.26

HostState carries `slide_colors` with `page_background` and `fit_padding` RGB byte
triples (default white/black). Color updates use the existing state sequence and
repaint existing textures without changing buffer generation or reading slide files.
Blank output remains black. Same-executable protocol only.

## Output visibility in 0.1.51

`Ack.request_blank` is a boolean, default false for older messages. Only explicit
output keyboard blank/close actions set it; diagnostic `status` messages never
change output visibility. The presenter sends fresh state after host failure,
including its current blank flag. Fullscreen/routing trials, confirmation,
rollback and monitor topology changes preserve that flag. Missing outputs and
operator privacy holds still suppress unavailable windows; no presenter content
is sent to the output host.

## Editable notes in 0.1.52

`PrepareNotes {path, cache, source_sha}` and
`SaveNotes {path, cache, source_sha, revision, page, text}` run through the dedicated
notes bridge. `page` is zero-based, `source_sha` identifies the loaded PDF/slide list,
and `revision` is the notes file SHA-256 observed when editing began. A successful
`Reply.notes` contains `{path, revision, notes}` with all resolved notes in slide
order. Errors use the existing error field. The presenter ignores stale-generation
completions and late background extraction after an editor snapshot has been accepted.
The current page draft survives save failure. None of this payload enters output IPC.


## Notes import in 0.1.53

`PickNotes {owner}` uses the existing owned, supervised file chooser with TXT/MD
filters. `PreviewNotesImport {path, cache, source_sha, import}` returns
`Reply.notes_import {path, revision, sections, total}`: destination sidecar path,
optional existing-file SHA, parsed strings and total slide count. It writes nothing.
`ApplyNotesImport {path, cache, source_sha, preview}` revalidates identity/revision,
merges notes and writes atomically, returning the existing `Reply.notes` snapshot.
Both notes jobs use the dedicated 30-second bridge, bounded JSON messages and source
generations. Notes picker results carry the source generation too. These messages
are private same-executable presenter/worker traffic, never output-host traffic.
