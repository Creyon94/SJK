"""Cut the settings menu's holocron icons out of Sol's holocron board.

Sol generated one board (Game_icon_grid_on_display_2K_20261013005251, 2752x1536,
13/10/2026): four rows of eight square holocron faces (a bronze frame, a glowing
blue crystal panel, a white-blue symbol) on a black ground. Each face is cut out
on transparency along its own outline (the frame's bevelled corners), with a soft
one-pixel edge, and scaled to the UI atlas cell, 128x128.

    python scripts/settings_icons.py board.jpg crates/sjk-viewer/assets/settings

Needs Pillow, numpy and scipy (pip install pillow numpy scipy).
"""

import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter
from scipy import ndimage

# The board's faces, row by row, left to right.
NAMES = [
    "first_setup", "video", "graphics", "sound", "mouse_joystick", "key_bindings", "game_options", "interface",
    "hud", "chat", "mods", "network", "scoreboard", "identity", "settings", "quick_wheel",
    "image", "lighting", "shadows", "weather", "camera", "crosshair", "nameplates", "movement",
    "interaction", "weapons", "force_powers", "other_controls", "changelog", "credits", "update", "import",
]
SIZE = 128
# Brighter than this is a face (frame or panel); the ground is near black.
GROUND = 45


def faces(image):
    """The faces' boxes (left, top, right, bottom), row by row, left to right."""
    luma = np.asarray(image.convert("L"), dtype=np.float32)
    mask = ndimage.binary_closing(luma > GROUND, iterations=3)
    mask = ndimage.binary_fill_holes(mask)
    labels, _ = ndimage.label(mask)
    found = []
    for rows, columns in ndimage.find_objects(labels):
        h, w = rows.stop - rows.start, columns.stop - columns.start
        # A face is large and about square; specks are not.
        if min(h, w) < 150 or abs(h - w) > 0.15 * max(h, w):
            continue
        found.append((columns.start, rows.start, columns.stop, rows.stop))
    if len(found) != len(NAMES):
        raise SystemExit(f"expected {len(NAMES)} faces, found {len(found)}")
    found.sort(key=lambda box: box[1])
    rows = []
    for box in found:
        if rows and box[1] - rows[-1][0][1] < (box[3] - box[1]) / 2:
            rows[-1].append(box)
        else:
            rows.append([box])
    return [box for row in rows for box in sorted(row, key=lambda box: box[0])]


def cut(image, box):
    """Face `box` on transparency along its outline, SIZE square."""
    left, top, right, bottom = box
    side = max(right - left, bottom - top) + 4
    cx, cy = (left + right) / 2, (top + bottom) / 2
    square = (int(cx - side / 2), int(cy - side / 2), int(cx + side / 2), int(cy + side / 2))
    crop = image.crop(square)
    luma = np.asarray(crop.convert("L"), dtype=np.float32)
    mask = ndimage.binary_fill_holes(ndimage.binary_closing(luma > GROUND, iterations=3))
    # Only the face itself (a neighbour's edge may reach into the square).
    labels, _ = ndimage.label(mask)
    centre = labels[labels.shape[0] // 2, labels.shape[1] // 2]
    alpha = Image.fromarray(((labels == centre) * 255).astype(np.uint8), "L")
    alpha = alpha.filter(ImageFilter.GaussianBlur(1.0))
    picture = crop.convert("RGBA")
    picture.putalpha(alpha)
    return picture.resize((SIZE, SIZE), Image.LANCZOS)


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: settings_icons.py board.jpg out_dir")
    board, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    image = Image.open(board).convert("RGB")
    for name, box in zip(NAMES, faces(image)):
        cut(image, box).save(out / f"{name}.png", optimize=True)
        print(f"{name}: {box}")


if __name__ == "__main__":
    main()
