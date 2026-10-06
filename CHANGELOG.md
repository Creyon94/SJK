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

- Quick wheels: hold Q (general: third person, nameplates, HUD, screenshot, cosmetics, AFK, game menu, first setup) or R (weather: the map's own, drizzle, rain, storm, snow, ground fog, clouds, weather on/off), point the mouse at a choice and let go; Escape or letting go in the middle cancels (`+wheel general`, `+wheel weather`, Settings > Key bindings > Other) _(Sol)_
- Credits: Sol JK's own section, JKR's with Bishop, and every one of Creyon's merged pull requests; GitHub handles and the cards' links open in your browser when clicked _(Sol)_

## 2026.1006.1 (Alpha) | 06/10/2026

Weather, nameplates and the radial HUD arrive, SJK can now update itself, and your
movement reaches servers 125 times a second.

- Fixed a crash right after joining some games (FFA2, T2_Rogue on JA+ servers): a see-through player or afterimage on a map with material maps stopped the client _(Sol)_
- Settings regrouped (classic menus): Options now lists First setup (the old Quick setup), Graphics, Sound and Gameplay; Graphics holds Video, the renderer's Image, Lighting and Shadows and a new Weather group (also a Weather tab in the modern renderer settings); Gameplay holds Mouse, Game options, Interface, HUD, Scoreboard and Network _(Sol)_
- V cycles the nameplates: off, names only, bars on the player you aim at and your duel opponent, bars on everyone; the mode's name shows for a moment (`nameplates` command, Controls > Other) _(Sol)_
- Rain splashes are now 3D: a ring and a wet spot on the ground, a crown of water that rises and collapses, and drops thrown out on arcs, all placed in the world so they hold their shape from any angle; on water, widening ripples _(Sol)_
- The nameplate health estimate counts a JA+ grapple hook hit as exactly one point _(Sol)_
- The nameplate health estimate knows JA+'s chat protection: a player with the chat balloon up takes no damage unless swinging, kicking, punching or grabbing _(Sol)_
- Your HUD shows overheal and overshield like the nameplates: health and armour over 100 (125 at spawn, up to 199 armour) fill a thinner, deeper band inside their meter, in the radial, modern and classic HUDs _(Sol)_
- Weather, round two: rain splashes stay where they land instead of following you, and rain is a faint blue-grey instead of white; rain and snow leave a haze outdoors and the maps' fog becomes real ground fog that drifts with the wind (never indoors); 3D clouds drift over every map with sky, lit by its sun and darker in a storm; Settings > Renderer adds Weather quality (`r_weatherQuality`), Force weather (`r_weatherForce`: drizzle, rain, storm or snow on any map with sky), Ground fog (`r_weatherFog`) and Volumetric clouds (`r_clouds`) _(Sol)_
- Nameplate bars: shield on top, always shown (broken grey when empty), in your HUD's green, and health in its red; health and shield over the maximum (125 at spawn, up to 199 shield) show as a deeper second band; a yellow "?" stands over a bar the estimate cannot place yet; the estimate follows JA+ private duels (100 and 100 at the start, 100 and 25 for the winner) and the pain sounds servers play when they hide health _(Sol)_
- SJK identity needs nothing from you: your key is registered under the name you play with, and the SJK hub keeps the names you have worn (the Identity page shows them; the name field is gone, the bio stays optional); verified players get a gold badge after their name on nameplates, and `cg_nameplateSelf` shows your own nameplate in third person _(Sol)_
- Weather: maps with rain, snow, dust or blowing mist (T1_Rail, Hoth...) now show it; it stays under open sky, so it stops on roofs and the ground, never falls indoors and is cut at eaves and windows; rain splashes on the ground and ripples on water, a fainter far layer carries the storm into the distance, and the wind blows it; Settings > Renderer > Weather (`r_weather`) and its density (`r_weatherDensity`, 1 as the original game, 2 by default), and `r_we heavyrain` (or snow, fog, clear...) tries it on any map _(Sol)_
- The game goes silent while its window is alt-tabbed out or minimised, music included, and plays again when you come back; Settings > Sound > Mute in background (`snd_mute_losefocus`, on by default) turns it off _(Sol, after EternalJK)_
- Nameplates estimate the health and shield the server does not send for other players, from their spawns, pains, shield and saber hits, falls, pickups and Force heals (`cg_nameplatePredict`); every estimated bar, Force included, shows how unsure it is as a grey haze around its edge; the Force bar takes your HUD's Force colour; and the weapon a player holds shows left of the plate, a saber ringed in its stance's colour (`cg_nameplateWeapon`) _(Sol)_
- Your movement reaches the server 125 times a second at any frame rate, as with JoF EJK's `cl_cmdratecap`: a user command every 8 ms on the server's 8 ms grid instead of one every 25 ms, so quick taps are never merged or dropped (a `pmove_fixed` server loses commands from a client above 125 FPS) and you move in the same steps as a 125 FPS player; `cl_maxpackets` now sets how many packets a second carry them (default 125, 15 to 1000), `cl_packetdup` repeats earlier packets like the original game, and the flip kick counts one command per EJK frame again _(Sol, after JoF EJK)_
- New defaults for a new profile, an existing config keeps its values: the classic scoreboard (`cg_scoreboardStyle`) and the retail fonts (`ui_gameFont`) to go with the classic menus; the match timer and team overlay on (`cg_drawTimer`, `cg_drawTeamOverlay`); cut-off limbs shown on servers that enable dismemberment (`cg_dismember 2`); 120 snapshots a second asked of servers (`snaps`, the slider now goes to 125); and 30 FPS while the window has no focus (`com_maxfpsUnfocused`) _(Sol)_
- A radial HUD look, "SJK radial" (Settings > HUD > HUD look, or `cg_hudStyle radial`): health and armor as segmented arcs left of the crosshair, Force and ammunition right of it, low on the screen and sized with the window height, each on a rounded shadow that joins the pill carrying the numbers, which hug the bars; a saber's style is one full line in its colour (Fast blue, Medium yellow, Strong red, Dual green, Staff magenta); drawn by the engine, no HUD pack needed _(Sol, after TheRisqe's Radial HUD)_
- A console socket for external apps: `cl_consoleSocket <port>` streams the console, chat included, to a program on this computer and runs the commands it sends, as JoF EJK does, so Sol's Archive and chat tools work again; it needs `cl_consoleSocketPassword`, and `consolesocket` explains it _(Sol, after JoF EJK)_
- Nameplates show the holocron icons of the Force powers a player has on (lightning, grip, protect, absorb, speed and more) over the name when close (`cg_nameplateIcons`) _(Sol)_
- Protect and Absorb used together show one cyan shell on the body, as in single player (`cg_spProtAbsColor`, Settings > Game), instead of a green and a blue one _(Sol, after JoF EJK)_
- Nameplates like an online RPG over players: a small name from afar, and up close a framed plate with health, shield (teammates) and estimated Force bars under the name, in the classic font with its colours; shrink and fade with distance, ease out behind walls, anchored to the head, optional NPC plates; Settings > HUD has a row for each (`cg_nameplate*`); the plain `cg_drawPlayerNames` stays _(Sol)_
- SJK updates itself: it checks for a newer release at start (`cl_autoUpdate`), and main menu > Update (or the `update` command) downloads it, verifies it and installs it _(Sol)_
- A Changelog page in the main menu (and the `changelog` command) lists every release with its credits, in the classic+ look with the classic menus _(Sol)_
- The classic profile's Force page shows every power's holocron and level numbers again (Dark Rage and Team Energize were missing, and a hover hid a Lightning number) _(Sol)_
- A Credits page (main menu, the in-game SJK menu, or the `credits` command) shows the people who make Sol JK, animated, in retail colours with the classic menus _(Sol)_
- Classic menus: Controls and Setup are one Settings screen with KEY BINDINGS and OPTIONS tabs, every key binding in one list, a search field that finds any option or binding, and dropdowns for choices that change only when applied (Escape keeps the value); the main menu's SJK button gathers Changelog, Credits and Update _(Sol)_
- An SJK button left of About on the in-game bar (an SJK row in the modern game menu) opens SJK's own screens, starting with the changelog _(Sol)_
- A duel challenge from a player whose name has a symbol such as the multiplication sign no longer crashes the client, and the name shows that symbol instead of `?` _(Creyon)_
- Centre prints break only at a space, so a name with hidden codes stays on one row _(Creyon, after EternalJK)_
- The third-person camera follows fast moves, flips and rolls closely, damped per frame (`cg_cameraFPS`, default 125) instead of per 50 ms step _(Creyon, after EternalJK)_
- The crosshair is one of the game's crosshair pictures (`cg_drawCrosshair` 1 to 10, a 0-10 picker in Settings) instead of SJK's own "+" _(Creyon, after EternalJK)_
- Dismemberment (`cg_dismember`): cut-off limbs fly off with their cap and smoke; and bodies burn _(Creyon, after EternalJK)_
- The mouse wheel can be bound in Controls, so `flipkick` can go on it _(Creyon)_
- Small and far impact marks no longer show as black holes on walls, and scorch marks sit under explosions instead of over them _(Creyon)_
- Player animation fixes: runs and walks without a saber, a thrown saber's torso, the old Bryar's stance, with the unfixed attack animations predicted _(Creyon, after EternalJK)_
- Saber clashes no longer flash the whole screen _(Creyon)_
- Full servers (jaPRO) show every scoreboard row instead of repeating one name _(Creyon)_
- Symbols from the old Windows character set and hidden codes in names show as in the original game _(Creyon, after EternalJK)_
- Copying and pasting symbols on Windows keeps them instead of turning them into `?` _(Creyon)_
- JoF's HD effects load (a bare key no longer eats an effect block's closing brace), and images are tried as jpg, png then tga like the engine _(Creyon)_
- The first-person weapon follows `cg_fovViewmodel` _(Creyon, after EternalJK)_
- Zooming goes into first person, the HD scope mask is used, and camera shakes are measured from the view _(Creyon, after EternalJK)_
- The weapon selection row shows in every HUD style _(Creyon, after EternalJK)_
- Trip mine beams, placed charges lying flat, charge glow and the concussion beam are drawn _(Creyon, after EternalJK)_
- `fx_debug` and `cg_debugMissiles` diagnostics for effects and missiles _(Creyon)_
- With EternalJK installed, its character set (jaPRO's `charsgrid_med`) is used _(Creyon, after EternalJK)_
- `flipkick`: one press starts a run of jump taps for JA+ flip kicks (`cg_fkDuration`, `cg_fkFirstJumpDuration`, `cg_fkSecondJumpDelay`; bindable in Controls > Movement) _(Sol, after JoF EJK)_
- Player identity: SJK keeps an identity key and tells the SJK hub (sjk.dfox.app, `cl_hubUrl`) which game server and slot you play in, with your public key and in-game name, so other SJK players can see you; main menu > SJK > IDENTITY (or the in-game SJK menu, or the `identity` command) has a switch, your name and bio, and a button to copy your key id, and `cl_identity 0` stops it all _(Sol)_
- Player card: look at a player with a steady view for 1.5 seconds (`cg_playerCard`, `cg_playerCardDelay`) and a card beside them shows their model, sabers and duel record, and for SJK hub players the SJK emblem, hub name and a gold VERIFIED; the scoreboard marks hub players with the emblem too _(Sol)_

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
