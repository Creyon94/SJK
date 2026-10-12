"""Cut the settings menu's icons out of Sol's gold-on-navy icon sheets.

Sol generated five sheets (Game_UI_icon_sheet / Sci-fi_game_UI_icon(s), 2752x1536,
13/10/2026), each holding two rows of four navy discs with gold rims and gold
symbols on a black ground, in the SJK UI's colours. Each disc is cut out as
wheel_icons.py cuts the wheels' (a circle a little inside its rim, anti-aliased,
scaled to the UI atlas cell, 128x128). Pass the sheets in the order they were
generated; a `None` name leaves that disc out (spares).

    python scripts/settings_icons.py sheet1.jpg ... sheet5.jpg crates/sjk-viewer/assets/settings

Needs Pillow, numpy and scipy (pip install pillow numpy scipy).
"""

import sys
from pathlib import Path

from PIL import Image

from wheel_icons import cut, discs

# Each sheet's discs, top row then bottom row, left to right.
SHEETS = [
    ["first_setup", "video", "graphics", "sound", "mouse_joystick", "key_bindings", ("game_options", "gameplay"), "interface"],
    ["hud", "chat", "mods", "network", None, "scoreboard", "identity", "settings"],
    ["image", "lighting", "shadows", "weather", "camera", "crosshair", "nameplates", None],
    ["movement", "interaction", "weapons", "force_powers", "other_controls", "changelog", "credits", "update"],
    ["import", None, None, None, None, None, None, None],
]
# How far inside the found rim the cut runs, in sheet pixels: the gold rim stays,
# the black ground around it goes (the discs are about 450 pixels round).
INSET = 6.0


def main():
    if len(sys.argv) != len(SHEETS) + 2:
        raise SystemExit(f"usage: settings_icons.py {' '.join(f'sheet{i + 1}.jpg' for i in range(len(SHEETS)))} out_dir")
    out = Path(sys.argv[-1])
    out.mkdir(parents=True, exist_ok=True)
    for sheet, names in zip(sys.argv[1:-1], SHEETS):
        image = Image.open(sheet).convert("RGB")
        for name, (x, y, radius) in zip(names, discs(image, len(names), lower_half=False)):
            if name is None:
                continue
            icon = cut(image, x, y, radius, INSET)
            for each in name if isinstance(name, tuple) else (name,):
                icon.save(out / f"{each}.png", optimize=True)
                print(f"{each}: centre ({x:.0f}, {y:.0f}), radius {radius:.0f}")


if __name__ == "__main__":
    main()
