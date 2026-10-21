#!/usr/bin/env python3
"""Draw the falling blossoms' petal pictures (`crates/sjk-viewer/assets/petals`,
`saber_petals.rs`): one cherry blossom petal, a rounded teardrop with the little notch at its
end, pale at its base and blushing toward its tip, in three tints.

    python3 scripts/petal_assets.py
"""
import math
from pathlib import Path
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "crates/sjk-viewer/assets/petals"
SIZE = 64
SAMPLES = 4
TINTS = {
    "blossom_pink": ((1.0, 0.86, 0.92), (1.0, 0.45, 0.68)),
    "blossom_pale": ((1.0, 0.95, 0.97), (1.0, 0.66, 0.82)),
    "blossom_white": ((1.0, 1.0, 1.0), (0.96, 0.84, 0.95)),
}


def inside(x, y):
    """Whether (x, y) in -1..1 (y from base -1 to tip 1) is in the petal, and how far along."""
    t = (y + 1.0) / 2.0
    if not 0.0 <= t <= 1.0:
        return False
    # a teardrop: narrow at the base, widest toward the tip, rounded there
    half = 0.8 * math.sin(math.pi * min(t, 0.999) ** 1.7) ** 0.8
    if abs(x) > half:
        return False
    # the notch at the tip
    if t > 0.8 and abs(x) < 0.16 * (t - 0.8) / 0.2 * 1.6 and t > 1.0 - 0.14 * (1.0 - abs(x) / 0.26):
        return False
    return True


def petal(base, tip):
    img = Image.new("RGBA", (SIZE, SIZE))
    px = img.load()
    for j in range(SIZE):
        for i in range(SIZE):
            hits, along = 0, 0.0
            for sj in range(SAMPLES):
                for si in range(SAMPLES):
                    x = ((i + (si + 0.5) / SAMPLES) / SIZE) * 2.0 - 1.0
                    y = 1.0 - ((j + (sj + 0.5) / SAMPLES) / SIZE) * 2.0
                    if inside(x * 1.1, y * 1.05):
                        hits += 1
                        along += (y + 1.0) / 2.0
            if not hits:
                px[i, j] = (0, 0, 0, 0)
                continue
            t = along / hits
            # a faint vein down the middle
            vein = 0.06 * math.exp(-(((i + 0.5) / SIZE * 2.0 - 1.0) / 0.05) ** 2) * (1.0 - t)
            c = [min(1.0, (b + (p - b) * t ** 1.4) - vein) for b, p in zip(base, tip)]
            a = hits / SAMPLES ** 2
            px[i, j] = tuple(int(round(v * 255)) for v in c) + (int(round(a * 235)),)
    return img


OUT.mkdir(parents=True, exist_ok=True)
for name, (base, tip) in TINTS.items():
    petal(base, tip).save(OUT / f"{name}.png", optimize=True)
    print("written", name)
