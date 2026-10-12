"""Cut the settings menu's icons out of Sol's gold-on-navy icon sheets.

Sol generated five sheets (Game_UI_icon_sheet / Sci-fi_game_UI_icon(s), 2752x1536,
13/10/2026), each holding two rows of four navy discs with gold rims and gold
symbols on a black ground, in the SJK UI's colours. Pass the sheets in the order
they were generated; a `None` name leaves that disc out (spares).

The sheets' discs carry fine detail (spokes on the face, tick marks on a double
rim) that turns to noise at the rail's 38 pixels, so only each disc's gold symbol
is kept, a little bolder and in one clean gold, on a disc drawn again: a flat navy face and one smooth
gold rim, the same on every icon. The quick wheel's icon, which no sheet holds,
is drawn here on the same disc: a ring of eight small discs round a centre one.

    python scripts/settings_icons.py sheet1.jpg ... sheet5.jpg crates/sjk-viewer/assets/settings

Needs Pillow, numpy and scipy (pip install pillow numpy scipy).
"""

import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

from wheel_icons import discs

# Each sheet's discs, top row then bottom row, left to right.
SHEETS = [
    ["first_setup", "video", "graphics", "sound", "mouse_joystick", "key_bindings", "game_options", "interface"],
    ["hud", "chat", "mods", "network", None, "scoreboard", "identity", "settings"],
    ["image", "lighting", "shadows", "weather", "camera", "crosshair", "nameplates", None],
    ["movement", "interaction", "weapons", "force_powers", "other_controls", "changelog", "credits", "update"],
    ["import", None, None, None, None, None, None, None],
]
SIZE = 128
# The disc is drawn this many times larger, then scaled down: smooth edges.
WORK = 512
# Of the disc's radius: the gold symbol is kept inside this, the rim drawn from
# RIM_INNER outwards.
SYMBOL = 0.80
RIM_INNER = 0.90
FACE_CENTRE = (24, 34, 62)
FACE_EDGE = (13, 19, 38)
GOLD_LIGHT = (240, 210, 140)
GOLD_DARK = (176, 132, 60)


def gold_alpha(rgb):
    """How gold each pixel is, 0 to 1: warm (red over blue) and not dark."""
    r, g, b = (rgb[..., i].astype(np.float32) for i in range(3))
    # Solid wherever the sheet is gold, dark gold included: its shading is
    # dropped, only the shape is kept.
    warm = np.clip((r - b - 20.0) / 25.0, 0.0, 1.0)
    lit = np.clip((r - 55.0) / 25.0, 0.0, 1.0)
    return warm * lit


def symbol(image, x, y, radius):
    """The gold symbol of the disc at (x, y), at WORK size, on transparency, its
    strokes thickened so they hold at small sizes."""
    half = radius * SYMBOL
    box = (int(x - half), int(y - half), int(x + half), int(y + half))
    crop = image.crop(box).resize((int(WORK * SYMBOL), int(WORK * SYMBOL)), Image.LANCZOS)
    rgb = np.asarray(crop, dtype=np.float32)
    alpha = gold_alpha(rgb)
    # Only inside the symbol's circle (the rim's gold stays out).
    size = alpha.shape[0]
    ys, xs = np.mgrid[0:size, 0:size]
    inside = np.hypot(xs - size / 2 + 0.5, ys - size / 2 + 0.5) < size / 2 - 2
    alpha *= inside
    # Bolder: the shape's maximum over a small neighbourhood.
    mask = Image.fromarray((alpha * 255.0).astype(np.uint8), "L").filter(ImageFilter.MaxFilter(5))
    # One clean gold, light at the top, in place of the sheet's metallic shading,
    # whose streaks read as noise when small.
    t = (ys / size)[..., None]
    gold = np.array(GOLD_LIGHT) * (1 - t) + np.array(GOLD_DARK) * t
    return Image.fromarray(
        np.dstack([gold, np.asarray(mask, dtype=np.float32)]).astype(np.uint8), "RGBA"
    )


def disc(glyph):
    """A navy disc with a smooth gold rim and `glyph` (WORK*SYMBOL square) in its
    middle, scaled down to SIZE."""
    canvas = np.zeros((WORK, WORK, 4), dtype=np.float32)
    ys, xs = np.mgrid[0:WORK, 0:WORK]
    centre = WORK / 2
    distance = np.hypot(xs + 0.5 - centre, ys + 0.5 - centre) / centre
    # Face: navy, lighter in the middle.
    t = np.clip(distance, 0.0, 1.0)[..., None]
    face = np.array(FACE_CENTRE) * (1 - t) + np.array(FACE_EDGE) * t
    canvas[..., :3] = face
    canvas[..., 3] = np.clip((1.0 - distance) * centre, 0.0, 1.0) * 255.0
    # Rim: gold, lit from the top left.
    light = np.clip(((centre - xs) + (centre - ys)) / (2 * centre) * 0.5 + 0.5, 0.0, 1.0)[..., None]
    gold = np.array(GOLD_DARK) * (1 - light) + np.array(GOLD_LIGHT) * light
    rim = np.clip((distance - RIM_INNER) * centre, 0.0, 1.0)[..., None]
    canvas[..., :3] = canvas[..., :3] * (1 - rim) + gold * rim
    picture = Image.fromarray(canvas.astype(np.uint8), "RGBA")
    if glyph is not None:
        offset = (WORK - glyph.width) // 2
        layer = Image.new("RGBA", (WORK, WORK))
        layer.paste(glyph, (offset, offset))
        picture = Image.alpha_composite(picture, layer)
    return picture.resize((SIZE, SIZE), Image.LANCZOS)


def wheel_glyph():
    """The quick wheel's symbol: eight small gold discs on a ring round a larger
    centre one, as the wheel's own icon draws it."""
    size = int(WORK * SYMBOL)
    glyph = Image.new("RGBA", (size, size))
    draw = ImageDraw.Draw(glyph)
    c = size / 2
    ring, small, big = size * 0.31, size * 0.07, size * 0.11
    for i in range(8):
        angle = i * np.pi / 4
        px, py = c + ring * np.cos(angle), c + ring * np.sin(angle)
        draw.ellipse((px - small, py - small, px + small, py + small), fill=GOLD_LIGHT + (255,))
    draw.ellipse((c - big, c - big, c + big, c + big), fill=GOLD_LIGHT + (255,))
    width = int(size * 0.025)
    draw.ellipse((c - ring, c - ring, c + ring, c + ring), outline=GOLD_DARK + (255,), width=width)
    return glyph


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
            disc(symbol(image, x, y, radius)).save(out / f"{name}.png", optimize=True)
            print(f"{name}: centre ({x:.0f}, {y:.0f}), radius {radius:.0f}")
    disc(wheel_glyph()).save(out / "quick_wheel.png", optimize=True)
    print("quick_wheel: drawn")


if __name__ == "__main__":
    main()
