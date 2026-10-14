# Rig models

A rig model is a player model drawn from a glTF file with its own skeleton (any
number of joints, fingers, face, cloth), animated by the game's ordinary
`_humanoid` animations. Built 14/10/2026 ([status](status.md#rig-models-gltf-players-with-their-own-skeleton)).

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

## Checks

Unit tests in [gltf.rs](../crates/sjk-model/src/gltf.rs) and
`rig_model_tests.rs`; the world shot `world_shot::rig_model_shots::rig_model_on_a_demo`
puts the rig model `SJK_TEST_RIG` names on a player of the demo `SJK_TEST_DEMO`.
The bone map was first checked in Blender beside posed Kyle with the tools of the
unmerged branch `personal/rig-retarget-test` (`gla_dump`, `glm_pose_obj`,
`scripts/blender/jka_retarget_preview.py`).
