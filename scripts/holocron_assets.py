"""Make the Illuminate holocron's game files from Sol's two generated pictures.

Illuminate is SJK's own Force-wheel entry: a holocron that floats by the player's
shoulder and lights the way. Sol generated its pictures (07/10/2026): a holocron
icon in the style of Jedi Academy's Force icons (1024x1024, on a flat grey ground)
and one face of the holocron (2048x2048). This script writes, into the output
folder:

- `force_illuminate.png`: the icon cut out of its ground with its amber glow, cube
  at the stock icons' size, 128x128 like `gfx/mp/f_icon_*`;
- `holocron.jpg`: the face, 512x512;
- `holocron_glow.jpg`: the face's emblem alone on black, for the additive stage that
  keeps it lit in the dark;
- `holocron.md3`: a cube of HOLOCRON_EDGE units with the face on all six sides.

    python scripts/holocron_assets.py icon.jpg face.jpg crates/sjk-viewer/assets/holocron

Needs Pillow, numpy and scipy (pip install pillow numpy scipy).
"""

import math
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage
from scipy.spatial import ConvexHull

ICON_SIZE = 128
# The icon's flat ground, and the share of the frame the cube's box takes (the
# stock icons' cubes span about two thirds of theirs).
ICON_GROUND = 39.0
ICON_CUBE_SHARE = 0.68
ICON_GLOW = np.array([255.0, 168.0, 72.0])
FACE_SIZE = 512
HOLOCRON_EDGE = 6.0
SHADER = "models/sjk/holocron"


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
    picture.convert("RGBA").save(output / "force_illuminate.png", optimize=True)


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


def cube_md3(output: Path) -> None:
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

    def name(text: str, size: int) -> bytes:
        return text.encode("ascii").ljust(size, b"\0")

    shaders = name(SHADER, 64) + struct.pack("<i", 0)
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
    surface = (
        b"IDP3"
        + name("holocron", 64)
        + struct.pack(
            "<10i", 0, 1, 1, len(positions), len(triangles),
            ofs_tris, ofs_shaders, ofs_st, ofs_verts, surface_end,
        )
        + shaders + tris + sts + verts
    )
    frame = struct.pack("<10f", -h, -h, -h, h, h, h, 0, 0, 0, h * math.sqrt(3)) + name("holocron", 16)
    header_size = 108
    ofs_frames = header_size
    ofs_surfaces = ofs_frames + len(frame)
    end = ofs_surfaces + len(surface)
    header = (
        b"IDP3"
        + struct.pack("<i", 15)
        + name("models/sjk/holocron.md3", 64)
        + struct.pack("<9i", 0, 1, 0, 1, 0, ofs_frames, ofs_surfaces, ofs_surfaces, end)
    )
    (output / "holocron.md3").write_bytes(header + frame + surface)


def main() -> None:
    icon_source, face_source, output = (Path(arg) for arg in sys.argv[1:4])
    output.mkdir(parents=True, exist_ok=True)
    icon(icon_source, output)
    face(face_source, output)
    cube_md3(output)


if __name__ == "__main__":
    main()
