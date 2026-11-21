# Embedded CJK font

`NotoSansCJK-Regular.ttc` is the unmodified Noto Sans CJK 2.004 collection used by
Canvas Pilot, under the included [SIL Open Font License 1.1](OFL.txt). Upstream:
<https://github.com/notofonts/noto-cjk/releases/tag/Sans2.004>.

SHA-256: `b76b0433203017ca80401b2ee0dd69350349871c4b19d504c34dbdd80541690a`.

PDF Presenter uses collection index 0 for Japanese and index 2 for Simplified
Chinese. The collection is embedded in the executable; no runtime font lookup or
download occurs. Font definitions are installed only when a CJK interface language
is active and only when the language selection changes. Other interface languages
retain egui's existing embedded fonts.
