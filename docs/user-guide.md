# Using PDF Presenter

Run the portable executable and choose Menu → Open PDF or Open picture folder.
You can also drop a file/folder on the presenter, pass its path on the command line,
or use `--open PATH`. The source is read-only. A failed load keeps the previous show.
Only one file/folder chooser can be open at a time: Open commands are disabled until
you select a source, cancel or the chooser times out. On Windows the chooser belongs
to the active presenter window and appears above its fullscreen view. Repeated clicks
do not queue additional dialogs.
A single connected monitor starts presenter-only rehearsal; `--windowed` disables
fullscreen for the session. `--list-monitors` lists CLI display numbers.

## Screens and presentation safety

Both roles default to fullscreen. Displays lists connected screens: assign each
Unused, Presenter or Presentation, including several screens per role. Swap groups
exchanges the roles. Apply and remember saves the preferred assignments globally.
Unavailable screens fall back to surviving assignments and available distinct screens;
the preferred saved setup remains available for reconnection. Review the setup after
connecting/disconnecting displays: output is blanked until you explicitly show it.
If the operator screen disappears and only previous presentation screens remain, the
presenter is withheld to protect notes. Reconnect an operator screen, or restart in
windowed rehearsal after verifying privacy. Duplicate monitor names are distinguished
by enumeration order; verify identical models.

Output is enabled by default when opening a document. Menu → Preferences → Show presentation
automatically lets you save an opt-out. **Show presentation** restores blanked output; **Blank presentation** or B hides it. Navigation
preserves blanking. Until an uncached requested slide is ready, output holds the last
usable frame. The footer distinguishes requested and acknowledged displayed slides. A red
**NO OUTPUT DISPLAYED** warning marks blanking, no assigned output or no document;
it remains visible when the footer is hidden. Preparing output has a separate status.
Escape cancels a pending slide number/menu, closes the browser, or safely leaves
fullscreen and blanks output. Closing a presentation window blanks it; close a
presenter or choose Menu → Quit to end the application.

Arrow keys, Page Up/Down and Space navigate from the presenter. Type a 1-based slide
number then Enter to jump; Backspace edits and Escape cancels. Out-of-range numbers
do not navigate. Output windows accept B/Escape for emergency blanking.

## Layout and notes

Layout independently enables and sizes previous/current/next slides, notes, elapsed
time and the local HH:MM clock. Hidden widgets release their space. Notes wrap and
automatically shrink to fit; extremely long notes can become too small to read.
Menu → Reset timer starts elapsed time again. Bundled UI fonts cover their supplied
character sets; broad CJK text coverage remains a packaging acceptance item.

Save layout (this project) stores an override for the canonical document/folder path.
Save layout (all projects) changes the default; existing project overrides win.
Use all-project layout removes this project's override. Project settings live in the
user configuration, not in or beside the source document. Moving the source changes
its project identity. The menu shows the configuration location.

Hide either header/footer with Hide, or from Layout. Moving the mouse near that edge
temporarily reveals it; Pin makes it persistent again. Right-click restores either
or both bars. Temporary reveals are not saved.

For pictures, `slide_2.png` sorts before `slide_10.png`. A matching `slide_2.txt` supplies
notes, including an empty file clearing a global note. Alternatively provide one
`notes.json` or `notes.toml` mapping exact image filenames to text. See the
[folder contract](interfaces/picture-folders.md). PDF annotation Contents are extracted
in the background; separate notes pages/cropped slide-and-notes PDFs are not interpreted.
Notes never go to the presentation host.

## All slides

The browser opens as a separate window, initially windowed on the presenter screen.
Choose another monitor in its toolbar. Preview size changes the grid. Clicking selects
without navigating unless Change on click is checked; it resets to OFF each time the
browser opens. Show selected navigates explicitly and respects output blanking.
The selected tile and current slide have distinct outlines. Closing the browser saves
its monitor and preview-size preferences. The grid uses the slides already in memory;
only visible thumbnail textures are retained.

## Preparation, processed slides and RAM

