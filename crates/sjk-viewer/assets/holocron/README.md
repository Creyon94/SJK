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

The holocron drops come in four tiers, each a holocron of its own since 17/10/2026
([holocrons.md](../../../../docs/holocrons.md#the-four-holocrons)): the Jedi cube
(uncommon, green), the Great Holocron (rare, blue), the Sith pyramid (legendary, purple)
and, since 21/10/2026, the engram (mythical, gold: thirty plates with glowing seams and a
white-hot core, breathing). The kyber crystals (`kyber_uncommon`, `kyber_rare`,
`kyber_legendary`, `kyber_mythical`, 21/10/2026: the kyber prism that was the mythical
holocron until then, in each tier's colours, plainer the commoner) are made with them
([holocrons.md](../../../../docs/holocrons.md#kyber-crystals)). The shapes are in
[holocron_forms.py](../../../../scripts/holocron_forms.py), the pictures and shaders in
[holocron_assets.py](../../../../scripts/holocron_assets.py), which writes everything
below. Colour is not the only cue: besides the shapes, the tiers climb in how much they
glow (the emblem's and the lit parts' brightness and breathing), so they also tell apart
by lightness, for viewers who cannot rely on hue.

Per form `<tier>` (`uncommon`, `rare`, `legendary`, `mythical`, then `kyber_uncommon` ...
`kyber_mythical`), all mounted in memory like the files above (the list the client mounts,
`holocron_files.rs`, is written by the script too):

| File | Game path | What |
| --- | --- | --- |
| `holocron_<tier>.md3` | `models/sjk/holocron_<tier>.md3` | The body: the part that stands still against the holocron |
| `holocron_<tier>_<part>.md3` | `models/sjk/holocron_<tier>_<part>.md3` | Each moving part, about its own centre: `rare_halo`; `legendary_cap`, `legendary_eye`; `mythical_shell` (the engram's plates; its body is the core), `mythical_mote` (a spark, drawn eight times); `kyber_<tier>_crystal`, `_ring_a`, `_ring_b` (legendary and mythical only), `_mote` (drawn 4, 6, 8, 10 times) |
| `..._locked.md3` | the same, `_locked` | Each of those in the locked look |
| `holocron_<tier>.jpg` | `models/sjk/holocron_<tier>.jpg` | The face, 512x512: the cube's plate, the pentagon, the pyramid's side, the engram's plate (four shades in a 2 by 2 grid, each plate taking one by its facing), the kyber crystal's facet |
| `holocron_<tier>_glow.jpg` | `models/sjk/holocron_<tier>_glow.jpg` | Its lit parts alone, for the additive and glow stage |
| `holocron_<tier>_locked.jpg` | `models/sjk/holocron_<tier>_locked.jpg` | The face drained of colour and dark |
| `holocron_<tier>_light.jpg` | `models/sjk/holocron_<tier>_light.jpg` | The lit parts' colour (lamps, edges, halo, seam, eye, motes), 64x64 |
| `holocron_<tier>.png` | `gfx/sjk/holocron_<tier>.png` | UI icon, 256x256 with alpha: the holocron drawn from its parts, a little from above, with a halo in the tier's colour (the mythical ones a glint too), for a dark UI |
| `holocron_<tier>_small.png` | `gfx/sjk/holocron_<tier>_small.png` | The same, 64x64 |

Shared:

| File | Game path | What |
| --- | --- | --- |
| `holocron_steel.jpg`, `holocron_steel_locked.jpg` | `models/sjk/...` | Brushed steel in the face's plate colour (the cube's edges, the pyramid's foot, the kyber crystals' sheaths and rings), tiling |
| `holocron_dark.jpg`, `holocron_dark_locked.jpg` | `models/sjk/...` | The Sith pyramid's dark metal, tiling |
| `holocron_light_locked.jpg` | `models/sjk/holocron_light_locked.jpg` | Every tier's lit parts in the locked look |
| `holocron_sheen.jpg` | `models/sjk/holocron_sheen.jpg` | A soft diagonal stripe (128x128, tiles along s + t) that the kyber crystals' shaders scroll over the crystal (`tcMod scroll`) |
| `holocron_mythical_seam.jpg`, `_core.jpg` (and `_locked`) | `models/sjk/...` | The engram's seams' glow and its core's facet (white-hot in the middle) |
| `holocron_mythical_env.jpg` | `models/sjk/holocron_mythical_env.jpg` | The engram's reflection, read by the view (`tcGen environment`): a faint warm sky and two thin streaks that sweep the facets as the shell turns |
| `holocron_tiers.shader` | `shaders/sjk_holocron_tiers.shader` | `models/sjk/holocron_<tier>`: the holocrons' faces, lit, glowing faintly of themselves, their lit parts breathing in the glow; the engram's plates also breathe out and back (`deformVertexes wave`, every vertex of a plate carrying the plate's normal so the slab moves as one) |
| `holocron_kyber.shader` | `shaders/sjk_holocron_kyber.shader` | `models/sjk/holocron_kyber_<tier>`: the crystals' facets (lit, own light, glow, sheen) and each crystal's light |
| `holocron_parts.shader` | `shaders/sjk_holocron_parts.shader` | The steel, the dark metal, each holocron tier's light (full bright, breathing in the glow), the engram's seams (pulsing, breathing with the plates) and core |
| `holocron_locked.shader` | `shaders/sjk_holocron_locked.shader` | The locked look of every material: lit, a faint light of its own, no glow, nothing moving |
| `holocron_parts.rs` | (not mounted) | The forms' table the client includes (`holocrons/stage.rs`, `FORMS`): each form's id and light, each part's models, where it rests and how it bobs, orbits and turns |
| `holocron_files.rs` | (not mounted) | Every file above at its game path, included by `illuminate.rs`, which mounts them |

A surface's shader is named by its model file, so every part is a model of its own in each
look. The shaders use only stock Quake 3 / Jedi Academy keywords (`rgbGen lightingDiffuse`,
`identity` and `wave`, `blendFunc GL_ONE GL_ONE`, `tcMod scale` and `scroll`, `glow`); the
tests in [illuminate.rs](../../src/illuminate.rs) parse them with the client's shader
parser and check every picture they name is mounted. Everything comes out of
`holocron.jpg`, so nothing of the originals is needed to remake the tiers.

### Sol's art for the faces

Three pictures come from Sol's generated pictures (17/10/2026), kept in `art/` at 1024x1024
(not mounted; the script reads them each time it remakes the tiers, and draws its own
face for a tier whose picture is missing). A picture is any size and colour; its lit
parts (bright, or strongly coloured) are painted along the tier's colours and glow:

- `art/rare.jpg`: one pentagon of the Great Holocron on black, a corner at the top; the
  script cuts it out and fits it to the face's pentagon;
- `art/legendary.jpg`: a triangle of the Sith pyramid on black, its tip at the top; cut
  out and fitted to the whole picture, the pyramid's sides take its foot (up to 0.53 of
  its height, where it is as wide as their top) and the floating cap its tip;
- `art/kyber.jpg`: a tiling crystal texture, worked into the kyber crystals' facets (it was
  `art/mythical.jpg` while the kyber prism was the mythical holocron).

The engram's plate is drawn by the script (21/10/2026): gold with a rim, an engraved groove
and lit line, and a seal (a ring with an old-republic style rune, three small runes by the
foot), faintly lit.

`--art <key>=<picture>` (`rare`, `legendary`, `kyber`) tries another picture without replacing the file:

    python scripts/holocron_assets.py tiers crates/sjk-viewer/assets/holocron --art rare=pentagon.png

The Jedi cube keeps the shipped face (Sol's, 07/10/2026).

### Light colours

The point light of each tier (the one the 3D menu uses; a kyber crystal casts its tier's), in the scale
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
