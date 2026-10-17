"""Make the Illuminate holocron's game files from Sol's two generated pictures,
and the four loot-box holocrons from the shipped face.

Illuminate is SJK's own toy (`toy_illuminate`): a holocron that floats by the player's
shoulder and lights the way. Sol generated its pictures (07/10/2026): a holocron
icon in the style of Jedi Academy's Force icons (1024x1024, on a flat grey ground)
and one face of the holocron (2048x2048). This script writes, into the output
folder:

- `toy_illuminate.png`: the icon cut out of its ground with its amber glow, cube
  at the stock icons' size, 128x128 like `gfx/mp/f_icon_*`;
- `holocron.jpg`: the face, 512x512;
- `holocron_glow.jpg`: the face's emblem alone on black, for the additive stage that
  keeps it lit in the dark;
- `holocron.md3`: a cube of HOLOCRON_EDGE units with the face on all six sides.

    python scripts/holocron_assets.py icon.jpg face.jpg crates/sjk-viewer/assets/holocron

That also remakes the tiers (below) from the new face. The tiers alone, from the
shipped `holocron.jpg` in the folder (no originals needed):

    python scripts/holocron_assets.py tiers crates/sjk-viewer/assets/holocron

The tiers (uncommon green, rare blue, legendary purple, mythical gold) are each an
object of their own (`holocron_forms.py`: the Jedi cube, the Great Holocron, the Sith
pyramid, the kyber prism), a model per part so the client can move the parts. Per tier
it writes the parts' models (`holocron_<tier>.md3`, the body, and
`holocron_<tier>_<part>.md3`, each also as `..._locked.md3`), the face picture
`holocron_<tier>.jpg` with its `_glow.jpg` and `_locked.jpg`, the lit parts' picture
`holocron_<tier>_light.jpg`, and the UI icons `holocron_<tier>.png` (256x256, drawn from
the parts) and `holocron_<tier>_small.png` (64x64). Shared: the steel and the dark metal
(`holocron_steel.jpg`, `holocron_dark.jpg`, and locked), the locked light, the mythical
sheen, and three shaders (`holocron_tiers.shader`: the faces; `holocron_parts.shader`:
steel, dark metal and the lights; `holocron_locked.shader`: the locked look, every
material drained and dark, without glow). The Jedi cube's face is the shipped face in
the tier's colour; the other three faces are drawn here until Sol's art replaces them:

    python scripts/holocron_assets.py tiers crates/sjk-viewer/assets/holocron         --art rare=pentagon.png --art legendary=sith.png --art mythical=crystal.png

A given picture is square, in any colour: its bright parts are painted along the tier's
ramp and glow. The rare one is a pentagon, a corner at the top, its corners on the
picture's inscribed circle; the legendary one a trapezoid, its foot along the bottom
edge and its top PYRAMID_TOP as wide, centred; the mythical one a facet of the crystal,
its tip at the top middle and its foot along the bottom edge. The whole table (colours,
glow ladder, shader waves, point lights) is TIERS below; the output is deterministic.

Needs Pillow, numpy and scipy (pip install pillow numpy scipy).
"""

import math
import struct
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage
from scipy.spatial import ConvexHull

import holocron_forms as forms

ICON_SIZE = 128
# The icon's flat ground, and the share of the frame the cube's box takes (the
# stock icons' cubes span about two thirds of theirs).
ICON_GROUND = 39.0
ICON_CUBE_SHARE = 0.68
ICON_GLOW = np.array([255.0, 168.0, 72.0])
FACE_SIZE = 512
HOLOCRON_EDGE = 6.0
SHADER = "models/sjk/holocron"
JPEG_QUALITY = 88


def icon(source: Path, output: Path) -> None:
    a = np.asarray(Image.open(source).convert("RGB")).astype(float)
    r, b = a[..., 0], a[..., 2]
    # The cube is grey-blue and off the ground; its halo is orange (red over blue).
    cube = (np.abs(a - ICON_GROUND).max(-1) > 14) & (b >= r - 6)
    cube = ndimage.binary_opening(cube, iterations=2)
    labels, count = ndimage.label(cube)
    sizes = ndimage.sum(cube, labels, range(1, count + 1))
    cube = labels == 1 + int(np.argmax(sizes))
    ys, xs = np.nonzero(cube)
    points = np.stack([xs, ys], 1)
    hull = ConvexHull(points)
    mask = Image.new("L", (a.shape[1], a.shape[0]), 0)
    ImageDraw.Draw(mask).polygon([tuple(map(float, points[i])) for i in hull.vertices], fill=255)
    cube_alpha = ndimage.gaussian_filter(np.asarray(mask).astype(float) / 255.0, 0.8)
    # The halo's coverage from its red channel over the ground, in one amber.
    halo_alpha = np.clip((r - ICON_GROUND) / (ICON_GLOW[0] - ICON_GROUND), 0, 1)
    alpha = np.maximum(cube_alpha, halo_alpha)
    rgb = np.where(cube_alpha[..., None] > 0.5, a, ICON_GLOW)
    picture = Image.fromarray(np.dstack([rgb, alpha * 255]).clip(0, 255).astype(np.uint8), "RGBA")
    centre_x, centre_y = (xs.min() + xs.max()) / 2, (ys.min() + ys.max()) / 2
    side = max(xs.max() - xs.min(), ys.max() - ys.min()) / ICON_CUBE_SHARE
    box = tuple(
        round(v)
        for v in (centre_x - side / 2, centre_y - side / 2, centre_x + side / 2, centre_y + side / 2)
    )
    picture = picture.crop(box).convert("RGBa").resize((ICON_SIZE, ICON_SIZE), Image.LANCZOS)
    picture.convert("RGBA").save(output / "toy_illuminate.png", optimize=True)


