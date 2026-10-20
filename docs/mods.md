# Client mods

A client mod owns its commands, settings and optional community presentation,
kept out of the core client. Each one loads only on the servers it is made for, so a player who never
joins one sees none of its commands. The mods are built into the client; the line
they are written against is the [sjk-mod](../crates/sjk-mod/src/lib.rs) crate.

## Rules

- **Names.** Everything a mod adds starts with its id and a dot: `japlus.guntele`,
  `japlus.loginServer1`, `jof.commands` (Sol's choice, 11/10/2026). Typing the id
  and a dot, then Tab, lists the mod's commands. Server commands keep their real
  names (`amtele`, `amkick`, emotes), so binds made for other clients still work;
  a mod only completes and describes them.
- **Loading.** A mod's commands exist only on the servers it is made for: JA+
  tools on every JA+ server, JoF tools on JoF servers (a JoF server is a JA+ one, so
  both load there). Joining one loads the mod and leaving it unloads it, and Tab
  completes its commands only while it is loaded. A mod's command typed elsewhere
  says which servers it works on. A mod's own settings exist whether it is loaded
  or not, so saved values are kept.
- **The switch.** Each mod has an archived on/off setting, `mod_<id>`. It is on by
  default (Sol's choice, 11/10/2026). In Settings it is a row of Game options ("JA+
  tools", "jaPRO tools", "JoF tools"). With the switch off, the mod does not load even on its servers.
  Profiles saved while the mods were off by default are switched on once
  (`mod_defaultVersion`). `mods` lists the mods: loaded, on (waiting for its
  servers) or off.
- **What stays in the core.** A mod adds commands a player types. What a server
  needs to be played correctly is not a mod, with one exception: jaPRO's own
  movement styles and their ports belong to jaPRO tools (Sol, 20/10/2026), as each
  is jaPRO's alone. JA+ movement and saber prediction,
  `cjp_client`, `cp_pluginDisable` and its default, RGB sabers and cosmetics stay in
  the core client ([networking.md](networking.md)) and work with every mod off.
  JA+ flamethrower presentation also stays in the core; its detection and player
  override are documented under [flamethrower presentation](client.md#ja-flamethrower-presentation).
- **Community presentation.** The Destruction extension, the JoF dialogue window and
  JoF's firing-confirmation sounds belong to `sjk-mod-jof`, not the base compatibility
  profile. The mod owns their definitions, parsers and state; optional viewer
  bridges draw/play their output only while JoF tools are loaded. Base sessions
  forward unhandled reliable operands without interpreting community commands.
  These additions are compiled out of the viewer without `mod-jof`.
- **What a mod can do.** A mod sees the game only through
  [`Host`](../crates/sjk-mod/src/lib.rs): it can send a reliable server command,
  read the server's kind, address and serverinfo, the predicted player, the crosshair
  trace's end point and player, and the roster with where each player is drawn, and
  read or set a setting. It does not reach the renderer, prediction or the wire.
  Typed mod settings distinguish text, booleans and integer choices. A mod may
  declare scoped userinfo fields through the host; the existing adapter validates
  and encodes them, and removes them when their mod/setting no longer permits them.
  Commands run on the main thread from the client command queue
  ([mods.rs](../crates/sjk-viewer/src/mods.rs),
  [mods_host.rs](../crates/sjk-viewer/src/mods_host.rs)).
- **Passwords.** A mod declares the passwords its servers take
  ([`Secrets`](../crates/sjk-mod/src/lib.rs): commands whose arguments are one, parts of
  cvar names whose value is one) and its chat commands, whose typed lines stay out of
  crash reports and logs as received chat does. Unlike its commands, these do not
  wait for its servers: the client adds every built-in mod's table to the console's
  masking ([secret.rs](../crates/sjk-shell/src/secret.rs)) at startup, before any
  cvar or line, so they are masked offline, in binds and configs run at startup, on
  every server and with the mod off ([Passwords](#passwords)).
- **Building without them.** Each mod is a crate behind a default feature of
  `sjk-viewer` (`mod-japlus`, `mod-japro`, `mod-jof`, which brings JoF content too); `cargo build
  -p sjk-viewer --no-default-features` builds a client without any.

Mods are not loaded from files. A native library has no stable interface between
Rust builds and a downloaded one would run with the player's rights. If mods
made by other people are ever wanted, the plan is a sandboxed (WebAssembly) host
for the same `Host` interface. Nothing of that is built.

## JA+ tools (`japlus`)

[sjk-mod-japlus](../crates/sjk-mod-japlus/src/lib.rs). The teleports are JoF
EternalJK's (`codemp/cgame/cg_consolecmds.c`), renamed. The client works out
where to go, then sends JA+'s `amtele`; the server checks the admin rights, so
they do nothing for a player who is not a JA+ admin. Positions are sent with six
decimals, as EternalJK's `%f` does.

| Command | From EJK | What it does |
| --- | --- | --- |
| `japlus.guntele [distance] [yaw offset]` | `teleGun` | With no arguments: to the crosshair trace's end point, 24 units up, keeping your yaw. With a distance: that far along your view (pitch included) from your predicted origin, the yaw turned by the offset |
| `japlus.bring [id\|name\|gun] [distance] [yaw offset]` | `get` | Brings a player (the one under the crosshair without a name or with `gun`) in front of you, 100 units by default, facing you. Without a distance it lands level with you, 24 up |
| `japlus.goto [id\|name\|gun] [distance] [yaw offset]` | `goto` | Teleports you in front of a player as it is drawn, facing it. With only a name you land 24 above its sent position. A player not drawn for a second or more is reached with `amtele <id>` |
| `japlus.teleoffset x [y [z]]` | `amTeleOffset` | Moves you by whole units from your predicted origin |
| `japlus.mark`, `japlus.recall` | `PTelemark`, `PTele` | Keeps your position and yaw in whole units; recall sends `setviewpos`, which only a server that allows it runs |
| `japlus.autologin` | `autoLogin` | Sends `amlogin <password>` with the password saved for the current server in `japlus.loginServer1-3` / `japlus.loginPass1-3`. A server without a port is `:29070`, and a host name is looked up when the command runs. Like EJK it runs only when asked, not during the intermission, and the passwords are saved as plain text |
| `japlus.serverconfig` | `serverconfig` | Lists a JA+ server's `jp_cinfo` options (flip kick, roll fix mode, DFA variants, kata, ledge grab, alternate dimension); on jaPRO/TaystJK asks the server |
| `japlus.plugin [id]` | `pluginDisable` | Lists the fifteen JA+ plugin features as Allowed/Disallowed, or toggles one bit of `cp_pluginDisable` |
| `japlus.color <r> <g> <b>` | `amColor` | Sets `char_color_red/green/blue`, each 0 to 255; the core `amcolor` does the same on any server ([client.md](client.md)) |

A player is named by client number, by name (colours and case ignored) or by a
fragment of exactly one name. On a JA+ server the mod also completes JA+'s server
commands: the admin set, clan commands, duels and emotes, from EternalJK's `gcmds[]`.

Before the mod, `serverconfig` and `pluginDisable` were core commands. A bare
`serverconfig` is now sent to the server like any unknown command, which
jaPRO/TaystJK servers answer. TaystJK's completion lists it.

### Passwords

[secrets.rs](../crates/sjk-mod-japlus/src/secrets.rs) is JA+ tools' password table,
each entry with its source: `amlogin` (JA+ admin login, JoF's too), `login`,
`register` and `changepassword` (JoF EternalJK's accounts), `clanpass` (JA+ clan
chat) and `clanwhois` (JoF), whose arguments are masked, and `loginpass`, which makes
`japlus.loginPass1-3` secret cvars. Its chat commands, `say_team_mod`, `clansay`,
`amsay` and `ampsay`, keep a line typed with one out of the session log, crash
reports and log uploads ([identity.md](identity.md#crash-reports)). The console shows `amlogin ********` in its echo,
listings and binds, and keeps no password in the scrollback, `qconsole.log`, the
session log, crash reports or log uploads ([client.md](client.md#passwords-in-the-console)).
The table applies from startup whatever the server and `mod_japlus`, and no setting
turns an entry off; a build without `mod-japlus` does not mask these words.

## jaPRO tools (`japro`)

[sjk-mod-japro](../crates/sjk-mod-japro/src/lib.rs) loads on every server SJK sees as
jaPRO or its TaystJK lineage (the `TaystJk` profile). It owns jaPRO's movement styles
([movement_styles.rs](../crates/sjk-mod-japro/src/movement_styles.rs)).

A jaPRO server lets each player pick a style (`/move`) and sends it in
`ps.stats[13]` (`STAT_MOVEMENTSTYLE`, read as EternalJK's `PM_GetMovePhysics` does:
Swoop on a vehicle, the co-op styles folded onto their base). SJK's pmove is stock
JKA's, so only Siege, JKA, Swoop and Speed (the server's `ps.speed`, no pmove branch)
move as it predicts. CPM, QW, Q3, PJK, WSW, the rocket-jump styles, Jetpack, SP, Slick
and BotCPM have physics of their own: predicting them as JKA snapped the view back
every frame (hub reports #97/#98). While the mod is loaded, the viewer asks it each
frame ([interpolated_view.rs](../crates/sjk-viewer/src/interpolated_view.rs),
`japro_unpredicted`, no allocation or lock); a style it does not model is shown as the
server has it, as `cg_noPredict 1` does, keeping the mouse, and the log names the style
once.

| Name | What it does |
| --- | --- |
| `japro.styles` | Lists the styles and which ones SJK predicts |
| `japro.predictMoveStyles` | Off by default. On, every style is predicted as JKA, the earlier behaviour (Settings > Mods, "Predict all styles") |

With `mod_japro 0`, or a build without `mod-japro`, every style is predicted as JKA.

**Porting a style.** Its physics go in this crate beside the table, one module per
family (CPM first: Q3, Slick and BotCPM build on it; then QW and PJK, SP, WSW with its
dash and wall-jump timers, the rocket-jump styles and Jetpack), and the viewer's
prediction calls them through the same bridge. The reference is JoF EternalJK's
`codemp/game/bg_pmove.c` (every `moveStyle` branch). A style turns `predicted: true`
only once a recorded jaPRO demo predicts it without misses.

Other jaPRO-only behaviour still in the core and candidates for this mod: the race
mode reads of `ps.stats[11]` (chat, dynamic crosshair, enemy info, Ysalamiri tint)
and jaPRO's entry in the duel-hiding table.

## JoF tools (`jof`)

[sjk-mod-jof](../crates/sjk-mod-jof/src/lib.rs). A JoF server is a JA+ server
whose serverinfo `V` is `2.5B0`, as JoF EternalJK's `CL_JoFTrustedServer` checks.
The mod completes the commands only JoF servers run: the `gun*` admin set, which
acts on the player you aim at (the server works out who), `jetpack`,
`toggleFlame`, `refuseTele`, `immortal`, `sleep`, and the commands JoF's `amInfo`
lists under "JoF JA+ Commands" (`amHelp`, `amTips`, `amEmotes`, `amShop`,
`amCredits`). The `am*` player commands and emotes JoF EJK registered with them
(`amKnockMeDown`, `amDropSaber`, `amAtEase` to `amWon`) are JA+ ones: JA+ tools
complete them (Sol, 18/10/2026). `jof.commands` lists the JoF set and says
whether the current server is a JoF one.

JoF tools also bring [JoF content](jof-content.md) (`jof.downloads`, `jof.content*`):
JoF's maps, skins, hilts and packs downloaded from JoF's repository, as JoF's launcher
does, off until the player turns it on. Unlike the commands, it does not wait for a
JoF server: the card and its folder work from the menus, and `mod_jof` does not
switch it.

Not ported: `binoScan` and `cg_pickupConfirm`; these request additional
server data/protocol behavior which this mod does not yet implement.

### EJK server opt-in and firing confirmations

`jof.ejkOptIn` is an archived JoF mod opt-in, on by default. With it on, the loaded mod asks the
existing userinfo adapter to send `jofejk=1` to a recognized JoF server. It is
never a global `CVAR_USERINFO` value: other JA+, jaPRO, base, disconnected and
mod-off sessions omit the field. The setting and `mod_jof` changes mark userinfo
dirty; removal uses the normal next rate-limited userinfo update (up to one
second), while local handlers stop immediately. The mod's settings appear under
Mods in SJK Settings and Game options in classic.

**This is a server-extension opt-in, not a local sound mute.** A nonempty
`jofejk` value asks the server to treat the player as EJK-extension capable;
`jofejk=0` would still be nonempty, so opting out removes the key. Public EJK
sources explicitly document Stasis/Repulse firing confirmations behind it
([cg_xcvar.h](https://github.com/JediofFreedom/JoF_EJK/blob/beta/codemp/cgame/cg_xcvar.h),
[cg_servercmds.c](https://github.com/JediofFreedom/JoF_EJK/blob/beta/codemp/cgame/cg_servercmds.c)).
Those client sources do not enumerate the private JoF server's other checks of
`jofejk`. Sol (who wrote JoF EJK's side of it) confirmed on 18/10/2026 that the JoF
server gates all of its abilities on it, plus some quirks that are server-side
only; that is why it is on by default, as JoF EJK players have it. In
particular this switch does not advertise `binoScan` or `cg_pickupConfirm`:
those separately opt into binocular/party data and versioned pickup confirmation,
and are still not implemented here. The existing `cjp_client` identity stays SJK.

`jof.abilitySounds` (archived, on by default) controls only local Stasis/Repulse
confirmation audio; muting it does not withdraw the broader server opt-in.
The JoF mod owns the documented command/sound table and independent 250 ms gates
in [feedback.rs](../crates/sjk-mod-jof/src/feedback.rs). Only server `stasis` and
`repulse` confirmations trigger installed local channel-1 sounds; key presses
do not. Gates reset on unload/map changes and handle server-time rewinds.
Audio is loaded only when the JoF mod and both opt-ins permit playback, including
when enabled after joining. Missing installed audio stays silent. Turning the
feature off stops queued/current confirmations. EJK #117 has no separate Dash
confirmation cue. Nothing is bundled or downloaded, and base SJK owns none of
these command names, sounds or repeat rules.

### Destruction client controls

Destruction's contract, name, installed icon and capability policy live in
[sjk-mod-jof/destruction.rs](../crates/sjk-mod-jof/src/destruction.rs).
It is offered only on a recognized JoF server with `mod_jof 1`, nonzero
`g_forceDestruction` and the living player's known bit 21, outside spectator/follow.
`jof.destructionSupport` is an archived mod setting: Auto (`-1`) follows the flag,
Off (`0`) disables it, and Grant contract (`1`) accepts the bit without the flag.
The override cannot load the mod on another server or grant a power.

The Force bar, SJK selector, quick wheel and key-binding editor support the entry.
`force_destruction` keeps existing binds working; completion loads with the mod.
Where the entry is not active (another server, the mod or the setting off), the
command goes to the server unchanged, as it did before the mod owned it.
`+useforce` sends it once per press. Slot 21 is local-only and never becomes the
network Force selection. Loaded extensions use EJK #338's anchor order; base SJK
keeps its original wheel and never treats bit 21 as Destruction. Turning the mod
off or leaving clears that local selection. The optional icon is read/uploaded
only on first activation in the installed world; absent assets use Lightning.
Settings > Mods (SJK) and Game options (classic) expose the preference.

### Dialogue window and node audio

[sjk-mod-jof/dialogue.rs](../crates/sjk-mod-jof/src/dialogue.rs) owns EJK
#301/#307/#342's `jof_dialogue` parser, bounded node state and serial-safe replies.
The SJK window opens only on recognized JoF servers with `mod_jof 1` and
`jof.dialogue 1` (archived, on by default). It uses SJK's UI in every menu style;
Settings > Mods and classic Game options expose the preference.

The server supplies speaker/body text and up to twelve choices, five visible at
a time. Arrows or mouse wheel scroll without wrapping; hover selects, click or
Enter answers, 1-9 and 0 choose responses 1-10, and Escape/Leave conversation
cancels. Opening releases held gameplay keys. Replies are generated as
`dialogueresponse <serial> <choice>`, never executed as scripts; answer retries
are limited to one per second. Stale commands and duplicate show messages are
ignored. Replacing/closing the node clears unsent old replies. Disconnects, map
changes and disabling/unloading JoF tools reset the modal.

An optional `show` sound plays once from installed VFS data on
`ENTITYNUM_NONE`/`CHAN_VOICE_GLOBAL`, listener-relative. A replacement or close
stops the previous voice, including queued decodes. Missing audio leaves the
window usable. Paths are bounded and reject absolute/traversal forms. Nothing is
bundled or downloaded; ordinary silent show messages remain valid. Other servers,
mod-off sessions and builds without `mod-jof` do not interpret this extension,
open its window, send responses or load its node audio. Demos send no replies.
The remote server retains all quest conditions, stages, rewards and persistence;
this is no native SJK quest server system.

## Verification

Unit tests cover the mod crates (teleport positions, player lookup, plugin bits,
login matching, JoF detection) and the client's switch, completion and queue
(`mods::tests`, `console_command::tests`). Nothing has been checked on a JA+
server yet: the teleports need a server where the tester is a JA+ admin.
