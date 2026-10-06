# Rendering and UI

The wgpu renderer lives in `sjk-viewer`. JKA content enters through owned BSP,
model and shader data; GPU resources stay in the viewer. The renderer consumes
BSP geometry, PVS visibility, lightmaps, shader stages and legacy models.

## Where to work

| Area | Entry point |
| --- | --- |
| GPU/device setup | [gpu_context.rs](../crates/sjk-viewer/src/gpu_context.rs) |
| World loading | [world_load.rs](../crates/sjk-viewer/src/world_load.rs) |
| World materials | [world_materials.rs](../crates/sjk-viewer/src/world_materials.rs) |
| Optional material maps | [material_maps.rs](../crates/sjk-viewer/src/material_maps.rs) |
| Main scene passes | [main_scene_pass.rs](../crates/sjk-viewer/src/main_scene_pass.rs) |
| Secondary views | [scene_views.rs](../crates/sjk-viewer/src/scene_views.rs) |
| Sun and real-time lighting | [sun_shadows.rs](../crates/sjk-viewer/src/sun_shadows.rs) |
| Post processing | [post_aa.rs](../crates/sjk-viewer/src/post_aa.rs) |
| Dynamic glow | [post_glow.rs](../crates/sjk-viewer/src/post_glow.rs), [glow_pass.rs](../crates/sjk-viewer/src/glow_pass.rs) |
| Eye adaptation | [post_exposure.rs](../crates/sjk-viewer/src/post_exposure.rs) |
| Frame timing | [frame_pacing.rs](../crates/sjk-viewer/src/frame_pacing.rs) |
| HUD integration | [hud.rs](../crates/sjk-viewer/src/hud.rs) |
| Material map generator (tool) | [sjk-materialgen](../crates/sjk-materialgen/src/lib.rs) |

The normal BSP path supports additional lighting, shadows, GI probes, ambient
occlusion, reflections and post processing. Feature presence does not establish
correctness on every map or GPU. Preserve the ordinary BSP/material path when
working on optional effects and validate shared WGSL programs on an actual GPU.

wgpu picks the graphics backend; JKR requests none. On Windows Vulkan is enumerated
first, and `WGPU_BACKEND=dx12` selects DX12, which compiles shaders with FXC
unless `dxcompiler.dll` is on the `PATH`. FXC can only assign a runtime-indexed
vector or matrix component (`v[i] = ...`) by unrolling the loops around it, and
fails (error X3511) when any of those loops has a runtime trip count. Write such
updates as whole-vector operations, as the skinning loop in
[gpu_skinning.wgsl](../crates/sjk-viewer/src/gpu_skinning.wgsl) does;
[world_shader_fxc_tests.rs](../crates/sjk-viewer/src/world_shader_fxc_tests.rs)
checks the world programs for this pattern.

Material compilation merges a shader's first two stages into one multitextured
pass under rd-vanilla's `CollapseMultitexture` rules, and never a later pair. The
colour generators are compared after `ParseStage` defaults: an unset rgbGen is
identity, or identityLighting when the blend source is `GL_ONE` or `GL_SRC_ALPHA`
([stage_colour.rs](../crates/sjk-shader/src/stage_colour.rs),
[world_stage_collapse.rs](../crates/sjk-viewer/src/world_stage_collapse.rs)).

Programs are embedded with `include_str!`, so they carry the checkout's line
endings. Code that patches a program by text with a pattern spanning a line break
normalises it first with [wgsl_source.rs](../crates/sjk-viewer/src/wgsl_source.rs);
`.gitattributes` keeps `*.wgsl` LF in new checkouts.

## Server shader remaps

The client consumes multiplayer `CS_SHADERSTATE` on joining and live updates,
and reliable `remapShader` commands, including during demo playback. Replacements
use the shared world/model material compiler: stage images, blending, alpha tests,
texture animation, scrolling, waves and deforms are replaced together. Shader
names are case-insensitive and extension-independent. Aliases resolve one hop;
remapping a shader to itself restores it. The destination's time offset is shared
by its users and subtracted from shader time; server offsets are parsed like C
`atof`. Removing an entry from the server's configstring alone does not undo it,
matching stock cgame.

The map's own worldspawn remaps apply when its world loads, as rd-vanilla
`R_LoadEntities` applies them: every key starting with `remapshader` (case-sensitive,
so `remapshader2` too) whose value is `old;new`, split at the first `;`. A value
without `;`, or an empty key or value, ends the scan as in C. Both shaders must
exist. These remaps are not gated by `cg_remaps` and last as long as the loaded
map. `vertexremapshader` keys are skipped: they apply only under `r_vertexLight`,
which JKR does not have.

Map, server and local remaps of one shader follow rd-vanilla, where every source
writes the same remapped-shader slot: the latest remap wins. Worldspawn remaps come
first; a later server remap replaces a local one, including a local restore, and
each `CS_SHADERSTATE` update applies all its entries again, as `CG_ShaderStateChanged`
does. The session keeps the server state and stamps each entry; the displayed map
keeps its worldspawn and local remaps and picks the latest of the three.