def face(source: Path, output: Path) -> None:
    picture = Image.open(source).convert("RGB").resize((FACE_SIZE, FACE_SIZE), Image.LANCZOS)
    a = np.asarray(picture).astype(float)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    ys, xs = np.mgrid[0:FACE_SIZE, 0:FACE_SIZE]
    from_centre = np.hypot(xs - FACE_SIZE / 2, ys - FACE_SIZE / 2) / FACE_SIZE
    # The emblem and its baked halo are warm; the rays' cores are near white, so
    # inside the emblem's circle brightness counts too. Rust specks on the plates
    # are warm as well, hence the circles.
    warm = np.clip((r - b - 12) / 90.0, 0, 1) * (from_centre < 0.37)
    luminance = 0.3 * r + 0.59 * g + 0.11 * b
    bright = np.clip((luminance - 150) / 70.0, 0, 1) * (from_centre < 0.246)
    emblem = ndimage.gaussian_filter(np.maximum(warm, bright), 0.7)
    picture.save(output / "holocron.jpg", quality=92)
    glow = (a * emblem[..., None]).clip(0, 255).astype(np.uint8)
    Image.fromarray(glow).save(output / "holocron_glow.jpg", quality=92)




def encode_normal(n) -> int:
    # The inverse of sjk-model's `decode_normal` (stock's MD3 normal): the high
    # byte is the angle around Z, the low byte the angle from +Z.
    around = round(math.atan2(n[1], n[0]) * 255 / (2 * math.pi)) & 0xFF
    from_up = round(math.acos(max(-1.0, min(1.0, n[2]))) * 255 / (2 * math.pi)) & 0xFF
    return (around << 8) | from_up


def md3_name(text: str, size: int) -> bytes:
    return text.encode("ascii").ljust(size, b"\0")


def write_md3(output: Path, file: str, surfaces) -> None:
    """Write `<file>.md3`: one frame, a surface per `(shader, positions, normals, uvs,
    triangles)`, triangles clockwise seen from outside (stock's front side).

    The renderer takes a surface's shader from the model file, so a look that is not
    the base one needs its own model: `holocron_<tier>.md3` names `models/sjk/holocron_<tier>`
    and the locked look's models their locked shaders (see `tiers`).
    """
    every = np.array([p for _, positions, _, _, _ in surfaces for p in positions], float)
    low, high = every.min(0), every.max(0)
    radius = float(np.linalg.norm(every, axis=1).max())
    blocks = []
    for index, (shader, positions, normals, uvs, triangles) in enumerate(surfaces):
        shaders = md3_name(shader, 64) + struct.pack("<i", 0)
        tris = b"".join(struct.pack("<3i", *t) for t in triangles)
        sts = b"".join(struct.pack("<2f", *uv) for uv in uvs)
        verts = b"".join(
            struct.pack("<3hH", *(round(c * 64) for c in p), encode_normal(n))
            for p, n in zip(positions, normals)
        )
        surface_header = 108
        ofs_shaders = surface_header
        ofs_tris = ofs_shaders + len(shaders)
        ofs_st = ofs_tris + len(tris)
        ofs_verts = ofs_st + len(sts)
        surface_end = ofs_verts + len(verts)
        blocks.append(
            b"IDP3"
            + md3_name("holocron" if index == 0 else f"holocron_{index}", 64)
            + struct.pack(
                "<10i", 0, 1, 1, len(positions), len(triangles),
                ofs_tris, ofs_shaders, ofs_st, ofs_verts, surface_end,
            )
            + shaders + tris + sts + verts
        )
    frame = struct.pack("<10f", *low, *high, 0, 0, 0, radius) + md3_name("holocron", 16)
    header_size = 108
    ofs_frames = header_size
    ofs_surfaces = ofs_frames + len(frame)
    body = b"".join(blocks)
    end = ofs_surfaces + len(body)
    header = (
        b"IDP3"
        + struct.pack("<i", 15)
        + md3_name(f"models/sjk/{file}.md3", 64)
        + struct.pack("<9i", 0, 1, 0, len(blocks), 0, ofs_frames, ofs_surfaces, ofs_surfaces, end)
    )
    (output / f"{file}.md3").write_bytes(header + frame + body)


def cube_md3(output: Path, file: str = "holocron", shader: str = SHADER) -> None:
    """Write Illuminate's cube as `<file>.md3` with the surface shader `shader`."""
    h = HOLOCRON_EDGE / 2
    # Each face: outward normal and the picture's up; right = up x normal, so the
    # picture reads upright seen from outside.
    faces = [
        ((1, 0, 0), (0, 0, 1)),
        ((-1, 0, 0), (0, 0, 1)),
        ((0, 1, 0), (0, 0, 1)),
        ((0, -1, 0), (0, 0, 1)),
        ((0, 0, 1), (1, 0, 0)),
        ((0, 0, -1), (1, 0, 0)),
    ]
    positions, normals, uvs, triangles = [], [], [], []
    for normal, up in faces:
        n, u = np.array(normal, float), np.array(up, float)
        right = np.cross(u, n)
        base = len(positions)
        for s, t in ((0, 0), (1, 0), (1, 1), (0, 1)):
            positions.append(n * h + right * (2 * s - 1) * h + u * (1 - 2 * t) * h)
            normals.append(n)
            uvs.append((s, t))
        # Clockwise seen from outside: stock's front side.
        triangles += [(base, base + 1, base + 2), (base, base + 2, base + 3)]
    write_md3(output, file, [(shader, positions, normals, uvs, triangles)])


