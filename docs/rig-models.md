# Rig models

A rig model is a player model drawn from a glTF file with its own skeleton (any
number of joints, fingers, face, cloth), animated by the game's ordinary
`_humanoid` animations. Built 14/10/2026 ([status](status.md#rig-models-gltf-players-with-their-own-skeleton)).

## Why this design

Three ways were weighed on 14/10/2026 before building this:

- **Bake the model onto Kyle's GLM skeleton** (re-skin it to JKA's 53 bones). Works
  with every client, but loses fingers, face and any extra bone, and every model
  needs hand weighting.
- **Replace the skeleton outright** (animate the glTF with its own clips). Needs
  every JKA animation re-made for each skeleton, and the server, saber traces and
  bolts all speak `_humanoid` bone numbers.
- **Hybrid, the one built:** keep `_humanoid` as a hidden driver and retarget its pose
  onto the rig every frame. The server, protocol, traces and animation numbers stay
  untouched; any skeleton works through a bone map; joints the map leaves out stay at
  rest today and can play the rig's own clips later (emotes, face).

A profile before the work (world shot `animation_cost_on_a_demo`, release build, a
19-player demo) measured pose, skinning and upload at 0.064 ms a frame with vertex
skinning already on the GPU, so the rig path is about looks, not speed.

## How it works

The player keeps a stock **driver** model, Kyle unless the manifest names another.
Its GLM and `_humanoid.gla` do everything they did before: the server's animation
numbers, the split torso and legs, look angles, saber and effect bolts, saber traces,
Force effects, cosmetics. Only its surfaces are not drawn, and its bolts are moved
onto the rig (below).

The glTF mesh is drawn in their place. Every frame, after the driver's bone matrices
are evaluated ([actor_pose_steps.rs](../crates/sjk-viewer/src/actor_pose_steps.rs)),
[rig_model.rs](../crates/sjk-viewer/src/rig_model.rs) poses the rig:

- a **mapped** joint takes its JKA bone's model-space rotation change from the base
  pose. At load, its bind pose is first turned so its direction to a chosen child
  matches the JKA bone's direction to its own child (an A-pose rig fits JKA's base
  pose this way); `Rest` joints (head, feet) keep their bind orientation;
- the joint mapped to `pelvis` also follows the pelvis translation, so jumps,
  crouches and rolls move the body;
- **unmapped** joints (twist bones, ears, face, cloth) keep their bind pose under
  their parent.

The result is a joint palette skinned on the GPU like any player
([gpu_skinning.rs](../crates/sjk-viewer/src/gpu_skinning.rs)), or on the CPU when
`SJK_GPU_SKINNING=0`. Posing costs one affine multiply per joint per player and
allocates nothing per frame.

### Bolts on the rig

The driver's bolts (`*r_hand`, `*l_hand`, `*head_top`, the feet, cosmetics, the
jetpack's `*chestg`) and the Force effect points (hands, head, lumbar for a carried
flag) sit where Kyle's bones put them, which misses a rig with other proportions.
`PlayerPreview::bolt` and `RigModel::retarget_bolt` move each one onto the rig: the
bolt keeps the driver's orientation (mapped joints turn as the driver's bones do)
and its offset from its bone, stretched by the limb's length ratio along the map's
`Toward` pair, but starts from the joint mapped to that bone (or to its nearest
mapped ancestor). A saber then sits in the rig's hand, a hat on its head. Saber
traces and hit boxes stay the driver's.

### Code map

