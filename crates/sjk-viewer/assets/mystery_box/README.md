# Mystery box

The mystery boxes' chest, lid, sounds and the Box hunt toy's icon (SJK's own,
[mystery-boxes.md](../../../../docs/mystery-boxes.md)). The sounds are made by
[mystery_box_assets.py](../../../../scripts/mystery_box_assets.py) from synthesized tones
(SJK's own tune), the rest by [mystery_box_model.py](../../../../scripts/mystery_box_model.py)
from noise and shapes; nothing is taken from another game, and only `mystery_box.shader`
is written by hand.

| File | Game path | What |
| --- | --- | --- |
| `roll.wav` | `sound/sjk/mystery_box/roll.wav` | The roll: a music box tune slowing like a wound-down spring, its last chord at 5.5 s as the roll stops on what came out, 22050 Hz mono |
| `take.wav` | `sound/sjk/mystery_box/take.wav` | Taking what came out: a quick rising sparkle |
| `mystery_box.md3` | `models/sjk/mystery_box.md3` | The chest, 60 long (y) x 26 deep (x, its front facing +X) x 24 tall, its origin in the middle of its bottom |
| `mystery_box_lid.md3` | `models/sjk/mystery_box_lid.md3` | The lid, its origin on the hinge (the chest's back top edge), reaching +X; the client pitches it open |
| `mystery_box.jpg` | `models/sjk/mystery_box.jpg` | 1024x1024: a long side (planks, iron bands, two question marks), the lid's top, an end, the dark inside, the iron edge |
| `mystery_box_glow.jpg` | `models/sjk/mystery_box_glow.jpg` | The same layout, black but for the question marks' glow |
| `mystery_box.shader` | `shaders/sjk_mystery_box.shader` | `models/sjk/mystery_box`: lit as a player model is, the glow added on top, pulsing |
| `toy_boxhunt.png` | `gfx/sjk/toy_boxhunt.png` | The Box hunt toy's icon, 128x128: the chest's front |
