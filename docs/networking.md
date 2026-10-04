# Networking and gameplay

JKR targets Jedi Academy multiplayer protocol 26. Keep the network representation
compatible with ordinary servers and clients; internal engine modernization does
not authorize a wire change.

## Ownership

- [jkr-protocol](../crates/jkr-protocol/src/lib.rs) owns message encoding/decoding,
  gamestates, snapshots and legacy field representations. It has no sockets.
- [jkr-network](../crates/jkr-network/src/lib.rs) owns transport, discovery and the
  legacy endpoints. The server endpoint calls a `LegacyGameHost` interface.
- [jkr-client](../crates/jkr-client/src/lib.rs) owns live client session state,
  reliable commands, snapshot history and client-side integration.
- [jkr-game-jka](../crates/jkr-game-jka/src/lib.rs) owns game behavior shared by
  prediction and server simulation.
- [jkr-dedicated's bridge](../crates/jkr-dedicated/src/bridge.rs) connects those
  rules to native server storage and legacy replication. `jkr-server` itself
  owns worlds/entities, not the JKA game loop or network format.

Client [compatibility profiles](../crates/jkr-client/src/compat_profile.rs)
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
RGB) and appends a deaths field to each `scores` row (15 fields instead of 14);
the client reads either row width from the argument count.
Player blade tints in a player configstring's `c3`/`c4` keys are read for every
profile; the JA+ 2.4 server module formats both keys too. The JA+ client
plugin's `serverconfig` and `pluginDisable` commands are client commands (see
[client.md](client.md#useful-console-commands)).

### Server-dialect movement rules

Prediction follows rules the server advertises in `CS_SERVERINFO`, read by
[pmove_rules.rs](../crates/jkr-game-jka/src/pmove_rules.rs): the roll fixes of
JA+ (`jp_cinfo`) and TaystJK/jaPRO, and `g_debugMelee`
([pmove_debug_melee.rs](../crates/jkr-game-jka/src/pmove_debug_melee.rs)). Stock
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
([duel_isolation.rs](../crates/jkr-client/src/duel_isolation.rs), applied where
[prediction_movers.rs](../crates/jkr-viewer/src/prediction_movers.rs) builds the
entity solids). Stock and unknown servers keep duellers solid.

## JA+ grapple hook

On JA+ servers the client predicts the grapple hook (`+button12`) with the rules
in [pmove_grapple.rs](../crates/jkr-game-jka/src/pmove_grapple.rs); other
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
[pmove_japlus.rs](../crates/jkr-game-jka/src/pmove_japlus.rs) reads from
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