| Step | Where |
| --- | --- |
| Folder with `sjk_rig.cfg` found, driver loaded instead of the folder's GLM | `load_player_appearance` in [player_assets.rs](../crates/sjk-viewer/src/player_assets.rs) |
| glTF read (skin, joints, primitives, images) | [gltf.rs](../crates/sjk-model/src/gltf.rs) |
| Manifest, presets, fit (axes, scale, alignments, anchors), pose, CPU skin | [rig_model.rs](../crates/sjk-viewer/src/rig_model.rs), tests in `rig_model_tests.rs` |
| Rig surfaces put in the shared vertex buffer | `append_rig_mesh` in [actor_load.rs](../crates/sjk-viewer/src/actor_load.rs) |
| Per-frame pose and palette upload (GPU and CPU paths) | [actor_pose_steps.rs](../crates/sjk-viewer/src/actor_pose_steps.rs), `rig_skin_records` in [gpu_skinning.rs](../crates/sjk-viewer/src/gpu_skinning.rs) |
| Bolts and Force points moved onto the rig | `PlayerPreview::bolt`, `RigModel::retarget_bolt` and `retarget_point`, [force_bones.rs](../crates/sjk-viewer/src/force_bones.rs) |
| Dismemberment skipped, JoF first-person body off | [dismember.rs](../crates/sjk-viewer/src/dismember.rs), [actor_world_submission.rs](../crates/sjk-viewer/src/actor_world_submission.rs) |
| Blender export | [scripts/blender/sjk_rig_export](../scripts/blender/sjk_rig_export/__init__.py) |

Units and axes are converted at load: glTF metres with +Y up and +Z front become the
GLA's model space (+Z up, -Y front, as a Blender export of a model facing -Y). The
scale fits the rig's pelvis height to the driver's (`scale auto`).

## The manifest: `sjk_rig.cfg`

A model folder (`models/players/<name>/`) with `sjk_rig.cfg` is a rig model; `/model
<name>` picks it like any other. One `key value` per line, `//` comments:

| Line | Meaning | Default |
| --- | --- | --- |
| `model <file>` | the glTF, relative to the folder: `.gltf` with its `.bin` and images, or `.glb` | `model.gltf`, else `model.glb` |
| `driver <folder>` | the stock model whose skeleton drives the rig | `models/players/kyle` |
| `preset <name>` | the bone map to start from ([`PRESETS`](../crates/sjk-viewer/src/rig_model.rs)) | `mannequin` |
| `scale <n>` or `scale auto` | game units per glTF metre | `auto` |
| `map <joint> <bone>` | the joint follows the JKA bone, bind orientation kept | |
| `map <joint> <bone> <joint end> <bone end>` | ... aimed along the bone | |
| `map <joint> <bone> as <other joint>` | ... with another joint's alignment (finger chains) | |

`map` lines replace the preset's entry for that joint or add one.

### Presets

- `mannequin`: Unreal Engine mannequin names (`pelvis`, `spine_01`..`spine_05`,
  `neck_01`, `neck_02`, `head`, `clavicle_l`, `upperarm_l`, `lowerarm_l`, `hand_l`,
  `thumb_01_l`..`pinky_03_l`, `thigh_l`, `calf_l`, `foot_l`, and `_r`). Five spine
  joints share JKA's three; JKA's two-joint finger groups (thumb `d1`, index and
  middle `d2`, ring and pinky `d4`) drive all three joints of each finger. Clavicles
  are left to the spine.

Another skeleton naming is a new preset entry, not code elsewhere.

## The glTF

Read by [gltf.rs](../crates/sjk-model/src/gltf.rs): the first skin, its joints'
bind poses from the inverse bind matrices, and the triangle primitives of the meshes
that skin deforms; up to eight influences, of which the strongest four are kept.
Triangles are turned to the game's winding at load (glTF fronts are
counter-clockwise, the game's default `cull front` keeps clockwise ones; unturned,
the inside of the far side shows through). Each primitive's base colour texture
becomes its shader (the image's path, so a
`.shader` script of that name can override it); a primitive without one is drawn
`*white`. Not read: animations, morph targets (shape keys), normal and PBR maps,
images embedded in a buffer, data URIs.

## From Blender: the SJK Rig Model Export add-on

[scripts/blender/sjk_rig_export](../scripts/blender/sjk_rig_export/__init__.py) is a
Blender extension (4.2 or later) that writes a model folder in one click.