# ---------------------------------------------------------------------------
# The loot-box tiers.

TIER_ICON_SIZE = 256
TIER_SMALL_SIZE = 64
# Seen from the centre of the face, the ring is an octagon: flat sides at m = |dx| or
# |dy| and corners cut where (|dx| + |dy|) / RING_CUT = m.
RING_CUT = 1.475
# How much of the icon the holocron spans; the rest is the halo's room.
ICON_FILL = 0.8
SHEEN_SIZE = 128
SHEEN_NAME = "holocron_mythical_sheen"
# The look of a tier the player holds none of: every picture with its colour drained and
# its light put out, its own shaders without glow (the tier shows in the dim point light
# the Holocrons tab gives it, and in the shape).
LOCKED_KEEP = 0.14
LOCKED_DIM = 0.42
# The Sith pyramid's side: its top's width against its foot's (`holocron_forms.py`).
PYRAMID_TOP = 1.6 / 3.0


@dataclass(frozen=True)
class Tier:
    """One tier's look. Colours are RGB 0..255, the rest scales.

    `ramp` is the emblem's gradient from its dark edge to its white-hot core; the
    tiers climb in how much they glow (`ring`: the share of the ring's runes and bevels
    lit, `halo`: the emblem's light on the plate, `tint`: the plate's tint, `emblem`:
    the emblem's brightness), so they tell apart by that and by lightness, not by hue
    alone. `light` is the point light's colour in illuminate.rs's COLOR scale
    ([1.5, 1.3, 1.0]). `self_glow` and `breath` are the shader's waves (base,
    amplitude, phase, frequency): the face's own light, and the emblem's (the lit
    parts' too).
    """

    id: str
    label: str
    ramp: tuple
    emblem: float
    ring: float
    halo: float
    tint: float
    light: tuple
    self_glow: tuple
    breath: tuple


TIERS = (
    Tier(
        "uncommon", "Uncommon (green)",
        ((6, 46, 18), (52, 178, 76), (150, 240, 160), (226, 255, 228)),
        emblem=0.80, ring=0.12, halo=0.08, tint=0.10,
        light=(0.55, 1.50, 0.60),
        self_glow=(0.35, 0.0, 0.0, 0.0), breath=(0.40, 0.10, 0.0, 0.30),
    ),
    Tier(
        "rare", "Rare (blue)",
        ((4, 24, 88), (36, 120, 255), (140, 196, 255), (222, 240, 255)),
        emblem=0.90, ring=0.40, halo=0.16, tint=0.14,
        light=(0.50, 1.00, 1.75),
        self_glow=(0.40, 0.0, 0.0, 0.0), breath=(0.50, 0.15, 0.0, 0.35),
    ),
    Tier(
        "legendary", "Legendary (purple)",
        ((34, 6, 82), (176, 64, 255), (226, 160, 255), (250, 228, 255)),
        emblem=1.00, ring=0.72, halo=0.26, tint=0.18,
        light=(1.25, 0.45, 1.75),
        self_glow=(0.45, 0.0, 0.0, 0.0), breath=(0.60, 0.20, 0.0, 0.40),
    ),
    Tier(
        "mythical", "Mythical (gold, shiny)",
        ((92, 40, 0), (255, 186, 38), (255, 232, 130), (255, 252, 226)),
        emblem=1.00, ring=1.00, halo=0.38, tint=0.22,
        light=(1.85, 1.30, 0.40),
        self_glow=(0.45, 0.20, 0.25, 0.35), breath=(0.85, 0.35, 0.0, 0.55),
    ),
)


