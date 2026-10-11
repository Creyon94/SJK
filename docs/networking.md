# Networking and gameplay

SJK targets Jedi Academy multiplayer protocol 26. Keep the network representation
compatible with ordinary servers and clients; internal engine modernization does
not authorize a wire change.

## Ownership

- [sjk-protocol](../crates/sjk-protocol/src/lib.rs) owns message encoding/decoding,
  gamestates, snapshots and legacy field representations. It has no sockets.
- [sjk-network](../crates/sjk-network/src/lib.rs) owns transport, discovery and the
  legacy endpoints. The server endpoint calls a `LegacyGameHost` interface.
- [sjk-client](../crates/sjk-client/src/lib.rs) owns live client session state,
  reliable commands, snapshot history and client-side integration.
- [sjk-game-jka](../crates/sjk-game-jka/src/lib.rs) owns game behavior shared by
  prediction and server simulation.
- [sjk-dedicated's bridge](../crates/sjk-dedicated/src/bridge.rs) connects those
  rules to native server storage and legacy replication. `sjk-server` itself
  owns worlds/entities, not the JKA game loop or network format.

Client [compatibility profiles](../crates/sjk-client/src/compat_profile.rs)
explicitly distinguish BaseJKA, JA+, TaystJK/jaPRO and unknown modules from
serverinfo, or before connecting from a `getinfo` reply's `game` directory.
Profile detection and implemented adapter behavior are not a promise that every
feature of those servers is reproduced by SJK's dedicated server.

On JA+ and TaystJK/jaPRO servers the client identifies as a client-plugin user
through extra userinfo keys. JA+ gets `cp_clanPwd none` and JoF EJK's
`cjp_client` (`1.6.4 JoF`, the value JoF EJK's latest release sends there), so
an admin's `amstatus` lists an SJK player as a JoF EJK one until SJK is
announced; it was the JA+ 1.4B4 plugin's `1.4B4` before 11/10/2026. The player's
`cjp_client` setting (archived, empty by default) replaces the plugin name on both
plugin profiles, to match a new JoF EJK release or show another name, without a new
build; it is sent nowhere else, and a value the info string cannot carry (`\`, `;`,
`"`) leaves the default. The userinfo carries it as a generic override of an adapter
extension (`LegacyUserInfo::extension_overrides`). Both plugin profiles also get the player's
`cp_pluginDisable` cvar (archived; a set bit switches a plugin feature off),
from the connect packet on and in every later `userinfo` update; stock and
unknown servers never receive it. Its default is EternalJK's 1536, which opts
out of the holstered-saber and ledge-grab features drawn with the plugin's
extra animations. A JA+ server then treats the client as a plugin user: it
serves custom RGB blades (`cp_sbRGB1`/`cp_sbRGB2`, sent whenever a blade selects
RGB) and appends a deaths field to each `scores` row (15 fields instead of 14).
The server sends at most 20 rows (`MAX_CLIENT_SCORE_SEND`) but announces every
connected player, so the row width comes from the fields actually sent rather than
the announced count: the width that divides them, and when both do, the one whose
rows read as players (unique client numbers below 32, a ping of -1 to 999, a
non-negative time).
Player blade tints in a player configstring's `c3`/`c4` keys are read for every
profile; the JA+ 2.4 server module formats both keys too. The JA+ client
plugin's `serverconfig` and `pluginDisable` commands are the JA+ tools mod's
`japlus.serverconfig` and `japlus.plugin` ([mods.md](mods.md)); the setting stays
in the core client.

### Player text

Names, chat and other typed text travel as byte strings that every client draws
through a 256-glyph codepage font. The retail client, a Win32 ANSI program, sends
the `WM_CHAR` bytes of the Windows code page; EternalJK sends one byte per
character (`ConvertUTF32ToExpectedCharset`). SJK sends text whose every character
has a Windows-1252 byte (ASCII, Latin-1, and Windows-1252's `€`, `™`, `œ`, curly
quotes and dashes) as those bytes rather than UTF-8
([legacy_text.rs](../crates/sjk-protocol/src/legacy_text.rs)), in the connect
packet's userinfo, every reliable command (chat, `userinfo` updates, forwarded
console commands) and `rcon` lines, so retail and EternalJK players read `ø` as
`ø`, not `Ã¸`. Text with any other character (Cyrillic, CJK, emoji) is still sent
as UTF-8, which only SJK clients decode; EternalJK's Windows-1251 and
Windows-1250 mappings are not reproduced. Incoming text, including the
names servers embed in `print` and `cp` commands, is decoded by
[decode_legacy](../crates/sjk-client/src/legacy_text.rs): valid UTF-8 as UTF-8,
anything else as Latin-1. A byte in 0x80..=0x9F therefore stays the C1 character
of that value, draws the font glyph of that byte and is sent back as the same
byte, so a name copied from the game round-trips exactly. The renderer picks a
glyph by Windows-1252 byte ([text.rs](../crates/sjk-viewer/src/text.rs)), so a
typed `€` or `’` draws the same glyph as byte 0x80 or 0x92 from another client,
and the modern font fills those slots with the Windows-1252 characters instead
of blank C1 controls.

A player's name is read from the bytes of the `n` key of its `CS_PLAYERS`
configstring and decoded on its own (`LegacyClientInfo::name`,
[client_info.rs](../crates/sjk-client/src/client_info.rs)), never from a UTF-8
view of the whole string: the server keeps Latin-1 letters in names
(`ClientCleanName` drops only control bytes and a few unused Windows-1252 ones),
and one such byte (`é` is 0xE9) made the whole string invalid UTF-8, so the
console's kill messages printed that player as `noname` and the gendered suicide
messages took the male form. Those messages follow `CG_Obituary`
(`cg_event.c:124-454`): each name ends with `^7`, so a colour a name leaves open
stops there. `noname` remains only for a slot with no name at all, where OpenJK
prints an empty one, and SJK prints a whole name where OpenJK cuts it to 29 bytes
(`obituary_name`, [obituary.rs](../crates/sjk-client/src/obituary.rs), which the
[kill feed](client.md#kill-feed) shares).

### Player-state arrays

The snapshot's `stats`, `persistant` and `ammo` arrays carry 16-bit entries that
codemp reads with `MSG_ReadShort` (`MSG_ReadDeltaPlayerstate`), sign-extending
them into the int fields; only `STAT_WEAPONS` is an unsigned 19-bit field.
[snapshot.rs](../crates/sjk-protocol/src/snapshot.rs) decodes them the same way, so
a score of -1 after a suicide, negative health on death and ammo's -1 sentinel
read as -1, not 65535.

### Server-dialect movement rules

Prediction follows rules the server advertises in `CS_SERVERINFO`, read by
[pmove_rules.rs](../crates/sjk-game-jka/src/pmove_rules.rs): the roll fixes of
JA+ (`jp_cinfo`) and TaystJK/jaPRO, and `g_debugMelee`
([pmove_debug_melee.rs](../crates/sjk-game-jka/src/pmove_debug_melee.rs)). Stock
`codemp` turns on the melee kicks, the grapple and holding a grabbed wall at any
nonzero `g_debugMelee`. JA+ splits the levels (1: melee attacks, 2: also the wall
hold), never turns a player holding a wall to face it, and kicks forward on an
alternate attack standing still. Kicks, like saber attacks, are predicted only with
an animation length table: a joined game reads the humanoid
`models/players/_humanoid/animation.cfg` for it
([local_prediction.rs](../crates/sjk-viewer/src/local_prediction.rs)), as EternalJK
hands Pmove the local player's animation set (`cg_predict.c:1311` at a40e793). The dedicated
server does not simulate `g_debugMelee`; its default there is 0, which keeps
prediction on the stock behavior.

JA+ and TaystJK/jaPRO isolate private duels: the two duellers and everyone else
pass through each other. A JA+ 2.4 server leaves part of that to client-plugin
users, sending them a dueller as a solid player box flagged with `bolt1`, so on
those profiles prediction skips duelling players for a bystander and every player
or NPC but the opponent for a dueller
([duel_isolation.rs](../crates/sjk-client/src/duel_isolation.rs), applied where
[prediction_movers.rs](../crates/sjk-viewer/src/prediction_movers.rs) builds the
entity solids). Stock and unknown servers keep duellers solid.

## JA+ grapple hook

On JA+ servers the client predicts the grapple hook (`+button12`) with the rules
in [pmove_grapple.rs](../crates/sjk-game-jka/src/pmove_grapple.rs); other
servers, SJK's dedicated server included, get no hook movement. The JA+ game fires the hook,
stores its anchor in `lastHitLoc` and flags the pulled player with `PMF_GRAPPLE`
(pm_flags bit 15). Each move then aims 16 units short of the anchor along the
view and replaces the velocity with a pull of 800 units/s (10 units/s per unit
inside 100 units), EternalJK's arithmetic and the JA+ 2.4 B7 module's, followed
by an air move whatever the ground or water below. A client-plugin user
who lets go of the key stays on the rope: the game clears the flag and
sets entity flag bit 16, and each move runs an air move and then swings the
player on a rope as long as the distance from the anchor to where the move began.
Use lets go of the hook in the game before the move, so a pull or hang with use
held is predicted as neither. Where EternalJK and the JA+ module differ (the pose
sets the legs only; a crouched player is pulled too; the pull always ends in an
air move), prediction follows the module. The game-side edges, the hook firing,
taking hold and letting go, arrive with the next snapshot and cannot be predicted.

### JA+ movement rules

On a JA+ server, prediction follows the rules
[pmove_japlus.rs](../crates/sjk-game-jka/src/pmove_japlus.rs) reads from
`CS_SERVERINFO`: the dialect and its `jp_cinfo` bits. JA+ is closed source; the
client side follows EternalJK's reimplementation of the JA+ client plugin and,
where that and a JA+ 2.4 server disagree, the server as replays observed it.
Stock and other servers, SJK's dedicated server included, keep the stock rules.

| Rule | When | Effect |
| --- | --- | --- |
| Flip kick | `jp_cinfo` flip kick (`jp_allowFlipKick`, default on) | Wall flips off a player beside, and a flip back off a player ahead when jumping at one while still rising (above 200); a run up a wall is unchanged when no player is there |
| Head slide | `jp_cinfo` head slide (`jp_slideOnPlayer`, default off) | Without it, standing on a player has ground friction instead of stock's frictionless slide |
| Yellow DFA | `jp_cinfo` yellow DFA (`jp_improveYellowDFA`, default on) | The medium flip over leaps 60 forward (stock 150) and neither turns nor locks the view |
| Wall run from flips | Every JA+ server | A run up a wall may start from the Force jump's forward, left and right flips, not only from a plain jump |
| Grip speed | Every JA+ server | Gripping keeps 0.8 of the run speed (stock 0.4; `jp_gripSpeedScale` default) |
| Melee buttons | Every JA+ server | With melee, an attack pressed with the holdable button is not cancelled |
| Taunts | Every JA+ server | Meditation keeps the player in place but the view free; other taunts leave movement and view free |
| Staff kick | Every JA+ server | A staff's alternate attack standing still is a front kick |
| Melee kicks | Every JA+ server | JA+ plays its own kicks server-side (melee's spin and back kicks in place of the stock W+A and W+D kicks, the jumping back kick, the backflip kick and the flip stab); the kicker is held still with the view free. A kiss, a ledge, a get-up, a stab or a backflip kick taken holds the player and the view (JoF EternalJK `bg_pmove.c:12470-12498`, `SVMOD_JAPLUS`, at bd5e202). The client predicts the stock kick until the server's kick arrives, as JoF EternalJK does |

Not predicted: the options EternalJK never reads outside its `serverconfig`
listing (single-player attacks, new DFA, model scale, kata, auto replier, ledge
grab, alternate dimension, macro scan), the Jedi Outcast red DFA
(`jp_jk2RedDFA`, off by default), a changed `jp_gripSpeedScale` (not published)
and the animation holds for JA+'s extra GLA animations.

### Wall-move view holds

Wall runs, runs up walls, ledge flips and stock wall rebounds report when their
last simulated slice called `PM_SetPMViewAngle`
([pmove_wall_moves.rs](../crates/sjk-game-jka/src/pmove_wall_moves.rs)). The
camera then presents that slice's held pitch and yaw, including frames that
reuse prediction at the same command time. Applying only changes in
`delta_angles` left fresh mouse movement visible on those frames; the local
character's root also follows the camera yaw. The hold clears on the next
slice that does not hold the view. JA+'s free-look rebound does not set it.
This is local presentation state, with no protocol field or movement change.

### Equipped jetpack prediction

`PM_JETPACK` (movement type 1), used by the equipped jetpack on stock-derived
servers including JA+, runs the reference's gravity, thrust and air-control
rules ([pmove_jetpack.rs](../crates/sjk-game-jka/src/pmove_jetpack.rs)). The
4096-unit box trace uses `MASK_SOLID`; below 128 units gravity is 0.1 of normal,
otherwise 0.25, truncated to an integer and restored after each command slice.
Jump applies up to 12 units of vertical thrust per slice, reduced by height
when already rising; the 256 threshold gates thrust rather than clamping its
result. Idle flight limits descent to -100 and adds 2 units below 64 units.
Air control is 0.8 while idling and 2 while thrusting. Ground tracing leaves
the pack airborne even directly above a floor, its legs follow movement
direction, and jump pads accept movement type 1.

Reference: OpenJK `codemp/game/bg_pmove.c` `PmoveSingle`, `PM_AirMove` and
`PM_GroundTrace`, at `4d0dfaf1`. Focused tests cover 8/7/4/3 ms commands and
height/thrust boundaries. This is not verification against a running JA+ server;
its unpublished settings, the stock spinning-saber random horizontal impulses,
and jaPRO's separate new/race jetpack path remain outside this implementation.
Equipping and fuel state still arrive from the server.

### When prediction steps aside

Three cases show the server's state instead of predicting, as JoF EternalJK's
`CG_PredictPlayerState` does
([prediction_policy.rs](../crates/sjk-game-jka/src/prediction_policy.rs), applied by
[interpolated_view.rs](../crates/sjk-viewer/src/interpolated_view.rs)):

- `cg_noPredict 1`: the view takes the server's position and keeps the mouse; `2` takes its
  angles too. Following another player and `g_synchronousClients` behave the same way.
- On a JA+ server, the victim of an added side or back kick (`forceDodgeAnim` 4 or 5, or
  the `BOTH_BACK_FALLING` and `BOTH_JUMP_BACKFLIP_ATCKEE` animations): the server runs
  knockdown rules the client's pmove lacks, so predicting shakes the camera. With
  `cg_noPredict 1` the server's angles are also taken while JA+ holds the view locked
  (kick, get-up, kiss, ledge).

A JA+ server that walks a player through others (amghost, the grace after unghosting
inside someone, a duel's walk-apart) sets `GHOST_KNOWN_FLAG`, bit 31 of
`fd.forcePowersKnown`; prediction then drops `CONTENTS_BODY` and `CONTENTS_PLAYERCLIP` from
its trace mask, so the client no longer stops where the server walks on.

`/fakenoclip` (`cg_fakeNoclip`, EternalJK's debugging aid) flies the local predictor in
noclip while the server is sent a still player: zero movement, `BUTTON_TALK` and the view
held from the moment it began, with the command time intact. The predicted state is not
reseeded from snapshots, every map area is drawn, and turning it off (or dying,
spectating or boarding a vehicle) snaps back to the server's position without error
smoothing. Turbo (attack held) sets the velocity along the aim directly.

A ghost replay ([Ghost replay](client.md#ghost-replay)) or the mimic
([Mimic](client.md#mimic)) sends its own commands in place of the keys' ones through the
same path, with fresh stamps on the 125 Hz grid; the command loop asks them before the
Force-wheel reliable command, which both suppress.

### Detached free camera

`/freecam` toggles detached camera flight; `/freecam on` and `/freecam off`
set it explicitly. Bind it in Settings > Key bindings > Other. It uses JoF
EJK's existing fake-noclip movement: movement keys fly locally, jump/crouch
move vertically, and attack/alt-attack accelerate flight. The server receives
stationary movement, frozen aim and the talk button, as with `/fakenoclip`.
Your body remains at its server entity, with no view weapon. It is hidden
from the main view while the camera is within 32 units of its eye, so the
initial view stays clear; it becomes visible after flying away. Turning
it off returns to the real player on the next snapshot and restores the chosen
first/third-person mode. It is available while alive and on foot; death,
spectating or entering a vehicle ends flight. It is temporary and mutually
exclusive with `/fakenoclip`. Generic actions, weapon changes and Force-wheel reliable commands are suppressed.
Disconnecting, reconnecting or changing maps clears it. `/freecam off` also
works from the menu. This does not change server noclip permissions.
Servers using `g_inactivity` can still drop a flying player: talk-only commands
do not reset the inactivity timer, as with fake noclip.

## User commands and move packets

The client makes user commands on a clock of their own, on a steady grid rather
than one per rendered frame, as JoF EternalJK's `cl_cmdratecap 1` does. In the
original game the frame rate is the movement (one command a frame, `1000 /
com_maxfps` whole milliseconds), and players lock 125, 142, 250 or 333 for how jumps
and strafes feel at 8, 7, 4 or 3 ms. The step
([command_rate.rs](../crates/sjk-client/src/command_rate.rs) `command_step`) is, in
order: on a `pmove_fixed` server its `pmove_msec`, since it moves everyone in those
steps and would drop a second command in one; the gameplay FPS
when the player set one (`gameplay_fps`, archived, 20 to 1000; Settings > Video >
Gameplay FPS, `help_gameplay_fps`, the step after an import,
[client.md](client.md#gameplay-fps)), `1000 / gameplay_fps` whole milliseconds as
stock rounds it (400 makes 2 ms steps, 500 a second); and on Auto (`gameplay_fps 0`,
the default, Sol's choice on 11/10/2026) the FPS cap's step, so `com_maxfps 333` moves
like 333 and AUTO's 125 like 125, as in the other clients (uncapped is 1 ms, the
shortest a server takes; the unfocused and minimized caps are not followed). Never
shorter than the step of a server family known to cap commands. Families are told
apart in one table, [known_servers.rs](../crates/sjk-mod/src/known_servers.rs)
(`KNOWN_SERVERS`: an id, a name, the game module, a serverinfo key and value), which
the client and the mods (the `jof` mod's `is_jof`) both read; the caps are a second
table keyed by family id, [server_caps.rs](../crates/sjk-viewer/src/server_caps.rs)
(`KNOWN`: the family, the step). Its one entry so far is JoF's servers (JA+ with
serverinfo `V` 2.5B0) at 8 ms: they are not `pmove_fixed` (they report 0),
yet faster commands made the game unplayable there (Sol, 11/10/2026), and JoF EJK
sends them 125 a second. The player lifts every listed cap with `gameplay_fps_nocap 1`
(archived, 0; Settings > Video > Ignore server caps), and the log names the cap when
a listed server is joined. A `pmove_fixed` server's step is not lifted: the server
itself moves everyone in it. On any other server, inputs
that do not go through at a step shorter than 8 ms open the gameplay FPS card by
themselves with 125 suggested
([input_health.rs](../crates/sjk-viewer/src/input_health.rs)): over two windows of 3
seconds in a row (30 predicted snapshots at least), 40% or more of the snapshots say
the server ran none of our commands (`ps.commandTime` stalled or not one we sent) or
corrected the prediction by more than 8 units. It opens once per step and server, and
says so in the console. Each command is
stamped on a boundary of the step in server time, one per slot
([command_rate.rs](../crates/sjk-client/src/command_rate.rs)); a changed step starts
at the next boundary after the last command sent.
Above 125 FPS a stock client makes a command per frame, and a `pmove_fixed` server,
which rounds a command's time up to its `pmove_msec` grid, drops the one that lands
in a slot it already ran (`msec < 1` in `ClientThink_real`), button presses
included; a grid on the server's own step never shares a slot. A frame slower than
a step makes a command for each slot it passed, back to 32 ms (four at 125, ten at
333); after a longer hitch the older slots are skipped instead of sent as a burst. Until the clock is anchored on the new
timeline (a map load), commands are stamped 0 and still made every 8 ms of real
time, so packets keep acknowledging the server. Every command is predicted when
it is made; between commands the frame presents a preview through its own input
([prediction_preview.rs](../crates/sjk-viewer/src/prediction_preview.rs)).

`rate` defaults to 90000 bytes a second (11/10/2026, Sol's choice): stock's 25000
leaves about 600 bytes a snapshot at a server's 40 a second, less than a busy
free-for-all's snapshot, so the server delayed snapshots and other players looked
choppy. Servers still cap it with `sv_maxRate`. Every profile saved stock's 25000, so a
saved 25000 is moved once to 90000 (marker `rateDefaultVersion`); one set after that
stays. The userinfo's `rate` and `snaps` are sent clamped to what a server accepts (a rate
of at least 1000, 1 to 1000 snapshots; OpenJK's `SV_UserinfoChanged` raises lower
values the same way), so `rate 0` or `snaps 0` typed in the console no longer stops
joins and every later userinfo update (until 11/10/2026 the client refused them).
The server still applies its own `sv_maxRate` and `sv_fps`.

Commands wait for a move packet. `cl_maxpackets` (SJK default 125, clamped 15 to
1000 like JoF EternalJK) sets how often one may leave: at most every
`1000 / cl_maxpackets` whole milliseconds, carrying every command made since the
previous packet, and with `cl_packetdup` (0 to 5) the commands of that many earlier
packets as well, at most 32 (`MAX_PACKET_USERCMDS`)
([command_history.rs](../crates/sjk-client/src/command_history.rs)). At 125 or
more each command leaves in a packet of its own; stock's 30 batches about four.
Losing window focus lets the next packet leave at once with the released keys.
Prediction keeps 512 unacknowledged commands (JoF EJK's `cl_commandsize` ceiling),
1.5 seconds at a gameplay FPS of 333. The flip kick's frame cvars stay EJK's frames
at 125 FPS, converted to commands at the step in use.

## Parity requirements

Movement includes integer-millisecond user-command quantization. Validate common
steps of 8, 7, 4 and 3 ms, corresponding to the customary 125, 142, 250 and 333 FPS
caps. Do not smooth away simulation quirks that players depend on. Presentation
interpolation is separate from authoritative movement and command timing.

Shared prediction/server code prevents duplicate implementations but can still
share the same mistake. Establish gameplay behavior against OpenJK multiplayer
`codemp`, including relevant animation, events and timing. Wire changes require
byte-level reference evidence; reasoning from matching Rust structures is insufficient.

Check ordinary native and legacy client joins on isolated servers for integration.
A handshake or successful movement run does not verify downloads, every reliable
command, map restarts, all combat, vehicles or every game type. The current evidence
and open validation work are recorded in [status.md](status.md).