- **Install:** zip the folder's two files (`__init__.py`, `blender_manifest.toml`),
  then Edit > Preferences > Get Extensions > Install from Disk. The preferences take
  the GameData folder; left empty, the usual Steam and retail places are tried.
- **Use:** 3D view sidebar (N), SJK tab. Pick the armature; the panel counts the
  bone map's joints it has (`pelvis` and `hand_r` are required) and the meshes it
  deforms. Name the model and press Export to SJK.
- **What it writes:** `<GameData>/base/models/players/<name>/` with `model.gltf`,
  `model.bin`, the colour images and `sjk_rig.cfg` (only when missing, so `map` lines
  added by hand survive a re-export; a checkbox rewrites it). glTF Separate, +Y up,
  skins on, the armature's rest position, no animation, no shape keys, four
  influences.
- **Materials:** each becomes a plain Principled BSDF with the image its base colour
  most likely comes from: an image node feeding a socket named Base Color, Diffuse
  or Albedo, else one labelled so, else an image named `_D`, `_BaseColor`,
  `_Albedo`..., else the first sRGB image. A material with none keeps its viewport
  colour.
- **The blend file is untouched:** the meshes and materials exported are temporary
  copies, removed afterwards; the pose mode and selection come back as they were.

By hand instead: glTF Separate (`.gltf` + `.bin` + textures), +Y up, skinning on,
animations and shape keys off, the armature in its rest pose, each material a
Principled BSDF whose Base Color is an Image Texture.

