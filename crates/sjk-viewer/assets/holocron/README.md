# Illuminate's holocron

The Illuminate toy (SJK's own, not a Force power, [client.md](../../../../docs/client.md#illuminate))
shows a holocron floating by the player's shoulder. Its pictures were generated
by Sol (07/10/2026): a holocron icon in the style of Jedi Academy's Force icons
(1024x1024, on a flat grey ground) and one face of the holocron (2048x2048). The
files here are made from those two by [holocron_assets.py](../../../../scripts/holocron_assets.py),
so none is edited by hand except `holocron.shader`.

| File | Game path | What |
| --- | --- | --- |
| `toy_illuminate.png` | `gfx/sjk/toy_illuminate.png` | The toy's icon (quick wheel, Collection, key bindings), 128x128, like `gfx/mp/f_icon_*` |
| `holocron.jpg` | `models/sjk/holocron.jpg` | The face, 512x512 |
| `holocron_glow.jpg` | `models/sjk/holocron_glow.jpg` | The face's emblem alone, for the additive stage |
| `holocron.md3` | `models/sjk/holocron.md3` | A 6-unit cube with the face on every side |
| `holocron.shader` | `shaders/sjk_holocron.shader` | `models/sjk/holocron`: lit face, glowing emblem |

The client mounts them in memory below all game data
([illuminate.rs](../../src/illuminate.rs)), so a PK3 that has the same paths
replaces them.

## Loot-box tiers

The holocron drops come in four tiers, each a holocron of its own since 18/10/2026
([holocrons.md](../../../../docs/holocrons.md#the-four-holocrons)): the Jedi cube
(uncommon, green), the Great Holocron (rare, blue), the Sith pyramid (legendary, purple)
and the kyber prism (mythical, gold, shiny). The shapes are in
[holocron_forms.py](../../../../scripts/holocron_forms.py), the pictures and shaders in
[holocron_assets.py](../../../../scripts/holocron_assets.py), which writes everything
below. Colour is not the only cue: besides the shapes, the tiers climb in how much they
glow (the emblem's and the lit parts' brightness and breathing), so they also tell apart
by lightness, for viewers who cannot rely on hue.

Per tier `<tier>` (`uncommon`, `rare`, `legendary`, `mythical`), all mounted in memory like
the files above:

| File | Game path | What |
| --- | --- | --- |
| `holocron_<tier>.md3` | `models/sjk/holocron_<tier>.md3` | The body: the part that stands still against the holocron |
| `holocron_<tier>_<part>.md3` | `models/sjk/holocron_<tier>_<part>.md3` | Each moving part, about its own centre: `rare_halo`; `legendary_cap`, `legendary_eye`; `mythical_crystal`, `mythical_ring_a`, `mythical_ring_b`, `mythical_mote` (drawn ten times) |
| `..._locked.md3` | the same, `_locked` | Each of those in the locked look |
| `holocron_<tier>.jpg` | `models/sjk/holocron_<tier>.jpg` | The face, 512x512: the cube's plate, the pentagon, the pyramid's side, the crystal's facet |
| `holocron_<tier>_glow.jpg` | `models/sjk/holocron_<tier>_glow.jpg` | Its lit parts alone, for the additive and glow stage |
| `holocron_<tier>_locked.jpg` | `models/sjk/holocron_<tier>_locked.jpg` | The face drained of colour and dark |
| `holocron_<tier>_light.jpg` | `models/sjk/holocron_<tier>_light.jpg` | The lit parts' colour (lamps, edges, halo, seam, eye, motes), 64x64 |
| `holocron_<tier>.png` | `gfx/sjk/holocron_<tier>.png` | UI icon, 256x256 with alpha: the holocron drawn from its parts, a little from above, with a halo in the tier's colour (the mythical one a glint too), for a dark UI |
| `holocron_<tier>_small.png` | `gfx/sjk/holocron_<tier>_small.png` | The same, 64x64 |

Shared:

| File | Game path | What |
| --- | --- | --- |
| `holocron_steel.jpg`, `holocron_steel_locked.jpg` | `models/sjk/...` | Brushed steel in the face's plate colour (the cube's edges, the pyramid's foot, the kyber's sheath and rings), tiling |
| `holocron_dark.jpg`, `holocron_dark_locked.jpg` | `models/sjk/...` | The Sith pyramid's dark metal, tiling |
| `holocron_light_locked.jpg` | `models/sjk/holocron_light_locked.jpg` | Every tier's lit parts in the locked look |
| `holocron_mythical_sheen.jpg` | `models/sjk/holocron_mythical_sheen.jpg` | A soft diagonal stripe (128x128, tiles along s + t) that the mythical shader scrolls over the crystal (`tcMod scroll`) |
| `holocron_tiers.shader` | `shaders/sjk_holocron_tiers.shader` | `models/sjk/holocron_<tier>`: the faces, lit, glowing faintly of themselves, their lit parts breathing in the glow |
| `holocron_parts.shader` | `shaders/sjk_holocron_parts.shader` | The steel, the dark metal and each tier's light (full bright, breathing in the glow) |
| `holocron_locked.shader` | `shaders/sjk_holocron_locked.shader` | The locked look of every material: lit, a faint light of its own, no glow |
| `holocron_parts.rs` | (not mounted) | The parts' table the client includes (`holocrons/stage.rs`): each part's models, where it rests and how it bobs, orbits and turns |

A surface's shader is named by its model file, so every part is a model of its own in each
look. The shaders use only stock Quake 3 / Jedi Academy keywords (`rgbGen lightingDiffuse`,
`identity` and `wave`, `blendFunc GL_ONE GL_ONE`, `tcMod scale` and `scroll`, `glow`); the
tests in [illuminate.rs](../../src/illuminate.rs) parse them with the client's shader
parser and check every picture they name is mounted. Everything comes out of
`holocron.jpg`, so nothing of the originals is needed to remake the tiers.

### Sol's art for the faces

Three faces are drawn by the script for now. A generated picture replaces one with
`--art <tier>=<picture>` (square, any size and colour; its bright parts are painted along
the tier's colours and glow):

- `rare`: one pentagon of the Great Holocron, a corner at the top, its corners on the
  picture's inscribed circle (the pentagon touches the top edge);
- `legendary`: one side of the Sith pyramid, a trapezoid, its foot along the bottom edge
  and its top 0.53 as wide, centred (dark metal, a column of runes);
- `mythical`: one facet of the kyber crystal, a triangle, its tip at the top middle and its
  foot along the bottom edge.

    python scripts/holocron_assets.py tiers crates/sjk-viewer/assets/holocron --art rare=pentagon.png

The Jedi cube keeps the shipped face (Sol's, 07/10/2026).

### Light colours

The point light of each tier (the one the 3D menu uses), in the scale
of `COLOR` in `illuminate.rs` (`[1.5, 1.3, 1.0]`, Illuminate's warm white): linear RGB
floats, not clamped. They are in [holocron_assets.py](../../../../scripts/holocron_assets.py)
as `Tier.light` too; the tiers rise in strength (the sum of the three) as they rise in
tier.

| Id | R | G | B | Sum |
| --- | --- | --- | --- | --- |
| `uncommon` | 0.55 | 1.50 | 0.60 | 2.65 |
| `rare` | 0.50 | 1.00 | 1.75 | 3.25 |
| `legendary` | 1.25 | 0.45 | 1.75 | 3.45 |
| `mythical` | 1.85 | 1.30 | 0.40 | 3.55 |

```rust
const TIER_LIGHTS: [(&str, [f32; 3]); 4] = [
    ("uncommon", [0.55, 1.50, 0.60]),
    ("rare", [0.50, 1.00, 1.75]),
    ("legendary", [1.25, 0.45, 1.75]),
    ("mythical", [1.85, 1.30, 0.40]),
];
```

## Regenerating

With Python 3 and Pillow, numpy and scipy (`pip install pillow numpy scipy`),
from the repository root. Everything, from Sol's two originals (the tiers follow
from the new face):

```sh
python scripts/holocron_assets.py icon.jpg face.jpg crates/sjk-viewer/assets/holocron
```

Only the tiers (pictures, icons, sheen and shader), from the `holocron.jpg` in the
folder; the output is deterministic, and it is the `TIERS` table in the script that
sets the colours, the glow ladder, the shader's waves and the lights:

```sh
python scripts/holocron_assets.py tiers crates/sjk-viewer/assets/holocron
```
