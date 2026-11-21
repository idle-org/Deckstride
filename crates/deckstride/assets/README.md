# Application artwork

`app-icon.png` is the unchanged generated master and repository avatar.
`window-icon.png` is its 256-pixel derivative embedded in all native windows
and used by the optional Linux desktop launcher. `app-icon.ico` contains
16/24/32/48/64/128/256-pixel derivatives embedded in the Windows executable.
Artwork is decoded once per GUI process, with no runtime disk reads.
See [generation provenance](../../../docs/development/icon-prompts.json).

Regenerate with ImageMagick:

```sh
magick app-icon.png -resize 256x256 window-icon.png
magick app-icon.png -define icon:auto-resize=256,128,64,48,32,24,16 app-icon.ico
```
