# Using PDF Presenter

## Install and update

Download a preview from the [releases page](https://proxy.dandaedre.com/gitea/idle-org/pdf-presenter/releases).
Choose the portable ZIP for Windows x86_64 or portable `.tgz` for Linux x86_64,
extract the whole archive, and run `pdf-presenter.exe` or `./pdf-presenter`.
No Rust toolchain, Python or separate PDF renderer is needed to run the application.
Linux uses the system desktop services and X11/Wayland libraries. Windows binaries
are unsigned; macOS packages are not currently provided. Release assets include
`SHA256SUMS` for checking downloads.

For optional Linux application-menu and Wayland icon integration, keep the extracted
folder in a permanent location and run `python3 install-linux-launcher.py` there.
Python is only needed for this optional installer; no administrator privileges
are required.

To update, close PDF Presenter, download and extract the new portable release,
and run its executable. Settings and saved layouts live in your user configuration
directory, outside the extracted folder. If you installed the Linux launcher, rerun
its installer from the new location so it points to the updated binary.

These are preview releases: physical display routing, sleep prevention and
clean-machine compatibility still need acceptance testing. Rehearse on the displays
you plan to use before a live event.

## Open your slides

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
exchanges the draft roles. **Apply changes** previews the assignments
and fullscreen choices while preserving the current output visibility. **Keep changes** confirms and saves;
**Revert now**, Escape, or doing nothing for 15 seconds restores the previous setup.
Confirmation controls appear on presenter and output windows so either screen can
recover the change. Unconfirmed choices are never saved, and a disconnect cancels
the trial. Navigation is held until the trial is resolved; explicit blank/show controls remain available.

Disabling either fullscreen option shows a positioning warning. Window placement
can be unreliable in windowed mode, especially on Linux; switch back to fullscreen
if you experience positioning issues.

Before the first explicit slide navigation or Show presentation action, right-click
either screen for **Swap displays…** (on output: **Swap presenter and presentation
displays…**). This uses the same timed trial. Merely opening a PDF and automatically
showing its first slide does not disable startup recovery. Once you start navigating
or explicitly showing slides, the output startup menu disappears; use Displays on
the presenter for later changes. Startup swap exchanges whole role groups, including
mirrors. Trial output controls contain no notes, paths or presenter content.
Confirmation and rollback preserve output visibility, including an explicit manual blank.
Unavailable screens fall back to surviving assignments and available distinct screens;
the preferred saved setup remains available for reconnection. Review the setup after
connecting/disconnecting displays: output visibility is preserved on available outputs.
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
Escape cancels a pending slide number/menu or closes the browser; it does not
leave presenter fullscreen or blank the presentation. Use **Enter fullscreen /
Exit fullscreen** in the header to toggle both display roles. Like other
display changes, this previews the change with Keep/Revert confirmation. While the presenter is connected, closing a presentation window blanks it; close a
presenter or choose Menu → Quit to end the application.

Arrow keys and Page Up/Down navigate from the presenter or All slides, including
while a presenter menu is open. Space moves forward; Shift+Space moves back. Notes
editors and Ctrl/Alt/Command combinations retain keyboard priority. Type a 1-based slide
number then Enter to jump; Backspace edits and Escape cancels. Out-of-range numbers
do not navigate. Output windows accept B for emergency blanking. Escape never blanks a connected
show; it only reverts a display trial or closes an orphaned output.

Hold Next/Previous, an arrow key, Page Up/Down or Space (Shift+Space backwards)
to accelerate. The first step is immediate; after 400 ms, single steps repeat every
100 ms. After two seconds, steps become batches of up to five; after five seconds,
up to ten, every 160 ms. Batches never exceed 10% of the deck (minimum one slide).
Release or change focus to stop. Missed repeats are never queued or replayed.
Buffered slides display immediately; uncached intermediate destinations are skipped,
with rendering resumed at the latest destination on release or after input settles.

## Pointer and drawing tools

Choose **Tools** in the presenter header. **Pointer** follows the mouse over the
current-slide preview and appears at the same normalized page position on every
presentation output. **Arrow** and **Highlighter** add one straight vector mark per
drag. Choose a color and size, then use **Undo** or **Clear** as needed. P toggles the
pointer, A selects the arrow and H selects the highlighter when no text editor or
modified shortcut has priority. Marks clear when you change slides and are not saved
to the source document or settings.

The toolbox is off by default. It keeps at most 64 marks and draws them over the
already buffered slide texture. Pointer movement uses the bounded output-state queue;
it does not rerender the PDF, decode an image or read a slide from disk. Selecting a
drawing tool temporarily gives dragging to that tool; select **Off** to pan a
zoomed slide again. A first pointer or drawing action starts an armed timer just like
navigation or zoom.

## Layout and notes

Each presenter display has its own Layout controls and saved layout. Open Layout
on the display you want to customize; the panel identifies that display. Presets,
widget visibility/sizes/positions and header/footer settings affect only that screen.

Layout independently enables and sizes previous/current/next slides, notes, elapsed
time and the local HH:MM clock. Hidden widgets release their space. Notes wrap and
automatically shrink to fit; extremely long notes can become too small to read.
The previous-slide preview is off by default; enable **Previous slide** in Layout
when you want it. Saved layouts keep their chosen visibility.
The header's **Reset timer** button returns elapsed time to zero and arms it for the next presentation action. Bundled UI fonts cover their supplied
character sets; broad CJK text coverage remains a packaging acceptance item.

Save layout (this project) stores an override for this display and the canonical
document/folder path. Save layout (all projects) changes this display's default;
its existing project overrides win. Use all-project layout removes only this
display's override. Other presenter layouts are unaffected. Timer mode, duration,
automatic start and start/pause/reset stay shared; timer appearance is per display.
Layouts follow the same display identities as the routing settings on reconnect.
Existing layouts remain the fallback until you save a display-specific profile. Project settings live in the
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
only visible thumbnail textures are retained. Empty tiles say whether a slide is
actively preparing, queued, or cached on disk until it is close enough to load into
the bounded RAM buffer.

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
visible during initial preparation if the footer is hidden. Routine rolling-buffer
information hides with the footer. Notes reading and failures have explicit status.
ETAs use observed throughput and may change on complex slides.

**READY** means every target-resolution slide is held in presenter RAM and, when
outputs are assigned, acknowledged in the independent presentation host's RAM.
Navigation then performs no application slide-file reads, PDF rendering or image
decoding. Raster composition still happens, and the OS can page memory under pressure.
The entire deck stays resident when it fits the configured RAM budget.

Menu → Preferences → RAM budget controls the decoded-slide allowance for **each process**, default
15% of available RAM, with a soft 512 MiB minimum that yields when it would
exceed 20% of available RAM. The adjustable range is 64–16384 MiB. Saved budgets
are preserved; the automatic default is chosen when no budget has been saved. Presenter and output can each consume
that allowance; textures, workers and other application memory are additional.
Preparation reserves capacity for in-flight frames. When the deck cannot fit, the
**Rolling buffer** frees distant slides and continues preparing the remaining pages.
The requested slide and last acknowledged output remain protected. The status shows
prepared pages, actual RAM-resident pages, evictions and preparation ETA. Evicted
slides keep their processed files and reload in the background when requested;
this mode can require disk access and does not claim READY or zero-load navigation.
The last output frame stays visible until its replacement is available.

Increasing the budget lets the app refill the buffer; reducing it frees non-active
frames while preserving room for outstanding renders. If even the active frames and
render reservations cannot fit, the app reports that specific pressure. Cold rendering
and cache preparation can continue while you present early.
After a host restart or resolution change, wait for READY again for RAM-only playback.

## Recovery and portability

Rendering, notes, file dialogs and settings I/O run in supervised child processes of
the same executable. A stuck renderer is terminated after its deadline; the UI and
last output remain available. Reopen the source to retry failed pages. A presentation
host failure restarts with the current output visibility and a status message, up to three starts. If the
presenter crashes, the output process holds its last frame. B can blank it; Escape or
its normal window Close control exits the disconnected output. Closing the presenter
normally shuts down the output process too, with a separate termination fallback if
its IPC writer is stuck.
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
choices. Repeat a role to mirror it, then choose **Apply changes** and confirm with
**Keep changes**.

The numbered screen rectangles in **Displays** reflect each monitor’s pixel aspect
ratio, so portrait, standard and ultrawide displays are easier to distinguish.
They represent proportions, not physical screen size.

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

### Timer placement and slide progress

In Layout, set **Elapsed time position** and **Clock position** independently to
Header, Above slides, Below notes, or left/right of the notes or slides.
Timer text can be set up to 320 logical pixels; it scales down only to fit its
allocated space. Timers on the same side stack vertically. Timers have separate labeled boxes; size and
visibility controls still apply. Body positions remain visible when bars are hidden.
Use **Start timer**, **Pause timer** and **Reset timer** in the presenter header or Layout.
Pause preserves the time. Reset returns to zero and waits for the next slide navigation,
zoom change or output visibility action. Start always begins immediately. Under Layout →
Timers and progress, choose **Elapsed time** or **Countdown** and set a countdown
in minutes and seconds (up to 24 hours). **Start on first action** is enabled by
default: timing begins on the first presentation action after opening or resetting.
Automatic output after opening a document does not count. Turn it off for manual start.
Rehearsal without an output screen starts on explicit navigation or Show presentation.
Merely displaying the initial slide does not start it. Loading a new source resets the
timer; pausing is never undone by slide navigation.
Countdown stops at zero. Changing mode, duration or autostart resets the timer.
Save the layout for this project or all projects to remember these preferences.

Enable **Slide progress bar** for a presenter-only current slide / total indicator
at the bottom of the content area. Adjust **Bar height** from 8 to 120 pixels.
It starts disabled and tracks requested slide
navigation, even while output is blank or preparing; it is separate from loading
progress. Use Save layout · This project or All projects to remember these choices.

## Slide background colors

In **Menu → Appearance**, **Page background** fills transparent areas of PDFs and
pictures (white by default). **Fit padding color** fills the space above/below or
beside a slide when its proportions differ from the display (black by default).
Both controls update the presenter previews, slide browser and audience output,
and are saved for future sessions. Opaque artwork keeps its original colors.
Blanking the audience output still shows black. Source documents remain unchanged.

## End of the deck

At the last slide, the presenter’s **Next** preview shows **End of slides**.
This placeholder is not a real slide: it cannot be selected or shown to the
audience, does not appear in All slides, and does not change the slide count.
Advancing at the end keeps the last real slide selected. Hiding the Next preview
also hides the placeholder.

## All slides on tiling Linux desktops

All slides remains a separate, resizable window. On Linux it identifies a presenter
window as its parent (Wayland) or transient owner (X11), and requests the X11 dialog
window type. These hints let compositors treat it as a floating auxiliary window.
[Niri automatically floats parented windows](https://github.com/niri-wm/niri/wiki/Floating-Windows),
unless a window rule overrides that behavior. No desktop configuration is changed.
The compositor can still override placement, including the selected browser monitor.

If your Niri rules still tile it, you can add this optional rule to your own config:

```kdl
window-rule {
    match app-id="^pdf-presenter$" title="^PDF Presenter — All slides$"
    open-floating true
}
```

See [Niri window rules](https://github.com/niri-wm/niri/wiki/Configuration%3A-Window-Rules).
The rule targets only All slides, not presenter or audience windows.

In All slides, double-click a thumbnail to open a larger, resizable preview inside
the browser. It does not change the presentation. Close it with its X or Escape.
When Change on click is enabled, clicks switch the live slide and do not open previews.

The All slides grid fills the browser width and adapts as you resize it. Preview
size sets the preferred thumbnail density; tiles share the available row width,
with slides fitted without distortion. The scrollbar stays at the right edge.

## Edit presenter notes

A `presentationX.notes.toml` file beside `presentationX.pdf` takes priority over PDF
annotation notes. Missing page entries fall back to the PDF; an empty entry hides
its embedded notes. For an image folder, the named file sits beside the folder and
uses image filenames as keys. See the [notes format](interfaces/presentation-notes.md).

**Double-click the notes pane** to edit that slide. If the named notes file does not
exist, PDF Presenter creates it with all notes found across the presentation before
opening the editable text. Creation and saving happen in the background.

Choose **Save notes** to update this page and its auto-fit presenter text. Other
slides and the source PDF remain unchanged. Close keeps saved changes;
**Discard changes and close** abandons an unsaved draft. A newly created sidecar
remains in place. The editor stays bound to the slide you opened, even if slide
navigation occurs elsewhere. Finish editing before opening another presentation.

Save errors keep your draft visible. If another program changed the notes file,
copy your draft before choosing **Reload (discard edits)**; the editor won't
silently replace a detected external edit. Notes files require a writable source
folder. They travel alongside the presentation, independently of saved layouts.


### Import notes from text or Markdown

With a presentation open, choose **Menu → Import notes…** or drop a `.txt`/`.md`
file onto the presenter. Review the destination and slide count, then choose
**Import and save notes** to create/update the presentation's `.notes.toml` file.
Cancel leaves files unchanged. See the [README examples](../README.md#import-your-notes)
and [format contract](interfaces/presentation-notes.md#importing-txt-and-markdown).


## Zoom and move around a slide

Zoom is **off by default**. Enable **Menu → Preferences → Enable page zoom**.
The preference is saved globally; it does not change presentation visibility.

By default, zoom expands the page into the available screen area without stretching.
For example, an A4 portrait page fills a widescreen display as you zoom in; drag to
see the cropped top/bottom. Each screen adapts to its own dimensions. To retain
the original page-shaped area and its margins, enable **Menu → Preferences →
Keep fitted page area when zooming**. This choice is saved globally. It changes
the visible area, not the cached image resolution.

- On the **current slide preview** or any **presentation screen**, scroll up/down
  to zoom in/out (1×–8×). The point under the mouse stays anchored when possible.
- Hold the left mouse button and drag to move the enlarged image. Movement stops
  at the page edges. When viewing the whole page, **double-click to fit its width**
  to the presentation screen (up to 8×) and aim the crop near the point clicked.
  Near page edges, the crop stops at the edge as closely as possible. Double-click again
  while zoomed to reset to the complete fitted page. Pages already filling the
  width stay unchanged. The optional fixed page area still applies if enabled.
- **Menu → Page zoom** also has a zoom slider and **Reset zoom** button. If the
  current preview is hidden, show it in Layout to pan from the presenter.
- All mirrored presenter/current previews and presentation outputs share zoom and
  page center. With zoom enabled, the current preview matches the first connected
  presentation output’s proportions, so its crop and fit-width level agree with
  that screen. Resizing the output updates the preview within about a second.
  Other mirrored outputs with different shapes can show different amounts of the page.
  Previous/next previews, All slides thumbnails and browser previews stay unzoomed.
  Moving to another slide or opening another presentation resets zoom. Display
  swaps/fullscreen changes preserve it. Zoom gestures pause during display trials.

Zoom uses the already buffered image: no PDF re-render, additional image buffer,
cache rewrite or disk access is triggered by changing the zoom. This prioritizes
response time; text/images may look softer when enlarged, especially while only a
low-resolution preview is available. Normal background preparation continues.

The Preferences disclaimer explains that software drawing can slow down on large
or multiple displays. If three consecutive zoomed frames take over 33 ms each to
render/present on a window, a **Zoom performance** warning appears on the presenter
with reset, disable and keep-enabled choices. This is a detected frame-budget
warning, not proof that zoom caused the delay. It does not blank or interrupt the
output. It cannot predict compositor/hardware delays or guarantee a frame rate.

### Disk cache cleanup

In **Menu → Preferences → Disk cache**, set the maximum total size in MiB and
lifetime in days since rendering (defaults: 10 GiB and 30 days). Automatic cleanup runs once at startup, after initial document loading and slide
preparation complete, deleting expired files and then the oldest files to meet the
size target. Changing limits takes effect at the next startup; manual expunge remains
available immediately.
**Expunge unused cache now** clears eligible cached slides immediately in the
background. Active decks are protected and may temporarily exceed either limit.
The cleanup report shows removed and remaining data; deleted slides regenerate
when reopened. Source documents and presenter notes are preserved.
