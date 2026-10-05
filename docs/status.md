# Status and priorities

Reviewed 2026-10-04 against GitHub baseline `b394022` and the owner-approved
client, server, rendering and loading changes described below.

JKR currently contains a native client and standard dedicated server in a
20-crate Rust workspace. This page records scope and verification, rather than
claiming complete parity from the presence of an implementation.

## Worldspawn shader remaps and remap order

Local change against `af65396` (2026-10-05, Windows 11): a map's worldspawn
`remapshader` keys now apply when its world loads, as rd-vanilla `R_LoadEntities`
does, and the latest of a shader's map, server and local remaps wins, as in
rd-vanilla. A local restore no longer blocks later server remaps, each shader-state
update re-applies its entries, and a server remap that a live `cg_remaps` change
excludes reveals the remap it had replaced. Server time offsets are parsed like C
`atof`; `listRemaps` shows map remaps and marks overridden entries. See
[rendering](rendering.md#server-shader-remaps).

Unit tests cover worldspawn key parsing (prefix, first `;`, the C scan stops),
ordering across sources, self-restore, `cg_remaps` gating and `atof`. Workspace
formatting, the locked build of all targets and the locked tests passed on
Windows 11. Not run in the client: no map with worldspawn remaps, server or demo
has exercised this change, and `vertexremapshader` keys stay unsupported.

## Effect shader remaps

Local change on `af65396` (2026-10-05, Windows 11): effects drawn from the effect
atlas (EFX particles, missile trails, muzzle flashes, beams, impact marks and blob
shadows) follow server shader remaps and local `remapShader` overrides, with the
world materials' precedence. A remapped entry samples a copy of its target's
original stages and time offset, loading a missing target into the atlas first;
removal, a self-remap or `cg_remaps 0` restores it. This runs after the world
applies a remap change, never per frame; sampling adds one subtraction per stage.
No wire code changed. See [rendering scope and limitations](rendering.md#server-shader-remaps).

Workspace formatting, the locked build of all targets and locked tests passed.
New unit tests cover remap planning through the shared world lookup (aliases,
extensions, destination clocks, local overrides, self-remaps, `cg_remaps 0`) and
copying/restoring atlas stages. Unverified: nothing was run in the client, so no
server, demo or visual check exercised remapped effects, atlas growth or the time
offset, and the rebuild time of a grown atlas was not measured. HUD pictures,
saber blades and trails, surface sprites and menu previews still ignore remaps.

## Shader remap restore

Change on `af65396` (2026-10-05, Windows 11): undoing a remap (self-remap,
`cg_remaps 0`, gamestate reset, cleared local override) recompiled the slot through
the replacement path, which treats every slot as a map material. Late entity
materials came back with world gloss, view bounds and light-pass classification,
and sky or colourless shaders lost their loaded stages. The first replacement now
keeps the slot's stages, sort and draw/fog classification aside, with their bind
groups and pipeline indices; restoring swaps them back, followed by the existing
order, fog, range, stage-table, SSAO and caster rebuilds. Native users of a
destination at offset zero are no longer recompiled; a nonzero offset still is.
Replacements and stamps are unchanged, and slots never replaced keep no copy.

Formatting, the locked workspace build (all targets) and tests passed on Windows 11,
including a unit test of the keep/restore/replace decision. Not run in the client:
restores on a live server and their visual result remain unverified.

## Shader remap clearing and setting

Local change against `af65396` (2026-10-05, Windows 11) adds EternalJK's
`clearRemaps` console command and a Settings > GAME row for `cg_remaps`
(0 off / 1 map / 2 all; the default stays 1). EternalJK's `R_ClearRemaps_f`
(`codemp/rd-vanilla/tr_init.cpp`) resets every renderer shader's remap and keeps
destination time offsets. JKR clears the live or demo session's server remaps and
local overrides the same way, sends nothing to the server, and lets a later
shader-state change, reliable `remapShader` command or new gamestate apply again.
See [shader remap controls](client.md#shader-remap-controls).

Workspace formatting, locked build of all targets and tests passed on Windows 11,
including a unit test for the clear logic. Not run in the client: material
restoration after `clearRemaps` and the Settings row were not checked in game.

## Leading-slash asset paths

Change on `af65396` (2026-10-05, Windows 11): VFS reads (`read`, `contains`,
`read_from_mount`, cache identity) drop one leading `/` or `\` before the lookup,
as OpenJK `FS_FOpenFileRead` (`codemp/qcommon/files.cpp`) does. A JoF map's
`/models/items/a_pwr_converter.md3` item model failed to load with "virtual path
must be relative". Mounted names, PK3 entries and directory listings keep refusing
a leading slash, and a second one still fails as before. Unit tests cover the path
rule and a read through a mounted source; formatting, the locked workspace build
and tests passed. Not run in the client: the item on that map is unverified.

## Server shader remaps

Local changes on `dc36792`, verified on Linux/RADV on 2026-10-05, add initial and
live multiplayer shader-state consumption, reliable `remapShader` commands,
demo playback support, destination animation offsets and Tayst-style controls.
`cg_remaps` defaults to 1 (exclude player-texture configstring remaps); 0 disables
server remaps and 2 includes player textures. The setting applies live.
`listRemaps` and temporary local `remapShader` overrides are available.
See [rendering scope and limitations](rendering.md#server-shader-remaps).

External evidence used the unmodified OpenJK multiplayer
`CG_ShaderStateChanged` function: 6,000 valid entries matched the compatibility
parser's result, including extension/case variants, repeated sources, shared
clocks and truncated tails. Separate checks covered one-hop aliases, self-reset,
empty configstrings, nonfinite offsets, policy filtering and gamestate reset.
A 400-snapshot synthetic demo derived from a local recording exercised a direct
remap command followed by reapplication of an unchanged shader-state string.
Both its decoded state and offscreen blue-to-green rendering passed.

An isolated local Tayst server and an original small BSP verified join-time green
replacement, live pulsing material, self-reset to red, enable/disable, translucent
local replacement, and the default/player-inclusive policies (24 player material
slots changed when enabled). Automated checks used an ALSA null sink and isolated
zero-volume settings. Simple remap updates measured 0.5–0.8 ms. One settled
2,048-frame sample at 1280×720 measured 0.419 ms mean and 0.609 ms p99 CPU frame
work; this is a small fixture, not a populated-server benchmark, and excludes cold
pipeline compilation. No wire, movement or combat rules changed. Workspace
formatting, locked build/tests and the optimized Linux viewer build passed.

Shader replacement does not imply geometry editing. Existing server entity and
sub-BSP paths were inspected but not changed. Arbitrary HUD and generated sprite
remaps, full lighting reconstruction and broad custom-map parity remain outside
this implementation (effects: see above); these limitations are recorded on the
rendering page.

## Third-person camera collision and vehicle framing

Local change against `dc36792` (2026-10-04): restore the multiplayer camera's
collapsed-target fallback, pitch bounds/offset sign, rapid-turn damping,
vehicle-authored framing and mount/teleport resets. Camera collision now includes
presented inline doors/platforms and excludes BODY from the stock camera mask.
Vehicle definitions are read at appearance loading, with no per-frame file reads.
See [client camera behavior](client.md#third-person-camera).

An external harness compiled the six original OpenJK `codemp/cgame/cg_view.c`
camera functions and compared 10,000 camera frames at 8/7/4/3 ms. Cases include
open space, confined rooms, fully collapsed traces, rapid yaw, animal offsets,
vehicle overrides, pitch-dependent offsets, fighter strafing, unrestricted pitch
and sideways offsets. Maximum component error was 0.000132; this is a floating
point tolerance comparison with shared synthetic trace conditions, not full
client parity certification.

A separately compiled original BSP exercised the production collision adapter:
player-clip obstruction, a translating inline door and ignored packed vehicle
bodies passed. All 8,640 low-ceiling/corner camera frames remained finite. The
optimized camera/collision microbenchmark took 336 ns/frame on this small fixture;
it does not establish populated-map frame performance. Workspace formatting,
build and tests passed, and an optimized Linux viewer was built. Offscreen Vulkan
validation exercised the confined fixture and a mounted stock swoop on an
isolated loopback server. Broader vehicle/mod playtesting remains open; vehicle
rider animation is outside this camera correction. The follow-up below addresses
local vehicle model prediction.

### Local mount presentation follow-up

The local pilot's vehicle model and rider seat now consume its committed/per-frame
vehicle prediction, instead of interpolating old snapshot transforms behind the
predicted camera. This follows OpenJK `codemp/cgame/cg_ents.c`:
`CG_AddPacketEntities` publishes `cg.predictedVehicleState`, and
`CG_CalcEntityLerpPositions` bypasses snapshot interpolation for that vehicle.
No movement, command quantization, animation timing or wire code changed.

An external harness exercised the production vehicle placement/seat module for
8,000 frames at 8/7/4/3 ms, with 50 ms snapshots and 100 ms simulated delay.
The old vehicle-root lag reached 38.735 units; the new root exactly matched the
predicted input. Rider bolt placement, remote/demo fallback and unrelated-vehicle
isolation passed. These are presentation fixtures, not a network or physics
parity certification. Workspace format/build/tests and the Linux release build
passed. An isolated offscreen Vulkan session mounted, moved and turned a stock
tauntaun; a recorded demo confirmed mounted state throughout all 53 snapshots.
Owner testing found continued severe jitter. The follow-up investigation found
that the vehicle's snapshot body remained in its own prediction collision list:
movement started all-solid, then corrected at the next server snapshot. The
adapter now applies vehicle skip/ownership exclusions. Rider-based error decay
also incorrectly treated gait motion as a prediction miss; it now measures the
vehicle root, as stock does. Presentation samples the animated driver seat every
frame instead of holding the snapshot-time offset.

The unmodified OpenJK `CG_VehicleClipCheck` confirmed pilot/own-vehicle exclusion
and foreign-vehicle collision. A production collision-adapter fixture reproduced
all-solid before the fix and clear motion afterward across 4,000 hull sweeps at
8/7/4/3 ms; foreign ownership and dismount restore collision. Another 4,000 frames
using the installed tauntaun's real skeleton reduced the maximum seat step from
5.8814 units (snapshot-held) to 0.9411 (frame-sampled), with no gait-induced
vehicle prediction correction. Seat evaluation measured about 2 microseconds per
frame in the optimized harness; this is not a populated-server performance claim.
In isolated Linux Vulkan play, the same eight-second riding/turning input sequence
went from 19 logged corrections above eight units (maximum 27.78) to none after
the self-collision fix. The final demo confirmed mounting in all 183 snapshots.
Workspace format/build/tests passed. This closes the reproduced self-collision
fault; owner playtesting and broader vehicle/mod coverage remain open.

The exhausted-boost follow-up found another reset: snapshot reseeding copied the
fresh vehicle template over client-only runtime state, losing turbo expiry and
recharge. The same ride now retains that state, as multiplayer cgame retains its
`Vehicle_t`; a changed entity, pilot or definition resets it. An external harness
compiled OpenJK's unmodified `AnimalNPC.c` `ProcessMoveCommands` and compared
27,237 production prediction steps with 50 ms reseeds at 8/7/4/3 ms. Held boost
through expiry/recharge, with alternating primary attack, matched all speed
samples within 0.001 units/second. Separate identity checks verified retention
for the same ride and resets for changed vehicle, pilot and definition.

A silent, headless Linux Vulkan client/server check at 1280x720 and 125 FPS held
boost while turning and attacking for 32 seconds. Logged corrections above eight
units fell from 75 (maximum 17.23) to one (17.10, at a boost transition). Demos
confirmed mounted state in all 696 baseline and 697 corrected snapshots, with
180 boosted snapshots in each. This fixes the repeated exhausted-boost mismatch;
it does not establish zero correction at boost boundaries or full vehicle parity.
Workspace format/build/tests and the owner release build passed. Automated checks
used isolated zero-volume settings and an ALSA null sink from startup.

## Vehicle assets, boarding and native sand-creature AI

Local work on `dc36792`, verified on Linux on 2026-10-05:

- The appearance loader now uses the shared `.veh` parser. A commented example
  in retail `template.veh` previously selected an X-wing for `tie-fighter`.
  Direct lookup and native Vulkan play now load `models/players/tie_fighter`.
- Vehicle weapon indices now resolve `.vwp` EFX and rigid projectile models.
  The local AT-ST firing check produced 243 primary and 86 alternate missile
  samples, selecting `atst/shot_red` and `atst/side_alt_shot` respectively.
  Replaying that capture at 8/7/4/3 ms exercised 6,345 presentation frames with
  correct authored model selection and no default rocket model on laser shots.
  Optimized effect dispatch averaged 66 ns/frame on that small capture; this
  does not establish populated-map performance. Vehicle flight-loop audio is
  not changed in this pass.
- Landing/standing boarding follows multiplayer conditions. The original C
  landing branch matched 20,090 eligibility cases. Production prediction at
  8/7/4/3 ms emitted one authoritative request per landing and none for client
  prediction. A loopback capture confirmed the player boarded the tauntaun by
  landing without a use-key press. Broader mod vehicles remain unverified.
- The reported missing glider/minemonster meshes were absent community content
  in the newer local installation. Mounting the existing pack restored their
  own models. Its malformed glider animation remains a content limitation;
  other actors retain the existing error isolation.
- Native sand-creature AI is enabled only outside stock-rules mode, as described
  in [server.md](server.md#vehicle-boarding-and-sand-creatures). An isolated native
  capture confirmed hidden pursuit, `BOTH_WALK2` breach, both attack animations,
  a normal player death and a successful visible respawn. A second server with
  stock rules enabled retained visible generic NPC behavior throughout all 154
  post-spawn snapshots, with no native ambush. This is a separate
  server extension inspired by SP, not a change to multiplayer class IDs or a
  claim of full single-player AI parity.

Automated native checks used headless Gamescope/Vulkan, isolated settings, all
volumes zero and an ALSA null sink. No public server, owner profile or running
owner game was used for these checks. Formatting, locked workspace build/tests
and optimized client/server builds passed. The workspace has no bundled gameplay
tests; the external checks above provide the focused evidence. Wire encode/decode
paths are unchanged.

## Actor animation error isolation

Local fix based on `3a70c22` (2026-10-04): a custom glider's run clip ends at
frame 235 in a 180-frame skeleton. Its evaluation error previously returned
before the shared joint upload, freezing otherwise healthy actors until the NPC
disappeared. Preparation and upload now contain errors per actor, retain its last
uploaded pose, suppress failed-frame animation audio and log once per failure
episode. Valid animation selection, timing, movement and wire code are unchanged.
The malformed custom clip is still rejected; no content files are modified.

An external headless Vulkan check on the RX 9060 XT loaded the installed glider
and Kyle, exercised failure, recovery and removal, and compared both healthy
actors' uploaded palettes against independently evaluated controls. All 576
comparisons matched exactly at 8/7/4/3 ms with one/four evaluation lanes. The bad
actor kept its previous palette and emitted no active animation-audio request;
only one diagnostic was emitted per failure episode. Native playtesting remains
open. See [rendering](rendering.md#actor-animation-failures).

## Distributable builds

Windows x64 and Linux x64 release ZIPs for merged source `3a70c22` were built
and checked on 2026-10-04 using the
[GameData packages workflow](https://github.com/Bishop-R/JKR/actions/runs/37194839545).
Both native jobs extracted their archives, started/stopped an isolated loopback
dedicated server, and verified client discovery and portable settings with
synthetic assets. The source snapshots match on both platforms; checkout and
archiving preserve embedded shader line endings.

Final inspection checked archive CRCs, binary hashes, source revision and
dependency notices. Linux binaries require at most glibc 2.35; Windows imports
contain only system DLLs, with no separate VC++ or MinGW runtime DLL requirement.
No retail data, personal settings or debug symbols are packaged. These checks do
not cover Windows graphical gameplay. See [packages.md](packages.md) for layout,
runtime requirements and the repeatable build procedure.

### Simplified release archives

The `dc36792` Windows/Linux playtest archives were repacked on 2026-10-04
with exactly four files: the two executables, `README.txt` and `LICENSES.txt`.
Binary bytes and executable permissions are preserved. All 317 Linux and 306
Windows original license/attribution sections are retained in the consolidated
text. Build manifests are separate release assets, covered by the updated
checksums; the source archive is unchanged. GitHub asset digests match the local
archives and manifests.

The extracted Linux archive passed synthetic adjacent-asset discovery, portable
configuration and isolated loopback dedicated-server startup/shutdown. Archive
CRCs, four-file contents, binary hashes and notice preservation passed for both
platforms. The updated dependency collector also matched all 315 entries emitted
by the previous Linux collector. Windows execution was not repeated for this
packaging-only update; its executables are identical to the previous release.

## Drop-in client installation

Local change based on `da8adc9` (2026-10-04): the client discovers game data
beside its executable (or in its `GameData` subdirectory), independently of the
working directory. This takes priority over saved paths; explicit positional
paths and `JKR_GAME_DATA` remain overrides. Direct `--connect HOST:PORT` also
supports discovery. See [client launch](client.md#launch) for the full order.

Linux verification: 19 external startup checks passed with synthetic asset-file
markers, covering unrelated working directories, spaces/non-ASCII paths,
discovery precedence, invalid/incomplete locations, known-install fallbacks and
both direct-connect syntaxes. The unmodified optimized binary, placed beside
synthetic `base/` assets and launched from elsewhere with isolated configuration,
found and attempted to read those archives; it then exited on the intentionally
invalid content before creating a window. No retail files were copied. Formatting,
locked workspace build/tests and the optimized client build passed. Windows
double-click/shortcut behavior and full rendering from a drop-in install have not
been exercised by these checks.

The same local work now defaults generated client files to `GameData/jkr/`.
Storage is selected before the console, browser or HUD is created, so settings,
marks, screenshots, recordings, favorites, friends and identity share one root.
A one-time, non-overwriting import copies supported files from the old user
folder and leaves originals intact. Unwritable installations use the existing
per-user profile. Downloaded content keeps its separate cache.

Eleven external Linux storage scenarios passed using synthetic profiles:
first-run creation, supported-file import and byte preservation, identity-file
permissions, PK3/link exclusion, existing destination conflicts, repeated launches,
real permission-denied fallback, both roots unwritable, obstructing files, failed
import/retry, linked destinations, saved installation hints and same-root aliases.
The checks execute the production storage module in separate processes; they do
not migrate the owner's profile. The unmodified release binary also passed
first-launch, repeat-launch and read-only-installation checks with isolated
synthetic content: imported settings were loaded, autosave used the selected
root, marks/key bytes survived, and originals remained unchanged during portable
launches. Intentionally invalid PK3s stopped these runs before window creation.
Formatting, locked workspace build/tests and the optimized client build passed.
Windows ACLs and native Windows launch remain unverified.

## Console editing and command browser

The console includes the command/cvar browser contributed in PR #6 and the
caret/output-selection controls from PR #33. Browser Apply/Cancel pointer actions
match the keyboard, the footer Filter control works, and underlying menu shapes,
text and FPS output are suppressed while browsing. Printable opening shortcuts
become text when the console is open. Dead-key `^` inserts a literal colour-code
prefix; toggling the console clears pending accent composition so the next
command letter is not changed or swallowed. Escape and non-text toggle bindings
still close it. Gameplay and wire code are unchanged.

Linux verification (2026-10-04, based on `7155455`): 22 temporary checks passed
for caret motion, selection/copying, UTF-8 byte limits, glyph alignment and browser
footer pointer actions at 960×540, 1920×1080 and 3840×2160. Test infrastructure
remains outside the repository. A release X11/Vulkan desktop run exercised
browser opening, search and edit mode. Native keyboard probes reproduced and
corrected the pending-accent problem; the owner confirmed that fix and accepted
the console preview. A separate-profile release run typed a dead-circumflex
followed by `1Bishop` and saved exactly `^1Bishop`, with the console still open.
Formatting, locked workspace build/tests and the release build passed. Clipboard
round trips, drag behavior and platform/layout combinations are not exhaustively
verified.

## Classic console (SJK)

SJK's default console (`con_style classic`) follows EternalJK's: the `console`
shader's background with its stage motion, `con_ratioFix`, the bar, a monospaced
character grid with per-row local timestamps (`con_timestamps 2`), word wrap, the
green-clock input row with overstrike, the version line and corner clock, the
scrollback arrows, EternalJK's heights and keys, and notify lines; it draws on a
layer after all other 2D, so text under it is hidden. See
[client.md](client.md#classic-console). The modern console is unchanged
(`con_style modern`).

Verification (2026-10-04, Windows 11): formatting, the locked workspace build and
tests, including unit tests for the grid, background extent, `con_ratioFix`,
heights, page steps, word wrap with stamps, colour carry-over, overstrike, clock
formatting, the retail and JoF `console` shaders' stage programs, texture motion
and colour waves, and `con_style` parsing. The layer was not run on a GPU or in a
game by the change's author; side-by-side comparison with EternalJK is pending.

## Alt codes (SJK)

SJK-only branch `personal/alt-codes` (2026-10-05, based on `2696590`) types
Windows Alt codes (Alt + numeric keypad) in the console, chat and menu fields,
which winit 0.30 drops; see [client typing](client.md#useful-console-commands).
Unit tests cover the code page 1252 and 437 tables, modulo 256, control codes,
withheld keypad digits, play keeping the keypad, AltGr and Ctrl+Alt, cancelling
keys, Shift, key repeat and focus loss; the locked workspace build and tests
passed. No game was started: typing a code in the running client is unverified.
Codes without a leading 0 always use code page 437, not the system OEM page.

## Classic profile Force page and cosmetics (SJK)

The classic profile gains a Force page (retail's `ingame_playerforce`, on both
frames, with holocrons, cost-numbered level stars, side cards, a points meter
and a detail panel), a Force summary and button on the profile page, and JoF
EJK's hats and capes: the Cosmetics window, the `cosmetics` command,
`cg_cosmetics`, the name carried after the colour in `color1`/`color2`, and the
pieces drawn on players' `*head_top` and `*back` bolts. Another player's blade
colour is now read with `atoi`, so JoF's `c1 "8santahat"` no longer draws
blue. Clicking a cell of the profile pages' lists picks that cell; before, the
list's own pointer region, registered after its cells, took the click and
stepped to the next entry. See [client.md](client.md#menu-style) and
[Hats and capes](client.md#hats-and-capes).

Follow-ups on the same branch: retail's Force templates (`forcecfg`, the
player's own saved beside `config.cfg`; `sjk-vfs` gains `original_name` for
the capitalised names packs ship), part icons and tint-base swatches in
character creation (64 more UI atlas cells), Hat and Cape rows on the modern
player screen and the pieces on its stage model, and a live model preview
on character creation and in the cosmetics window: the stage actor drawn
offscreen with its own camera into a scene-format target and encoded for
the UI ([rendering](rendering.md#classic-model-preview)), and on lightsaber
creation holding the lit sabers (blades through the game's blade renderer
into a texture of their own). jaPRO's race-unlock hats are left out on
purpose: every hat and cape is open to everyone. Unit tests cover the
template listing, naming and loading, the original names, the part-icon
cells, the preview's target sizes and camera framing. The preview pass is
new GPU code that has not run on any GPU: its validation, colours, lighting
and framing are untested.

Verification (2026-10-05, Windows 11): formatting, the locked workspace build
and tests, including unit tests for the level costs against spending, the
draft's next-level status and one-step level setting, the Force page's
entries, geometry and star tokens, the region order of the menu canvas, the
colour and name split and join, userinfo with a worn name, the saber colour
of a JoF clientinfo, the installed-piece scan (new and old folders), the
`.cosmetic` offset rules, the bolt placement, the drawing rules and the
`cosmetics` command. The behaviour was compared with JoF EJK's source
(`4eb081be`) by reading it. Nothing was run in the client or on a GPU by the
change's author: the pages' look, a piece's fit on a model and other clients
seeing it are untested.

## Model grid icons and search (SJK)

SJK-only branch `personal/model-grid` (2026-10-05, based on `0fc6e24`): the
character grids' icons are a cache over the 207 atlas cells instead of the
first 207 catalogue entries, so every model shows its icon; tiles answer to the
pointer by their place on screen (in the classic profile, a click on a head past
the 200th used to pick a part, tint or hilt). Both profile styles gain a model
search. See [menu style](client.md#menu-style).

Verification (2026-10-05, Windows 11): formatting, the locked workspace build
and tests, including unit tests for the icon cache (four times as many models
as cells, scrolled through a screenful at a time; tiles on screen keep their
cells; a new catalogue empties it) and the search (words, case, the team
filter). A temporary check against the owner's installation (not bundled)
listed 844 characters and 72 species whose icons all decode, so the black tiles
were only the cell limit. No client window was opened: the grid on screen and
the cache while scrolling are untested.

## Accepted client improvements

The owner approved publishing the current playtest improvements on 2026-10-04.
These changes retain the accepted rendering defaults. Each topic is verified
and published separately; platform/content coverage limits below still apply.

## Current transition policy

Local gameplay continuation during match-end intermission and server map changes
is suspended at the owner's request. Intermission uses the real server's camera,
scores, chat and ready controls; map changes show a loading notice with gameplay
paused. The native continuation adapter and the viewer's dedicated-server
dependency have been removed. Background loading, shared GPU context, archive
inventory optimization, gate-world adoption and matching same-map reuse remain.
Earlier continuation results below describe the historical implementation, not
current enabled behavior. Fast joining and map loading are the current priority;
no universal loading-time target has been verified.

Verification of this policy (local change based on `7155455`): formatting,
locked workspace build/tests and the optimized Linux client build passed. An
external release/Vulkan run against isolated loopback TaystJK exercised a natural
FFA3 timelimit exit into FFA1 and a same-map restart. It observed 1,666 normal
intermission frames and 2,023 loading frames, asserted that no local simulation
started, checked that attempted movement/mouse input could not move the loading
camera, and verified return to the remote session and disconnect to the menu.
Captures confirmed the scoreboard, loading notice and absence of the local body
at the intermission camera. The visibility rule follows codemp `CG_Player`;
scripted NPC and vehicle intermission scenes were not separately exercised.

On RX 9060 XT at 960×540, this final run took about 7.9 seconds from connection
request to playable FFA3 (excluding initial menu construction) and 9.3 seconds
for FFA1 map preparation/adoption, of which 0.52 seconds was CPU map preparation.
These are individual observations, not a controlled speedup or cold-cache result.
Native-window owner playtesting and Windows runtime checks remain pending.

### Loading optimization verification

Local loading changes based on `7155455` plus the suspended-continuation policy
reduce emission-mask preparation, lamp patch searches and serial mip generation.
An interleaved optimized/baseline/optimized Linux release run on RX 9060 XT,
Vulkan, 960×540 and an isolated loopback TaystJK server measured:

| Operation | Baseline | Optimized runs |
| --- | --- | --- |
| Connection request to playable FFA3, including gate animation | 6.82 s | 4.75 / 4.76 s |
| Natural FFA3 → FFA1 map preparation/adoption | 7.88 s | 3.82 / 3.64 s |

Initial menu construction is excluded. These are warm-machine observations from
one host, not cold-cache guarantees or internet-server latency measurements. All
three runs checked ordinary intermission, paused loading, same-map restart,
remote-session adoption and disconnect to the menu.

External reference checks matched all generated lamp-source float bits and
ordering on FFA3 (978 sources), FFA1 (3,562) and `t2_rancor` (4,333). Mip pixels
matched the original algorithm in 12 dimension/layer cases; changed content,
concurrent reuse and byte/entry eviction checks passed. Emission reduction
matched every float bit in 54 rectangular, power-of-two and NPOT cases.
Before/after 1280×720 Vulkan captures retained the scene appearance; animated
materials and temporal rendering mean whole screenshots are not bit-identical.
The isolated checks live outside the source tree and do not add a regression
suite. Gameplay rules, command quantization and protocol encoding are unchanged.
Formatting, locked workspace build/tests and the optimized Linux client build
passed. The owner accepted the faster loading in native playtesting.

## Server content references

Local fix based on `7155455` (2026-10-03): downloading and world content selection
now accept the common prefix of unequal pak-name/checksum lists, matching OpenJK
codemp `FS_PureServerSetReferencedPaks`. Previously both rejected such lists and
prevented joining some servers. External checks compared 441 list-length cases
with the actual OpenJK `1a6a643` C function (whitespace tokenization stubs), and
exercised both production consumers for equal, unequal and absent lists,
installed/duplicate content, malformed checksums, retail-pack exclusions,
unsafe download paths and the reference-count limit. Formatting, locked workspace
build/tests and the optimized Linux client build passed. No wire codec changed.
The owner's EFF retry exposed a second assumption: references were treated as
mandatory archives even with server downloads disabled. The follow-up now skips
UDP transfers when disabled by either side, skips unrequestable/unsafe/retail
download names, and permits absent optional references during mounting. External
checks using the full production storage/selection modules loaded installed FFA1
with missing and malformed references; covered server on/off/absent flags,
client downloads off, non-UTF-8 hostname bytes and a valid community request;
and retained missing-map and wrong-map-checksum rejection. The matching BSP
checksum passed. Workspace checks and the optimized build passed again.
EFF's read-only status advertised stock FFA1 and downloads disabled. Its complete
join with this follow-up remains unverified; no public server was joined for
these checks.

## Chat player menu preview

Local preview `chat5` (2026-10-04) adds a compact square-edged dropdown left of
chat with only `whisper`, `ignore`, `friend`, and `copy`. It follows the clicked
name, stays above typing controls, and leaves chat positions unchanged. At a
narrow left margin it falls back inside the right edge of the chat lane, clear
of the scoreboard. Active ignore/friend toggles are highlighted. There are no
headers, descriptions, standing hints, or success notices; failures still show.
Friends have a small five-point star before the name. Name hover fits the glyph
bounds; each dropdown highlight matches its button rectangle without the wider
menu-row sweep. The dropdown starts without a selected action and switches
cleanly between mouse hover and keyboard focus, so whisper is not permanently
highlighted. See [chat player actions](client.md#chat-player-actions).

Whispers preserve drafts and send only on Enter. Ignores hide messages for the
current map without muting gameplay sounds; friends are saved as local name
bookmarks. Copy preserves name colour codes. Draft editing shares the console's
caret and glyph metrics, including clipboard shortcuts, word motion/deletion,
Shift/mouse selection, double-click token selection and literal dead-key `^`.

Earlier focused checks covered identity reuse, persistence failures, selection,
Unicode, draft limits and pointer actions. An offline X11 probe with an isolated
clipboard adapter verified a `^1Alice^7` clipboard round trip and word selection/
cut. Those checks predate the compact layout; no windows or game instances are
launched to verify this layout revision, per owner preference. Visual playtesting
remains with the owner. Formatting, locked workspace build/tests and the release
build passed.

## Classic scoreboard preview

Local change based on `dc36792` (2026-10-04): `cg_scoreboardStyle classic` draws
the retail scoreboard layout after JoF EternalJK's `cg_scoreboard.c`, with the
client ID column, head icons, flag icons, compact rows, fades and gliding rows
([client.md](client.md#scoreboard-styles)). Score rows now also keep the stock
powerups, defend, assist and capture fields. Verified on Windows 11 with
formatting, the locked workspace build and tests, including layout tests for the
retail columns, row sizes, group order, team bands and the many-clients layout.
A temporary CPU render of the draw list (free-for-all at 1920x1080, team game at
3440x1440, 26 clients) checked placement and was then deleted. Not run in the
client; head icons, flag icons and the game-font option's retail fonts on this
layout are unverified on screen.

## Leader HUD placement preview

Local preview `leader1` moves the portrait and leader/opponent name/score from the
old minimum 230-unit vertical offset to a 32-unit top margin. The existing right
margin and sizes remain. Optional inventory/snapshot readouts and the default
team-overlay placement follow below the visible block; explicit team coordinates
are preserved. Server selection, scores, visibility and asset resolution are unchanged.
No windows or game instances are launched for this layout-only revision; visual
playtesting remains with the owner. Formatting, locked workspace build/tests
and the optimized build passed.

## Game-data HUD preview

Local change based on `dc36792` (2026-10-04): `cg_hudStyle game` draws the
status HUD from the game's menu files (`cg_hudFiles`), giving the retail HUD
and custom HUD packs; a nonzero integer `cg_hudFiles` gives the stock text HUD
([rendering](rendering.md#game-data-hud)). Windows 11 checks, no client window:
a temporary test read the real files and composited one HUD frame (health 60,
armor 40, Force 80, blaster ammo 120 and a medium-style saber) to PNG at
1920×1080 for retail `assets1.pk3`'s `ui/hud.menu` (13 menus, 31 pictures), the
TheRisqe Radial HUD PK3 (33 pictures, centred on the crosshair) and JoF's
`ui/elegance_hud.txt` (22 pictures); every picture resolved and the placement,
tic fades and digits matched the reference logic. Unit tests cover the reader,
item resolution, tic/number/blink/ammo-colour logic, `cg_hudFiles` values,
widescreen placement and atlas packing. The checks are not bundled. Not run in
the client; vehicle/siege HUD menus and the out-of-Force flash are not drawn.
Formatting, locked workspace build/tests passed.

## Manual slider entry preview

Local preview `sliders1` (2026-10-04, based on `7155455`) adds direct numeric
entry to every Settings slider, both sabers' RGB sliders, and all Shot sliders.
Click the value or press Enter on its row; Enter applies, Escape cancels.
Bounds are enforced without drag-step quantization. Drafts stay attached to
their original row, and invalid values leave the previous setting intact.

Eleven temporary offline checks passed on Linux, covering actual pointer routing,
all numeric settings/cvar types, all six saber channels, Shot preview actions,
sub-step values, cancellation, bounds, malformed/non-finite input, caret editing,
and value targets at 1280×720, 1920×1080 and 3440×1440. The temporary checks are
not bundled with the source. No windows, game instances or servers were opened;
visual playtesting remains with the owner. Formatting, locked workspace build/tests
and the optimized build passed.

## Dynamic glow (SJK)

SJK-only branch `personal/dynamic-glow` (2026-10-04, based on `024c22a`) draws
stock's dynamic glow: `glow` shader stages get a blurred halo, with rd-vulkan's
blur by default and rd-vanilla's as `r_dynamicGlowStyle 0`. See
[Dynamic glow](rendering.md#dynamic-glow). Unit tests (glow flags through the
multitexture collapse, saber blade/core split, cvars, kernels) and naga validation
of the changed programs passed with the locked workspace build and tests. No game
or window was started: appearance, GPU cost and the first-use pipeline compile
remain to be checked on screen. Secondary views and fog do not affect glow yet.

## Player model tolerance (SJK)

SJK-only branch `personal/model-tolerance` (2026-10-05, based on `bcb0b76`) loads
player models, skins and animation tables as rd-vanilla and the retail cgame do
instead of drawing Kyle; see [player models](client.md#player-models). The
headless `player_model_scan` ran on a local Windows 11 install (782 model
directories in `base`, 5064 model/skin rows including one uninstalled skin per
model): SJK failures fell from 887 to 2, and from 796 rows (700 directories)
where rd-vanilla keeps the model to none. Fixed classes: skins without commas or
outside UTF-8 (the young* Jedi packs, aldrokoon), missing skins and parts falling
back to `model_default.skin`, weights past one (sad_scout), slash-prefixed
skeleton names and nameless or zero-frame animation lines (vehicle and creature
packs). With `EternalJK` and `japlus` mounted, every row loads. The two remaining
rows are the dianoga creature, whose animations lie past its skeleton's frames;
retail refuses it as a player model too. Unit tests cover each class on synthetic
files. No game or window was started: how the fixed models look and animate in
play, and the GPU skinning of the rescued meshes, remain to be checked.

## Player model fallback during a match (SJK)

SJK-only branch `personal/model-fallback` (2026-10-05, based on `2696590`): a
player whose clientinfo names a model this client cannot load now shows Kyle, as
`CG_LoadClientInfo` falls back to `DEFAULT_MODEL`, instead of keeping the slot's
previous model; body copies of that player use the same stand-in. It is the only
mechanism found for a `/model` change others see while the local player keeps
the old model (logged as `cs <1131+n>: clientinfo failed`); see
[player models](client.md#player-models). The locked workspace build and tests
passed; there is no unit test of the GPU-side rebuild, and no game was started.

## HUD picker (SJK)

SJK-only branch `personal/hud-picker` (2026-10-05, based on `0fc6e24`): the
game-data HUD (`cg_hudStyle game`, the retail HUD unless a HUD pack is
installed) is SJK's default for new configs. `cg_hudPack` uses one PK3's
`ui/hud.menu` and pictures as if the HUD packs mounted after it were not
installed. Settings > HUD > "HUD look" steps through every usable HUD, and
Enter opens a picker with a picture of each game-data HUD drawn for a sample
player. See [HUD style](client.md#hud-style) and
[game-data HUD](rendering.md#game-data-hud).

Verification (2026-10-05, Windows 11): formatting, the locked workspace build
and tests, including unit tests for the HUD choices (install order, labels, the
cvars finding their choice, a pack read without the later ones), the preview
(each pack drawn from its own pictures, mirroring, the text HUD's runs), the
picker (row stepping, opening on the HUD in use, one upload per preview, Escape
and Enter) and `without_mounts`. A temporary check against the owner's
installation (not bundled) listed Jedi Academy, JoF AssetsExtra, TheRisqe
Radial HUD, Text only and SJK's two layouts, rendered each game-data preview in
24-54 ms (release) and wrote them to PNG: the retail and JoF frames in the
bottom corners, the Radial HUD around the crosshair, over the retail `ffa3`
levelshot. No client window was opened: the picker on screen, the preview
texture upload and the live HUD switching are untested.

## Classic+ option panels and renderer page (SJK)

SJK-only branch `personal/classic-plus` (2026-10-05, based on `7643ca4`) writes
down how SJK modernises classic pages ([Classic+ menus](classic-plus.md)) and
applies it to the option panels: a detail box under the Setup and Controls rows
describes the focused setting or binding, rows changed from their default and
settings that apply later are marked, Backspace or the right button restores a
setting's default, and Setup's RENDERER opens a classic renderer page with
IMAGE, LIGHTING and SHADOWS groups instead of the modern form. See
[menu style](client.md#menu-style).

Verification (2026-10-05, Windows 11): formatting, the locked workspace build
and tests, including unit tests for the panel geometry (rows, detail box,
retail bounds), the help text (every setting described in two lines, label
notes), the detail box's facts and defaults, reset to default, the binding
detail and its shared-key line, and the renderer page's groups and way back.
The new [menu snapshots](classic-plus.md#seeing-a-page-without-the-game) drew
Setup, Controls and the renderer page on both frames from the owner's
installation; looking at them led to shorter row labels, wider description
lines and the scrollbar moving to the panel's edge (in the in-game pop-up it
covered the values). No client window was opened: the panels' pointer and
keyboard behaviour on screen are untested.

A second pass (same day) merges what retail split for room: one Video group,
one Force Powers group, and Interface, HUD and Scoreboard groups gathering
JKR's GAME, HUD, HUD+ and TEXT rows by subject (switching to the modern style
from a group continues on the tab holding its row). Key bindings carry the
retail picture of the weapon, item or Force power they select, in a column and
in the detail box, and the weapon rows are named. Join's team rows in the
in-game bar show their flag and player count, and the profile's Force strip
shrinks its holocrons instead of dropping the last known power. Settings whose
label overflowed the in-game label column have a short row name (the detail box
keeps the full one), and slider numbers read `0.9` rather than a
single-precision default's `0.8999999761581421`. Unit tests
cover the groups (every listed cvar is a setting, each GAME, HUD, HUD+ and TEXT
row in exactly one group), the binding pictures (cells, sharing, every weapon
and power pictured), the strip's fit, every row label fitting its column and
the slider numbers; the snapshots, now with atlas icons and
the in-game bar's pop-ups over a levelshot, showed the in-game panels, bindings
and Join pop-up. `cargo clippy --no-deps` on the viewer added no finding in the
changed code; clippy on the workspace stops on four `sjk-game-jka` errors merged
from JKR, which Sol's JKR PR #123 fixes. No client window was opened.

Merged into SJK main with `personal/hud-picker` and `personal/model-grid`
(2026-10-05): the HUD picker's "HUD look" row sits in the classic HUD group,
where LEFT and RIGHT step through the HUDs and ENTER opens the picker; it has
no default mark, since two cvars select the HUD together. The locked workspace
build and tests passed on the merged tree.

## Client devmap preview

Local `devmap1` preview (2026-10-04, based on `7155455`) exposes `devmap <map>`
in the client console and completion catalogue. It launches a fresh, owned,
loopback-only FFA server with `--cheats`, no bots and no match limits, then uses
the existing automatic join path. Invalid names/missing mounted maps are rejected
before session replacement. Create game keeps cheats disabled. See
[development maps](client.md#development-maps) for current server-command limits.

Four temporary offline/headless checks passed: map-name parsing and usage,
console action handoff, private launch arguments/cheat opt-in, and loading
`mp/ffa3`, joining over loopback and observing `give health 77` in a snapshot.
The reference for devmap cheat policy was OpenJK multiplayer `SV_Map_f`.
No gameplay or protocol codec changes were made. The test child stopped cleanly;
no windows were opened and the owner's running game was untouched. Visual
transition playtesting remains open. Formatting, locked workspace build/tests and
the optimized build passed.

## Noclip and talk balloons preview

Local `playfeatures1` preview (2026-10-04, based on `7155455`) integrates
PR #31 (`60533c8`) and PR #30 (`379aaa2`) into the current client/server sources,
retaining the later upright billboard correction and current UI changes.
Native `noclip` requires cheats and a living player; spawning clears it.
Talk/connection icons use stock priority, placement and visibility rules. See
[development maps](client.md#development-maps), [talk balloons](client.md#talk-balloons)
and [server noclip](server.md#noclip).

On Linux, eleven temporary contributed checks passed for movement, talk flags,
sprite selection and billboard orientation. An external harness compared 640
noclip states against unmodified OpenJK multiplayer movement at 8/7/4/3 ms:
origin, velocity and talk flags matched exactly, including vertical-only input.
Headless loopback checks passed for flying, toggling, respawn reset, normal-server
cheat rejection, and a second client receiving the talk flag and selecting the
balloon. No chat messages were sent. Test servers stopped cleanly; the owner's
running game was untouched. Temporary checks are not bundled in the repository.
Visual owner acceptance and populated-match performance remain unverified.
Formatting, locked workspace build/tests and both optimized binaries passed.


Owner follow-up `bubbleopacity1` fixes status/item icons fading like smoke near
geometry when soft particles are enabled. The new icon instance kind bypasses
only that depth fade; authored texture alpha, depth testing and bounded draw
regions remain intact. An offscreen Vulkan check on the RX 9060 XT compared the
production vertex/fragment shaders at 1/8/32-unit depth gaps with texture alpha
0/0.4/1. All nine icon outputs matched the plain shader byte-for-byte; ordinary
particles still faded at close gaps. No windows were opened. Native visual
confirmation remains with the owner. Formatting, locked workspace build/tests
and the optimized client build passed.


## Force power presentation

SJK change based on `024c22a` (2026-10-04), fixing gaps inherited from JKR.
Evidence is EternalJK's stock `codemp` code (`cg_players.c`, `cg_ents.c`,
`w_force.c`, `FxTemplate.cpp`, `FxScheduler.cpp`) and the retail EFX files.

- Own Force effects: the local player's Lightning and Drain beams, Push/Pull and
  Grip puffs and body push blur now come from its player state (JKR searched the
  snapshot's entity list, which never holds the local player). A trickster's
  beam and hand puffs stay visible to its victim, as in stock. See
  [rendering](rendering.md#entity-render-effects).
- Drain bolt shape: EFX `bounce` now sets an electricity bolt's jaggedness, as
  `intensity` did; both also set physics, and the bounce default is retail's
  0.1. This changes the 18 retail electricity primitives using `bounce` (Drain,
  crystal and scepter respawns, DEMP2 alt detonation, environment sparks, ship
  damage) and three expensive-physics emitters without a bounce key (`env/beam`,
  `mp/spawn`, `mp/jedispawn`), which now rebound weakly instead of stopping.
- Mind Trick: a trickster fades out for its victims and is then hidden, fading
  back in when the trick ends; the trickster sees the confusion effect over its
  victims' heads; active Force Sight sees through it. JKR drew tricksters fully.

Not done yet, with what each needs:

- Dodge afterimage (`PW_SPEEDBURST`, `cg_players.c:12612-12661`): stock
  duplicates the Ghoul2 instance frozen at its current frame and draws it at the
  player's current origin for 254 ms, alpha 254 down to 1. A copy in the live
  pose, as the speed trail draws, would coincide with the body; it needs a
  frozen pose, i.e. a second joint palette and vertex range per actor (as
  corpse-pool bodies have) staged once at the burst, drawn with forced alpha.
- Force Sight lighting (`cg_players.c:12258-12264`, `tr_light.cpp:351-357`):
  other players get `RF_MINLIGHT` with `shaderRGBA` 255,255,0, which adds that to
  their ambient light. Actors are lit per fragment from the light grid or, in
  real-time mode, the light buffer, not from the instance light, so this needs a
  per-instance flag through `stage_runtime.wgsl` and a rule for real-time mode.
  The Sight shell overlay is drawn.
- `surfaceparm forcesight` surfaces (`tr_main.cpp:1103-1106`,
  `cg_draw.c:10741-10742`) should draw only while the viewer's Sight is on. The
  shader parser ignores the parm, so they always draw; it needs a shader flag and
  a per-frame world draw toggle. No retail MP map uses it (only SP `rift.shader`).
- Push/Pull refraction (`cg_renderToTextureFX 1`, `CG_ForcePushBlur`
  `cg_players.c:5264-5395`, `tr_backend.cpp:1085-1130`): stock copies a square
  of the frame around the hand and draws `models/weaphits/testboom.md3` textured
  from it with `effects/refraction`, scale 1 to 0.2 (Pull 0.2 to 1) over 500 ms,
  alpha 244 to 10, fixed in place after 200 ms. SJK keeps the
  `cg_renderToTextureFX 0` puffs. It needs the scene pass split after opaque
  entities on frames with a push, a frame-sized copy target made at resize and a
  distortion pipeline.

Unit tests cover the effect selection (own beam levels 2-6, own Grip in first
and third person, own Push, a remote caster, a trickster, Force Sight), parse
the retail Drain EFX, and step the trick fade (fade-out, hiding, fade-in,
truncation, reset after a second's absence, the Sight exception). None of it
has been checked in a game window yet. Formatting and the locked workspace
build and tests passed; clippy finds nothing in the changed code apart from
the four known `sjk-game-jka` deny errors, which stop it otherwise.

## g_debugMelee prediction

Local change based on `da8adc9` (2026-10-04): client prediction reads
`CS_SERVERINFO` `g_debugMelee` as OpenJK `codemp` does (`cg_servercmds.c:137`)
and predicts its melee kicks, the grapple's early return and the wall hold
(`bg_pmove.c:1621-1641,7464-7581`). Melee's alternate attack is now predicted
with the cvar off too (stock punches). On the JA+ dialect the levels follow JA+:
1 is the melee attacks only and 2 adds the wall hold; a grabbed wall leaves the
view to the player, and alternate attack standing still is a front kick. The
TaystJK dialect takes the JA+ levels without the view or standing kick (from
jaPRO's server code; not observed). JKR's own server does not simulate the cvar,
whose default there is 0. See [networking](networking.md#server-dialect-movement-rules).

Evidence (Windows 11): a scratch harness outside the repository joined a local,
windowless JA+ 2.4 Build 7 server (EternalJK x86 dedicated, `mp/ffa3`, loopback),
sent scripted commands at 8/7/4/3 ms, and replayed them offline through
`sjk-game-jka` exactly as the viewer reseeds from each snapshot, comparing
origin, velocity, view, animations, timers, flags, weapon time/state, holster and
ground per snapshot (about 640 per run). Scenarios: punches, four ground kicks,
standing alternate attack, the grapple, an air kick after a Force jump, a kick out
of a run; and a wall grab approached squarely and 30 degrees off square, holding
jump while sweeping the pitch.

| `g_debugMelee` | Melee runs, clean intervals | Wall runs, clean intervals |
| --- | --- | --- |
| 0 | all clean at every step (489 of 639 before) | free look clean in every grab |
| 1 | all but 2 (the grapple) at every step | all clean at every step (about 20 view misses per grab before) |
| 2 | all but 2 (the grapple) at every step | all clean at every step (439-554 of about 640 before) |

The grapple (`TryGrapple`) is server-only; stock clients do not predict it
either. Remaining misses in other runs came from a moving platform and a corner
slide that the world-only harness collision does not model, one wall run-up
onset and one 1-unit Force-jump velocity difference (one interval each in about
30 wall runs), both outside the changed code. Saber staff kicks
standing still on JA+ are not changed. Not run in the client. Formatting, locked
workspace build/tests passed.

## JA+ private-duel pass-through

Local change based on `da8adc9` (2026-10-04): on JA+ and TaystJK/jaPRO profiles,
prediction lets a bystander pass through duelling players and a dueller pass
through every player and NPC but its opponent
([networking](networking.md)). Windows 11 check, no client window: a local JA+
Mod v2.4 Build 7 server (EternalJK x86 dedicated, loopback, `devmap mp/ffa3`,
scratch home) with three windowless clients, two in a private duel. The server
let a bystander walk through a dueller (closest approach 0.2–2.4 units) and a
dueller walk through the bystander. For a client sending no plugin identity the
dueller arrived with `solid 0` and bystanders were never sent to duellers, so
prediction already matched. With the JA+ plugin identity (sent by #108) the
dueller arrived as a solid box with `bolt1`: replaying the bystander's commands
through the predictor with player boxes, the stock rule mispredicted 30–33 of
about 88 intervals around the crossing at 8/7/4/3 ms steps, the duel rule none.
Bystanders were not sent to duellers with "Duel see others" on or off, so no
drawing change was needed on this server. jaPRO was not exercised live, and
the harness models world collision and player boxes only. Not run in the
client. Formatting, locked workspace build/tests passed.

## JA+ grapple prediction

Local change based on `3a70c22` (2026-10-04): the JA+ hook pull and rope hang
are predicted (see [networking.md](networking.md#ja-grapple-hook)). Evidence: a
windowless JA+ 2.4 B7 server (EternalJK x86 dedicated, loopback, `jp_altDim 1`
so players start in the dimension that allows the hook) and a scratch replay
harness that joined, fired,
held, released, re-pulled and used off the hook twice on `mp/ffa3`, then replayed
each snapshot interval through the predictor. With the plugin identity, intervals
matching the server went from 549/774, 643/772, 536/776 and 544/775 at 8/7/4/3 ms
to 762, 763, 763 and 763; pull intervals mismatched 8/159, 4/94, 9/174 and 8/166
(all before: the hook's game-side edges) and rope-hang intervals 0 of 62, 30, 62
and 61 (all before). Without the plugin identity, 685 → 762 of 774 at 8 ms.
The hook's rope is drawn as EternalJK draws it
([rendering.md](rendering.md#billboard-icons)), checked by unit tests only.
Nothing was run in the client. Crouched and in-water pulls and TaystJK/jaPRO's
own grapple were not exercised.

## JA+ movement rules prediction

Local change based on `3a70c22` (2026-10-04): on a JA+ server, prediction follows
JA+'s movement and saber rules ([networking](networking.md#ja-movement-rules)):
the flip kick off players, the head-slide setting, the improved yellow DFA,
wall runs from the Force jump's flips, grip speed, melee with the holdable
button, free taunts and the staff's standing front kick. JA+ is closed source;
EternalJK's JA+ plugin reimplementation is the reference, and the server decides
where they differ (yellow DFA launch 60, not EternalJK's 50; no flip-kick branch
blocking wall runs; wall runs from flips, which EternalJK lacks). Other servers,
JKR's own included, keep the stock rules.

Evidence (Windows 11): the windowless replay harness outside the repository (as
for `g_debugMelee`), extended with an idle second client, `setviewpos`
placement on a devmap server and other players' boxes in the replayed
collision, against a local JA+ 2.4 Build 7 server on `mp/ffa3` with
`g_debugMelee 0` and the default `jp_cinfo` 196819. Clean intervals at
8/7/4/3 ms, with the JA+ rules and with them off (the previous prediction):

| Scenario | With JA+ rules | Rules off |
| --- | --- | --- |
| Yellow DFA, looking around in the flip | 501-502 of 502-503 (7/4/3 ms) | 412-413 |
| Staff alternate attack standing and moving | all 536-538 | 533-535 |
| Melee attacks with the holdable button | all 286-288 | 238-241 |
| Bow, flourish, gloat, meditate while moving and turning | 802-806 of 806-810 | 611-613 |
| Grip while walking | 238-240 of 239-240 | 179-187 |
| Walking off a player's head (103-104 snapshots on it) | 209/208 at 8/7 ms, 204 of 210 at 4/3 ms | 197-203 |
| Wall flip off a player beside | all 170-171 | 169-171 |
| Jump at a player, jump again close to it | 159-162 of 161-162 | 158-162 |
| Wall run-ups: plain jump, running Force jump, Force jump flips | all but one interval in 12 runs | 514-719, misses at every run-up from a flip |

A second server with flip kick off, the head slide on and the yellow DFA off
(`jp_cinfo` 196834) matched with the rules on in all of these: no flips off
players, frictionless heads, stock DFA, and wall runs from flips still allowed.

Remaining misses are the frames where the server applies a style change or a
taunt from a `generic_cmd` (server-only), slope stance animations the harness
cannot pose (no model feet), and a few one-unit velocity differences in contact
with the other player's box. One wall-run interval at 7 ms (a rebound where
prediction ended a run up the wall) is unexplained. Not predicted: the options
EternalJK never reads, the Jedi Outcast red DFA, a changed `jp_gripSpeedScale`
and holds for JA+'s extra animations. Not run in the client, and not checked
against a public JA+ server or another JA+ version.

## Eye adaptation (SJK only)

SJK's exposure follows the view (`r_autoExposure`, on by default, -0.5 to +1 EV
around `r_hdrExposure`, only brightening with `r_sceneHdr 0`); JKR's stays fixed.
Headless Vulkan and DX12 probes on 2026-10-04 (Windows 11, RTX 5080) showed the
resolve and effect layer byte-identical to the fixed exposure at exposure 1 and
checked metering, snapping and smoothing on synthetic scenes; both passes took
about 0.01 ms at 1080p and 0.02–0.03 ms at 4K. No client was run: the look in
play, the default key on real maps and the cost in a full frame are unverified.
See [rendering](rendering.md#eye-adaptation).

## Emission maps (SJK)

SJK-only branch `personal/emission-maps` (2026-10-05, based on `bcb0b76`): `_e`
emission maps on lightmapped world surfaces, added unlit with a dynamic-glow halo,
and, in real-time lighting, lamps for surfaces with no light of their own (capped at
1,024 per map); `sjk-materialgen` generation 3 writes them from shader, glow-image,
name and texel evidence. See [Emission maps](rendering.md#emission-maps). Workspace
build, tests and naga validation of the changed programs passed. A read-only survey of
the owner's installation (215 readable maps, 5,356 candidate textures) wrote 118
emission maps, refused 31 keyword or surface-light textures without luminous texels
and left 619 textures alone whose shaders already glow; on the 23 retail MP maps alone
it wrote 8 (neon signs, Bespin windows). No client was run: appearance, halo, the
light added in real-time lighting, load time and frame cost are unverified.

## Shader remaps from JKR (SJK)

SJK branch `personal/jkr-remaps` (2026-10-05, based on `7969f0b`) drops SJK's own
shader remap implementation (`personal/shader-remaps`) for JKR's: `main` at
`af65396` (#127) and Sol's open JKR follow-ups #128-#131, merged in that order.
The follow-ups overlap: #129's shared lookup and #131's `clearRemaps` were adapted
to #128's map remaps (latest remap wins), and `clearRemaps` also drops the map's
worldspawn remaps, as EternalJK's renderer command does. SJK keeps `cg_remaps 2`
(EternalJK's default; JKR's is 1). JKR's remap path did not know SJK's dynamic
glow: a remap that changed a material's stages left the glow pass with a stale
stage index, and joining a JoF server crashed (`world_glow.rs`, 05/10/2026). The
remap now rebuilds the glow order and the material's glow flag, and the glow pass
skips a stage that no longer exists. The locked workspace build of all targets and
the workspace tests passed. No game was started: remaps on screen, from a JoF
server or a map with worldspawn remaps, are unverified in this combination.

## First-person melee shows no baton (SJK)

SJK-only branch `personal/melee-viewmodel` (05/10/2026, based on `3f57938`): with
melee selected, the first-person view drew the stun baton on the baton's hand rig.
Stock registers no hand rig for `WP_MELEE`, which leaves the baton behind the eye,
so melee now has no view model. A unit test checks melee has none and the stun
baton keeps its model and three barrels; the locked workspace build and tests
passed. No game was started: the first-person view is unverified on screen.

## Outgoing text encoding (SJK)

SJK-only branch `personal/legacy-text` (2026-10-05, based on `2696590`) sends
names, chat, forwarded commands and `rcon` text in Windows-1252 when every
character fits, as retail and EternalJK do, instead of UTF-8; other text stays
UTF-8. See [player text](networking.md#player-text). Unit tests cover the
encoder (ASCII borrowed, Latin-1, the Windows-1252 typography bytes, C1
round-trip, non-Windows-1252 text left as UTF-8), the reliable-command path, a
name decoded from the wire going back byte-exact, and the compressed connect
packet and `rcon` datagram bytes; the locked workspace build and tests passed. No
game was started: how a retail or EternalJK client shows SJK's chat and names on
a live server is unverified.

## SJK emblem (SJK only)

SJK-only branch `personal/sjk-logo` (2026-10-05, based on `bcb0b76`) puts Sol's
emblem in the classic main menu's ring, where the `ja01` logo video played (the
video is no longer read), and above the modern main menu's title, with a pulsing
core and shimmering blade lights drawn as additive layers. `sjk.exe` and
`sjk-server.exe` carry it as their Windows icon, the client sets it as its window
icon, and the README, release notes and site use it. See
[client.md](client.md#menu-style) and [assets/branding](../assets/branding/README.md).

Verification (2026-10-05, Windows 11, MSVC): formatting, the locked workspace
build and tests, including unit tests for the glow curves, the emblem's place in
the ring at six window sizes and above the modern title, its texture ids, the
alpha-weighted mips, the additive runs and the decoding of every bundled picture
and window icon; the optimized build, whose executables were checked to contain
the 10-size icon group and the version strings. The menu was composited offline
from the retail art at two scales and three animation phases. No client window
was opened: the look on screen, the additive pipeline on a GPU, the window and
taskbar icons and the X11 icon are unverified, as is the GNU toolchain's
`windres` path.

## Visual defaults (SJK only)

SJK-only branch `personal/sol-visual-defaults` (2026-10-05, based on `78b8bf7`)
makes Sol's own settings the defaults: noon sun (`r_dayHour 12`), bloom
(`r_sceneBloom 1`), sunbeam dust (`r_dustMotes 1`) and material maps
(`r_normalMapping`, `r_specularMapping`, `r_parallaxMapping` 1, which act only
where a pack supplies maps). Saved `config.cfg` values still win; nothing is
migrated. See [Default visual profile](rendering.md#default-visual-profile).
Formatting, the locked workspace build, tests and Clippy passed. No client was
run: the look and frame cost of the new defaults are unverified.

## Implemented scope

- PK3/loose-file content, BSP maps/collision, legacy models and shader scripts.
- Protocol-26 client/server sessions, shared JKA game rules and client prediction.
- Graphical client with browser, menus/settings, HUD, console, audio, screenshots,
  demos and a Create game flow.
- wgpu BSP renderer with optional modern lighting and post processing.
- Offline generator of local rend2-convention material maps and SJK emission maps
  (`sjk-materialgen`).
- Dedicated-server game integration, console/configuration, stock game-type
  options, map entities, bots/NPCs and script integration.

Entry points and ownership are linked from [architecture.md](architecture.md).
The lists above describe source coverage; they do not close the validation gaps below.

## Verification recorded for this baseline

| Check | Result and scope |
| --- | --- |
| Linux workspace build, Cargo tests and formatting | Passed; no bundled regression tests currently run |
| Optimized client and dedicated-server build | Passed with Rust 1.96.1 on Linux |
| External OpenJK movement reference checks | 560 cases / 72,275 commands at 8/7/4/3 ms; movement, animation, events and compared wire fields matched |
| External player-angle checks | 125 samples matched the OpenJK reference |
| Local OpenJK client against JKR server | Joined `mp/ffa3`, walked, jumped and turned; no prediction misses observed in that run |
| Native Vulkan rendering | Local JKR client/server completed joined-map rendering on `mp/ffa3` with defaults and with day/night + lighting tier 2 + HDR; each continued for 15 seconds without panic or GPU validation error |

The reference checkout used for the external checks was OpenJK
`1a6a643427aa347553e9073dac5570b33337c4d9`, multiplayer `codemp`.
The external harnesses and raw reports are not part of this repository, so these
are recorded maintainer results, not checks reproducible by running `cargo test`
alone. The GPU runs used debug builds and establish startup/integration only;
they do not establish visual parity or release performance.

Sun-shadow correction (2026-10-02, based on `4a8fe31`): external release GPU
captures reproduced and removed ground self-shadow bands in the reported
`mp/ffa3` view, including a nearby camera position. An `mp/ffa1` comparison
showed no obvious regression. See [rendering](rendering.md) for settings,
timings and limits. The updated production release client also rendered
`mp/ffa3` on Linux/Vulkan (Radeon RX 9060 XT) with day/night and HDR enabled
without a panic or GPU validation error during a short startup check.
The owner also playtested the release build and confirmed that the reported
view looked clean.

Sun-shadow edge refinement (2026-10-02, based on `980e693`): the owner
accepted the release playtest with smoother structure shadows and more stable
player-shadow overlaps. Cascades share a reconstruction footprint and wider
transition bands. World and moving-caster depths are filtered independently;
a world-space blocker search and Gaussian reconstruction smooth broad edges
without letting a player change the building's separation estimate.

External release GPU evidence covers the three marked `mp/ffa3` views, camera
approaches and seven positions of an actor in the dynamic-caster pass. Adding
the actor did not brighten unchanged receivers in that probe. Tier 2, day/night
disabled and actor-only GPU smoke checks passed, as did formatting, workspace
build/tests and the release client build. The tested 4K view adds about 0.49 ms
of GPU work over the preceding playtest filter. See [rendering](rendering.md)
for settings, the overlap approximation, memory cost and remaining limits.

Performance work based on `b4debe4` (2026-10-02) preserves the accepted shadow
filter while skipping provably constant footprints, unused baked-lightmap reads
and hidden opaque shading. External 31-player replays show 4K GPU means falling
from 5.30 to 4.57 ms on `mp/ffa3` and 6.71 to 5.81 ms on `mp/ffa1`. At
2560×1080, `ffa3` GPU work is 1.85 ms but total frame time remains 2.47 ms:
the 2 ms target is still open. Shader reference comparisons, alternate-mode
captures, workspace checks and a native release smoke check passed; see
[rendering](rendering.md) for settings, evidence and limitations.
An additional PVS/area candidate cache reduced CPU world-pass encoding by
0.052 ms on the smaller `ffa3` replay and matched 3,178,666 direct-traversal
results, including forced visibility transitions. Its total-frame gain was
0.042 ms there; the reflection-heavy `ffa1` route was essentially unchanged.
Extending opaque depth priming to floor reflections then saved about 0.036 ms
of GPU work in paired 4K `ffa1` runs; lower-resolution timing and captures on
both maps also passed. Reflection resolution and shadow filtering are unchanged.
Lazy particle-stage sampling removes about 0.025 ms of measured effect preparation
on the `ffa3` replay; 3,360 sampled stage values matched the previous implementation
bit for bit. Its total-frame effect was within run variation.
Caching immutable material sort keys saves another 0.01–0.014 ms in instance
preparation on the two routes. The original comparator matched 1,362,200 ordered
draw entries; total-frame improvement remains below run variation.
Reusing each draw's stage-major classification removes a further 0.011–0.016 ms
of CPU world-pass encoding in paired runs, with unchanged draw storage size
and reference-checked classification.
Fixed AO sample tables and equivalent depth-ray arithmetic save another
0.023–0.026 ms of total GPU time in paired 4K runs, with unchanged AO settings
and checked captures on both routes. The owner playtested and accepted the
combined performance preview before publication.


Submission and deferred-lighting work based on `f3f3db2` (2026-10-02) records
frame uploads for a bounded submission worker, restricts HUD shading, shades
uncached lamp receivers in compute, avoids redundant clears/copies and uses
conservative shadow-bound mip levels. On Linux with Ryzen 5 5500 / RX 9060 XT,
external 31-player replays at 2560×1080 reduced mean total frame time from
2.464 to 1.822 ms on `ffa3` and 3.042 to 2.157 ms on `ffa1`. The latter's p99
increased from 3.757 to 4.263 ms, so improved tail latency is not established.
Finite image comparisons and workspace/release checks passed; see
[rendering](rendering.md#submission-and-lighting-work-reduction) for evidence
and limits. Gameplay and protocol code are unchanged.

Optional dust (`r_dustMotes`, default off in JKR, on in SJK) is restricted to local godray scattering,
with colour and visibility sampled at each mote's depth. It requires active
volumetrics and follows their shadows and clarity. Linux workspace/release
checks, GPU sampling probes and native HDR/SDR captures passed on 2026-10-02
(Ryzen 5 5500 / RX 9060 XT). The earlier everywhere-dust preview was superseded
after owner feedback. See [rendering](rendering.md) for
current verification and remaining visual/readability limits.

Resident world transitions (2026-10-03, local changes based on `8f692ac`):
menu joining and live map changes retain a rendered, locally playable world.
External release checks against an isolated loopback TaystJK server exercised
FFA3 entry, FFA3 → FFA1, same-map restart, and two consecutive map changes.
Cancellation after entering the destination restored the menu, and the transition
sequence also passed with a local bot present. Artificially delaying delivery of
the completed session from the connection worker allowed more than 1,600 locally controlled
frames before the verified session attached without rebuilding the map. That
check exercises delayed handoff, not a real slow-network handshake. The
same-map restart reattached about 50 ms after its transition event. Old-map
movement and rendering continued while the replacement was built, without a
loading overlay or old remote actors in the resident runtime world.

The installed-archive checksum comparison matched ordered CRC sequences for all
73 previously readable PK3s. Six small external ZIP cases covered empty archives,
empty files, Unicode names, duplicate names, an executable prefix and ZIP64
sizes; malformed central-directory data was rejected. A catalogue with an invalid
unused local payload header is now inventory-readable, matching OpenJK's
central-directory inventory behavior; loading an affected asset still validates
its header/data. One installed-set comparison measured 4,697 ms for the old
payload-header inventory and 134 ms for the directory reader. These checks do not
constitute a new pure-server/wire certification; protocol codecs were unchanged.

Checks used Linux/RADV on a Radeon RX 9060 XT, 960×540, owner graphics settings,
and external ignored harnesses. Normal joins measured about 7–18 seconds in these
runs, depending on machine/cache pressure; the early pre-optimization run took
41 seconds. Typical FFA1 background preparation was about 8–12 seconds. These
are observations, not a controlled cold-cache speedup or a populated-match frame
budget result. Simultaneous build pressure worsened some runs substantially.
One default-backend headless run emitted an EGL destruction panic on the submit
thread after all assertions passed and while exiting. The explicit Vulkan rerun
completed without that diagnostic; native-window/backend shutdown coverage is
still needed. Cold loading, missing-content downloads, platform coverage and visual acceptance
remain open. The movement adapter calls the existing predictor; the external
OpenJK on-foot/force-jump comparison still passed 560 cases / 72,275 commands,
including 8/7/4/3 ms caps. See [client transitions](client.md#joining-and-changing-maps)
for local-authority and exploration limits.

Local gameplay continuation (2026-10-03, same unmerged baseline): departed live
worlds now run the native dedicated gameplay behind a socket-free client session.
Release GPU checks on isolated loopback TaystJK exercised FFA3 → FFA1, same-map
restart, rapid map changes and cancellation, both with and without a bot. They
asserted that only the local player remained, local saber moves advanced, and
the real connection reattached. Screenshots confirmed the third-person model and
saber remained visible. A further run granted test-only weapons and observed
pistol projectiles after releasing the saber attack and switching weapons. It
injected an intermission movement type at handoff to check recovery from the
cached playable state; natural match-end timing remains unverified. A stock
FFA3 door import check preserved its open position, area portal and closing timer.
Native import/reuse checks covered slots 0/17/31 at
8/7/4/3 ms; the 560-case movement/animation/event fixtures and compiled codemp
weapon zoom/charge fixture passed. These are focused checks, not complete native
combat, mod or vehicle compatibility certification. First gate entry is still
movement-only until a server player is available. Its original connection notice
and pointer Cancel action are restored for menu joins; in-server map changes
keep the local gameplay presentation without that overlay. The notice restoration
passed workspace build/test/format checks; native visual acceptance is pending.

In the final 960×540 Vulkan run, local render calls after the first ten local
frames measured 0.98 ms median, 2.72 ms p99 and 38.35 ms maximum, excluding the
harness sleep and capture work. This is a single-player continuation while
background loading, not a 31-player benchmark. Initial model, effect and shader
work can still hitch; moving native skeleton loading into background preparation
removed the observed roughly 0.6-second first-saber-command stall in that run.
It does not establish hitch-free transitions or complete content coverage.

Transition/input polish (2026-10-03, unmerged changes based on `8f692ac`):
a queued Alt bind was reproduced surviving focus loss; focus handling now drops
that frame's gameplay input and ignores synthetic keyboard events. Isolated
TaystJK and native JKR runs observed a genuine saber throw return after focus
loss. The retained actor keeps its animation tracks and displayed prediction,
with local presentation paced by the same wall-clock origin as local commands.
Remote world adoption and backwards server time retire stale runtime samples;
a forced provisional-clock rollback restored every entity to the current epoch.

Native server lifecycle checks also found bots waiting for an impossible network
acknowledgement after a map change, loss of bot identity during `map_restart`,
and old saber entity handles surviving a rebuilt entity pool. Bots now begin
immediately on the new map, and transient player state is reset while preserving
session/bot ownership, following multiplayer `SV_SpawnServer`/`ClientConnect`.
External integration checks cover active bot slots, current-pool saber handles
and advancing bot commands after both kinds of transition. No packet codec or
movement/combat rules changed. The external 560-case on-foot reference checks
passed again at 8/7/4/3 ms. Native visual acceptance and wider mod/vehicle coverage
remain open.
The final 960×540 Vulkan native-server run covered eight bots, FFA3 → FFA1,
same-map restart, rapid map changes and cancellation. All 8,650 presented remote
actor endpoints matched their received snapshot positions. The saber returned
about 1.21 seconds after the test press, following focus loss. Local render calls
measured 0.56 ms median and 1.93 ms p99, with a 287.81 ms maximum during background
loading; this does not establish hitch-free transitions or 31-player performance.
Workspace build/tests, formatting, standalone clock checks and release builds
passed. The owner playtested the updated transitions and accepted the combined
preview for publication. Wider mod, vehicle and platform coverage remains open.

Natural intermission and chat (2026-10-03, local changes based on `a993436`):
the previous forced-map checks missed the frozen scoreboard phase at normal
match end. The viewer now starts full local continuation before presenting that
snapshot, while the real remote session retains scores and communication.
A Linux/RADV 960×540 release check against an isolated loopback TaystJK server
expired its timelimit, displayed authoritative scores over the local character,
used the stock ready button and followed `nextmap` from FFA3 to FFA1. The final
run observed 1,404 scoreboard frames and 2,450 loading frames with exactly one
local actor and continuing movement, then adopted the destination. Local render
calls measured 0.92 ms median, 2.37 ms p99 and 309.76 ms maximum; an earlier run
under concurrent build pressure reached 3.11 seconds. This is not hitch-free or
a populated-match performance certification.

Socket-free mock endpoints verified global/team/private composer dispatch and
console chat routing without sending test messages to any server. Incoming
server announcements continued during intermission. They now go to the console
exclusively: the owner requested a general separation of console prints from
chat after observing TaystJK's `PrintStats` table in the conversation overlay.
Global/team/private chat and separate center-print HUD notices are preserved;
no server-specific table filter is used. An external offline check confirmed
that 101 console prints, including a stats table, could not enter or displace
chat history or close its composer; chat/team messages and a center notice still
reached their intended presentation. Workspace build/tests, formatting and the
updated production release build passed. This routing check sent no messages
to a server. Captures cover the composer
beside real scores and a synthetic 32-player team board; 576 geometry cases cover
1–32 rows, FFA/team layouts and nine viewports from 640×480 through 4K, including
ultrawide and portrait. These checks do not establish human-to-human delivery or
all mod/platform behavior. Client changes leave gameplay rules and wire codecs
untouched. External compiled OpenJK on-foot/force-jump checks passed again at
8/7/4/3 ms (560 cases / 72,275 commands). Workspace build/tests, formatting and
the production release build passed. The owner accepted the combined preview
for publication; broader mod and platform coverage remains open.

Default visual profile (local change based on `a993436`): fresh profiles now
use the selected day/night, volumetric, shadow, HDR, AO and filtering defaults.
An external release/Vulkan check on Linux/RADV RX 9060 XT verified the graphics
values for empty, explicit and existing override configs and rendered FFA5 and
FFA1. The fresh and explicit FFA5 captures matched at 99.94% of pixels, with
mean absolute RGB difference 0.00022 levels and maximum 2/255. Existing saved
values remained authoritative. Formatting, workspace build/tests and the
production release build passed on the combined local changes. Personal configuration is
excluded; see [default visual profile](rendering.md#default-visual-profile).
This is startup/rendering evidence, not a new populated-match performance claim.

## Open validation and limitations

Volumetric silhouette correction (local changes based on `a993436`): excluded
depth samples no longer dilute the visible-air lighting estimate. External
Linux/RADV release checks reproduced and removed the sampled FFA5 player fringe
without increasing grid resolution. Paired 720p/4K timings showed no material
change in the tested scene; captures also cover FFA3, a Rancor interior and 24
camera turns. See [volumetric coverage](rendering.md#volumetric-silhouette-coverage)
for measurements and reproduction limits. Formatting, workspace build/tests and
the production release build passed. Owner acceptance and broad content coverage
remain open.

- Complete server/gameplay parity remains unverified. Audit concrete scenarios
  across game types, combat, vehicles, NPCs, scripting and map transitions before
  marking individual capabilities complete.
- Model animation sounds now follow `animevents.cfg` frames. Local release checks
  against OpenJK `3e465e7c`'s extracted `CG_PlayerAnimEvents` predicate matched
  238,328 frame-crossing cases. Walk/run and blue-style gesture cues were stable
  at 8/7/4/3 ms steps; a six-second gesture produced its ten authored spin cues
  at every cap, and held frames did not replay them. Include overrides, material
  selection and missing voice-family fallbacks passed external checks. A
  32-actor cursor-only microbenchmark averaged 0.44 microseconds per iteration
  with zero measured heap allocations; this excludes bone queries, collision
  traces, mixing and rendering.
  An isolated loopback TaystJK `5802c99` run produced stone/metal running steps
  and blue-taunt spin cues through a real decoder and null-output mixer, with
  zero decode failures or missing handles. Authored custom saber sound fields
  also passed an external parsing check. Formatting, locked workspace build/tests
  and the optimized Linux client build passed. Native owner listening, broader
  custom-model coverage and animation effect/footprint rendering remain open.
- Snapshot entities draw a model from `modelindex` only for the entity types
  whose codemp cgame function does so; the per-type rules and their reference
  are in [entity_models.rs](../crates/sjk-client/src/entity_models.rs). Models
  codemp draws that the client still does not: force holocrons, non-brush
  `ET_MOVER` models and a mover's secondary `modelindex2` model, and the
  portable shield (`ET_SPECIAL`) and `ET_BEAM` effects.
- Mod compatibility is scoped by explicit profiles; broad BaseJKA/JA+/TaystJK
  feature parity is not established by profile detection.
- Community PK3 compatibility needs broader map/model coverage. One retail map
  cannot establish every shader, animation or content combination.
- Windows runtime behavior is largely unverified. Do not infer platform support
  from source conditionals alone. Windows reserves 1 MiB for the main thread and
  the client overflowed it after loading `mp/ffa3`; [the viewer build
  script](../crates/sjk-viewer/build.rs) now links Windows binaries with the
  8 MiB Linux size. On Windows 11 (Rust 1.96, MSVC), release and debug clients
  then loaded `mp/ffa3`, and a release client joined a local JKR server and
  completed its map load. Longer play, other maps and the GNU toolchain are unchecked.
- DX12 rendering is unverified. JKR does not select DX12 itself (Vulkan is preferred
  where present); with `WGPU_BACKEND=dx12` and no `dxcompiler.dll`, wgpu compiles
  shaders with FXC. A headless pipeline build of every entry point of the 39 viewer
  shader modules on DX12/FXC (Windows 11, RTX 5080, 2026-10-02) fails only for the
  HUD fragment program (error X3507: the function ends in `discard` without a
  return) and the GI probe update (error X4026: a workgroup barrier after
  storage-dependent early returns). The stage programs' skinning loop no longer
  fails with X3511.
- The 500+ FPS / roughly 2 ms frame target is not certified. Measure representative
  release workloads, including populated matches and chosen graphics settings.
- SJK material-map work (2026-10-04, Windows 11, change based on `024c22a`): the
  lamp/bounce direction target, reflection probes, material-mapped floor mirrors,
  `r_normalMapStrength` and materialgen generation 2 pass the workspace build and
  tests, and naga validates every new or changed program; a dry run of the generator
  on `mp/ffa3`/`mp/duel1` picked 19 metal textures and one polished texture. None
  of it has run on a GPU: appearance, wgpu validation at run time, capture load time
  and frame cost in a match are unverified (estimates are in
  [rendering](rendering.md#reflection-probes)).
- The repository does not bundle a regression suite. Required reference evidence
  must be supplied externally until an in-repository verification approach is agreed.

Sky/terrain correction (2026-10-03, local preview based on `8f692ac`): the three
`t1_danger` marks exposed sky draw-buffer reuse between views and smoothed normals
flipping across visible hills. Both terrain bands are removed in fixed GPU views;
a 24-turn capture reproduced sky disappearance in 12 old-reset frames and verified
the corrected path against direct draws. Workspace checks and owner release build
passed. Native owner playtesting remains pending; see
[rendering](rendering.md#sky-scenery-and-hillside-orientation) for scope and timings.

## Current priorities

1. Evaluate dark-area readability while preserving the accepted lighting style
   (owner priority, 2026-10-03). A local, default-preserving indirect gain and fill
   occlusion experiment passed workspace and external GPU checks; see
   [rendering](rendering.md#indirect-lighting-and-dark-area-readability). Owner
   acceptance and populated-match measurements are pending. The marked custom-map
   ceiling lights were recognized but underpowered; an explicit-glow inference
   refinement passed source-policy and GPU checks on that map plus two stock maps
   ([fixture inference](rendering.md#inferring-fixture-light-from-legacy-materials)).
   That refinement also awaits owner acceptance. Static model fixtures were also
   missing from source extraction: both `t2_rancor` marks now gain local illumination,
   with unchanged stock-map appearance in the sampled `ffa1`/`ffa3` views. The
   Rancor checks add approximately 0.017–0.144 ms of GPU work; see
   [static model fixtures](rendering.md#static-model-fixtures) for evidence and
   remaining visibility limits. Owner acceptance is pending. A subsequent GPU audit
   confirmed a sign error in GI voxel traversal. The corrected forward distances
   and range checks pass 8,302 external GPU/reference cases and workspace checks.
   Both Rancor captures remain byte-identical, so the correction has not solved
   their low visibility. Readback confirms nonzero live probe lighting; receiver
   coverage and effective bounce strength remain to investigate. See
   [GI traversal correction](rendering.md#gi-traversal-correction).
   Dust remains parked.
2. Improve populated-match release frame times while preserving the owner-accepted
   appearance (owner priority, 2026-10-02). Target below 2 ms with 31 players;
   reaching that target is not a reason to stop investigating useful savings.
3. Stabilize normal client and dedicated-server use with reproducible local reports.
4. Audit compatibility gaps by subsystem and scenario; preserve exact combat and wire behavior.
5. Broaden community-content and platform validation.

These priorities guide requested work; they do not authorize an assistant to
start an unrelated task. Update this page when evidence or agreed priorities
change. For a new result, record the source revision, environment, scenario,
reference and limits. Keep resolved change history in Git rather than appending
session-by-session notes here.
