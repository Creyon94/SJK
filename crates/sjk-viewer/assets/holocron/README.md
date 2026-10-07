# Illuminate's holocron

The Illuminate entry of the Force wheel (SJK's own, [client.md](../../../../docs/client.md#illuminate))
shows a holocron floating by the player's shoulder. Its pictures were generated
by Sol (07/10/2026): a holocron icon in the style of Jedi Academy's Force icons
(1024x1024, on a flat grey ground) and one face of the holocron (2048x2048). The
files here are made from those two by [holocron_assets.py](../../../../scripts/holocron_assets.py),
so none is edited by hand except the shader.

| File | Game path | What |
| --- | --- | --- |
| `force_illuminate.png` | `gfx/sjk/force_illuminate.png` | Force wheel icon, 128x128, like `gfx/mp/f_icon_*` |
| `holocron.jpg` | `models/sjk/holocron.jpg` | The face, 512x512 |
| `holocron_glow.jpg` | `models/sjk/holocron_glow.jpg` | The face's emblem alone, for the additive stage |
| `holocron.md3` | `models/sjk/holocron.md3` | A 6-unit cube with the face on every side |
| `holocron.shader` | `shaders/sjk_holocron.shader` | `models/sjk/holocron`: lit face, glowing emblem |

The client mounts them in memory below all game data
([illuminate.rs](../../src/illuminate.rs)), so a PK3 that has the same paths
replaces them.

## Regenerating

With Python 3 and Pillow, numpy and scipy (`pip install pillow numpy scipy`),
from the repository root:

```sh
python scripts/holocron_assets.py icon.jpg face.jpg crates/sjk-viewer/assets/holocron
```
