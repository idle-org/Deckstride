# Deckstride language and theme packs

Install either example from **Preferences → General → Appearance**. Packs remain
ZIP files and are copied into Deckstride's configuration directory; they contain
data only and are never extracted or executed.

## Language pack

`deckstride-language-pack.zip` uses the same version-2 manifest shape as Canvas
Pilot: `manifest.json` and one root `<locale>.json` per language. Catalog keys are
Deckstride's exact visible English source strings, and missing keys fall back to
English. A pack may contain 1–16 languages. Pack/locale IDs use lowercase ASCII
letters, digits and hyphens. Increase the positive `revision` to publish an update.

Limits: 2 MiB ZIP, 4 MiB expanded data, 32 installed packs, 4,096 messages per
catalog, and no paths, directories, symlinks or unexpected files.

## Theme pack

`deckstride-theme-pack.zip` is a Canvas Pilot format-1 theme pack containing one
root `manifest.json`. It can contain 1–16 complete themes. Colors are opaque
`#RRGGBB` values using the shared Canvas Pilot roles. `light` selects egui's light
or dark base behavior. Keep license and attribution text in `notices`.

Limits: 2 MiB ZIP, 4 MiB manifest, 32 installed packs, and no executable content.

Higher revisions replace an installed pack with the same ID. Equal or older
revisions are rejected, preventing accidental downgrades. Invalid packs are skipped
at startup and reported in the Presenter status line.
