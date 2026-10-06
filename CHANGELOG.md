# Changelog

Every Sol JK release, newest first. The client shows this page in game
(main menu > Changelog, or the `changelog` console command).

Credits close each line: who made the change, and "after <client>" when it
follows that client's behaviour. SJK is built on Bishop's JKR; see
[CREDITS.md](CREDITS.md). Many of Sol's changes were written with Claude
(Anthropic) as a coding assistant.

<!--
Format, read by the client (crates/sjk-viewer/src/changelog_data.rs, whose
tests check this file):
- "## <version> | <dd/mm/yyyy>" starts a release ("## Unreleased" for main
  after the last release, without a date);
- other lines up to the first item are the release's introduction;
- "- <change> _(<credit>)_" is one change; every change ends with its credit;
- ASCII only, as the menu font draws bytes.
Add each release's notes here when it is tagged (docs/sjk.md "Releases").
-->

## Unreleased

- SJK updates itself: it checks for a newer release at start (`cl_autoUpdate`), and main menu > Update (or the `update` command) downloads it, verifies it and installs it _(Sol)_
- A Changelog page in the main menu (and the `changelog` command) lists every release with its credits, in the classic+ look with the classic menus _(Sol)_
- A Credits page (main menu, the in-game SJK menu, or the `credits` command) shows the people who make Sol JK, animated, in retail colours with the classic menus _(Sol)_
- An SJK button left of About on the in-game bar (an SJK row in the modern game menu) opens SJK's own screens, starting with the changelog _(Sol)_
- A duel challenge from a player whose name has a symbol such as the multiplication sign no longer crashes the client, and the name shows that symbol instead of `?` _(Creyon)_
- `flipkick`: one press starts a run of jump taps for JA+ flip kicks (`cg_fkDuration`, `cg_fkFirstJumpDuration`, `cg_fkSecondJumpDelay`; bindable in Controls > Movement) _(Sol, after JoF EJK)_

## 2026.1005.1 (Alpha) | 05/10/2026

Releases are now numbered by date and a counter. The top of the screen shows
the version, date and commit of your build (`cg_drawVersion 0` hides it).

- Classic+ menus: setting panels with a description box, marks on changed values and Backspace for the default, grouped pages, pictures for key bindings and a classic renderer page _(Sol)_
- A HUD picker that previews every installed HUD, the game's own included; new configs start on the game's HUD _(Sol)_
- Player profile: every model's icon in the grid, model search and a live model preview _(Sol)_
- The classic Force page with saved templates and readable costs _(Sol, after JoF EJK)_
- Lightsaber creation shows the saber alone in 3D, with RGB colour sliders _(Sol, after JoF EJK)_
- The Force wheel, with Stasis, Repulse and Dash on JoF servers, and the weapon selection row _(Sol, after JoF EJK)_
- Hats and capes, open to everyone (`cosmetics` window; needs the JoF cosmetics pack) _(Sol, after JoF EJK)_
- The console key also closes the console _(Sol, after EternalJK)_
- The F3 command browser has the classic look with the classic console _(Sol)_
- Talk balloons no longer vanish in busy fights, and yours shows while you are alt-tabbed or minimised _(Sol, after EternalJK)_
- Rocket trails no longer disappear when many rockets fly at once _(Sol)_
- Alt codes: hold Alt and type a number on the numeric keypad in the console, chat and menus _(Sol)_
- Accented letters you type are readable by EternalJK and retail players (sent as Windows-1252) _(Sol)_
- A player whose model you don't have shows as Kyle instead of keeping their previous model _(Sol)_
- A negative score shows as negative, and melee no longer shows a stun baton in first person _(Sol)_
- On servers that add map pieces, the camera no longer goes through their walls, and server props use their scale _(Sol)_
- Shader remaps use JKR's implementation (JKR #127), with worldspawn remaps, effects that follow remaps and `clearRemaps` (JKR #128-#131) _(Bishop; Sol)_
- Multiplayer camera and vehicle behaviour, and burrowing sand creatures on SJK's own server (JKR #124-#126) _(Bishop)_
- Map items named with a leading slash, as on some JoF maps, load (JKR #133) _(Sol)_
- The modern main menu says SJK, and the client shows its copyright and licence notice at startup _(Sol)_

## 0.1.0-alpha.2 | 05/10/2026

The programs are now `sjk` and `sjk-server`, and settings live in
`GameData/SJK/` (alpha.1 settings are imported once; old `jkr_*` setting
names keep working).

- SJK's own name and emblem: program and window icons, and the emblem in the main menu _(Sol)_
- A classic console, now the default (`con_style modern` brings back the other one) _(Sol, after EternalJK)_
- Truer colours: text and menus are drawn the way retail draws them, with retail's `^` colour codes _(Sol)_
- Dynamic glow for glowing skins and sabers (`r_DynamicGlow`) _(Sol, after EternalJK)_
- Eye adaptation: brightness follows dark rooms and bright light (`r_autoExposure`) _(Sol)_
- Your own Force powers show (Drain, Lightning, Grip, Push), Drain's bolts have their retail shape, and Mind Trick hides the trickster _(Sol)_
- Many more custom player models load instead of falling back to Kyle _(Sol)_
- Shader remaps sent by servers and maps (`cg_remaps`) _(Sol)_
- With a generated material pack: reflections on metal, normal-mapped lamp light, mapped mirror floors and glowing emission maps _(Sol)_
- New defaults: sun at noon, bloom and dust in sunbeams _(Sol)_

## 0.1.0-alpha.1 | 04/10/2026

SJK's first public build: Bishop's JKR with Sol's changes, played and tested
by a small group.

- JKR itself: the Rust engine, the wgpu renderer and its lighting, the native client and dedicated server, protocol 26 networking, prediction, the Jedi Academy game rules, content loading, console, menus, HUD, server browser and demos _(Bishop)_
- Drop-in installs with portable settings, distributable ZIPs and actor animation fixes (JKR #103, #104, #112) _(Bishop)_
- The classic menu style as the default, the classic scoreboard and game-data HUDs, retail game fonts, text spacing and high-resolution scaling _(Sol)_
- JA+ servers: plugin identity, `serverconfig`, JA+ movement and saber rules, the grapple and duel pass-through _(Sol)_
- Reliable joining, old player models, material maps and the renderer settings page _(Sol)_
- Key handling: layout key names, dead keys, locked binds and capital key names _(Sol)_
- Sharp levelshots, saber trails, the third-person camera, Force Speed afterimages, death animations and dust motes _(Sol)_
- The `debug_panel` test list of each build's changes _(Sol)_
