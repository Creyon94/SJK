"""Make the Hax toy's icon (`toy_hax`, SJK's own).

Hax is a meme toy: a player who has it on hacks another who has it on by holding `+use`
on them, then steers them for a few seconds. Nothing here comes from another game: the
icon is drawn from shapes and SJK's bundled JetBrains Mono font. This script writes,
into the output folder:

- `toy_hax.png`: the toy's icon, 128x128 like `gfx/mp/f_icon_*`: a dark terminal screen
  with a green ">_" prompt and a line of binary, on transparency.

    python scripts/hax_assets.py crates/sjk-viewer/assets/hax

The output is deterministic. Needs Pillow (pip install pillow).
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ICON_SIZE = 128
SCALE = 4
FONT = Path(__file__).resolve().parent.parent / "crates/sjk-viewer/assets/fonts/JetBrainsMono-Regular.ttf"
FRAME = (34, 40, 44, 255)
SCREEN = (8, 18, 12, 255)
GREEN = (70, 255, 120, 255)
DIM_GREEN = (40, 150, 70, 255)


def halo(canvas: Image.Image) -> Image.Image:
    """A soft dark outline under `canvas` so it reads on any ground, as the stock icons
    do; scaled down to the icon's size."""
    size = canvas.size[0]
    alpha = canvas.split()[3]
    shadow = alpha.filter(ImageFilter.MaxFilter(9)).filter(ImageFilter.GaussianBlur(3))
    ground = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ground.putalpha(shadow.point(lambda v: int(v * 0.7)))
    ground.alpha_composite(canvas)
    return ground.resize((ICON_SIZE, ICON_SIZE), Image.LANCZOS)


def terminal(output: Path) -> None:
    """The toy: a rounded screen in a frame, "01101" dim on top, ">_" bright below,
    the prompt glowing."""
    size = ICON_SIZE * SCALE
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas)
    margin = size * 0.1
    draw.rounded_rectangle(
        [margin, margin * 1.4, size - margin, size - margin * 1.4], radius=size * 0.08, fill=FRAME
    )
    inset = size * 0.045
    draw.rounded_rectangle(
        [margin + inset, margin * 1.4 + inset, size - margin - inset, size - margin * 1.4 - inset],
        radius=size * 0.05,
        fill=SCREEN,
    )
    small = ImageFont.truetype(str(FONT), int(size * 0.13))
    draw.text((size / 2, size * 0.34), "01101", font=small, fill=DIM_GREEN, anchor="mm")
    glow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    big = ImageFont.truetype(str(FONT), int(size * 0.34))
    ImageDraw.Draw(glow).text((size / 2, size * 0.6), ">_", font=big, fill=GREEN, anchor="mm")
    canvas.alpha_composite(glow.filter(ImageFilter.GaussianBlur(size * 0.02)))
    canvas.alpha_composite(glow)
    halo(canvas).save(output / "toy_hax.png")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    terminal(output)


if __name__ == "__main__":
    main()
