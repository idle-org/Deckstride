#!/usr/bin/env python3
"""Optional per-user Linux menu integration; keep the extracted bundle in place."""
import os
from pathlib import Path
import sys


def desktop_entry(binary, icon):
    # Desktop Entry strings and Exec have different escaping rules.
    def string(value):
        return str(value).replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r")
    command = str(binary).replace("%", "%%")
    for char in ("\\", '"', "`", "$"):
        command = command.replace(char, "\\" + char)
    return ('[Desktop Entry]\nType=Application\nName=Deckstride\n'
            'Comment=Multiscreen PDF and picture presenter\n'
            f'Exec="{string(command)}" %f\nIcon={string(icon)}\n'
            'Terminal=false\nCategories=Office;Presentation;\n'
            'MimeType=application/pdf;\nStartupWMClass=deckstride\n')


def main():
    if not sys.platform.startswith("linux"):
        raise SystemExit("This optional launcher is for Linux only.")
    root = Path(__file__).resolve().parent
    binary, icon = root / "deckstride", root / "deckstride.png"
    if not binary.is_file() or not icon.is_file():
        raise SystemExit("Run this script from the extracted portable Linux bundle.")
    data = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share")
    if not data.is_absolute():
        raise SystemExit("XDG_DATA_HOME must be absolute.")
    target = data / "applications/deckstride.desktop"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(desktop_entry(binary, icon), encoding="utf-8")
    print(f"Installed {target}. Keep the portable folder at {root}.")


if __name__ == "__main__":
    main()
