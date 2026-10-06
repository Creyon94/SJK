# Networking and gameplay

JKR targets Jedi Academy multiplayer protocol 26. Keep the network representation
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
feature of those servers is reproduced by JKR's dedicated server.

On JA+ and TaystJK/jaPRO servers the client identifies as a client-plugin user
through extra userinfo keys. JA+ gets the JA+ 1.4B4 plugin's `cjp_client
1.4B4` and `cp_clanPwd none`. Both plugin profiles also get the player's
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
plugin's `serverconfig` and `pluginDisable` commands are client commands (see
[client.md](client.md#useful-console-commands)).

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
alternate attack standing still. JKR's server does not simulate `g_debugMelee`;
its default there is 0, which keeps prediction on the stock behavior.

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
servers, JKR's own included, get no hook movement. The JA+ game fires the hook,
stores its anchor in `lastHitLoc` and flags the pulled player with `PMF_GRAPPLE`
(pm_flags bit 15). Each move then aims 16 units short of the anchor along the
view and replaces the velocity with a pull of 800 units/s (10 units/s per unit
inside 100 units), EternalJK's arithmetic and the JA+ 2.4 B7 module's, followed
by an air move whatever the ground or water below. A client-plugin user
(#108) who lets go of the key stays on the rope: the game clears the flag and
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
Stock and other servers, JKR's own included, keep the stock rules.

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

Not predicted: the options EternalJK never reads outside its `serverconfig`
listing (single-player attacks, new DFA, model scale, kata, auto replier, ledge
grab, alternate dimension, macro scan), the Jedi Outcast red DFA
(`jp_jk2RedDFA`, off by default), a changed `jp_gripSpeedScale` (not published)
and the animation holds for JA+'s extra GLA animations.

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
