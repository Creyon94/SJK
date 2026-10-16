# Client mods

A client mod is a set of console commands and settings kept out of the core
client. Each one loads only on the servers it is made for, so a player who never
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
  tools", "JoF tools"). With the switch off, the mod does not load even on its servers.
  Profiles saved while the mods were off by default are switched on once
  (`mod_defaultVersion`). `mods` lists the mods: loaded, on (waiting for its
  servers) or off.
- **What stays in the core.** A mod adds commands a player types. What a server
  needs to be played correctly is not a mod: JA+ movement and saber prediction,
  `cjp_client`, `cp_pluginDisable` and its default, RGB sabers and cosmetics stay in
  the core client ([networking.md](networking.md)) and work with every mod off.
- **What a mod can do.** A mod sees the game only through
  [`Host`](../crates/sjk-mod/src/lib.rs): it can send a reliable server command,
  read the server's kind, address and serverinfo, the predicted player, the crosshair
  trace's end point and player, and the roster with where each player is drawn, and
  read or set a setting. It does not reach the renderer, prediction or the wire.
  Commands run on the main thread from the client command queue
  ([mods.rs](../crates/sjk-viewer/src/mods.rs),
  [mods_host.rs](../crates/sjk-viewer/src/mods_host.rs)).
- **Building without them.** Each mod is a crate behind a default feature of
  `sjk-viewer` (`mod-japlus`, `mod-jof`, which brings JoF content too); `cargo build
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

## JoF tools (`jof`)

[sjk-mod-jof](../crates/sjk-mod-jof/src/lib.rs). A JoF server is a JA+ server
whose serverinfo `V` is `2.5B0`, as JoF EternalJK's `CL_JoFTrustedServer` checks.
The mod completes the commands only JoF servers run: the `gun*` admin set, which
acts on the player you aim at (the server works out who), plus `jetpack`,
`refuseTele`, `immortal` and the extra emotes. `jof.commands` lists them and says
whether the current server is a JoF one.

JoF tools also bring [JoF content](jof-content.md) (`jof.downloads`, `jof.content*`):
JoF's maps, skins, hilts and packs downloaded from JoF's repository, as JoF's launcher
does, off until the player turns it on. Unlike the commands, it does not wait for a
JoF server: the card and its folder work from the menus, and `mod_jof` does not
switch it.

Not ported: JoF EJK's userinfo opt-ins (`jofejk`, `binoScan`, `cg_pickupConfirm`).
Each one asks the server for something the client must then show, and SJK shows none
of it yet.

## Verification

Unit tests cover the mod crates (teleport positions, player lookup, plugin bits,
login matching, JoF detection) and the client's switch, completion and queue
(`mods::tests`, `console_command::tests`). Nothing has been checked on a JA+
server yet: the teleports need a server where the tester is a JA+ admin.