def smoothstep(edge0, edge1, x):
    t = np.clip((x - edge0) / (edge1 - edge0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def octagon_metric(n: int, cut: float):
    ys, xs = np.mgrid[0:n, 0:n]
    dx, dy = np.abs(xs - (n - 1) / 2), np.abs(ys - (n - 1) / 2)
    return np.maximum(np.maximum(dx, dy), (dx + dy) / cut)


def face_masks(face_rgb: np.ndarray):
    """The shipped face's emblem (with its halo) and ring (runes and bevels), 0..1."""
    n = face_rgb.shape[0]
    r, g, b = face_rgb[..., 0], face_rgb[..., 1], face_rgb[..., 2]
    ys, xs = np.mgrid[0:n, 0:n]
    from_centre = np.hypot(xs - n / 2, ys - n / 2) / n
    luminance = 0.3 * r + 0.59 * g + 0.11 * b
    # As face() cuts the emblem, but inside the emblem's own reach only: the base's
    # glow picture keeps a few warm specks of the ring, a tier's must not.
    warm = np.clip((r - b - 12) / 90.0, 0, 1)
    bright = np.clip((luminance - 150) / 70.0, 0, 1) * (from_centre < 0.246)
    reach = 1.0 - smoothstep(0.27, 0.31, from_centre)
    emblem = ndimage.gaussian_filter(np.maximum(warm, bright) * reach, 0.7)
    # The ring: the bevels' and runes' highlights inside the octagonal band.
    metric = octagon_metric(n, RING_CUT)
    band = smoothstep(126, 130, metric) * (1.0 - smoothstep(179, 183, metric))
    high_pass = luminance - ndimage.gaussian_filter(luminance, 6)
    ring = np.clip(high_pass / 25.0, 0, 1) * band * (1.0 - emblem)
    return emblem, ring


def ramp(values: np.ndarray, stops) -> np.ndarray:
    """Colour the 0..1 `values` along the four stops of a tier's ramp."""
    at = [0.12, 0.50, 0.80, 0.97]
    return np.stack([np.interp(values, at, [stop[c] for stop in stops]) for c in range(3)], -1)


def luminance_of(rgb: np.ndarray) -> np.ndarray:
    return 0.3 * rgb[..., 0] + 0.59 * rgb[..., 1] + 0.11 * rgb[..., 2]


def tier_layers(face_rgb: np.ndarray, tier: Tier):
    """The Jedi cube's face and glow pictures (float RGB 0..255) from the base face."""
    emblem, ring = face_masks(face_rgb)
    luminance = luminance_of(face_rgb)[..., None]
    paint = ramp(luminance[..., 0] / 255.0, tier.ramp) * tier.emblem
    mid = np.array(tier.ramp[1], float)
    pale = np.array(tier.ramp[2], float)
    # The plate: steel with a faint tint of the tier's colour, the emblem's light
    # spilling over it, the ring's bevels and runes lit.
    plate = face_rgb + tier.tint * (luminance * (mid / mid.max()) - face_rgb)
    spill = ndimage.gaussian_filter(emblem, 14)[..., None]
    plate = plate + tier.halo * spill * pale
    ring_light = tier.ring * ring[..., None]
    plate = plate + ring_light * (mid * 0.55 + pale * 0.45)
    face = plate * (1.0 - emblem[..., None]) + paint * emblem[..., None]
    # The glow picture, on black: the emblem and the lit ring (what the additive
    # stage and the glow pass pick up).
    glow = paint * emblem[..., None] + ring_light * (mid * 0.55 + pale * 0.45)
    return face.clip(0, 255), glow.clip(0, 255)


def art_layers(picture: Path, tier: Tier):
    """A face picture given on the command line (Sol's art for a tier's form, any size,
    square): the picture lightly tinted in the tier's colour, its bright parts painted
    along the tier's ramp and glowing."""
    rgb = np.asarray(
        Image.open(picture).convert("RGB").resize((FACE_SIZE, FACE_SIZE), Image.LANCZOS)
    ).astype(float)
    luminance = luminance_of(rgb)
    lit = ndimage.gaussian_filter(smoothstep(150.0, 225.0, luminance), 0.7)[..., None]
    paint = ramp(luminance / 255.0, tier.ramp) * tier.emblem
    mid = np.array(tier.ramp[1], float)
    plate = rgb + tier.tint * (luminance[..., None] * (mid / mid.max()) - rgb)
    face = plate * (1.0 - lit) + paint * lit
    return face.clip(0, 255), (paint * lit).clip(0, 255)


def noise_field(n: int, seed: int, blur, wrap: bool = True) -> np.ndarray:
    """Smooth noise of unit spread, tiling when `wrap`."""
    rng = np.random.default_rng(seed)
    field = ndimage.gaussian_filter(rng.normal(size=(n, n)), blur, mode="wrap" if wrap else "reflect")
    return field / field.std()


def steel_picture(base: np.ndarray, n: int = 256) -> np.ndarray:
    """Brushed steel in the base face's plate colour, tiling."""
    plate = base[16:96, 16:96].reshape(-1, 3).mean(0)
    brushed = noise_field(n, 11, (0.6, 18.0))
    mottle = noise_field(n, 12, 9.0)
    value = 1.0 + 0.07 * brushed + 0.08 * mottle
    return (plate * 1.05 * value[..., None]).clip(0, 255)


def dark_picture(n: int = 256) -> np.ndarray:
    """The Sith pyramid's dark metal, a little warm, tiling."""
    mottle = noise_field(n, 13, 7.0)
    grain = noise_field(n, 14, 1.2)
    value = 64.0 + 9.0 * mottle + 3.0 * grain
    return (value[..., None] * np.array((1.04, 0.93, 1.12))).clip(0, 255)


def tiled(picture: np.ndarray, n: int) -> np.ndarray:
    reps = -(-n // picture.shape[0])
    return np.tile(picture, (reps, reps, 1))[:n, :n]


def light_picture(tier: Tier, n: int = 64) -> np.ndarray:
    """The tier's lit parts (lamps, edges, halo, seam, eye, motes): its light colour."""
    mid = np.array(tier.ramp[1], float)
    pale = np.array(tier.ramp[2], float)
    mottle = noise_field(n, 15, 4.0)
    colour = (0.45 * mid + 0.55 * pale) * tier.emblem
    return (colour * (1.0 + 0.05 * mottle[..., None])).clip(0, 255)


def segment_distance(xs, ys, a, b):
    ax, ay = a
    bx, by = b
    dx, dy = bx - ax, by - ay
    t = np.clip(((xs - ax) * dx + (ys - ay) * dy) / (dx * dx + dy * dy), 0, 1)
    return np.hypot(xs - ax - t * dx, ys - ay - t * dy)


def pentagon_layers(steel: np.ndarray, tier: Tier):
    """The Great Holocron's face: a steel pentagon (its corners on the picture's
    inscribed circle, one at the top) with a bevelled rim, a ring of lit runes, and a
    glowing pentagon in the middle with a star."""
    n = FACE_SIZE
    ys, xs = (np.mgrid[0:n, 0:n] + 0.5) / n - 0.5
    up = -ys
    normals = [math.radians(126 + 72 * k) for k in range(5)]
    apothem = np.max([xs * math.cos(a) + up * math.sin(a) for a in normals], 0)
    edge = 0.5 * math.cos(math.radians(36))
    mid = np.array(tier.ramp[1], float)
    pale = np.array(tier.ramp[2], float)
    plate = tiled(steel, n) * 0.9
    # The rim's bevel, lit from the top left, and a groove inside it.
    bevel = smoothstep(edge - 0.045, edge - 0.03, apothem)
    slope = np.clip(0.5 - (xs + ys), 0, 1)[..., None]
    plate = plate * (1.0 - bevel[..., None]) + plate * (0.7 + 0.7 * slope) * bevel[..., None]
    groove = smoothstep(0.012, 0.0, np.abs(apothem - (edge - 0.07)))
    plate = plate * (1.0 - 0.55 * groove[..., None])
    # Runes: dashes around a pentagonal band, a few per side.
    angle = np.arctan2(up, xs)
    band = smoothstep(0.014, 0.004, np.abs(apothem - 0.29))
    dashes = (np.sin(angle * 25.0) > 0.25) * band
    dots = smoothstep(0.012, 0.004, np.abs(apothem - 0.255)) * (np.sin(angle * 40.0) > 0.85)
    runes = ndimage.gaussian_filter(np.maximum(dashes, dots).astype(float), 0.6)
    # The middle: glass in the tier's colour, a star and a ring of lines.
    inner = smoothstep(0.215, 0.205, apothem)
    glass = ramp(0.38 + 0.4 * (1.0 - apothem / 0.215).clip(0, 1), tier.ramp) * tier.emblem
    corners = [(0.165 * math.cos(math.radians(90 + 72 * k)), 0.165 * math.sin(math.radians(90 + 72 * k))) for k in range(5)]
    star = np.min([segment_distance(xs, up, corners[k], corners[(k + 2) % 5]) for k in range(5)], 0)
    lines = np.maximum(smoothstep(0.009, 0.003, star), smoothstep(0.007, 0.002, np.abs(apothem - 0.19)))
    lines = np.maximum(lines, smoothstep(0.045, 0.035, np.hypot(xs, up)))
    paint_lines = ramp(np.full_like(xs, 0.95), tier.ramp) * tier.emblem
    spill = ndimage.gaussian_filter(inner, 14)[..., None]
    plate = plate + tier.halo * spill * pale
    rune_light = (0.35 + 0.65 * tier.ring) * runes[..., None] * (mid * 0.55 + pale * 0.45)
    plate = plate * (1.0 - runes[..., None] * 0.6) + rune_light
    lit = inner[..., None]
    middle = glass * (1.0 - lines[..., None]) + paint_lines * lines[..., None]
    face = plate * (1.0 - lit) + middle * lit
    glow = middle * lit * 0.8 + rune_light
    return face.clip(0, 255), glow.clip(0, 255)


def glyph_mask(n: int, centres, size: float, seed: int) -> np.ndarray:
    """Made-up runes at `centres` (picture fractions), each a few strokes between the
    points of a three by three grid, drawn large and brought down smooth."""
    rng = np.random.default_rng(seed)
    big = n * 4
    mask = Image.new("L", (big, big), 0)
    draw = ImageDraw.Draw(mask)
    for cx, cy in centres:
        grid = [(cx + (i - 1) * size / 2, cy + (j - 1) * size / 2) for j in range(3) for i in range(3)]
        strokes = [(1, 7)] + [tuple(rng.choice(9, 2, replace=False)) for _ in range(3)]
        for a, b in strokes:
            draw.line(
                [(grid[a][0] * big, grid[a][1] * big), (grid[b][0] * big, grid[b][1] * big)],
                fill=255, width=max(1, round(size * big * 0.14)),
            )
    return np.asarray(mask.resize((n, n), Image.LANCZOS)).astype(float) / 255.0


def sith_layers(dark: np.ndarray, tier: Tier):
    """The Sith pyramid's side: dark metal on the side's trapezoid (its foot along the
    picture's bottom, its top PYRAMID_TOP as wide), an engraved border with a lit line,
    and a column of runes glowing in the tier's colour."""
    n = FACE_SIZE
    ys, xs = (np.mgrid[0:n, 0:n] + 0.5) / n
    half = 0.5 * (PYRAMID_TOP + (1.0 - PYRAMID_TOP) * ys)
    across = np.abs(xs - 0.5) / half
    mid = np.array(tier.ramp[1], float)
    pale = np.array(tier.ramp[2], float)
    plate = tiled(dark, n).copy()
    streaks = noise_field(n, 16, (14.0, 0.8), wrap=False)
    plate *= (1.0 + 0.06 * streaks)[..., None]
    border = np.maximum(smoothstep(0.84, 0.86, across), np.maximum(smoothstep(0.06, 0.04, ys), smoothstep(0.94, 0.96, ys)))
    plate *= (1.0 - 0.45 * border)[..., None]
    line = smoothstep(0.012, 0.0, np.abs(across - 0.8)) * (ys > 0.08) * (ys < 0.92)
    centres = [(0.5, y) for y in (0.25, 0.45, 0.65, 0.84)]
    runes = glyph_mask(n, centres, 0.16, 66)
    glyph = ramp(np.full_like(xs, 0.9), tier.ramp) * tier.emblem
    edge = (0.25 + 0.5 * tier.ring) * line[..., None] * (mid * 0.6 + pale * 0.4)
    halo = ndimage.gaussian_filter(runes, 10)[..., None] * tier.halo * mid
    face = plate * (1.0 - runes[..., None]) + glyph * runes[..., None] + edge + halo
    glow = glyph * runes[..., None] + edge
    return face.clip(0, 255), glow.clip(0, 255)


def crystal_layers(tier: Tier):
    """The kyber prism's facet: tip at the picture's top middle, the girdle along its
    bottom; brighter toward the girdle, with streaks running to the tip and lit edges."""
    n = FACE_SIZE
    ys, xs = (np.mgrid[0:n, 0:n] + 0.5) / n
    across = (xs - 0.5) / np.maximum(ys, 1e-3)
    rng = np.random.default_rng(17)
    streak = np.zeros_like(xs)
    for at, width, strength in zip(rng.uniform(-0.45, 0.45, 7), rng.uniform(0.01, 0.04, 7), rng.uniform(0.4, 1.0, 7)):
        streak += strength * np.exp(-(((across - at) / width) ** 2))
    edges = np.exp(-(((np.abs(across) - 0.5) / 0.03) ** 2))
    value = 0.32 + 0.5 * ys**0.8 + 0.18 * streak.clip(0, 1) + 0.25 * edges
    face = ramp(value.clip(0, 1), tier.ramp) * tier.emblem
    glow = face * (0.45 + 0.4 * streak.clip(0, 1) + 0.3 * edges)[..., None]
    return face.clip(0, 255), glow.clip(0, 255)


def form_layers(base: np.ndarray, steel: np.ndarray, dark: np.ndarray, tier: Tier, art: dict):
    """The face and glow pictures of a tier's form: Sol's picture for it when given,
    else the one made here."""
    if tier.id in art:
        return art_layers(art[tier.id], tier)
    if tier.id == "uncommon":
        return tier_layers(base, tier)
    if tier.id == "rare":
        return pentagon_layers(steel, tier)
    if tier.id == "legendary":
        return sith_layers(dark, tier)
    return crystal_layers(tier)


def tier_icon(tier: Tier, looks: dict) -> Image.Image:
    """The 256x256 icon: the tier's holocron drawn from its parts, a little from above,
    with a halo in the tier's colour, made for a dark UI."""
    size, oversample = TIER_ICON_SIZE, 4
    big = size * oversample
    drawn = forms.render(tier.id, looks, big, ICON_FILL)
    colour, solid = drawn[..., :3], drawn[..., 3]
    mid = np.array(tier.ramp[1], float)
    spread = ndimage.gaussian_filter(solid, 9.0 * oversample)
    halo = np.clip(spread * 2.0 * (0.45 + 0.55 * tier.ring), 0, 0.85) * (1.0 - solid)
    colour = colour * solid[..., None] + mid * halo[..., None]
    alpha = np.clip(solid + halo, 0, 1)
    if tier.id == "mythical":
        star = sparkle(big, (0.80 * big, 0.17 * big), 0.13 * big)
        colour = colour * (1.0 - star[..., None]) + np.array((255.0, 250.0, 225.0)) * star[..., None]
        alpha = np.maximum(alpha, star)
    # Average in premultiplied terms down to the icon's size.
    flat = np.dstack([colour * alpha[..., None], alpha * 255.0])
    reduced = flat.reshape(size, oversample, size, oversample, 4).mean((1, 3))
    a = reduced[..., 3:4] / 255.0
    rgb = np.where(a > 1e-4, reduced[..., :3] / np.maximum(a, 1e-4), 0.0)
    return Image.fromarray(np.dstack([rgb, reduced[..., 3]]).clip(0, 255).round().astype(np.uint8), "RGBA")


def sparkle(n: int, centre, reach: float) -> np.ndarray:
    """A four-pointed glint, 0..1."""
    ys, xs = np.mgrid[0:n, 0:n]
    dx, dy = xs - centre[0], ys - centre[1]

    def arm(along, across):
        return np.exp(-3.0 * (along / reach) ** 2) * np.exp(-((across / (reach * 0.07)) ** 2))

    core = np.exp(-((np.hypot(dx, dy) / (reach * 0.22)) ** 2))
    return np.clip(arm(dx, dy) + arm(dy, dx) + core, 0, 1)


def small_icon(picture: Image.Image) -> Image.Image:
    """The 64x64 icon: the large one resized in premultiplied colour."""
    small = picture.convert("RGBa").resize((TIER_SMALL_SIZE, TIER_SMALL_SIZE), Image.LANCZOS)
    return small.convert("RGBA")


def sheen(output: Path) -> None:
    """A soft diagonal stripe that tiles along (s + t): the mythical shader scrolls it
    over the crystal, one stripe crossing at a time."""
    n = SHEEN_SIZE
    ys, xs = np.mgrid[0:n, 0:n]
    phase = ((xs + ys + 1.0) / n) % 1.0
    distance = np.abs(phase - 0.5)
    band = np.exp(-((distance / 0.07) ** 2)) + 0.18 * np.exp(-((distance / 0.22) ** 2))
    picture = np.clip(band[..., None] * np.array((255.0, 232.0, 160.0)), 0, 255)
    Image.fromarray(picture.astype(np.uint8)).save(output / f"{SHEEN_NAME}.jpg", quality=JPEG_QUALITY)


def wave(values) -> str:
    return "rgbGen wave sin " + " ".join(f"{value:g}" for value in values)


def tier_shader() -> str:
    lines = [
        "// The holocron's loot-box tiers (SJK; see README.md): each tier's face (the Jedi",
        "// cube's plates, the Great Holocron's pentagons, the Sith pyramid's sides, the kyber",
        "// crystal). Lit, also glowing faintly of itself, and its lit parts kept bright and in",
        "// the dynamic glow, breathing; the tiers climb in how bright. Mythical also breathes",
        "// the whole face and has a sheen stripe sliding across.",
        "// Made by scripts/holocron_assets.py from its TIERS table: edit that, not this.",
    ]
    for tier in TIERS:
        name = f"models/sjk/holocron_{tier.id}"
        lines += [
            name,
            "{",
            "\tq3map_nolightmap",
            "\t{",
            f"\t\tmap {name}",
            "\t\trgbGen lightingDiffuse",
            "\t}",
            "\t{",
            f"\t\tmap {name}",
            "\t\tblendFunc GL_ONE GL_ONE",
            f"\t\t{wave(tier.self_glow)}",
            "\t}",
            "\t{",
            f"\t\tmap {name}_glow",
            "\t\tblendFunc GL_ONE GL_ONE",
            f"\t\t{wave(tier.breath)}",
            "\t\tglow",
            "\t}",
        ]
        if tier.id == "mythical":
            lines += [
                "\t{",
                f"\t\tmap models/sjk/{SHEEN_NAME}",
                "\t\tblendFunc GL_ONE GL_ONE",
                "\t\ttcMod scale 0.5 0.5",
                "\t\ttcMod scroll 0.06 0.06",
                "\t\trgbGen wave sin 0.5 0.15 0 0.25",
                "\t}",
            ]
        lines.append("}")
    return "\n".join(lines) + "\n"


def lit_shader(name: str, own: float) -> list:
    """A material lit by the world that also glows faintly of itself (a wave of no
    amplitude is a constant), so it never reads black with the light behind it."""
    return [
        name,
        "{",
        "\tq3map_nolightmap",
        "\t{",
        f"\t\tmap {name}",
        "\t\trgbGen lightingDiffuse",
        "\t}",
        "\t{",
        f"\t\tmap {name}",
        "\t\tblendFunc GL_ONE GL_ONE",
        f"\t\trgbGen wave sin {own:g} 0 0 0",
        "\t}",
        "}",
    ]


def parts_shader() -> str:
    lines = [
        "// The holocrons' other materials (SJK; see README.md): steel (the cube's edges, the",
        "// pyramid's foot, the kyber's sheath and rings), the Sith pyramid's dark metal, and",
        "// each tier's light (lamps, edges, halo, seam, eye, motes): full bright, breathing",
        "// in the glow as the tier's face does.",
        "// Made by scripts/holocron_assets.py: edit that, not this.",
    ]
    lines += lit_shader(f"models/sjk/{forms.STEEL}", 0.3)
    lines += lit_shader(f"models/sjk/{forms.DARK}", 0.35)
    for tier in TIERS:
        name = f"models/sjk/{forms.light(tier.id)}"
        lines += [
            name,
            "{",
            "\tq3map_nolightmap",
            "\t{",
            f"\t\tmap {name}",
            "\t\trgbGen identity",
            "\t}",
            "\t{",
            f"\t\tmap {name}",
            "\t\tblendFunc GL_ONE GL_ONE",
            f"\t\t{wave(tier.breath)}",
            "\t\tglow",
            "\t}",
            "}",
        ]
    return "\n".join(lines) + "\n"


def locked_face(face_rgb: np.ndarray) -> np.ndarray:
    """A picture with its colour drained to a share and darkened (float RGB)."""
    grey = luminance_of(face_rgb)[..., None]
    drained = grey + LOCKED_KEEP * (face_rgb - grey)
    return (drained * LOCKED_DIM * np.array((0.96, 1.0, 1.08))).clip(0, 255)


def locked_materials() -> list:
    return [f"holocron_{tier.id}_locked" for tier in TIERS] + [
        forms.locked_material(forms.STEEL),
        forms.locked_material(forms.DARK),
        forms.LIGHT_LOCKED,
    ]


def locked_shader() -> str:
    lines = [
        "// The holocrons of a tier the player holds none of (SJK; see README.md): every",
        "// material drained and dark, lit by the world and a faint light of its own, no glow.",
        "// Made by scripts/holocron_assets.py: edit that, not this.",
    ]
    for material in locked_materials():
        lines += lit_shader(f"models/sjk/{material}", 0.16)
    return "\n".join(lines) + "\n"


def save_jpg(picture: np.ndarray, path: Path) -> None:
    Image.fromarray(picture.round().clip(0, 255).astype(np.uint8)).save(path, quality=JPEG_QUALITY)


def write_form(output: Path, tier: Tier) -> list:
    """The tier's parts as models, each in its own look and the locked one; the
    models' names."""
    names = []
    meshes = forms.FORMS[tier.id](tier.id)
    for part, mesh in meshes.items():
        name = forms.model_name(tier.id, part)
        for file, rename in ((name, lambda m: m), (f"{name}_locked", forms.locked_material)):
            surfaces = [
                (f"models/sjk/{rename(material)}", *data)
                for material, data in mesh.surfaces.items()
            ]
            write_md3(output, file, surfaces)
            names.append(file)
    return names


def rust_number(value: float) -> str:
    """A Rust f32 literal: a multiple of pi by its constant (clippy reads a rounded pi
    as a mistake), else a few significant digits."""
    for name, constant in (("PI", math.pi), ("FRAC_PI_2", math.pi / 2), ("FRAC_PI_4", math.pi / 4), ("FRAC_PI_8", math.pi / 8)):
        for sign in (1, -1):
            if abs(value - sign * constant) < 1e-9:
                return ("-" if sign < 0 else "") + f"std::f32::consts::{name}"
    text = f"{value:.6g}"
    return text if any(c in text for c in ".e") else f"{text}.0"


def parts_table() -> str:
    """The parts' table for the client (`holocrons/stage.rs` includes it)."""
    lines = [
        "// The loot-box holocrons' parts and how they move (SJK; see README.md), included by",
        "// crates/sjk-viewer/src/holocrons/stage.rs. Made by scripts/holocron_assets.py from",
        "// holocron_forms.py's PARTS: edit that, not this.",
        "",
        "/// Each tier's parts, in `TIERS` order, its body first.",
        "#[rustfmt::skip]",
        "pub(crate) const PARTS: [&[Part]; COUNT] = [",
    ]
    for tier in TIERS:
        lines.append("    &[")
        for motion in forms.PARTS[tier.id]:
            name = forms.model_name(tier.id, motion.part)

            def array(values):
                return "[" + ", ".join(rust_number(v) for v in values) + "]"

            lines.append(
                f'        Part {{ model: "models/sjk/{name}.md3", locked: "models/sjk/{name}_locked.md3", '
                f"at: {array(motion.at)}, bob: {array(motion.bob)}, orbit: {array(motion.orbit)}, "
                f"spin: {array(motion.spin)}, tumble: {array(motion.tumble)}, precess: {array(motion.precess)} }},"
            )
        lines.append("    ],")
    lines.append("];")
    return "\n".join(lines) + "\n"


def tiers(output: Path, art: dict) -> None:
    """The tiers' pictures, models and shaders, from the folder's holocron.jpg (and
    Sol's pictures in `art`, by tier id)."""
    base = np.asarray(Image.open(output / "holocron.jpg").convert("RGB")).astype(float)
    steel = steel_picture(base)
    dark = dark_picture()
    save_jpg(steel, output / f"{forms.STEEL}.jpg")
    save_jpg(dark, output / f"{forms.DARK}.jpg")
    save_jpg(locked_face(steel), output / f"{forms.locked_material(forms.STEEL)}.jpg")
    save_jpg(locked_face(dark), output / f"{forms.locked_material(forms.DARK)}.jpg")
    save_jpg(locked_face(light_picture(TIERS[1])), output / f"{forms.LIGHT_LOCKED}.jpg")
    looks = {
        forms.STEEL: forms.Look(steel, own_light=0.3),
        forms.DARK: forms.Look(dark, own_light=0.35),
    }
    layers = {}
    for tier in TIERS:
        face_picture, glow_picture = form_layers(base, steel, dark, tier, art)
        layers[tier.id] = (face_picture, glow_picture)
        save_jpg(face_picture, output / f"holocron_{tier.id}.jpg")
        save_jpg(glow_picture, output / f"holocron_{tier.id}_glow.jpg")
        save_jpg(locked_face(face_picture), output / f"holocron_{tier.id}_locked.jpg")
        lit = light_picture(tier)
        save_jpg(lit, output / f"{forms.light(tier.id)}.jpg")
        looks[f"holocron_{tier.id}"] = forms.Look(face_picture, glow_picture, own_light=tier.self_glow[0] * 0.5)
        looks[forms.light(tier.id)] = forms.Look(lit, lit=False)
        write_form(output, tier)
    for tier in TIERS:
        picture = tier_icon(tier, looks)
        picture.save(output / f"holocron_{tier.id}.png", optimize=True)
        small_icon(picture).save(output / f"holocron_{tier.id}_small.png", optimize=True)
    sheen(output)
    (output / "holocron_tiers.shader").write_text(tier_shader(), encoding="ascii", newline="\n")
    (output / "holocron_parts.shader").write_text(parts_shader(), encoding="ascii", newline="\n")
    (output / "holocron_locked.shader").write_text(locked_shader(), encoding="ascii", newline="\n")
    (output / "holocron_parts.rs").write_text(parts_table(), encoding="ascii", newline="\n")


def art_options(arguments) -> dict:
    """`--art <tier>=<picture>` pairs: Sol's face pictures for the tiers' forms."""
    art = {}
    for argument in arguments:
        tier, _, picture = argument.partition("=")
        if tier not in forms.TIER_IDS or not picture:
            sys.exit(f"--art takes <tier>=<picture>, the tier one of {', '.join(forms.TIER_IDS)}")
        art[tier] = Path(picture)
    return art


def main() -> None:
    arguments = sys.argv[1:]
    art = art_options(arguments[i + 1] for i, a in enumerate(arguments) if a == "--art")
    positional = [a for i, a in enumerate(arguments) if a != "--art" and (i == 0 or arguments[i - 1] != "--art")]
    if len(positional) == 2 and positional[0] == "tiers":
        tiers(Path(positional[1]), art)
        return
    icon_source, face_source, output = (Path(arg) for arg in positional[:3])
    output.mkdir(parents=True, exist_ok=True)
    icon(icon_source, output)
    face(face_source, output)
    cube_md3(output)
    tiers(output, art)


if __name__ == "__main__":
    main()