Opening a show starts two supervised preparation workers. The current slide gets a
512-pixel preview first if a verified full-resolution cache entry is unavailable.
Then the **entire deck** is prepared at the largest connected role screen's pixel
edge, preserving slide aspect ratio. Smaller screens use that same frame scaled to
fit; separate copies for every smaller resolution would waste RAM. The current
renderer limit is 4096 pixels per side, shown in Menu → Preferences. Resolution changes start a
new preparation pass with a new buffer identity.

Processed frames are kept in the application's persistent cache; Menu → Preferences → Storage locations shows its
location. Reopening with unchanged source contents, resolution and renderer reuses
those frames without PDF rasterization or image decoding. It still verifies source
identity/checksums and reads frames into RAM: a restart cannot literally take zero
time. A warm full-size cached frame appears directly, without an unnecessary preview
render. Sources and their folders remain unchanged.

The footer shows the active operation, slides prepared/total, cache reuse, decoded
RAM usage, and an approximate ETA once there are enough completed slides. It also
shows output-process buffering separately. A compact progress notification remains
visible while preparing if the footer is hidden. Notes reading and failures have
explicit status. ETAs use observed throughput and may change on complex slides.

**READY** means every target-resolution slide is held in presenter RAM and, when
outputs are assigned, acknowledged in the independent presentation host's RAM.
Navigation then performs no application slide-file reads, PDF rendering or image
decoding. Raster composition still happens, and the OS can page memory under pressure.
The entire prepared deck stays resident instead of evicting distant slides.

Menu → Preferences → RAM budget controls the decoded-slide allowance for **each process**, default
1024 MiB, configurable from 256 to 16384 MiB. Presenter and output can each consume
that allowance; textures, workers and other application memory are additional.
Preparation reserves capacity for in-flight frames. If the deck cannot fit, it pauses
and reports that it is **not fully loaded**, rather than claiming READY and loading
slides later. Increase the budget within available physical RAM to resume. Lowering
the budget does not evict already buffered slides; reopen to apply a lower allocation
to an existing show. Cold rendering/decoding may continue while you present early.
After a host restart or resolution change, wait for READY again for RAM-only playback.

## Recovery and portability

Rendering, notes, file dialogs and settings I/O run in supervised child processes of
the same executable. A stuck renderer is terminated after its deadline; the UI and
last output remain available. Reopen the source to retry failed pages. A presentation
host failure restarts blanked with a status message, up to three starts. If the
presenter crashes, the output process holds its last frame; B/Escape can blank it.
Automatic reconnection to that orphaned show is not implemented yet.

Sleep protection is requested for both native processes, including while blanked.
Read the footer: OS services can grant partial protection or refuse it. On X11 an
additional process-scoped screensaver/DPMS suspension is used where available. No
permanent power settings are modified. Forced/manual sleep remains outside this lease.

Caches are in the user's application cache directory; source folders and executable
locations need no write access. The current cache has no automatic disk quota cleanup;
you may remove its `slides-v1` directory while the app is closed. A full/unwritable cache
reports an error and retains the last working output. OS window services/system
libraries are prerequisites; PDF/image/font libraries are bundled in the executable. `--licenses` prints bundled
dependency/font notices without launching a window.

## Appearance and language

Menu opens a dropdown below its button; click outside or press Escape to dismiss it.
Displays uses numbered screen cards with direct Unused / Presenter / Presentation
choices. Repeat a role to mirror it, then Apply and remember.

Menu → Appearance → Theme offers the same 20 built-in palettes as Canvas Pilot, including light,
dark and High Contrast. Menu → Appearance → Language offers System, English and Français. System
uses the native OS locale and falls back to English for other languages. Choices are
saved globally and apply to all presenter/browser windows; slides are never recolored.
The controls and preparation progress are translated; detailed worker/OS diagnostics
remain verbatim. Additional Canvas Pilot languages and external theme/language packs
are not included in this version.

All slides opens a windowed browser above fullscreen windows and requests focus.
Opening it again restores/focuses the existing browser. Close or Escape dismisses it.
The destination remains configurable in the browser, defaulting to the presenter.
Window-manager focus restrictions may still require clicking the browser on some systems.