`cg_remaps` follows Tayst's policy: **0** disables server remaps, **1** (JKR's
default) accepts them while excluding player-texture configstring entries, and
**2** (SJK's default, as in EternalJK) includes those entries. Like Tayst, a reliable `remapShader` command is accepted
in either nonzero mode. JKR applies this preference live rather than requiring a
map reload; a server remap it excludes reveals the earlier remap it had replaced.
`listRemaps` lists the map's, the enabled server and the local remaps in the order
they were applied, with their source, and marks those a later remap overrides.
`remapShader <old> <new>` sets a temporary local remap for the loaded map, without
sending anything to the server or saving it to config. `clearRemaps` drops every
active remap, the map's included, until the server sends new entries.

Material recompilation and draw/fog/table invalidation happen on changes, not
per frame. Late-loaded entity materials also receive the current remaps.
A slot's first replacement keeps its loaded stages and classification aside. When
the slot maps to its own shader again at time offset zero they are put back, not
recompiled, so it matches its load state; only replaced slots keep a copy. A
nonzero offset on a slot's own shader compiles a replacement. Map
replacement/reconnection isolates server state. Authored sky-box remaps replace
the sky images while retaining the existing day/night policy.

Effects drawn from the effect atlas (EFX particles, missile trails, muzzle
flashes, beams, impact marks and blob shadows) follow the same remaps and local
overrides, as rd-vanilla `RB_BeginSurface` swaps the shader for every surface.
A remapped atlas entry samples a copy of its target's original stages, with the
destination's time offset subtracted from the effect's shader time; a target
missing from the atlas is loaded first, and one that cannot be loaded leaves the
effect unchanged. This runs after the world applies a remap change, never per
frame. Removing or self-remapping an entry, or `cg_remaps 0`, restores the
original stages; aliases do not chain.

This is shader replacement, not BSP editing. Existing server entity and sub-BSP
presentation use their separate paths. Collision and baked lightmaps are unchanged;
the modern renderer's extracted lamps, GI and sealed BSP shadow boundaries are
not rebuilt by a live remap. Worldspawn remaps use the same replacement path after
the world loads, so that extraction also sees the map's original shaders.
HUD and 2D pictures, saber blades and trails, generated surface sprites,
detached menu previews and mirror/portal classification do not yet follow
arbitrary shader remaps.
Sky remaps do not turn ordinary geometry into new sky portals. Broad community-map
and multi-lightmap registration parity remain to be verified.

Implementation: [compatibility state](../crates/sjk-client/src/shader_remaps.rs),
[map and local remaps](../crates/sjk-viewer/src/world_map_remaps.rs),
[event-time material updates](../crates/sjk-viewer/src/world_shader_remaps.rs),
[replacement compilation](../crates/sjk-viewer/src/world_remap_material.rs),
[effect atlas entries](../crates/sjk-viewer/src/effect_remaps.rs).

## Actor animation failures

Actor animation failures are isolated to the affected mesh. An invalid clip or
pose leaves that actor's last uploaded pose in place, suppresses its animation
audio for the failed frame, and logs once until evaluation succeeds again.
Other actors still evaluate and upload their joint palettes. This includes
custom NPC packs whose animation ranges exceed their skeleton's frame count;
the renderer does not rewrite their files or relax frame bounds checks. The
per-actor work lives in
[actor_pose_steps.rs](../crates/sjk-viewer/src/actor_pose_steps.rs). Which
models load at all, and when a player is drawn as Kyle instead, is described in
[player models](client.md#player-models).

## Load-time texture and light preparation

World installation prepares unique mipmapped texture arrays on up to four CPU
workers before assembling material stages. Prepared batches are limited to
64 MiB of mip pixels; a single larger animated array is processed alone. GPU
uploads and material-cache mutation remain on the installer. Non-mipmapped
filter modes retain their existing upload path. Layer resizing and mip filtering
are unchanged.

The process-local CPU mip cache retains at most 128 MiB of source and mip pixel
storage and 1,024 entries. Keys include target dimensions and retained decoded
image identities, whose upstream cache checks mounted content, so replacing a
texture cannot select a stale chain by filename. Active uploads may temporarily
retain evicted data. These budgets do not describe total process memory. Cache
locking and allocation happen during loading, not during drawing.

Lamp extraction uses up to four workers for ordinary maps as well as large ones,
preserving emitter output order. Each emitter reuses sorted neighboring patch
IDs while its spatial cell and patch count remain unchanged; live patch moments
and the lowest-eligible-patch selection are preserved. Emission masks use an
exact single-cell reduction for aligned power-of-two blocks and the previous
area-overlap calculation for other dimensions. These changes reduce preparation
work without reducing source count, texture resolution or lighting quality.

## Selected controls

| Cvar | Behavior |
| --- | --- |
| `r_dayNight` | Map-relative sun/sky atmosphere; default 1, restart required |
| `r_liveLighting` | Lighting tier; default 0. Tier 1 retains world shadow casters between frames; tier 0 also uses available baked indirect light. Applies at map load |
| `r_dayHour` | Solar hour; default 12 (noon, SJK); updated live when day/night resources are installed |
| `r_dayMinutes` | Minutes per simulated day; 0 holds the hour |
| `r_indirectBoost` | Live sky/bounce illumination multiplier, 0–4; default 1. Does not amplify direct lights or add bounce iterations |
| `r_ambientFill` | Live material-lighting floor in dark areas, 0–0.2; default 0.025. Fades as existing illumination increases |
| `r_ambientFillOcclusion` | Fraction of ambient occlusion applied to the readability fill, 0–1; default 1 preserves the previous response. Real indirect lighting keeps full occlusion |
| `r_sceneHdr` | Scene precision: 0 display format, 1 RGBA16F (default); restart required |
| `r_hdrExposure` | Base exposure multiplier, 0.25–4; live. Eye adaptation adjusts around it; `r_sceneHdr 0` ignores it |
| `r_autoExposure` | Eye adaptation, default 1 (on); 0 holds `r_hdrExposure`. Live; see [eye adaptation](#eye-adaptation) |
| `r_autoExposureMin`, `r_autoExposureMax` | Adaptation range in EV around the base: -2–0 (default -0.5) and 0–2 (default 1). Live |
| `r_autoExposureToBright`, `r_autoExposureToDark` | Seconds to settle when the view gets brighter (default 0.4) or darker (default 2.5); 0 is instant. Live, console only |
| `r_autoExposureKey` | Metered scene luminance shown at the base exposure, 0.03–0.8, default 0.18; higher is brighter. Live, console only |
| `r_dustMotes` | Dust in godrays, 0 (off) to 1 (default, SJK); live; renderer IMAGE tab; requires `r_volumetrics` |
| `r_normalMapping` | Normal maps on lightmapped world surfaces (rend2 convention); default 1 (SJK; rend2 and JKR 0), restart required |
| `r_specularMapping` | Specular, roughness and metalness maps on the same surfaces; default 1 (SJK; rend2 and JKR 0), restart required |
| `r_parallaxMapping` | Parallax from the height in `_nh`/`normalHeightMap` images; needs `r_normalMapping`; default 1 (SJK; rend2 and JKR 0), restart required |
| `r_materialMapsDebug` | Material-mapped surfaces only: 1 mapped normal as colour, 2 tint by maps found, 3 normal-map relief, 4 reflection probes alone, 5 without reflection probes, 6 emission maps alone; default 0, live, not archived |
| `r_emissiveMaps` | Emission maps (`<texture>_e`, SJK's) on lightmapped world surfaces; default 1, restart required. See [Emission maps](#emission-maps) |
| `r_emissionStrength` | Brightness of emission maps, 0 (off) to 7.97 in steps of 1/32; default 1, live, archived |
| `r_emissiveGlow` | Dynamic-glow halo around emitting texels (with `r_DynamicGlow 1`); default 1, live, archived |
| `r_emissiveLights` | Emission-mapped surfaces as lamps of real-time lighting, a power multiplier 0 (off) to 4; default 1, applies when a map loads, archived |
| `r_normalMapStrength` | Multiplier on the normal maps' relief (their x/y slope, after rend2's `normalScale`), 0–3.98 in steps of 1/64; default 1, live, archived. Material-mapped surfaces only; the floor mirrors' lookup keeps the authored relief |
| `r_cubeMapping` | Reflection probes on specular-mapped surfaces (rend2's name and meaning); default 1, needs `r_specularMapping`, restart required |
| `r_cubeMapSize` | Reflection probe face size, a power of two 32–512; default 128, restart required |
| `r_floorReflections` | Polished floors mirror the scene (see [Floor reflections](#floor-reflections)); default 1, live |
| `r_DynamicGlow` | Halo around `glow` shader stages: 0 off, 1 on (default), 2 saber blades only, 3 the glow alone (debug); live. See [Dynamic glow](#dynamic-glow) |
| `r_dynamicGlowStyle` | Glow blur: 1 EternalJK rd-vulkan's (default), 0 retail rd-vanilla's; live |

See [day_night.rs](../crates/sjk-viewer/src/day_night.rs),
[sun_shadow_settings.rs](../crates/sjk-viewer/src/sun_shadow_settings.rs) and
[post_hdr.rs](../crates/sjk-viewer/src/post_hdr.rs). A lighting tier alone does not
enable the day/night system. HDR here describes the scene buffer and display
mapping, not a claim of HDR monitor output.

Dust motes ([dust_motes.rs](../crates/sjk-viewer/src/dust_motes.rs)) appear only
in the current main-view godray volume. Each mote samples local sunlit scattering
and subtracts the same wide slice mean used by volumetric clarity. This is a
sample at the mote's depth, not accumulated brightness along the whole screen
ray: a beam farther away cannot light dust in dark air in front of it. Shadowed
cells and uniformly lit air at clarity 1 produce no dust. Beam radiance supplies
both colour and a smooth visibility weight, so the camera's light-grid sample
no longer controls dust brightness. No active volume, no sunlight, or a hidden
volumetric pass means no dust. Secondary views remain excluded.

The shader still generates up to 2048 world-space motes in a wrapping 640-unit
cube, with distance and near-eye fades, depth testing and peak opacity 0.35.
`r_dustMotes` controls density and opacity; it does not enable volumetrics implicitly.
The renderer settings' IMAGE tab calls it **Sunbeam dust**. Shared froxel parameters and depth mapping
keep the sampling coordinates aligned with godrays. The pass reuses existing
volume textures and slice means after their current-frame computation, with no
extra volume, readback or per-frame CPU light-grid sample. Uniform writes use
`FrameQueue`. Dust work is included in the `volumetrics` GPU phase, so compare
that phase with dust off/on at the same view and settings.

Per-map tuning remains unimplemented. Motes behind glass do not inherit its
tint, and their beam boundaries have the resolution of the selected volumetric
grid. Owner visual acceptance and populated-match readability remain open.

Beam-dust verification on 2026-10-02 used Linux / Rust 1.96.1, Ryzen 5 5500
and Radeon RX 9060 XT (RADV). Formatting, locked workspace build/tests and the
release viewer build passed. An external GPU probe executing the production
sampling function checked dark cells, uniform haze, a coloured beam isolated to
a distant depth, out-of-volume coordinates and points behind the eye. Dark/haze
and excluded samples returned zero; the distant beam did not light the nearer
sample. The godray shader's existing calculations are unchanged; only its
parameter ABI and depth mapping moved to a shared include.

External native 1280×720 captures covered dust off/on on `ffa3` and `ffa1`,
volumetrics disabled, and SDR output. With volumetrics disabled, off/on captures
differed by at most 3/255 per channel; the missing volume bypasses the dust draw.
The sampled combined volumetric/dust GPU phase was 0.133 → 0.138 ms on `ffa3`
and 0.136 → 0.140 ms on `ffa1`. These short fixed-view runs used a frozen shader
clock for capture, omit populated-match workload, and establish integration and
indicative GPU cost only. No verification hooks or fixtures are shipped.

These and the other `jkr_*` rendering cvars can also be changed in the client's
renderer settings page (Settings > VIDEO > Renderer; see
[client.md](client.md#renderer-settings)), with the same ranges and restart rules.

## Eye adaptation

SJK adapts the exposure to what the camera sees, as eyes do: in a dark area the
view brightens over a few seconds, and stepping or looking into bright light
darkens it a little, quickly, before it settles. This is SJK's choice and a
deliberate departure from JKR, whose exposure is fixed so that camera contents
never change how visible another player is. That concern is real: brightening
a dark room shows a player in its shadows sooner, and darkening after a bright
sky hides one for a moment. SJK therefore keeps the range small by default
(-0.5 to +1 EV around `r_hdrExposure`), puts the switch on the Renderer page
(IMAGE tab, "Eye adaptation"), and `r_autoExposure 0` restores the fixed
exposure exactly.

How it works ([post_exposure.rs](../crates/sjk-viewer/src/post_exposure.rs),
[post_exposure.wgsl](../crates/sjk-viewer/src/post_exposure.wgsl)):

- After the main view's resolve, a compute pass meters the scene target in
  8×8-pixel cells (four bilinear taps each) into a 64-bin histogram of log2
  luminance, weighting the centre of the screen four times the edges. It reads
  the scene before exposure, the legacy effect layer, HUD and text, so sabers,
  particles and the interface never drive it, and its result cannot feed back.
- A one-thread pass averages the histogram between the 30th and 97th
  percentiles (dark corners and small bright sources such as lamps and sun
  glints are ignored), takes the target `log2(r_autoExposureKey) − mean`,
  clamps it to the range and moves the current value towards it in EV:
  `e += (target − e)(1 − exp(−dt/τ))`, with τ `r_autoExposureToBright` when the
  view got brighter and `r_autoExposureToDark` otherwise. Near-black cells are
  left out; a frame with almost nothing else (a cleared or black frame) changes
  nothing.
- The result, `r_hdrExposure × 2^e`, stays in a 16-byte GPU buffer that the
  resolve and the effect layer's encode and write-back read the next frame.
  There is no readback, and a frame records two fixed compute dispatches and
  one 32-byte parameter upload through `FrameQueue`; everything is created with
  the scene targets.

The adaptation snaps to its target instead of easing when the view cuts: on a
new map or scene targets (map load, `vid_restart`, resize, toggling bloom or the
tone curve), when the followed player changes, on respawn and teleport
(`EF_TELEPORT_BIT`), and on entering spectating or intermission. It holds still
while the world is hidden or replaced (classic menus, loading, the hyperspace
flash) and while the Renderer settings page is open, so `r_hdrExposure`
comparisons there are not blurred. Without a match, demo or explored map (the
menu world) it holds plain `r_hdrExposure`. Demos have no seek, so starting one
is a map load. Black frames never count, so a first cleared frame cannot drive
the exposure to the maximum.

With `r_sceneHdr 0` the scene is 8-bit and already clipped at white, so
darkening would only grey its highlights: the minimum is 0 EV (it only
brightens), `r_hdrExposure` still does not apply, and brightening uses a
shoulder that keeps white at white (identity up to an exposed value of 0.5,
then a rational curve). Darkening needs `r_sceneHdr 1`, the default.

Menus, HUD, text, levelshots and the display gamma pass are never exposed.
Screenshots and captures show the adapted image, as the screen does.

Verification (2026-10-04, Windows 11, RTX 5080, Vulkan and DX12, headless
compute/render probes outside the repository; no client was run): with exposure
1 the new resolve (HDR and 8-bit, FXAA on and off) and the effect layer's encode
and write-back (all three encodings) matched the previous programs byte for
byte, as did HDR at exposure 1.5 against the old fixed constant. Metering a
uniform mid-grey (0.18) scene gave a mean of log2 0.18 within one bin (target
+0.002 EV); two stops darker clamped to +1 EV; a black frame left the state and
its pending snap unchanged; a 2% patch at 50× brightness did not move the
target; an 8-bit scene decoded to the same mean; the smoothing followed the
expected exponentials. The 8-bit curve was monotonic, identity at 1 and
inverted to within 1.4e-6. Timestamps for both passes on a synthetic scene were
about 0.01 ms at 1920×1080 and 0.02–0.03 ms at 3840×2160. Unit tests cover the
cvars and their ranges, the smoothing step, the 8-bit clamp, the snap and freeze
decisions, and translate the three programs to SPIR-V and HLSL. How it looks in
play, the default key on real maps and its cost inside a full frame are
unverified; the key was chosen so a typical sunlit outdoor view (`mp/ffa3`)
stays near 0 EV and needs in-game tuning.

## Volumetric silhouette coverage

The volumetric injection pass averages lighting over visible air samples within
each cell. Samples behind the scene surface are excluded from both the sum and
the count; visible samples in shadow still count as zero illumination. Counting
hidden samples as darkness reduced the background's light beside a foreground
player or geometry edge. The final composite still integrates only to the
pixel's surface depth, and wholly hidden cells remain empty. Grid sizes, sample
counts, shadow filtering and clarity settings are unchanged.

External Linux/RADV release captures on RX 9060 XT, based on `a993436` plus local
changes, reproduced the fringe on `mp/ffa5`. Player-present/absent and volume-on/off
comparisons isolated a 6-level RGB loss just outside the helmet; the correction
reduced it to zero. A nearby outside strip's mean negative difference fell from
4.415 to 0.001 levels. The camera matches the owner's mark, but the preview actor
placement is approximate because marks do not retain actor state.

At 1280×720, 600-frame throughput measurements were 1.116 ms before and 1.112 ms
after; at 3840×2160 they were 3.375 and 3.380 ms. Increasing the old grid to quality
3 instead measured 1.471 ms at 720p and retained a narrower fringe. These are
single-actor scene measurements with another client open, not GPU-isolated or
populated-match performance certification. Paired captures also cover `mp/ffa3`,
the marked `T2_Rancor` interior and 24 camera turns on FFA5 at 4K. The correction
does not remove all finite-grid undersampling; broader maps, motion and owner
playtesting remain required.

## Inferring fixture light from legacy materials

Prefer compiler-authored `q3map_surfacelight` power and `q3map_lightimage` masks.
Where that information is absent, fixed, non-sky, undeformed materials can supply
emission through view-independent additive stages. The source classifier uses the
shader's blend, colour generation, glow flag and actual texture mask; a texture
name containing "light" is not sufficient. Environment reflections, entity-coloured
stages, unresolved images and ordinary opaque paint do not become inferred lamps.

Explicit `glow` stages on these undeclared fixtures now use a white-radiance
fallback of 16, compared with 4 for unmarked additive decoration. This distinguishes
an authored luminous fixture from a generic additive effect. It is an artistic
fallback, not a recovered physical intensity: the flag carries no wattage. Declared
surface lights and effect-sprite extraction retain their previous gains. Both mean
energy and the spatial emission mask use the same gain; black housing and gaps stay
dark, and alpha covers still attenuate the source. Inference happens during loading,
without a new per-frame classification pass or texture-name exceptions.

The owner-marked `mp/ffa_mtd` view exposed insufficient inferred power rather than
unrecognized ceiling fixtures. Its `textures/massassi/light7` shader has an explicit
additive glow but no compiler surface-light power. The 12×12-unit faces generated
about 298 units of patch power with a 277-unit influence radius: their contribution
was already fading strongly around floor distance. The new inference gives the
sampled patch about 1,191 power and a 550-unit radius. Disabling candidate importance
filtering or static shadows in an external diagnostic barely changed that view;
neither diagnostic change is included in production.

Verification (2026-10-03, local change based on `8f692ac`): workspace build,
all nine existing Cargo tests and doc tests, formatting, and the optimized owner
build passed. Eight external source-policy cases checked explicit glow versus
unmarked/declared fallback, shared mask energy, dark texels and entity/environment/
opaque/missing-image exclusions. Linux/RADV release captures at 1280×720 covered
the marked custom-map view and stock `ffa3`/`ffa1`; the custom view retained its
dark atmosphere with more local illumination. In that map the inferred source
count grew from 27,465 to 27,576, grid references from 5.79 to 5.96 million, and
the static visibility atlas from 121.5 to 122.2 MiB. Settled sampled GPU totals
were approximately 0.582→0.568 ms there, 0.506→0.505 ms on `ffa3`, and
0.742→0.715 ms on `ffa1`. These sparse samples with a separate owner client still
running show no observed regression in those views; they do not establish a
speedup or certify populated-match performance. Windows runtime and wider
custom-map coverage remain unverified. No test harness or assets were added to Git.

The fallback cannot recover compiler-stripped point lights or confidently identify
a lamp drawn only into ordinary diffuse paint. Improving those cases needs additional
authored evidence or a separately validated inference method. No claims of complete
custom-map light recovery are made.

### Static model fixtures

`misc_model_static` lamps now enter the same source collection as BSP fixtures.
Previously their model materials could display glowing bulbs while none of their
placed triangles supplied room lighting. Resolve each model once at map load,
reuse the existing static-placement parser, and integrate the luminous surfaces
with their placed position, rotation, inverse-transpose normal and scale. Each
placement keeps its own emitting area. Moving/network entities, pickups and
actors do not become permanent sources. The resulting lights use the existing
lamp grid, static visibility, receiver cache and probe-bounce path; there is no
new per-frame discovery or model loading.

Unlit opaque replacements can hide earlier shader stages. Source inference now
examines the surviving stage suffix, so an opaque luminous face following an
additive stage is eligible. Bright texels qualify an otherwise unlit fixture even
when dark housing lowers the whole-image average; power still uses the complete
area-weighted mask. Ordinary diffuse/vertex/entity-lit paint remains excluded.
A model surface's luminous pieces share a range calculated from their combined
power, so texture subdivision does not give every small piece a prematurely
short reach. Their summed energy is unchanged. This shared range is per placed
model surface; it does not combine all instances across the map or change BSP
surface-light range policy.

Verification (2026-10-03, local change based on `8f692ac`, on top of the inferred
glow/readability preview): release Linux/RADV captures at 1280×720 covered both
owner marks on `t2_rancor` and stock `mp/ffa1`/`mp/ffa3`. Static models were also
rendered in the external captures. Rancor gained 3,485 model patches (4,340 total
lights became 7,825); grid references rose from 755,662 to 1,289,445. Both marked
rooms gained localized illumination and remained dark. The sampled `ffa1` image
changed by at most 1/255, and `ffa3` by at most 2/255. External policy checks
covered hidden shader stages, unlit versus diffuse paint, bright inserts in dark
masks, unchanged integrated flux and combined fixture reach. Workspace build,
Cargo tests, formatting and the optimized owner build passed. A paired Rancor
capture with day/night disabled was byte-identical before and after.

Settled GPU medians, from 11–12 samples per view, were 0.893→1.036 ms and
0.822→0.839 ms at the Rancor marks, 0.722→0.721 ms at `ffa1`, and
0.521→0.527 ms at `ffa3`. The additional Rancor sources have a measurable cost.
A separate owner game was running; these are fixed-view GPU samples, not
exclusive-device total-frame or populated-match certification. Windows runtime
and other model-heavy maps remain unverified.

This repairs missing source participation, not all dark-room readability.
`t2_rancor` also requested baked ambient light (`ambient 20`); that background
illumination is not reconstructed by this change. Its inferred fixtures still
lack authored physical power, unlike several strong surface lights in `ffa1`.
Prop housing has not been added to the immutable world occluder geometry, and
moving/triggered model lamps remain outside this static-source path. No global
exposure, sun, ambient-fill or authored-light-power change is part of this step.

### GI traversal correction

The voxel GI tracer now measures a positive forward distance to the next cell
boundary for either ray direction. Previously negative-direction rays used a
signed coordinate difference divided by an absolute direction, producing hits
behind the ray origin and incorrect surface/depth samples. Traversal also checks
the maximum range before accepting an occupied cell, preventing hits beyond the
requested distance. This changes the probe tracer, not the separate triangle
visibility tracer used for direct lamp shadows.

Verification (2026-10-03, local change based on `8f692ac`, including the preceding
fixture preview): an external GPU harness compared the actual WGSL against an
independent double-precision ray/AABB reference. All 8,302 cases pass, covering
both axis directions, all diagonal octants, random oblique hits/misses, and ranges
below/at/above intersections. The preceding shader failed 3,174 of these cases.
Workspace build, all nine existing Cargo tests and doc tests, formatting, and an
optimized owner build passed. No diagnostic or test code was added to production.

Fixed-camera release GPU comparisons used the same 1280×720 settings, hour,
exposure, lamp data and indirect gain before and after. Both marked `t2_rancor`
images were byte-identical. The `ffa1` comparison changed by at most 1/255;
`ffa3` had a small local difference (mean absolute channel difference 0.0105/255,
maximum 10/255). Thus this is a verified correctness fix, not evidence that
Rancor's low visibility is solved. GPU readback confirms its live probes do update
and contain nonzero lighting; room-wide receiver coverage and effective bounce
strength still need investigation before deciding on another brightness change.
Rancor's 208-unit probe spacing aligns with its 16-unit voxel grid, so the old
sign error vanishes at those ray origins. The corrected range check still changes
some distant probe values. At the earlier `mp/ffa_mtd` mark (624-unit probe spacing,
64-unit voxels), the correction does change local illumination: mean absolute RGB
difference 0.0358/255, maximum 38/255. It does not broadly brighten that room.
Paired readback on Rancor changed the L0 coefficient of 11,026 of 29,541 live
probes above a 1e-7 threshold, while the closest probes to both marks remained
essentially unchanged.

Settled GPU medians (11–13 samples per view) were 1.038→1.038 ms and
0.825→0.825 ms at the Rancor marks, 0.742→0.733 ms at `ffa1`, and
0.512→0.533 ms at `ffa3`, and 0.639→0.649 ms at `mp/ffa_mtd`. A separate owner client was running. These sparse
fixed-view GPU results do not establish a speedup, exclusive-device performance,
31-player frame times or Windows behavior. The owner approved publication of the combined lighting/transition preview;
this does not close the remaining dark-interior investigation.

## Indirect lighting and dark-area readability

These controls affect the day/night real-time material-lighting path, including
world surfaces, actors and reflection receivers. They do not change the ordinary
baked-lighting path, direct sunlight, lamp intensity, emissive materials or exposure.
The indirect multiplier includes the probe sky contribution and the sky fallback;
it is not a multiplier exclusively on secondary bounces. It is applied at the
receiver after probe sampling, so boosted energy never feeds back into the probe
solver. World-space probe visibility and wall rejection remain unchanged.

Defaults retain the previous lighting response. Start a comparison with
`r_indirectBoost 2`, leaving `r_ambientFill 0.025` and
`r_ambientFillOcclusion 1`. For a separate, stronger readability comparison, try
fill `0.05` and fill occlusion `0.5`. These are experimental settings, not new
recommended defaults. Restore all three values to `1`, `0.025`, `1` respectively
for the original response. Controls are live and archived.

Fill occlusion only changes the small artificial fill: at `0.5`, fully occluded
fill retains half its unoccluded contribution. Physical sky/bounce stays fully
occluded. Fill remains material-modulated, so black materials stay black. A room
with almost no indirect energy may change little under the multiplier alone;
blindly increasing it can brighten outdoor shade before fixing that room.
Local exposure and player-specific contrast effects are not part of this
experiment; [eye adaptation](#eye-adaptation) is separate and scales the result.

Verification of the local change based on `8f692ac` (2026-10-03): Linux/Vulkan,
Radeon RX 9060 XT, external release captures at 1280×720, fixed 11:00 sun.
The outdoor `mp/ffa3` and indoor `mp/ffa1` views exercised live changes and floor
reflections; a Kyle preview also rendered under the live controls in `ffa1`.
Returning both controls to 1 reproduced each starting image exactly.
Against separately compiled preceding shaders, baseline captures differed by at
most 19/255 on `ffa3` (mean absolute channel error 0.00057/255) and 1/255 on `ffa1`
(mean 0.0000125/255); cross-build images are not claimed bit-identical.
An external 1,024-case GPU check passed baseline agreement, isolated indirect gain,
bounded fill and zero-energy behavior with fill disabled. HDR/tier 0,
SDR/tier 2 and day/night-disabled paths rendered successfully; the latter stayed
byte-identical across the live control changes. Workspace build,
tests and formatting, and the optimized owner client build passed.

Sampled steady-state light-pass medians for old/new default shaders were
0.212/0.214 ms on `ffa3` and 0.157/0.158 ms on `ffa1`; enabling the controls showed
no consistent additional cost at this resolution. A separate owner game remained
running, so these are non-exclusive GPU checks, not a performance certification.
Populated combat visibility, broader map coverage and 31-player performance remain
owner-playtest/benchmark work. No new shadow lights, passes or probe rays were added.

Sun-shadow filtering compensates for the receiver surface's slope across the
full texel footprint of a linear depth comparison. A half-texel allowance only
covers samples midway between texel centers and can produce bands of false
self-shadowing on flat surfaces. Keep this allowance tied to the cascade's
texel size and receiver slope; increasing a fixed world-space offset can detach
shadows from walls and ground contacts.

The full-footprint correction was checked against the preceding shader from
`4a8fe31` with external release captures on Linux/RADV (Radeon RX 9060 XT), at
1920×1080 with HDR, lighting tier 0, 2048-pixel shadow maps and a fixed 9:00 sun.
The reported `mp/ffa3` view and a nearby camera position lost the ground bands
while retaining the ship and wall shadows; an `mp/ffa1` interior comparison
showed no obvious regression. GPU frame means were 3.336 → 3.341 ms for the
nearby `ffa3` view and 4.122 → 4.125 ms for `ffa1` (64 measured frames each).
These isolated captures are not a full gameplay benchmark or coverage of every
map, sun angle, graphics backend or shadow quality setting.

Sun-shadow cascades share a world-space reconstruction footprint while blending.
The close transition covers the last half of its axial range; the view/far
transition covers the last quarter. The minimum footprint is based on 1.5 texels
of the active cascade, interpolated toward the finer map during a transition.
This avoids cross-fading independently sharp and soft versions of an edge.
The accepted transition refinement added approximately 0.074 ms of GPU work in
the marked 4K ship view on Linux/RADV (Radeon RX 9060 XT, tier 0, HDR,
2048-pixel shadow maps, 16 base taps, fixed 11:00 sun). It does not establish
complete temporal invariance.

Static world and moving casters now keep separate depths in the close and view
cascades. A nearby player cannot replace a distant building's depth in the world
filter's separation estimate. Receivers multiply the independently filtered
visibilities. This approximates their union; it is not exact area-light visibility
for multiple blocker depths. Volumetric lighting samples both depths at each tap.
Tiers 0/1 reuse the existing static maps without copying them under moving casters.
Tier 2 refreshes separate world maps each frame, adding two depth textures
(32 MiB at 2048 pixels, 128 MiB at 4096) and two render-pass boundaries.

World penumbra width still grows with caster-to-receiver separation. The blocker
search bilinearly reconstructs positive gaps and coverage from 16 positions,
correcting for the receiver plane at each texel. Its reach is 24 world units,
expanded if needed for the minimum reconstruction footprint; it no longer clips
the close cascade's broad penumbrae at twelve tiny shadow texels. Reconstruction
uses a truncated Gaussian disk to reduce the visible rim of an equal-weight disk.
`r_sunShadowTaps` (4–32) supplies the base count, with up to four times that budget
for broad filters and a fractional final tap for continuous count changes.
The full-texel slope correction and small normal offset remain unchanged.

Moving casters use the shared contact footprint, independently of the building's
penumbra. Their shadows therefore remain comparatively sharp even when a moving
caster is high above its receiver. Actor-only shadow mode retains separation-based
filtering. Neither approach resolves all shadow-map occlusion or sampling limits.

The preceding combined-map RMS filter was rejected in owner playtesting: the
reported ground-fixed boundary and player interaction remained visible, especially
when structures were far from the surface receiving their shadow. The current
separation/filter changes were checked at the third `mp/ffa3` mark
`(-425.691, -1061.451, 73.832)`, yaw `132.718`, pitch `38.102`, with an external
release harness carrying the production shadow code from the working tree based
on `980e693`. Linux/RADV captures on the same RX 9060 XT at 1080p and 4K,
HDR, tier 0, 2048 maps, 16 base taps and fixed 11:00 sun showed a smoother broad
edge. A frozen Kyle model was moved through seven positions in the actual dynamic
caster pass. GPU attribute readback found no visibility increase on unchanged
receivers when adding the actor (over 515,000 compared pixels per position).
The previous combined-map filter's maximum increase was only 0.000119 in this
particular probe; the more visible improvement is the distinct player shadow
instead of its inheriting the building's broad blur. This is not a complete
reproduction of every reported player interaction.

Earlier wall/ship views and a seven-position camera approach were also captured.
Tier 2, day/night disabled and actor-only modes passed GPU smoke checks. At 4K,
64-frame means compared with the preceding combined-map filter were
1.579 → 2.100 ms for the light pass and 4.212 → 4.700 ms for GPU work excluding
capture/readback, approximately 0.49 ms added. These are empty-scene measurements
on an active desktop, with release compilation running concurrently, not a
populated-match benchmark. Workspace build/tests, formatting and the production
release build passed; Cargo runs no bundled regression tests. The owner accepted
the release playtest on 2026-10-02. Broader live-animation checks, other GPUs and
exhaustive quality/map coverage remain open.

## Entity render effects

cgame custom shaders on actors (force shells, pickup placeholders) are extra
instances of the same mesh in [entity_materials.rs](../crates/sjk-viewer/src/entity_materials.rs).
The same path also draws rd-vanilla's `RF_FORCE_ENT_ALPHA`: the mesh keeps its
own surface shaders, the stage program replaces vertex alpha with the
instance's alpha, and each stage uses an alpha-blended, depth-tested,
non-depth-writing variant of its pipeline
([world_forced_alpha.rs](../crates/sjk-viewer/src/world_forced_alpha.rs)).
Like rd-vanilla's fixed `GL_State`, the variant has no alpha test: the stage
program is specialized to skip it, so a GE128 cut-out whose forced alpha is
below one half stays visible, and blending still hides its transparent texels.
These draws close the blended entity list, like the stock post-render queue,
but particle effects still composite after them. Model materials compile these
variants at map load (depth-tested only); stages without an alpha test share
keys with ordinary blended stages.

A shield hit (`EV_SHIELD_HIT`, timer in `shield_hit.rs`) is drawn as the body
re-drawn with `gfx/misc/personalshield`, like single player. `cg_shieldSphere 1`
draws stock multiplayer's `halfShieldShell` sphere instead
([force_overlay_submission.rs](../crates/sjk-viewer/src/force_overlay_submission.rs)).
The shader blends `GL_DST_COLOR GL_ONE`, a bare multiply of the pixels behind it, so the body
shell is faint; it is drawn `cg_shieldBrightness` times (default 4) and fades over its own
hit's length (`LegacyShieldHit::body_brightness`), where the sphere keeps stock's fixed 2 s.
The body shell's tint is weighted to green (`(0.3b, b, 0.3b)`), as the texture itself reads blue.

Force Protect is a green `gfx/misc/forceprotect` shell on the body and Force Absorb
a blue `gfx/misc/personalshield` one, drawn on every player holding Absorb (JoF EJK's
`cg_alwaysShowAbsorb`, always on here; stock draws it only on your own body and on
team-power hits). With `cg_spProtAbsColor 1` (default, JoF EJK's `cg_spprotabscolor`),
a player with both Protect and Absorb active gets a single cyan protect shell instead
of the green and blue pair, as single player draws it. Another player's Absorb is read
from their entity's own power bit, so both the combined and the plain shell show on them
([force_overlays.rs](../crates/sjk-client/src/force_overlays.rs)). JoF EJK's
base-enhanced server check is not ported.

`EV_PLAYER_TELEPORT_IN/OUT` play `mp/spawn` where the player's box (mins z -16, maxs z 40)
lands when dropped up to 4096 units, as `cg_event.c` does, not at the player's centre;
over a void no effect plays.

The Force Speed afterimages use it: two copies of the actor in its current pose
at alpha 100 and 50, spaced by `(int)(6 * speed * 0.004)` units along the
recent path, while the entity has `PW_SPEED` and `cg_speedTrail` is nonzero
([speed_trail.rs](../crates/sjk-viewer/src/speed_trail.rs), after
`cg_players.c:10841-10906`). Copies are excluded from shadow casting. The
mind-trick fade below suppresses them, out and back in, as in stock. The
`PW_SPEED` saber trail
(`cg_players.c:7319`) is not implemented. This has passed unit tests only;
appearance has not yet been checked on a GPU against the stock client.

Force hand and body effects
([force_power_submission.rs](../crates/sjk-viewer/src/force_power_submission.rs))
follow `CG_Player`: the Lightning (`activeForcePass` 1-3) and Drain (4-6:
`mp/drain`, `mp/drainwide` at level 3) beams from the left hand, the Push/Pull
or Grip puffs there while `PW_DISINT_4` is set, and the body push blur for
`EF_BODYPUSH`. The local player's effects come from its predicted player state,
since stock rebuilds the local entity from `cg.predictedPlayerState` and the
server never sends it; in first person they start at the hidden body's left
hand, and Grip's puffs are third-person only. A player who mind-tricked the
viewer still shows its beam and hand puffs, which stock draws before its
mind-trick cut-off; only the body push blur is hidden for it. JKR drew none of
the local player's own effects and hid all of a trickster's; SJK fixes both.

EFX `bounce` and `intensity` are one key in retail: both set a primitive's
single elasticity value (default 0.1) and its physics flag (`FxTemplate.cpp:44`,
`:448-458`, `:2128`). A particle bounces by it, an electricity bolt uses it as
its jaggedness (`FxScheduler.cpp:1502-1508`) and a camera shake as its strength.
JKR read `bounce` as a particle bounce only, so Drain's `bounce 0.8 2` bolts
kept a jaggedness of 0.1 and were drawn almost straight; SJK applies it as
stock. `elasticity` and `chaos`, which JKR also accepted, are not retail keys
and are ignored.

Effect particles share one pool of 4,096 slots, plus 256 kept for per-frame
billboards (talk balloons, pickup icons, hook ropes)
([particle_types.rs](../crates/sjk-viewer/src/particle_types.rs)). Stock caps its
FX list at 1,800 (`MAX_EFFECTS`, `FxPrimitives.h`) and, once full, frees and reuses
`effectList[0]` for every new primitive (`FX_GetValidEffect`, `FxUtil.cpp`), so each
new puff replaces the last and rocket trails vanish in a barrage. SJK instead makes
room at the start of each frame: when fewer than 256 to 1,024 slots are free (the
amount grows after a frame that ran out), the effect particles closest to the end of
their lives are removed early, so new trail puffs and impacts always spawn and old
smoke trails get shorter
([particle_room.rs](../crates/sjk-viewer/src/particle_room.rs)). The instance buffer
holds every slot's billboard at eight shader stages; it used to hold 1,024 instances
in all and dropped the newest particles first.

Mind Trick follows `CG_Player` (EternalJK `cg_players.c:10191-10345`, stock
code; [mind_trick.rs](../crates/sjk-client/src/mind_trick.rs)). A player who
tricked the viewer fades out at 0.5 alpha per millisecond from 255 (about half
a second), drawn with `RF_FORCE_ENT_ALPHA` like the speed afterimages, then is
hidden: no body, held weapon or held saber (a thrown saber still shows), no
shells or afterimages. It casts no blob shadow while the trick lasts. When the
trick ends it fades back in at
1 per millisecond. Its Force beam and hand puffs stay visible throughout, since
stock draws them before its mind-trick cut-off. A player unseen for over a
second starts again from opaque. The viewer's active Force Sight, at any level,
sees through every trick (`CG_IsMindTricked`); the server also ends the trick.
The trickster sees `force/confusion_old` over the head (`*head_top`, else
`ceyebrow`) of each player it tricked, unless that player's Sight is active.
Deviations: a held saber's hilt stays opaque during the fade (blades are opaque
in stock too), and a fading body casts no sun shadow. JKR drew tricksters fully
and had no confusion effect; SJK adds both.

Set `JKR_FRAME_BUDGET=1` for frame-work and GPU-phase diagnostics. Measurements
must name the build mode, GPU, resolution, settings, map and population. Separate
loading/shader warmup from steady frames and CPU work from GPU timings. The
500+ FPS target remains open; neither a single GPU timestamp nor an uncapped
empty scene demonstrates it.

### First-person view weapon

The first-person gun hangs on its `_hand.md3` tag rig, as `CG_AddViewWeapon` places
it ([first_person_weapon.rs](../crates/sjk-viewer/src/first_person_weapon.rs), policy
in [view_weapon.rs](../crates/sjk-client/src/view_weapon.rs)). Melee shows no
weapon: `WP_MELEE` registers no hand rig (`CG_RegisterWeapon`), so stock hangs the
baton view model of its item on handle 0, whose identity tag puts it at the view
origin with all of its geometry behind the eye. SJK used to draw it on the stun
baton's rig, so first-person melee showed a baton.

## Saber trails

[saber_trail.rs](../crates/sjk-viewer/src/saber_trail.rs) follows codemp
`CG_AddSaberBlade` and `CTrail`: every frame at least 3 ms after the last, a
blade adds one slice from its remembered muzzle and tip to the current ones.
A slice lives `trailLen / 5` ms of the current `saberMove` (30–40 ms for most
moves, 40 ms when the move authors none) and fades by scrolling the clamped
blur texture, not by alpha. The short visible arc is stock behavior; frame rate
changes the slice count, not the arc's duration. Slices split along new tip to
old muzzle as `CTrail::Draw` does. With `cg_saberContact` on, the tip stops at
the first world surface the blade enters
([saber_trail_edge.rs](../crates/sjk-viewer/src/saber_trail_edge.rs)); stock
also stops it at solid brush entities, which JKR does not trace yet. A flying
primary saber trails and shares the owner's blade state, as in stock. Not yet
drawn: the extra trails stock adds while `PW_SPEED` is set with `cg_speedTrail`
and during super-break win animations.

Blade/wall contact ([saber_contacts.rs](../crates/sjk-viewer/src/saber_contacts.rs))
plays a wall-hit sound once a blade has stayed in the wall since the previous
frame, at most every 100 ms per blade. Like stock's `S_StartSound(..., -1,
CHAN_WEAPON, ...)`, all wall hits share one source and channel, so each new hit
replaces the previous one instead of overlapping it.

Static sun cascades carry a small min/max depth atlas: one pair of full-precision
bounds per 64×64 shadow texels, with separate view, close and far layers. A layer
is rebuilt immediately after its source map changes. Receivers bound the entire
possible filter footprint, including bilinear neighbours and receiver-plane
variation. Only footprints proved fully lit or fully blocked bypass the blocker
search and Gaussian reconstruction; uncertain footprints run the existing filter.
Moving-caster filtering is unchanged. At 2048 resolution the atlas occupies
24 KiB. Coalesced tile reads keep rebuilding practical in tier 2 as well as the
held-cascade modes. Golden-angle sample directions are constant shader data;
sample counts, radii, weights and cascade transitions are unchanged.

Real-time world shading skips baked-lightmap texture reads when the composition
hook replaces their RGB. Uploaded BSP lightmaps and their fallback have alpha
one; authored image alpha and animation still follow their normal paths.
The expensive main-view and floor-reflection material passes also prime depth for static surfaces
whose first stage guarantees opaque coverage. Deforms, sprites, alpha tests,
polygon offsets and special depth/blend modes keep their ordinary path. Depth
priming reuses the existing visibility ranges and indirect argument storage,
with direct draws as a fallback. It is limited to active real-time lighting;
reflections use their own camera, depth target, receiver frustum and scissor.

The main view's PVS source cluster is the leaf of the camera actually used for the picture
(`view_position`, as `refdef.vieworg` in stock), not of the player's eye. In third person
the camera sits behind and above the eye, often in another cluster; taking the eye's cluster
culled walls the camera could see. Reflection, portal and scene views already used their own
eye.

Camera-range caches also retain their PVS/area selection independently of the
camera frustum. Turning or moving within a cluster rechecks bounds but reuses
the same candidate indices; a source-cluster, area-mask or PVS-mode change
invalidates that selection. Storage is reserved at map load, at one index per
static draw. Unbounded and blended materials keep direct traversal.

External release replay checks compared these changes with `b4debe4` on
Linux/RADV, Ryzen 5 5500 and Radeon RX 9060 XT. Each recording contains 31
player profiles on `mp/ffa3` or `mp/ffa1`; 19–23 and 16–27 actor groups,
respectively, were evaluated during the measured routes. Settings were render
scale 1, HDR, day/night held at 11:00, lighting tier 0, 2048 shadow maps, 16 base
taps, volumetrics 1 and anisotropy 16. After five seconds of replay warmup,
6,660 frames sampled a 20-second route at 333 simulated frames per second.

| Replay / resolution | GPU mean before → after | Total frame mean before → after |
| --- | --- | --- |
| `mp/ffa3`, 3840×2160 | 5.302 → 4.575 ms | 5.473 → 4.746 ms |
| `mp/ffa1`, 3840×2160 | 6.714 → 5.808 ms | 6.894 → 5.983 ms |
| `mp/ffa3`, 2560×1080 | 2.026 → 1.845 ms | 2.499 → 2.474 ms |

These are offscreen full-frame replays, with no per-frame readback or capture
copy and no live network/audio output. They show a CPU limit at the smaller
resolution, not achievement of the 2 ms total-frame goal. Native presentation,
live matches, other GPUs and exhaustive community-content coverage remain open.

External GPU instrumentation compared the conservative shadow result with the
full filter at 656,749,476 accepted samples across both maps, moving sunlight,
tier 2, 1025-pixel shadow maps with 32 base taps, and 4K output. Maximum
visibility error was 0.00000012; none exceeded the 0.000002 verification threshold.
Twenty 4K scene snapshots across both routes retained the reference appearance;
one snapshot's kill-feed expiry differed with wall-clock timing. Baked lighting
and actor-only captures matched byte for byte. Fullbright, lightmap debug and
the non-table/direct-draw fallback also passed GPU and image checks. These
finite samples are not exhaustive image equivalence for every view or material.

Formatting, locked workspace build/tests and the release client build passed.
Cargo still runs no bundled regression suite. A native 1280×720 `mp/ffa3` static
map run rendered for a 45-second process lifetime without panic or GPU validation
error, using an isolated config. This is an integration smoke check, not a
populated-match performance result. The combined retained changes also completed a
45-second native `ffa1` crowd-replay process run with an isolated configuration
and no panic or GPU validation error. Concurrent compilation makes that latter
run an integration check only.

A follow-up CPU cache comparison used the same routes/settings at 2560×1080:
`ffa3` world-pass encoding fell from 0.565 to 0.513 ms and total frame mean from
2.462 to 2.420 ms. `ffa1` encoding fell from 0.884 to 0.866 ms; its total mean
was essentially unchanged (3.119 to 3.131 ms). An external verifier matched
3,178,666 cached/direct range results across the replays and forced area-mask,
source-cluster and missing-PVS transitions. Twenty further 4K captures retained
the scene appearance, with small pixel differences and the known wall-clock
kill-feed difference. This is finite coverage, not a universal cache proof.

Floor-reflection depth priming was compared in alternating order on the first
10 seconds of the `ffa1` route at 4K (3,330 frames per run, two runs per variant).
Mean GPU time was 5.453 → 5.417 ms; the reflected-plane phase was
1.116 → 1.081 ms. On the full 20-second route at 2560×1080, total frame means
were 3.135 → 3.094 ms. Twenty 4K captures across both maps showed only small
pixel differences (maximum 8/255), with the shadow filter and reflection
resolution unchanged. Paired 4K `ffa3` runs measured essentially unchanged
GPU time (4.547 versus 4.550 ms), with almost no floor-reflection work on that
route. Other mirror/portal types retain their previous path.

Particle stage selection borrows the atlas and evaluates stages as they are
consumed, retaining the existing eight-stage cap, order, animation, waveform,
texture transforms and missing-shader fallback. This avoids filling and copying
eight samples for every effect, including callers that only need the first.
The sampling implementation lives outside the frame-orchestration module.
Paired 3,330-frame `ffa3` runs at 2560×1080 reduced billboard preparation from
about 0.067 to 0.047 ms and effect geometry preparation from 0.034 to 0.029 ms.
Total-frame differences were within run variation; this is a CPU phase result.
An external comparison matched 3,360 stage samples bit for bit across both
loaded atlases and missing, empty and over-capacity shader cases.


Entity sorting caches each source material's immutable shader sort and first-stage
pipeline keys at material creation, including late custom materials. It preserves
missing-material defaults, opaque tie-breaking and back-to-front blend order;
per-frame draw records and allocation behavior are unchanged. Alternating
6,660-frame runs at 2560×1080 reduced instance preparation from about 0.092 to
0.078 ms on `ffa3` and 0.085 to 0.075 ms on `ffa1`. Total-frame differences were
within run variation. An external comparison matched 1,362,200 ordered draw
entries against the original comparator across both routes. Twenty further 4K
captures retained the scene appearance (one pixel differed by 12/255; all others
by at most 8/255).
The draw queue also classifies stage-major eligibility once when a draw is added,
rather than repeating material lookups in every colour pass. Missing materials
and depth-less draws retain their previous handling, and the 32-byte draw size is
unchanged on the measured 64-bit build. Two alternating 6,660-frame runs per
variant reduced CPU world-pass encoding by roughly 0.011 ms on `ffa3` and
0.016 ms on `ffa1`; total means fell by 0.034 and 0.026 ms respectively. The
same external verifier checked every cached classification and all material keys,
including missing-key defaults, across both replays; 20 captures showed only
small pixel differences (maximum 13/255 at one pixel).

Volumetric integration carries each slice's end depth into the following slice
instead of recomputing the same boundary. Sample locations and accumulated
scattering retain their existing arithmetic. Paired 3,330-frame 4K runs on both
routes saved about 0.0033 ms in the volumetric phase; 20 captures retained the
scene appearance, with small pixel differences (maximum 12/255 at one pixel).

Ambient occlusion reuses the fixed 16 sample directions and radii, preserving their
f32 expressions, sample order, reach and strength. Depth reconstruction omits the
ray normalization that cancels in its intersection ratio and scales the degeneracy
guard accordingly. Paired 3,330-frame 4K runs reduced the AO pass from 0.369 to
0.344 ms on `ffa3` and 0.354 to 0.330 ms on `ffa1`; total GPU means fell by
0.023 and 0.026 ms. Twenty captures retained the scene appearance, with one pixel
differing by 12/255 and all others by at most 8/255. These are measurements on the
same Vulkan setup, not exhaustive equivalence across all maps and backends.

## Dynamic glow

Shader stages marked `glow` get a blurred halo, as in stock Jedi Academy and
EternalJK (`r_DynamicGlow`). Stock draws the frame's surfaces a second time with
only their glowing stages into a black image that shares the scene's depth, blurs
it and adds it to the frame (rd-vanilla `tr_backend.cpp:1736-1800`). SJK does the
same for the main view:

1. **Glow flags.** The parser keeps each stage's `glow`. A hardware pass carries the
   flag of its first source stage: `CollapseMultitexture` moves only texture
   bundles, so a glowing stage merged under a non-glowing lightmap stage does not
   glow, as in stock ([world_stage_collapse.rs](../crates/sjk-viewer/src/world_stage_collapse.rs)).
   A material glows when any pass does (`hasGlow`). Effect shaders keep the flag per
   stage; saber blades glow and their cores do not (`sabers.shader`: the `*_glow`
   sprites, `saberBlur` and `swordTrail` carry `glow`, the `*_line` cores do not).
2. **Glow pass** ([glow_pass.rs](../crates/sjk-viewer/src/glow_pass.rs),
   [world_glow.rs](../crates/sjk-viewer/src/world_glow.rs)). After the main view's
   effects, the glowing passes of visible world surfaces (opaque, then blended) and
   the glowing stages of the frame's entities draw into a scene-sized 8-bit image,
   cleared to black, with the scene's depth attached read-only. They use the scene
   pass's vertex paths, bind groups and blend with glow variants of the stage
   pipelines (the image's sRGB view, no depth write), compiled on first use like
   the scene's entity pipelines. Glowing billboards, cylinders, lines and
   electricity, saber blades and trails then draw through the image's plain view
   with their ordinary effect-layer pipelines. Within each effect blend slot the
   glowing layers are emitted after the others, so the glow pass draws one tail
   range per slot; this only reorders layers within one slot, which leaves additive
   and modulating blends unchanged (alpha-blended layers stay back to front within
   each part). Nothing glowing on screen means no pass, no blur and no composite
   work; a frame only walks the glowing world passes and the entity queue to decide.
3. **Blur** ([post_glow.rs](../crates/sjk-viewer/src/post_glow.rs)), in 8-bit images
   that clamp after every pass as stock's framebuffer copies did:
   - `r_dynamicGlowStyle 1` (default), rd-vulkan's: a four-level pyramid at 1/2,
     1/4, 1/8 and 1/16 of the window, each level a three-tap horizontal then
     vertical blur of the previous level, taps 1.2 texels apart, weights 6/16 and
     5/16 each raised by 0.15 (`blur.frag`, `vk_pipelines.cpp:1713`). The four
     levels are summed and scaled by `r_DynamicGlowIntensity - 1` (within 0.01-4,
     `vk_pipelines.cpp:1519-1523`) at half size, then added. This is what EternalJK
     players with `cl_renderer rd-vulkan` see; rd-vulkan sums the levels at full
     size, so the three smaller levels are slightly softer here.
   - `r_dynamicGlowStyle 0`, rd-vanilla's `RB_BlurGlowTexture`: `r_DynamicGlowPasses`
     passes at `r_DynamicGlowScale` of the window (or `r_DynamicGlowWidth` x
     `r_DynamicGlowHeight` when both are positive), each summing four diagonal taps
     (0.1 + pass x `r_DynamicGlowDelta`) texels away, weighted
     `r_DynamicGlowIntensity` / 4; the first pass reads the full-size image.
4. **Composite** in the final resolve ([post_aa.wgsl](../crates/sjk-viewer/src/post_aa.wgsl),
   `with_glow`), after the effect layer and before the `r_gamma` ramp, on display
   values with `r_sceneHdr` 0 or 1: retail with `r_DynamicGlowSoft 1` screens
   (`e + g - e*g`, `GL_ONE, GL_ONE_MINUS_SRC_COLOR`); otherwise the glow is added and
   clamped (rd-vulkan always adds). `r_DynamicGlow 0` builds none of this.

| Cvar | Default | Behavior |
| --- | --- | --- |
| `r_DynamicGlow` | 1 | 0 off, 1 on, 2 saber blades only (JoF EternalJK), 3 the blurred glow without the scene (debug); live, 0 frees the images |
| `r_dynamicGlowStyle` | 1 | 1 rd-vulkan pyramid, 0 retail kernel; rebuilds the blur |
| `r_DynamicGlowIntensity` | 1.13 | Retail per-pass gain; rd-vulkan scales its level sum by Intensity - 1; live |
| `r_DynamicGlowPasses` | 5 | Retail passes, 1-32; live |
| `r_DynamicGlowDelta` | 0.8 | Retail tap spread added per pass; live |
| `r_DynamicGlowSoft` | 1 | Retail screen composite (1) or additive (0); live |
| `r_DynamicGlowScale` | 0.25 | Retail blur size relative to the window; rebuilds the blur |
| `r_DynamicGlowWidth`, `r_DynamicGlowHeight` | 0 | Retail blur size in pixels when both are positive; rebuild |

All are archived and keep stock's names (lookups ignore case, so the retail menu's
`r_dynamicglow` is the same cvar); `r_dynamicGlowStyle` is SJK's. Stock defaults
`r_DynamicGlow` to 0; SJK turns it on. The renderer settings' IMAGE tab has rows
for `r_DynamicGlow` and `r_dynamicGlowStyle`. SJK's classic menus have no
counterpart of the retail Setup page's glow toggle.

Emission-mapped world stages are drawn too, writing only their emission (see
[Emission maps](#emission-maps)). Not drawn into the glow image: the sky, flares, the
menu stage, and secondary views (portals, sky portals, floor reflections), which show
no glow yet. Stock's
glow pass fogs towards black, SJK's does not, so glow inside fog is brighter than
stock's. Decals never glow. The image holds unexposed display values, equal to the
scene's at `r_hdrExposure 1`: an exposure belongs on the world's glowing stages
before they are encoded (see `world_format` in post_glow.rs), not on effects, which
are display values already.

Cost: a frame that glows clears and fills a scene-sized RGBA8 image (33 MB at
3840x2160) and redraws only the glowing passes; rd-vulkan's blur then runs eight
small passes and the level sum (about 2.8 million pixels at 4K, three or four
texture reads each), retail's five passes at a quarter size. The images take about
55 MB at 4K with rd-vulkan's style and 39 MB with retail's. These are estimates, not
measurements: no GPU timing has been recorded. Unit tests cover the glow flags
(Tavion's possessed skin, olol, the collapse quirk), the saber blade/core split,
cvar parsing and the kernels; the programs are validated with naga. On-screen
appearance is unverified.

## Material maps

The optional material maps follow OpenJK rend2 (`codemp/rd-rend2`), so rend2
texture packs apply without conversion. Stage keywords (`ParseStage` in
`tr_shader.cpp`) are parsed by
[sjk-shader](../crates/sjk-shader/src/material.rs): `normalMap`,
`normalHeightMap`, `specMap`/`specularMap`, the packed `rmoMap`, `moxrMap` and
`ormMap` families, and `specularReflectance`, `specularExponent`, `gloss`,
`roughness`, `normalScale`, `specularScale`, `parallaxDepth` and `parallaxBias`.
SJK adds emission maps (`<diffuse>_e`, below), which rend2 does not have.
Their order-dependent overrides are kept. rend2 selects a packed layout by
comparing the image name with the keyword, so `rmosMap`, `mosrMap` and `ormsMap`
load the three-channel layouts; JKR does the same. Without keywords,
[the lookup](../crates/sjk-viewer/src/material_map_images.rs) tries `<diffuse>_nh`
then `_n` for normals and `_specGloss`, ioquake3's `_s`, `_rmo` then `_orm` for
specular, as in rend2's `CollapseStagesToGLSL`. ioquake3's typed
`stage normalMap` stages are not supported. With the cvars off, the parser
records the keywords and nothing else changes: no image lookup, layout,
buffer or pipeline is created. SJK turns the cvars on by default; without a
pack (or keywords) a map load only checks the candidate names in the file
index, and no layout, buffer, pipeline or reflection probe is created.

Maps apply to lightmapped world surfaces (static and inline movers) whose
lightmap and diffuse stages collapse into one opaque pass. On the retail
`mp/ffa1`, `mp/ffa3` and `mp/duel1` this covers 84–87% of world triangles;
`mp/siege_hoth` covers 52%. Vertex-lit surfaces, stacks that do not collapse,
effect stages, deforms, sprites and models (MD3, Ghoul2) keep their authored shading.
Each material-mapped stage compiles to its own pipeline key and a second bind
group; ordinary stages keep their pipelines, groups and stage-table records.

Map load computes [vertex frames](../crates/sjk-viewer/src/material_map_frames.rs)
for the flattened world only when a stage has maps: a tangent with handedness,
averaged over the triangles of patch and triangle-soup vertices, and the
light-grid direction (`R_CalcVertexLightDirs`/`R_LightDirForPoint`). They use 8
bytes per vertex. Building them for `mp/ffa3` (114,088 vertices) took about 24 ms
in a release test build. Maps are uploaded as linear RGBA8 with box-filtered mips.

Shading lives in [material_maps.wgsl](../crates/sjk-viewer/src/material_maps.wgsl):

- Baked lighting: rend2's lightmap response, with the light-grid direction in
  place of a deluxemap. The texel is taken as arriving along that direction,
  divided by the face's own cosine (at most 4x) and received by the mapped normal;
  the remainder stays ambient. A flat normal map reproduces the texel. Retail BSPs
  have no deluxemaps, so this is an approximation.
- Real-time lighting (`r_dayNight 1`): the sun share of the half-resolution
  light buffer is moved to the mapped normal per pixel, using the visibility the
  buffer keeps. It fades out toward the terminator, so mapped bumps never light a
  face turned from the sun or a shadowed texel. The rest (lamps, probe bounce and
  sky) is redistributed the same way through a direction target (below). Without a
  specular map, the existing sun highlight and sky rim use the mapped normal.
- Specular maps use rend2's two paths: spec/gloss, and occlusion, roughness,
  metalness and specular with the albedo as metal colour. Highlights use rend2's GGX
  `CalcSpecular` for the sun or grid direction and for dynamic lights. They are
  added after the albedo and dynamic-light modulation. Occlusion darkens only the
  ambient share and the probe reflection.
- With specular maps, surfaces reflect the nearest reflection probe (below), with
  rend2's split-sum `CalcIBLContribution`; without a captured probe, real-time
  lighting keeps its sky rim.
- Parallax uses rend2's 16 linear and 8 binary steps through the height.

### Lamp and bounce direction in real-time lighting

On a map with material maps the light pass also writes a half-resolution RGBA8
direction target beside the light buffer
([sun_realtime.wgsl](../crates/sjk-viewer/src/sun_realtime.wgsl), `DirectedLight`):
the dominant direction of the non-sun light (octahedral), the share of it that
arrives from that direction and the lamps' part of that share. Lamps contribute
the luminance-weighted sum of their directions (each lamp's shadowed irradiance
times the unit vector to its centre); the probes contribute their L1 irradiance,
whose `a + b cos` form gives `2b cos` (at most the whole irradiance) as the
directional part. Both shares are applied to the light as finally shown (after the
lamp response, gain, fill and occlusion). The lamp cache bakes the same lamp vector
per texel into an RGB10A2 layer beside its light (`bake_light_directed`), so cached
receivers read one more bilinear sample instead of walking their lamp lists.

The material program reads the direction through the same depth- and normal-aware
weights as the light (`material_map_buffered` in
[material_maps_realtime.wgsl](../crates/sjk-viewer/src/material_maps_realtime.wgsl))
and treats the directional share as the baked path treats a lightmap texel: divided
by the face's own cosine (at most 4x), received by the mapped normal, fading toward
the face's terminator; the remainder stays ambient, so a flat map reproduces the
buffered light. The lamps' part casts GGX highlights along that direction with the
stage's roughness and metalness (`material_map_shade_light`); bounce and sky cast
none (they belong to the reflection probes). Baked lighting is unchanged.
`r_dayDebug 1024` (live) leaves the non-sun light as the light pass evaluated it, for
side-by-side comparison. Normal maps
therefore respond under lamps and in bounce-lit interiors, not only in sunlight.

Cost: only maps with a material-mapped stage get the target and the cache layer;
others compile and run exactly as before. The target costs 4 bytes per light-buffer
texel (8 MiB at a 3840×2160 window, twice that with floor-mirror images), the cache
layer half the lamp cache's size again (the load log prints the total, `Lamp light
cache: ... with directions`). Per frame the light pass writes the extra target and
does a little more arithmetic per lamp and probe; material-mapped pixels read four
more texels. Estimated, not measured: a few hundredths of a millisecond at 4K on a
current GPU. A single dominant direction cannot represent two lamps on opposite
sides; their vectors cancel and the light stays ambient, which is the safe failure.

### Reflection probes

Specular-mapped surfaces reflect prefiltered cube maps captured in the map, after
rend2's cubemaps ([reflection_probes.rs](../crates/sjk-viewer/src/reflection_probes.rs),
[reflection_capture.rs](../crates/sjk-viewer/src/reflection_capture.rs)). They exist
only with `r_specularMapping` and `r_cubeMapping` on, on a map with specular-mapped
stages; otherwise nothing is placed, allocated or captured.

- **Placement.** Probes stand at the map's `misc_cubemap` entities, else at every
  player spawn (deathmatch, start, duel and CTF spawns) lifted to eye height, else at
  the intermission spot, as rend2 picks the first class that has any. Points in solid
  space are dropped, points within 256 units merge into their mean, and at most 64
  are kept by farthest-point selection. Each probe measures its room with six axis
  traces against the world brushes (32–2048 units per side).
- **Assignment.** Every specular-mapped surface takes, among its four nearest probes,
  the nearest one a trace from the surface reaches unobstructed (else the nearest),
  as rend2 assigns cubemaps per surface. The index rides in the spare byte of the
  vertex frames, so the shader needs no search.
- **Parallax.** The reflected ray is intersected with the probe's room box and the
  cube is read toward that point from the probe (Lagarde's box-projected cubemap).
  rend2 approximates the room by a sphere of one radius; a box fits Quake's
  axis-aligned rooms and keeps floors and walls aligned with their reflections. A
  surface outside its probe's box reads the plain reflected direction.
- **Capture.** Faces are rendered through the ordinary scene path before the main
  view's light pass, with this frame's sun cascades: in real-time lighting the light
  pass lights the face in the light buffer's corner, as floor mirrors do; in baked
  lighting the faces show the lightmaps. Sky, opaque and blended world surfaces and
  movers are drawn; players, items, effects and fog are not, and captured surfaces do
  not reflect probes themselves (no reflections of reflections). After map load one
  whole probe is captured per frame until all are done; surfaces use the sky rim until
  their probe is ready.
- **Filtering.** Each captured cube is box-downsampled, then every level of its slot
  in an RGBA16F cube array is GGX-prefiltered with filtered importance sampling (64
  samples, rend2's `prefilterEnvMap.glsl`), down to 4×4 texels for roughness 1. The
  split-sum BRDF table (64², rend2's `R_CreateEnvBrdfLUT`) is computed once on the CPU.
- **Shading.** The cube's level `roughness × last level`, times `F0 × scale + bias`
  from the BRDF table, times occlusion, added after the albedo like the other
  highlights; F0 is the specular colour (packed maps: the dielectric value mixed toward
  the albedo by metalness).
- **Time of day.** When the sun turns by half a degree, or the sun, sky colour, light
  scale or indirect gain change by 2%, all probes are refreshed one face per frame
  (384 frames for 64 probes); each keeps its old content until its new faces are
  filtered. With a running day clock (`r_dayMinutes`) reflections therefore trail the
  sun by a few seconds.

Cost: the cube array takes about 1.05 MiB per probe at 128² (6 faces, 6 levels of
RGBA16F), plus a scratch cube and a 128² capture target (about 1.2 MiB); 64 probes
take about 65 MiB, a typical FFA map with 15–30 spawn clusters 16–32 MiB. The load
log prints `Reflection probes: N at 128x128, ...`. Capturing costs each of the first
frames after load six small scene renders (a light pass on a 64² corner and a 128²
colour pass each) and one filter, comparable to six floor mirrors; refreshing costs
one face per frame. Per pixel, specular-mapped surfaces take two more texture reads
and a box intersection. These are estimates from the pass structure; nothing was run
on a GPU. `r_materialMapsDebug 4` shows the reflections alone, 5 the scene without
them, live.

As in rend2, frames come from the untransformed texture coordinates (`tcMod`
rotation misaligns them) and an `animMap` stage uses its first frame's maps.
Material-mapped stages stay off the stage table (which only real-time mode
uses): the CPU colour pass draws them with per-stage bind groups and pipelines.

The controls are sampled once, when the GPU context is created. A change after
that logs `<cvar> changed: restart the viewer to apply material maps`; the
configuration file setting them at startup does not. Each map load logs what
it found, for example `material maps (normal+specular+parallax): 429 stages,
429 normal, 57 parallax, 429 specular; frames for 77283 vertices in 18 ms`, or
`no stage of this map has maps` when the cvars are on and nothing was found.

### What to expect, and checking it

`r_materialMapsDebug` is live and draws only material-mapped stages (one uniform
branch in the material program; the ordinary programs do not change): 1 shows the
mapped world-space normal as colour, 2 tints each stage by the maps it found (red
parallax, green normal, blue specular, so normal plus specular is cyan), 3 shows
the normal map's relief, four times its departure from the face, on grey.
Surfaces without maps keep their ordinary look in every view, so 2 shows at a
glance which surfaces take maps.

The baked response is subtle by construction. rend2 credits all of a lightmap
texel to one direction, so a normal tilted by a small angle α changes the texel
by about tan θ · α, where θ is the angle between the face and the light-grid
direction. A face whose grid direction is more than 78° from its normal falls
back to that normal (`R_LightDirForPoint`) and then only darkens by 1 − cos α. On
the retail `mp/ffa3`, 59% of lightmapped vertices have a usable grid direction
(55% on `mp/duel1`), with a mean tan θ of 1.5 there. Maps generated for ffa3
and duel1 by `sjk-materialgen` (strength 1) are gentle: their normals tilt 3.8°
on average (90th percentile 4–17° per image), so lighting changes by about 5%
on the surfaces that respond. A local headless render (not committed) of six
ffa3 spawn views at 960×540 (Vulkan, RTX 5080), with the generated maps and an
HD texture pack, measured against the cvars off: normal maps alone changed
pixels by 0.06–1.8/255 on average (at most 36/255), specular maps by
0.6–2.6/255 (at most 11/255; the generated `_rmo` maps average roughness 0.68
and metalness 0.02, so highlights are faint). Parallax changed pixels by up to
23/255 on average, but by shifting the texture: it reads as the same texture,
not as relief. Turning the cvars on in baked lighting therefore
shows no obvious change with these maps; that is the model, not a missing draw.
Authored rend2 packs with stronger normal maps respond in proportion to their
tilt.

Real-time lighting shows normal maps far more clearly: the sun share is moved
per pixel, and a low sun lights floors at grazing angles. Indoors, lamps light
walls and floors at grazing angles too, and their share now follows the mapped
normal as well. With `r_dayNight 1`
and `r_dayHour 7`, the sun stands 15° high, so a floor's tan θ is about 3.7
(2.4 at the default hour 7.5) against 1.5 for baked grid directions. This
estimate is from the formulas above; no real-time render of real content has
been measured.

Unit tests cover keyword parsing, lookup order and conversions (synthetic
in-memory images), frames (planar, mirrored and curved), the stage selection
and the pipeline key. naga validates both material programs. A headless Vulkan
render of a synthetic quad on an RTX 5080 (Windows 11, external harness) checked
the response in both modes. Flat maps matched the ordinary program exactly, tilted
normals brightened and darkened as computed (green follows +t, rend2's
convention), and normal maps did not lighten a shadowed buffer texel.

The same harness timed one full-screen 3840×2160 layer (64 runs, medians):
baked lighting 0.103 ms ordinary, 0.196 with a normal map, 0.219 with a
specular map as well and 0.50 with parallax. With the light buffer, those passes
took 0.226, 0.30, 0.34 and 0.54 ms. Measure a real scene with `JKR_FRAME_BUDGET=1`
in a release build: compare the same map, view and population with the cvars on
and off. No authored rend2 pack was available for testing. Generated maps on
real ffa3 data were rendered headless in baked lighting only (above); real-time
lighting on real content, an in-game image and the frame cost in a match remain
unverified.

### Emission maps

A light panel, lamp or screen painted into an ordinary lightmapped texture shows only
as bright as the light falling on it. An emission map makes it glow: `<diffuse>_e`
next to the diffuse image (found like rend2's automatic maps, through
[the lookup](../crates/sjk-viewer/src/material_map_images.rs)) holds the emitted
colour, sRGB-encoded like the diffuse image; black emits nothing. rend2 has no
emission map and no stage keyword for one (its `glow` stage flag only feeds its glow
buffer), so the name is SJK's; it matches the `_e` emission images some community
packs already ship for model textures (which this lookup, on world surfaces only, does
not read). [sjk-materialgen](#generating-material-maps) writes them.

- **Which surfaces.** The same stages as the other material maps (lightmap and
  diffuse collapsed into one opaque pass). A shader that already shows light of its
  own over its paint (a `glow`, additive or `GL_DST_COLOR GL_ONE` texture stage)
  takes no emission map, whatever images exist, so its light is never drawn twice;
  the generator skips the same shaders. `r_emissiveMaps` is on by default because it
  only acts where a pack has `_e` images; with nothing else enabled, a stage with an
  emission map takes the material program with a flat normal and no specular map,
  which reproduces the ordinary lit colour exactly, and the map needs no vertex frames
  and no light-direction target.
- **Surface.** The material program samples the map at the diffuse coordinates and
  adds it after lighting, highlights and dynamic lights (`material_map_finish` in
  [material_maps.wgsl](../crates/sjk-viewer/src/material_maps.wgsl)): no lightmap,
  light buffer, shadow or ambient occlusion dims it. `r_emissionStrength` scales it
  live (bits 24-31 of the scene lighting-mode word, so no buffer changes). It is
  added in the scene's linear units, so HDR, `r_hdrExposure` and eye adaptation treat
  it like any other bright surface. `r_fullbright` and `r_lightmap` show none.
- **Halo.** A stage with an emission map and no authored `glow` is drawn into the
  [dynamic glow](#dynamic-glow) image through a glow variant of the material program
  that writes only its emission, so only the emitting texels get a halo.
  `r_emissiveGlow 0` leaves these stages out of the glow pass (a bit test per glowing
  pass and frame); a stage with an authored `glow` keeps glowing as before.
- **Light, baked lighting.** The lightmaps already hold the light of the map's
  sources, and an emission map adds none: in baked lighting it is only seen.
- **Light, real-time lighting.** With `r_dayNight 1` the lightmaps are replaced by live
  light, so a surface whose emission map is its only light becomes a source.
  `compile_material` ([world_material_compile.rs](../crates/sjk-viewer/src/world_material_compile.rs))
  turns the map into the same textured emission field the
  [fixture inference](#inferring-fixture-light-from-legacy-materials) builds from
  glow stages: the map's colour times 16 (the radiance an explicit glow stage of an
  undeclared fixture gets) times `r_emissiveLights`, laid out by the diffuse stage's
  texture transforms. The existing lamp extraction then integrates it over the
  surface (texture-aware patches merged within 96 units, finite rectangles, the lamp
  grid, static visibility and the lamp cache), and the material's emission enters
  the GI voxels like any fixture's, so probes see it too; GI rays that hit a lamp
  face see no emission, as for other fixtures. A material that already emits (a
  declared `q3map_surfacelight`, an inferred glow fixture or self-lit paint) keeps its
  own light and its emission map adds none. Emission-map lamps are extracted
  separately and capped at the 1,024 most powerful per map (`lights::keep_brightest`
  in [material_maps.rs](../crates/sjk-viewer/src/material_maps.rs)); the load log
  prints `Emission maps: N area lights from M materials (K before the cap of 1024)`.

Cost: without `_e` images, map load does one more image lookup per collapsed stage
and nothing else changes. With them, each emitting stage reads one more texture
(uploaded as sRGB RGBA8 with mips) and draws through the material program; the glow
pass, when one of them is on screen, redraws those stages and runs the blur (see
[Dynamic glow](#dynamic-glow)); in real-time lighting each added lamp costs what an
inferred fixture costs, bounded by the cap and by the lamp grid's per-cell selection.
Load time grows by the patch extraction of the emitting surfaces. Estimates from the
code paths; nothing was measured on a GPU. `r_materialMapsDebug 6` shows the emission
maps alone (black on mapped stages without one).

Unit tests cover the lookup (synthetic images), the shader rule, the light gate and
the lamp cap, the emission field's mean and mask, the strength and glow bits, and naga
validation of the material program and its glow variant in both lighting modes. No
emission map has been rendered: appearance, halo and the light added in real-time
lighting are unverified.

## Floor reflections

Polished floors show a real mirror image of the scene
([floor_reflections.rs](../crates/sjk-viewer/src/floor_reflections.rs)). A floor
qualifies at map load when its shader asks for polish the stock way, with a
`tcGen environment` stage, and is otherwise plain opaque world paint: sorted opaque,
no light emission, sky, deforms, glow or alpha test
([floor_reflection_planes.rs](../crates/sjk-viewer/src/floor_reflection_planes.rs)).
Its triangles must lie in one plane (within 0.02 units) facing up (normal z ≥ 0.7);
coplanar faces share one mirror. The load log prints `Floor mirrors: N polished
planes, M with material maps`.

Each frame keeps at most six mirrors, the planes covering most of the screen: a plane
enters at 1% of the screen and leaves below 0.7%, so it does not flicker at the
threshold. Every mirror re-renders the whole scene (sky, world, players, effects)
from the reflected camera at half resolution into a shared target, cropped to the
plane's screen rectangle; in real-time lighting it is lit in its own light-buffer
images, so the main view's light stays intact. The finish
([floor_reflection.wgsl](../crates/sjk-viewer/src/floor_reflection.wgsl)) blends the
image over the floor with a Schlick rim from 0.12 head-on to 0.70 at grazing angles,
blurred by a 3×3 kernel whose reach follows roughness 0.4; strength falls by
`1 − 0.6 × roughness`.

With material maps the finish follows the floor material's maps: the normal map bends
the mirror lookup (the change of the reflected ray, mirrored back through the floor
and projected at an assumed 48 units, at most the margin below) and the specular
map's roughness sets the blur (at most 0.6) and its occlusion dims the reflection.
Planes with maps render 1.2 roughness units of margin around their rectangle instead
of 0.4 so the bent and wider lookups stay inside the mirror image. The maps are read
with the stage's untransformed texture coordinates; floors whose diffuse stage
scrolls or scales (`tcMod`) read them slightly misaligned. Floors without maps draw
exactly as before.

`r_floorReflections 0` (live, archived; Settings > VIDEO > Renderer, IMAGE tab, "Floor
mirrors") leaves polished floors with their ordinary material and renders no mirror.
The environment variable `JKR_FLOOR_REFLECTIONS=0`, which predates the cvar, still
forces them off whatever the cvar says; `JKR_FLOOR_COMMANDS=0` disables only the GPU
visibility commands that skip hidden mirrors. Measured mirror costs are in the
sections above (depth priming, light-buffer preservation); the material-map finish
adds two texture reads per mirrored floor pixel and the wider margin.

## Submission and lighting work reduction

The renderer records uploads with their frame and hands completed batches to a
submission thread. It finishes outstanding encoder work, applies those uploads,
submits and presents in order while the render thread prepares the next frame.
Only one handed-off frame can remain outstanding. With an offscreen scene target,
swapchain acquisition happens after world recording; direct-to-surface rendering
still acquires its image first. Resize, out-of-band submissions and teardown wait
for the outstanding batch. `JKR_SUBMIT_THREAD=0` selects inline submission as a
fallback. See [frame_queue.rs](../crates/sjk-viewer/src/frame_queue.rs),
[frame_split.rs](../crates/sjk-viewer/src/frame_split.rs) and
[frame_target.rs](../crates/sjk-viewer/src/frame_target.rs).

Upload staging reuses byte and operation storage after warmup. Queue clones share
a synchronized recording; this replaces immediate wgpu upload work on the render
thread, rather than making all queue operations lock-free. Large load-time writes
can bypass recording once pending submissions have completed.

The crosshair is retail's `gfx/2d/crosshair*` picture, drawn by the HUD draw list
as a textured quad ([hud/crosshair.rs](../crates/sjk-viewer/src/hud/crosshair.rs));
the in-game HUD shader draws its procedural cross only when that picture is missing.
The in-game HUD shader is restricted to crosshair and damage-indicator regions.
Intersecting regions become one rectangle so translucent pixels blend once.
Menus and the shader's status-bar fallback retain full-screen coverage.

Uncached deferred-light receivers are collected into a pixel list and shaded by
four compute lanes per receiver. Cached receivers keep their existing path. The
indirect dispatch uses bounded rows to support large light buffers; the final lamp
sum can differ slightly in floating-point rounding from a serial sum. Receiver
depth identifies valid texels, allowing attribute and light targets to retain data
outside regions that will be overwritten or sampled. Floor mirrors use separate
light images, preserving the main view without save/restore copies.

Static sun-shadow bounds form an exact min/max mip hierarchy, starting at 8-texel
tiles. A receiver chooses a level covering its filter footprint with at most four
tiles. It skips the existing filter only when those conservative bounds determine
the result; shadow radii, tap counts, cascades and visual settings are unchanged.


Verification on 2026-10-02 compared this change against `f3f3db2`, using external
release replay instrumentation on Linux, Ryzen 5 5500 and Radeon RX 9060 XT
(RADV). Each 2560×1080 run measured 3,330 frames after replay warmup, with a
31-player roster and unchanged graphics settings; visible/submitted actors ranged
from 20–23 on `ffa3` and 16–27 on `ffa1`.

| Route | Mean total frame | Mean GPU | Total-frame p99 |
| --- | --- | --- | --- |
| `mp/ffa3` | 2.464 → 1.822 ms | 1.780 → 1.697 ms | 3.517 → 3.316 ms |
| `mp/ffa1` | 3.042 → 2.157 ms | 2.337 → 1.986 ms | 3.757 → 4.263 ms |

These paired offscreen runs establish a mean improvement on the sampled routes,
not universal sub-2 ms performance or improved tail latency. The `ffa1` p99 was
higher despite its lower mean; longer native play and more hardware remain open.
They exclude live networking/audio and window presentation. Comparison captures
at 2560×1080 and 3840×2160 retained the scene appearance: maximum channel error
was 4/255 on `ffa3`, 1/255 on `ffa1` outside its wall-clock kill-feed text, and
3/255 in the 4K `ffa1` snapshot. This is finite image coverage.

External checks covered overlapping HUD regions and viewport bounds at five
resolutions through 8K, indirect-dispatch boundary cases and 7,308 shadow-bound
footprints including non-power-of-two maps. Formatting, locked workspace build
and tests, and release client/server builds passed. Verification harnesses and
reports are kept outside the source repository.
Two 45-second native `ffa1` replay process runs also passed without panic or GPU
validation errors: threaded submission with HDR/FXAA, and inline submission with
HDR/FXAA disabled to exercise direct surface acquisition. Both used isolated
1280×720 settings; these are integration checks, not performance measurements.

## Sky scenery and hillside orientation

Local correction on 2026-10-03, based on `8f692ac` plus the preceding lighting
preview, addresses the three reported `t1_danger` views.

Indirect draw storage now resets during scene preparation, before either sky or
main-view commands are recorded. Resetting in the main sun-caster pass could reuse
an earlier sky view's buffer region: queued main-view uploads then changed the
sky draw arguments on camera turns. The static world's identity instance also
permits sky-portal rendering, matching the table path when drawing directly or
falling back from indirect draws. Entity and mover visibility rules are unchanged.

Lighting now orients smoothed normals using the triangle plane reconstructed from
fragment position derivatives. Testing the smooth normal's own dot product with
the eye incorrectly flipped visible hillside normals at grazing angles, creating
camera-dependent dark bands. Pre-pass normals, receiver lighting, lamp-cache side
selection and material upsampling use the same geometric-side decision. This does
not change sun-shadow filter widths, exposure or source intensity.

External release GPU captures on Linux/RADV, Radeon RX 9060 XT, at 1280×720,
day hour 10.6 and unchanged owner graphics settings established:

- The unwanted dark bands disappeared in both marked terrain views. Disabling AO,
  contact shadows or sun maps separately did not remove the old bands.
- A 24-view sky test captured the first frame of each alternating camera turn,
  covering offsets through ±36 degrees. Restoring only the old buffer reset
  reproduced missing mountains in 12 views (over 100,000 affected pixels each).
  Corrected indirect and direct draws agreed within 11/255 maximum channel error;
  only one pixel across the 24 comparisons exceeded 8/255.
- The sampled `mp/ffa1` view differed by at most 1/255. The previous `mp/ffa3`
  structure-shadow mark differed by at most 22/255, with only 28 pixels above
  8/255. The sampled day/night-disabled `mp/ffa3` image was byte-identical.
- Settled GPU medians for the distant and close terrain views were respectively
  0.713 → 0.713 ms and 0.695 → 0.691 ms (11–12 samples per side). These fixed-view,
  nonexclusive measurements are not a populated-match performance certification.

Formatting, locked workspace build, all nine existing tests and doc-test targets,
and the optimized owner client build passed. Native owner confirmation and wider
map/hardware coverage remain open. Verification overlays and assets stay outside
versioned source.

## UI ownership

`sjk-ui` provides renderer-independent retained widgets. The viewer supplies GPU
and text integration and binds client state to the HUD. Layouts are data in
[assets/hud](../crates/sjk-viewer/assets/hud); menus and HUD may be modern while
movement, combat and network behavior remain compatible.

HUD text sizes and `px` layout units scale with the HUD factor: viewport height
over 1080, clamped to 2/3..2.5, times `cg_hudScale`. The crosshair name keeps
stock's size in that frame: `CG_DrawCrosshairNames` draws `ergoec` (point size 20)
at scale 1.0 in the 640x480 screen, so its line is 45 px at 1080p and 60 px at
1440p, times the layout's `type_scale`. The classic layout uses 0.9 because
`arialnb` has taller capitals per line than `ergoec`; Inter uses 1.0. Its
position is stock's too: the line top sits at y = 170 of the 480-line screen,
above the crosshair, so both layouts place it 157.5 px above the centre in the
1080-line frame. That matches stock wherever the HUD factor equals the height
over 1080 (720 to 2700 lines at `cg_hudScale 1`); outside it, the name keeps
the HUD's clamped frame, like the other HUD text.

2D layouts are authored in pixels of a 1080-line screen and scale with the
window height ([ui_scale.rs](../crates/sjk-viewer/src/ui_scale.rs)), as retail's
640×480 virtual screen did: 1440 lines draw them at 1.33× and 2160 lines at 2×.
The scale is clamped to 0.6–2.5 (648–2700 lines); the console keeps a 0.75
floor, the HUD 2/3 and the frame-rate and weapon labels 0.85. `cg_hudScale` and
`con_scale` multiply it. The operating system's display
scale (Windows scaling) is not applied: a fullscreen window already covers
the display, so it would count the density twice, and in a small window on a
scaled desktop it would push 1080-line layouts past the window edges. It only
sets the resolution the bundled Inter font is rasterized at, and text sized in
that font's own units is converted from line heights so it does not depend on it.

### Classic model preview

The classic profile's live model ([menu_stage/preview.rs](../crates/sjk-viewer/src/menu_stage/preview.rs))
is the menu stage's actor drawn into a target of its own: a colour texture in
the scene's format and a `Depth32Float` depth at the preview's size on screen
(sides rounded up to 16 pixels, at most 1024), so the world material
pipelines draw into it unchanged, with a camera of its own that frames the
whole body from in front and turns round it. `preview.wgsl` then writes the
8-bit texture the UI draws as `PREVIEW_TEXTURE`: display values as the
effect layer's `display` makes them for each scene format, at a neutral
exposure, with coverage from depth (an edge pixel averages its covered 3x3
neighbours and takes their share as alpha). On lightsaber creation the lit
blades of the actor's hilts go through the game's blade renderer into an
8-bit texture of their own (its own instance buffer, against the model's
depth), and the encode adds them: over the body they add to it, beyond it
their brightest channel becomes the alpha, so the glow shows over any page.
The passes run after the scene and before the UI, only while a classic page
shows a preview.

The actor is the stage's, marked preview-only so the world pass and the
saber blade list leave it out; it wears no sabers on character creation and
in the cosmetics window. Lightsaber creation shows its hilts alone
([menu_stage/showcase.rs](../crates/sjk-viewer/src/menu_stage/showcase.rs)):
the actor is not drawn (nor animated) and only lends its position and light
sample; each hilt lies along the camera's horizontal, its first blade to the
right, centred on its whole length (mesh and every used blade's root and tip,
measured along that blade once when the hilt loads), turned about its own
blade line, the second of two 14 units under the first, and the camera
stands back just far enough for the longest saber across the preview's
width. On the main menu it
stands on the backdrop's stage when the map has one; in a match, at the
local player's origin, so the map's light grid and lights shade it as the
player is shaded. It wears the stage's cosmetics. Not drawn into it: saber
trails, dynamic glow, shadows and fog.

A failure turns the preview off rather than the client: the pipeline and
textures are made inside wgpu validation and out-of-memory scopes, and the
first frame is a probe recorded in an encoder of its own and submitted
inside a scope (`FrameQueue::submit`, once). On any error the log says why
and the profile keeps the model's portrait for the rest of the session.

### UI colour model

The 2D layer (text, retained UI shapes, the shader HUD, the menu-file HUD and the
scope) works in display values, as retail's 2D drawing did: a colour is the
sRGB value shown on screen, and alpha mixes display values. `^1` is pure red,
`ui_accent ff6a3d` shows as `#FF6A3D`, and a black text shadow at 0.55 over
mid-grey shows 0.225 as in retail. The world, its resolve, bloom, HDR, the effect
layer and the in-world ground HUD stay in linear light.
[ui_target.rs](../crates/sjk-viewer/src/ui_target.rs) holds the model:

- Every 2D pipeline targets the display format without its sRGB encode
  (`Bgra8UnormSrgb` becomes `Bgra8Unorm`) and draws in its own pass after the
  scene resolve.
- With `r_gamma 1` that pass writes the swapchain image through a UNORM view.
  The surface is configured with that view format, and each frame makes one
  extra view object of the acquired image.
- With another `r_gamma`, or on an adapter without `SURFACE_VIEW_FORMATS`
  (Vulkan without `VK_KHR_swapchain_mutable_format`, GLES), the scene resolves
  into the display intermediate, the 2D layer draws through that texture's UNORM
  alias, and the display pass applies the ramp to world and UI together, as
  retail's hardware gamma did. Without aliasing this costs one full-screen pass
  at `r_gamma 1`.
- The float `r_hdr` target never receives 2D draws: HDR is encoded by the
  resolve before the 2D pass.
- Pictures sampled by the 2D layer (icon atlas, wordmark, classic menu art,
  SJK's menu emblem, levelshots, menu-file HUD art, scope art) are `Rgba8Unorm`,
  so their texels are not decoded.
- SJK's menu emblem adds its two glow layers as light (`src * alpha + dst`) in
  display values, through a second, additively blended pipeline of the shape
  renderer; everything else in the 2D layer is alpha blended. Font atlases contribute only alpha, which no format
  decodes, so the Inter atlas stays shared with the ground HUD.

Colours chosen by eye for the earlier linear model were re-authored so neutral
text keeps its on-screen lightness: each grey or white text colour is now the
value it used to show (the theme's foreground 0.94, 0.97, 1.0 became 0.973,
0.987, 1.0 and its muted 0.60, 0.68, 0.76 became 0.798, 0.843, 0.886), and a
translucent one also gained opacity to keep its lightness over a dark backing:
dimmed labels are now 0.77 to 0.96 opaque, disabled ones 0.58 to 0.63. Chromatic colours (accents, team and status colours, `^` codes,
retail menu values) keep their authored values and so show at full saturation.
Scrims and other translucent fills keep theirs: dark ones look darker, and faint
white washes (separators, borders, hover fills) look fainter than before.

### Menu readability

Menu screens draw their text straight over the live map, so a left-hand scrim
darkens the world behind the text column. The archived cvar `ui_menuContrast`
(Settings, GAME tab) sets how far that scrim is held under the text:

| Value | Effect |
| --- | --- |
| `off` | The original scrims; the in-game menu leaves the match untinted |
| `standard` (default) | Muted body text reaches WCAG AA (4.5:1) and the accent 3:1 (large text, UI components) over a backdrop of relative luminance 0.5 (about sRGB `#bcbcbc`) |
| `strong` | All enabled text, the accent included, reaches 4.5:1 over pure white; dark custom accents are capped at 95% darkening |

With a level on, each scrim keeps its original fade but does not drop below
the required darkness until the right edge of the text column, then eases
back over 12% of the screen width. The in-game menu gets the player screen's
column scrim, centred cards and the map picker's caption get the same floor,
and dimmed labels gain just enough opacity to reach 4.5:1 on that backing.
Disabled entries (drawn under 0.7 opacity) keep their dimmed look. The
figures follow the [UI colour model](#ui-colour-model): luminance linearises
the display values, and scrims and translucent text mix display values as the
GPU blends them. They ignore the glyph drop shadow, so they are conservative;
they are not measured on screen. With the default theme the standard column
needs 60% ink (the accent's 3:1 sets it; muted text alone needs 51%), leaving
the reference backdrop at luminance 0.073, and the strong column 81%, leaving
white at 0.032. The cvar is read once per frame, menus open or not, and
published as an atomic level; that read compares in place and does not
allocate. See
[contrast.rs](../crates/sjk-viewer/src/menu_widgets/contrast.rs) and
[hero.rs](../crates/sjk-viewer/src/menu_widgets/hero.rs).

UI text uses the bundled Inter font, rasterized once per display scale in
[text.rs](../crates/sjk-viewer/src/text.rs). Two options switch surfaces to
the game's own fonts, read from the player's game data and never bundled:
`cg_classicHudFont` draws the status HUD with `arialnb`, and `ui_gameFont`
("Classic game fonts", off by default) draws every surface the retail game drew
with its own fonts in that font, following OpenJK `codemp`:

| Retail font | Surfaces |
| --- | --- |
| `ergoec` (`FONT_MEDIUM`) | Menus, crosshair name, centre prints, warmup text, match timer, enemy info, scoreboard names and headings |
| `ocr_a` (`FONT_SMALL`) | Chat box and typing line, weapon/Force/inventory selection names, scoreboard numbers |
| Console character set `gfx/2d/charsgrid_med` | Console and notify lines, FPS, snapshot, vote, team overlay, connection interrupted, kill feed |

The routing is per text run: the HUD maps its text ids in
[text_values.rs](../crates/sjk-viewer/src/hud/text_values.rs), chat marks its
centre-print rows, and the scoreboard sends text made only of digits, `-` and `/`
to the small font. Everything else, including the command browser and overhead
names, stays on Inter (or `arialnb` for the status HUD). The `.fontdat` metrics
are read by [fontdat.rs](../crates/sjk-viewer/src/text/fontdat.rs); the atlas is
the highest-priority `fonts/<name>.tga` (or `.png`/`.jpg`), so an HD replacement
atlas in a later PK3 is used with the retail metrics and is mipmapped down to the
retail 512-texel size. The console character set
([charset.rs](../crates/sjk-viewer/src/text/charset.rs)) is a 16×16 grid of
Latin-1 cells; like `SCR_DrawSmallChar` and `CG_DrawChar`, each character is the
left half of its cell drawn twice as tall as wide, every character advances one
cell (the console is monospaced), and a space draws nothing. The console keeps its
own sizes (`con_scale`, row pitch), and its caret, selection and pointer hits
measure the same fixed advance it draws with. The game fonts load when a world is
installed with the option on, or on first use, from
[game_font.rs](../crates/sjk-viewer/src/game_font.rs); a missing font leaves
its surfaces on Inter. The console font is drawn after all other text, so the
console stays on top. The classic console (`con_style classic`, see
[client.md](client.md#classic-console)) goes further: its background, bar and
text are a layer of their own
([console_backdrop.rs](../crates/sjk-viewer/src/console_backdrop.rs)) drawn after
every other 2D element, so text under an opaque console is hidden.

The retail atlases are 256–512 texels on the long side, so 1440p and 4K text
magnifies them several times and bilinear sampling of their coverage blurs every
edge over several pixels. Atlases under 2048 texels are therefore converted at
load into signed distance fields by [sdf.rs](../crates/sjk-viewer/src/text/sdf.rs)
and drawn by the text shader's `fragment_sdf`, which rebuilds each edge one screen
pixel wide at any scale. The edge is the 0.5 contour of the bilinearly interpolated
coverage, found on a grid of about 2048 texels with an exact Euclidean distance
transform. Retail-size atlases (512 texels or less) store their field at twice the
atlas size, because a one-texel stroke would otherwise be rebuilt half as wide; the
field is the alpha of a white RGBA texture, so layout, mips and sampler are shared
with Inter. HD replacements of 2048 texels or more (such as an 8x `ergoec`) already
resolve 4K text and keep their own anti-aliasing, so they stay coverage atlases.

Checks on 2026-10-03 (Windows 11, release scratch tool outside the repository, real
retail atlases and the JoF HD pack): conversion took 36 ms (`arialnb` 256²),
70–72 ms (`ergoec` and `ocr_a` 512×256, `ocr_a` HD 1024×512) and 159 ms (`arialnb`
HD 512²) on one core; fields take 1–4 MiB before mips. At 1:1 the rebuilt coverage
of each glyph matched its original area within 10% for all but 2 of 170 retail
`ergoec`, 1 of 164 `ocr_a` and 3 of 157 `arialnb` glyphs (small, thin glyphs such as
`}` and `¾`, down to −43%), and within 5% for every HD `ocr_a` and `arialnb` glyph.
Magnified text keeps the retail letterforms, including the staircase detail of a
low-resolution atlas, but with sharp edges; because the old ramp is gone, strokes
read slightly heavier than the blurred version. The shader was validated with naga;
nothing was run on a GPU.

In the Inter atlas, byte 0xAC (`¬`) is an exception: the retail `ergoec` and
`ocr_a` fonts draw it as the boxed "WSI fonts" foundry logo, which players use in
names, so when the game data provides either font the atlas takes that glyph
instead of Inter's not-sign
([logo_glyph.rs](../crates/sjk-viewer/src/text/logo_glyph.rs)). It is cropped
from the mounted atlas (an HD replacement included), scaled so the retail
font's `H` matches Inter's cap height, and spliced into both faces at atlas
build and DPI rebuild; nothing is read or rasterized per frame. Without the
retail fonts `¬` stays Inter's. The console character set leaves 0xAC blank, as
retail did. Outgoing chat and names send `¬` as the single byte 0xAC
([player text](networking.md#player-text)), so other clients draw the logo too.

### Game-data HUD

`cg_hudStyle game` (SJK's default) replaces JKR's health, armor, Force and ammo
widgets with the status HUD the game's own menu files describe, as retail Jedi
Academy draws it ([menu_hud.rs](../crates/sjk-viewer/src/menu_hud.rs)).
`cg_hudFiles` (retail default `ui/jahud.txt`) names a list of `loadMenu` files;
the stock list loads `ui/hud.menu`, so a PK3 that replaces that file (a custom
HUD pack) or a list naming other menus changes the HUD with no JKR-specific
format. A nonzero integer selects the stock text-only HUD; as in EternalJK, `0`
is the default list and `3`/`4` name `ui/elegance_hud.txt`/`ui/jof_hud.txt`. A
missing list falls back to the default one, as `CG_LoadMenus` does; files
without a `lefthud` or `righthud` menu leave JKR's HUD in place.

When several PK3s replace `ui/hud.menu`, the one mounted last wins, and packs
usually replace the retail pictures (`gfx/hud/hudleft` ...) under the same names
too. `cg_hudPack` names the PK3 whose HUD to use instead: the HUD's menus and
pictures are then read from the files without the PK3s mounted after it that
ship `ui/hud.menu` (`VirtualFileSystem::without_mounts`), as if they were not
installed, so `assets1.pk3` gives the original HUD with a HUD pack still
installed. An empty value, or a PK3 that is not mounted, reads the files as
installed. The settings' HUD picker lists the choices
([choices.rs](../crates/sjk-viewer/src/menu_hud/choices.rs)) and shows each
game-data HUD composited on the CPU at 1280x720 for a sample player (health 74,
armor 46, Force 63, medium saber style) over a dimmed levelshot read from the
game's own `assets*.pk3`, with the HUD's text drawn by the menu
([preview.rs](../crates/sjk-viewer/src/menu_hud/preview.rs)); it renders on a
worker thread and is uploaded into a texture of its own.

The reader ([parse.rs](../crates/sjk-viewer/src/menu_hud/parse.rs)) keeps the
window fields HUDs use (`name`, `rect`, `visible`, `style`, `background`,
`forecolor`, `backcolor`) and skips other keywords with their arguments.
Drawing follows OpenJK codemp `CG_DrawHUD` and its helpers in `cg_draw.c`
([frame.rs](../crates/sjk-viewer/src/menu_hud/frame.rs)): the menus' visible
filled/shader backgrounds (`Menu_Paint`), `scanline` and `frame`, four tics per
meter with the partial one faded, the low-armor blink of the last armor tic,
three-cell numbers from the `gfx/2d/numbers/t_*` digits (`CG_DrawNumField`),
the saber style or ammo (grey while firing, yellow just after a pickup, red when
empty, `--` for weapons without ammo) and the score line. A HUD that provides
`saberstyle_desann`, `_tavion`, `_dual` or `_staff` gets that picture for the
style, as EternalJK draws them; otherwise the stock fast/medium/strong mapping
applies. The status HUD shows only for a living, non-spectating player without
the scoreboard, with `cg_draw2D`, `cg_drawHud` and `cg_drawStatus` on.

Items keep their 4:3 shape on wider screens: `righthud` items scale from the
right edge and every other menu from the left, as EternalJK's widescreen
correction (`cl_ratioFix`, `widthRatioCoef`) places them; at 4:3 this is
retail's plain 640x480 stretch. `cg_hudScale` grows the HUD from its bottom
corner. Pictures resolve through shader scripts and are packed once per load
into one atlas, each stored no larger than a 2160-line screen draws it (at most
512 texels); additive shaders (`blendFunc GL_ONE GL_ONE`) draw after the
alpha-blended ones. Score and `--` text use the HUD font (Inter, or `arialnb`
with `cg_classicHudFont`) rather than retail's `ergoec`. Not drawn: vehicle and
siege HUD menus, the out-of-Force flash, and item text or owner-draw fields,
which the retail and the checked custom HUDs do not use.

`cg_hudStyle classic` selects JKR's classic layout in either font; `modern` keeps
the existing behavior, where `cg_classicHudFont` also selects the classic layout.
JKR defaults to `modern`; SJK to `game`.

`cg_hudStyle radial` (picker name "SJK radial") is SJK's own take on the TheRisqe Radial
HUD, drawn by the engine with no PK3: health (red, outer) and armor (green, inner) as
arcs left of the crosshair, Force (blue, outer) and ammunition (amber, inner) right of
it, each cut into four segments that fill in turn, with a readout pill of the four
numbers (the saber style replaces the ammunition) and the weapon-name transient below.
It is the layout document [radial.json](../crates/sjk-viewer/assets/hud/radial.json): the
modern layout's other widgets (crosshair, team rows, votes, kill feed, timer, lagometer)
plus `arc` widgets, which `hud.json` overrides cannot yet replace for this style.
The arcs are the new `sjk-ui` draw command `DrawCommand::Arc`: one quad per stroke whose
fragment shader (`ui_shapes.wgsl`, mode 3) takes the signed distance to a round-capped arc,
so they stay smooth at any resolution and scale with the HUD scale. Segment geometry and the
distance function are in [arc.rs](../crates/sjk-ui/src/arc.rs) (unit-tested; the shader
evaluates the same expression); the ammunition ratio is the weapon's pool over
`ammoData[].max`, doubled with the Double Ammo rune ([radial.rs](../crates/sjk-viewer/src/hud/radial.rs)).
Health pulses red at 25 or less, as the modern HUD does. `menu_snapshot` renders the four
sample states to `target/menu-snapshots/hud-radial-*.png` with a CPU copy of the shader.

With the game-data HUD, a weapon change shows retail's weapon selection row for
1.4 s (`WEAPON_SELECT_TIME`), as `CG_DrawWeaponSelect` draws it and JoF EternalJK
keeps it square on wide screens ([weapon_select.rs](../crates/sjk-viewer/src/weapon_select.rs)):
the selected weapon's icon (80 units) between 40-unit icons 12 units apart, the
centre at 320 and the icons at `y` 400 and 420 of 480, with Concussion between
Flechette and Rocket, empty thermal detonators and trip mines left out, and an
empty weapon drawn with its `_na` icon (the saber with its staff or dual icon).
EternalJK's icons per side apply: 3 up to 16:10, 5 up to 16:9 and 7 wider, one
more with the text HUD. The name is retail's (`SP_INGAME_<item>` from
`strings/english/sp_ingame.str`, such as "E11-Blaster Rifle") in gold
(0.875, 0.718, 0.121), `FONT_SMALL` at scale 1 with its baseline 6 units above
the bottom (`ocr_a` with the game fonts on, otherwise the bundled font at that
size). Units are those of the game HUD, `cg_hudScale` included, growing from the
bottom centre. The row needs `cg_draw2D`, a living player who is not spectating,
following or on an emplaced gun, and no held scoreboard; a Force or inventory
cycle after it hides it, since `CG_Draw2D` shows only the most recent selector.
SJK's own layouts keep their weapon-name widget instead of the row. Unlike
retail, the name has SJK's text shadow.

## Billboard icons

Frame billboard icons follow OpenJK's `RT_SPRITE` image orientation: texture v=0
belongs at the top of the quad. Their local v is reflected before the shader's
scale/scroll transform; ordinary FX billboards retain their existing convention.
This covers simple-item icons and player-status icons when submitted through
that path. The owner reported an inverted talk balloon during the player-icon
PR playtest. An external probe compared the corrected production transform with
OpenJK `RB_AddQuadStampExt`: four corners with three scale/scroll transforms
matched, while ordinary FX transforms were unchanged. Native visual confirmation
of the correction remains pending.

World icons (talk/connection and simple-item sprites) preserve their authored
texture alpha even when `r_softParticles` is enabled. They use a distinct
instance kind to bypass the 16-unit intersection fade, while retaining ordinary
depth testing, orientation and bounded effect-pass coverage. Smoke and other FX
quads still soften against nearby surfaces. Applying the smoke fade to icons
made opaque bubble interiors transparent near geometry.

## Generating material maps

`sjk-materialgen` writes normal, height and roughness/metalness/occlusion maps
for the world textures of the player's own installation, in the rend2 naming
that the optional material maps (`r_normalMapping`, `r_specularMapping`,
`r_parallaxMapping`) look up next to a diffuse image, and SJK's emission maps
(`_e`, `r_emissiveMaps`). It runs offline and only reads the game data; [its crate documentation](../crates/sjk-materialgen/src/lib.rs)
and `--help` are the reference.

```sh
cargo run --release -p sjk-materialgen -- --maps mp/ffa3,mp/duel1
```

- **Input.** GameData is found like the client finds it (`--game-data`,
  `JKR_GAME_DATA`, the config's `fs_gameData`, then the usual Steam paths).
  `base`, an optional `--fs-game` directory and `JKR_CONTENT` are mounted in
  the client's order, case-insensitive. The tool's own earlier output is left
  out. The installed maps (or `--maps`) supply the shaders actually drawn: BSP
  shader lumps and surfaces plus the shader scripts.
- **Selection** ([select.rs](../crates/sjk-materialgen/src/select.rs)). A texture
  qualifies when a shader draws it on lightmapped surfaces with lightmap and
  diffuse stages that collapse into one opaque pass. These are the stages the
  renderer gives maps to. Skipped, each with a reason in the manifest: sky,
  fog, liquids, nodraw/clip/system shaders, interface and 2D images,
  lightmaps, blend-only effects, `deformVertexes`, glowing, animated,
  environment-mapped and non-plain colour diffuse stages, alpha-tested foliage
  (grates are allowed), images without relief (flat colours) and textures that
  already have rend2 maps. With an existing normal map or specular map only the
  missing kind is written. A shader whose diffuse pair is followed by its own
  `tcGen environment` stage (stock chrome and polished floors) qualifies; the
  environment stage marks the texture polished (below).
- **Generation** ([generate.rs](../crates/sjk-materialgen/src/generate.rs)),
  deterministic and wrap-around, so tiling textures stay seamless. Height comes
  from luminance, high-passed twice at 1/8 of the texture to suppress baked
  lighting gradients. It is then weighted by scale band and normalised.
  Normals are Scharr slopes (red +s, green +t down the image, rend2's frame),
  scaled by the class strength. Above 512 texels they are taken per 1/512 of
  the texture, so high-resolution replacements do not turn texel noise into
  steep bumps. The packed map
  holds roughness from the class plus local variation, brightness and cavities;
  metalness only on bright, unsaturated texels; and cavity occlusion. Source
  resolution is kept unless `--max-size` caps it, and alpha-tested textures
  keep their alpha in the normal map.
- **Classes** ([classes.rs](../crates/sjk-materialgen/src/classes.rs)): one table
  of strength, parallax, roughness, metalness and occlusion per class. A class is
  chosen by the BSP material id (`q3map_material`), then path keywords, then
  `surfaceparm metalsteps`. Stone, tiles and ground get `<texture>_nh`
  (height in alpha for parallax), and so do metal textures whose path names a
  panel, plate, floor, wall, hull, deck or door (seams and rivets); the rest get
  `<texture>_n`. A texture whose shader has a `tcGen environment` stage is
  polished: its roughness is at most 0.25 with half the variation.
- **Metal.** Metalness 0.8 and roughness 0.3 (±0.3 across the texture), for the
  reflection probes ([Reflection probes](#reflection-probes)). In rend2's packed
  path metal loses its diffuse share and takes its reflection from the probe;
  0.8 rather than 1 keeps a fifth of the diffuse light where the heuristics call
  painted or grimy parts metal, and metalness still falls on dark and saturated
  texels. Without probes (`r_cubeMapping 0`) such metal reads darker than its
  retail look. Earlier packs used 0.3 and 0.45, when nothing reflected the room
  into metal.
- **Emission** ([emission.rs](../crates/sjk-materialgen/src/emission.rs)). A texture
  gets `<texture>_e` when there is evidence that it gives light, strongest first: the
  overrides file; `q3map_surfacelight` on a shader drawing it (`q3map_lightRGB` only
  colours such a light and `q3map_lightImage` only averages an image for its colour,
  so neither counts alone); an authored glow image next to it (`<texture>_glow`,
  `<texture>glow`, `<texture>_glw`, the black-backed overlays retail shaders add with
  `blendFunc add` and `glow`); a fixture or screen word in its file name (`light`,
  `lamp`, `bulb`, `neon`, `screen`, `monitor`, `display`, `console`, `computer`,
  `comp_`, `holo`, `glow`; not `lightning`, `highlight`, `flight`, `lightgr…`,
  `lightsab…`, `clamp`, `switch`, nor names with an `off`, `broken`, `dead`, `unlit`
  or `dark` part); or the BSP material `computer`. Textures whose every shader already
  shows light (a `glow`, additive or `GL_DST_COLOR GL_ONE` stage) and textures with an
  `_e` image get none. With a glow image, the emission is that image. Otherwise the
  texels that emit are near-white or saturated ones clearly brighter than most of the
  texture (above its median value plus 0.2, at least 0.6; near-white needs 0.75);
  surface lights and overrides take any texel above the median plus 0.1 (at least
  0.45); a texture whose median is 0.8 or brighter is a light panel whose bright texels
  all emit. The emitted colour is the texel's, lifted so the brightest emitting part
  reaches about full brightness (at most 3x). Nothing luminous, no map; the manifest
  records each decision (`emission`: evidence, `written` or the reason, coverage, gain).
- **Overrides** ([overrides.rs](../crates/sjk-materialgen/src/overrides.rs)). A text
  file of per-texture rules fixes what the heuristics get wrong. Each line is a
  path pattern (the diffuse image without extension, case-insensitive, `*` and `?`
  wildcards) followed by `class=`, `roughness=`, `metalness=` (0–1),
  `height=on|off` or `emission=on|off|<strength>` (0–4, the emitted colour's
  multiplier; 0 is off); `#` starts a comment. Every matching line applies, in order
  (for `emission` the last that sets it):

  ```text
  textures/mp/floor*        class=tiles roughness=0.2
  textures/kor_*/*metal*    metalness=0.9 height=on
  textures/x/lightwall      emission=off
  textures/x/console2       emission=1.5
  ```

  The tool reads `--overrides FILE`, or `sjk-materialgen-overrides.txt` next to the
  output pk3 when it exists; the manifest records the file and, per texture, the
  lines that applied, its class, roughness and metalness, and whether it was
  polished.
- **Specular layout.** The tool writes `<texture>_rmo` (red roughness, green
  metalness, blue occlusion) rather than `_specGloss`. The heuristics produce
  roughness and metalness directly, and the packed path takes the metal colour
  from the albedo and a 0.04 dielectric reflectance by itself, without rend2's
  SDR gloss conversion.
- **Output.** One pk3 of PNGs plus `jkr-materialgen/manifest.json` (every
  source, its outputs, class, maps and shaders, skipped shaders with reasons,
  all settings). The archive is deterministic. The default path is
  `<JKR user data>/generated/zzz_jkr_materials.pk3`, `%APPDATA%\jkr\generated`
  on Windows. The tool refuses to write into the game installation. To use it,
  set `JKR_CONTENT` to that directory (the client mounts it above the game
  data), or copy the pk3 into `GameData/base` by hand; the `zzz_` name loads
  after the retail pk3s. `--dry-run` lists the choices (with polished textures
  and applied override lines), and `--limit` takes only the most-used textures.

**Regenerating.** The manifest records the generation of the tuning
(`"generation": 3` since emission maps; packs without it are generation 1). With
material maps or emission maps on, the client logs `material maps: the generated pack
is generation 1 of sjk-materialgen, this client expects 3 ...` once when the mounted
pack is older. Emission decisions depend on every map read: a texture drawn plainly on
one map and through a glowing shader on another gets its `_e`, and the client ignores
it where the shader glows.
Run the generator again for the same maps, then replace the old pk3 where the
client reads it (for example `GameData/base/zzz_jkr_materials.pk3`):

```sh
cargo run --release -p sjk-materialgen -- --maps mp/ffa3,mp/duel1 --out <folder>/zzz_jkr_materials.pk3
```

The tool leaves a pk3 of its output's name out of its input, so the old pack's maps
do not count as existing rend2 maps. Restart the client after replacing it.

The generated images are derived from retail textures. They stay on the
player's machine and must not be shared, uploaded or committed; the tests use
synthetic images only.

Heuristics fail where luminance is not relief. Painted stripes, signs and
decals become bumps. Lighting baked at panel scale (bright bevel edges, dark
undersides) becomes a ridge and a groove instead of a raised panel. Dark
grime reads as a dent, and smooth gradients within an eighth of the texture
remain. Unit tests cover flat input, slope direction, seams, gradient
suppression, alpha, class lookup, skip rules on synthetic scripts and the pk3
layout. A read-only run on `mp/ffa3` and `mp/duel1` of a Windows installation
with high-resolution texture packs took 11 s for 110 textures (160 MiB) and is
described in the generator's pull request. No in-game image has been checked.

JA+ grapple ropes ([grapple_rope.rs](../crates/sjk-viewer/src/grapple_rope.rs))
follow EternalJK's `CG_Missile`: on a JA+ server each `WP_STUN_BATON` missile is a
hook, drawn as `CG_TestLine` draws it, a one-unit-wide near-black (RGB 6, 0, 0)
`white` line from the hooked player's right hand to the hook, rebuilt every frame
as a frame-billboard streak and hidden while the local player duels. The hooks are
gathered once per frame into a fixed table. The effect atlas builds a generated
white tile for `$whiteimage`/`*white` stages so the `white` shader keeps its alpha
blend instead of falling back to the additive spark. The JA+ client plugin's hook
model is not drawn, as in EternalJK.

## Default visual profile

New profiles use the owner-approved rendering setup: day/night enabled at a fixed
noon (`r_dayHour 12`), volumetrics quality 3, actor/world sun shadows at 2048 resolution and
16 filter taps, and lighting tier 0 (available baked indirect light under the
live sun). Shadow gap closure and screen-space contact shadows are off.
The scene uses HDR with exposure 1 and SJK's eye adaptation (-0.5 to +1 EV), FXAA,
SSAO at strength 4, trilinear mipmapping
and 16× anisotropy where supported. Bloom is on (`r_sceneBloom 1`); the optional
LDR tone curve is off. Sunbeam dust is on at full density (`r_dustMotes 1`) and
shows only inside the godrays of `r_volumetrics`.
Dynamic glow is on with rd-vulkan's blur (SJK; stock defaults it off).
Soft particles, per-pixel model diffuse lighting and full rendering resolution
remain enabled. Material maps (`r_normalMapping`, `r_specularMapping`,
`r_parallaxMapping`) and reflection probes (`r_cubeMapping 1`, 128²) are on, but
take effect only where a pack such as the [generated one](#generating-material-maps)
supplies maps; without one nothing is drawn differently or created. Noon, bloom,
dust and material maps are SJK's defaults (Sol's own settings); JKR keeps 11:00 and
the rest off. Emission maps
(`r_emissiveMaps 1`) are on too and only act where a pack has `_e` images.
These are ordinary cvar defaults, not a config imported at launch.

Saved values take precedence, including explicitly disabled effects. The client
saves every archived setting, so a `config.cfg` written before a default changed
keeps the old value (for example `r_dayHour 11` or `r_sceneBloom 0`); a fresh
profile is needed to see the new defaults. Existing
profiles are not silently migrated (the one exception is the old `com_maxfps`
default, see [client](client.md)). Resolution/window mode, input and keyboard
layout, FPS caps, audio levels, HUD/crosshair preferences, player identity,
server history, credentials and filesystem locations retain their independent
defaults. Stale MSAA/light-scale entries are not part of this profile; the active
antialiasing path is FXAA.

External release/Vulkan validation on RX 9060 XT checks 36 graphics values with
an empty config, an explicit equivalent config and an existing override config.
Fresh FFA5 and FFA1 scenes rendered successfully. See [status](status.md) for the
image comparison result and validation limits. No personal config or assets are
included in the repository.