Use it with your own files on your own disk. SJK never uploads, hosts or passes on
models taken from other games ([the rule](../AGENTS.md#verification-and-scope)).

## Limits

- Dismemberment is off for rig models: their glTF has none of the driver surfaces a
  cut hides and shows.
- In first person, JoF's body view (the saber arm drawn) does not show a rig model's
  body: it is one surface, with no head to hide. The classic first person applies.
- Menus (Character, Profile) still show the driver model.
- Only the player sees it: other players see the rig model only when they have the
  same folder; otherwise they see their own fallback for an unknown model, as for any
  custom model.
- Feet can slide or float a little when the rig's leg proportions differ from the
  driver's; there is no foot IK yet.
- The add-on's export of a 177-joint mannequin character matched a hand export
  (joints, primitives, images) and rendered in the world shot below (14/10/2026).
- First tried in the game by Sol on 14/10/2026: drawn and animated; the faces
  showed through (winding) and the saber missed the hand, both fixed the same day.

## Pitfalls

What went wrong or nearly did, so the next change does not repeat it:

- **Winding.** glTF fronts are counter-clockwise, the game's clockwise. Unturned, the
  model looked see-through (the inside of the far side drawn). Fixed by swapping two
  indices per triangle at load; a test pins it.
- **Bolts stay on the driver.** Anything read through `surface_bolt_matrix` on the
  driver GLM lands where Kyle's bones are: the saber floated beside the paw. Player
  bolts go through `PlayerPreview::bolt`; a new player bolt use must too (the menu
  stage still reads the driver directly, which is fine while menus show the driver).
- **What the driver's matrices are.** The per-bone matrices are skinning matrices
  (posed times inverse base pose, the matrix that moves a base-pose vertex); a bone's
  posed position is that matrix applied to its base-pose translation.
- **JKA's vertical offset.** A GLA frame's pose sits about 27 units below the base
  pose (the entity origin is mid-body). The engine needs nothing for it (the rig
  follows the pelvis exactly as Kyle does), but an offline preview in Blender must
  measure the pelvis against the standing idle or the feet sink.
- **Axes.** glTF is +Y up, +Z front; the GLA +Z up, -Y front; a Blender export of a
  model facing -Y lines up. A model facing another way needs turning before export.
- **Scale.** `scale auto` divides the driver's pelvis height by the rig's; a rig whose
  pelvis joint sits at the origin needs `scale <n>`.
- **Several joints on one bone** (`spine_01` and `spine_02` both on `lower_lumbar`):
  fine for posing; for bolts, a bone's first map entry is its anchor.

## Recipes

### Rerun the checks

```sh
# Rig and glTF unit tests (sjk-viewer has no lib target, hence --bins):
cargo test -p sjk-viewer --bins rig_model
cargo test -p sjk-model gltf
# Off-screen render of a rig model on a recorded game; 12 close-ups in
# target/world-shots/rig_model_orbit.png:
JKA_GAME_DATA=<GameData> SJK_TEST_DEMO=<demo.dm_26> SJK_TEST_RIG=<model folder name> \
  cargo test -p sjk-viewer --bins rig_model_on_a_demo -- --ignored --nocapture --test-threads=1
# Host cost of animating a demo's players (release build for real numbers):
JKA_GAME_DATA=<GameData> SJK_TEST_DEMO=<demo.dm_26> \
  cargo test --release -p sjk-viewer --bins animation_cost_on_a_demo -- --ignored --nocapture
```

The render puts the rig on the first other player of the demo's first snapshot,
through the mid-match model change path. Look at the head and chest (nothing
see-through) and the right hand (the hilt in it).

### Support another skeleton naming

1. Add a function returning its `MapEntry` list and a `PRESETS` line in
   [rig_model.rs](../crates/sjk-viewer/src/rig_model.rs) (see `mannequin()`): each
   mapped joint names its JKA bone and, where it has a child to aim along, the
   `toward` pair; finger chains use `Align::As` for their second and third joints.
2. Add the same joints to `PRESET_JOINTS` and `PRESET_ITEMS` in the add-on (only
   used to tell the user whether a skeleton fits; keep the two in step).
3. Check it in Blender beside posed Kyle (next recipe), then in a world shot.

JKA's bone names and base poses: the `gla_dump` example below writes them.

### Preview a bone map in Blender

1. Dump some animations:
   `cargo run -p sjk-model --example gla_dump -- <_humanoid.gla> <animation.cfg> jka_anims.json BOTH_STAND1 BOTH_RUN1 BOTH_A2_T__B_`
   (the GLA and `animation.cfg` come out of the retail `assets1.pk3`, never into git).
2. Optional reference, posed Kyle as OBJ files to import beside the rig:
   `cargo run -p sjk-model --example glm_pose_obj -- <model.glm> <_humanoid.gla> <animation.cfg> <model_default.skin> <out dir> BOTH_STAND1:0 BOTH_RUN1:5`.
3. In Blender, make the rig's armature active, open
   [jka_retarget_preview.py](../scripts/blender/jka_retarget_preview.py) in the text
   editor, set `JSON`, edit `MAP` if the naming is not the mannequin's, run it, then
   call `run()`: each animation becomes a `JKA_<name>` action on the armature.
4. Compare the poses with Kyle's OBJs. The script's map mirrors `mannequin()`.

### Make a test model

With the add-on (above), from any rigged character you have the rights to. Keep test
models local: never commit them, attach them to issues or post them (the rule in
[AGENTS.md](../AGENTS.md#verification-and-scope)). Then in the game `/model <name>`,
third person, and the debug panel's `[rig-models]` checks.

## Next: emotes (planned, nothing built)

Sol asked for emotes on 14/10/2026. The plan: the add-on also exports the
armature's actions as glTF animations; [gltf.rs](../crates/sjk-model/src/gltf.rs)
reads animation channels (today it skips them); the rig plays a clip on its own
skeleton, overriding the retargeted pose while it runs (`/emote <name>`, client
side). Open questions: blending in and out of the driver's pose, whether moving
cancels it, and how others would see it (the protocol carries no custom animation,
so for now only the player would, as with the model itself).

## Checks

Unit tests in [gltf.rs](../crates/sjk-model/src/gltf.rs) and
`rig_model_tests.rs`; the world shot `world_shot::rig_model_shots::rig_model_on_a_demo`
puts the rig model `SJK_TEST_RIG` names on a player of the demo `SJK_TEST_DEMO`.
The bone map was first checked in Blender beside posed Kyle with `gla_dump`,
`glm_pose_obj` and `jka_retarget_preview.py` (recipe above).
