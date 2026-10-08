# Changelog

Every Sol JK release, newest first. The client shows this page in game
(main menu > Changelog, or the `changelog` console command).

Credits close each line: who made the change, and "after <client>" when it
follows that client's behaviour; [CREDITS.md](CREDITS.md) has the full record.
Many of Sol's changes were written with Claude (Anthropic) as a coding
assistant.

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

- Quick wheel: every choice has its own icon (custom commands share one), and the wheel plays the game's menu sounds as you change page, move to another choice and run one; Settings > Quick wheel > Wheel sounds turns them off _(Sol)_
- Running water on walls stays on the wall: it no longer flickers or slides as you move or look around, and runs down in thinner streaks _(Sol)_
- Your drain now empties your victim's nameplate Force bar by exactly what it took (your own heal says how much), and what it shows of the server's drain speed and strength is used for everyone else's drain and lightning _(Sol)_

## 2026.1008.1 (Alpha) | 08/10/2026

The SJK UI becomes the default menu style, with pop-up cards, its credits and the
character pages in game; rain wets the world; medals, player reports and Illuminate
arrive; nameplates follow the Force powers; the camera stays locked behind you by
default, and Sol JK stands on its own.

- Rain wets the world: what it falls on darkens and takes a sheen of the sky, while floors under roofs and players stay dry. With weather quality high, water runs down walls and slopes; with ultra, puddles gather on flat ground with rain rings in them (Settings > Graphics, Weather quality) _(Sol)_
- SJK UI Credits: the emblem is the page's sun on the left, its sunburst and god rays kept, with the sections under it (Tab jumps between them, E opens every fold) and the people in a column on the right. Cards show their medals in every menu style; Creyon and Lumaya wear Early Contributor _(Sol)_
- SJK UI Report a bug is a pop-up card, as are a player report's words and the world note: the question, your text with a gold caret, the rules and the count, Cancel and Send. A report's card then waits for the SJK hub and says it was sent (with its number) or why not, with Edit to change your text and send it again _(Sol)_
- Nameplates predict much more: lightning, grip and rage now cost health (and rage halves the hits a player takes), protect turns hits into Force spent, team heal and energize give their exact share, drainers heal by what they drain, and the Force bar charges force jumps by how fast they climb, katas, cartwheels, lunges and other saber specials, wall jumps, heals, team powers and the start of a grip. Absorb gives Force back, a knocked-away saber no longer counts as a throw, and a special refused for want of Force shows the bar under half _(Sol)_
- Shot controls is now Camera control (the game menu's entry, or F8), and with the SJK UI it has its look: a column down the right edge over a fade, the rest of the scene left clear with viewfinder corners and thirds marks, Camera and Sun as tabs, switches for live preview and the HUD, and the keys of what you are on. Up and Down stop once on each slider _(Sol)_
- Q opens the quick wheel on the page you used last (it always opened on General); R still opens Weather _(Sol)_
- Nameplates count Force drain: the Force bar of whoever you (or anyone) drain drops shot by shot and stops refilling for a moment, and drainers pay for their drain. The verified tick now sits level with the name _(Sol)_
- The quick wheel has pages: hold Q, then scroll or left and right click to go from General to Weather and any page you add. Settings > Quick wheel (Interface > Quick wheel pages with the classic and modern menus) adds, names, orders and removes pages and picks each page's choices from SJK's actions or your own console commands. The wheel now has the SJK UI's look _(Sol)_
- SJK UI First setup is a pop-up over the map: its settings scroll inside the card, and "Don't show at start" is a tick always in view at its foot, beside All settings and Done _(Sol)_
- SJK UI Update is a pop-up card too: the version, what the check found, the download's progress, and Release notes, Check again, Close and Install along its foot _(Sol)_
- Sol JK stands on its own: the old jkr_ names of settings and commands, the JKR_ environment variables and the one-time import of GameData/jkr are gone. Downloaded files now go to SJK's own folder (%LOCALAPPDATA%\SJK\downloads), the dedicated server saves sjk_server.cfg and the material-map generator writes zzz_sjk_materials.pk3 _(Sol)_
- SJK UI in game: the in-game menu's Character opens the SJK UI's character, saber and Force pages too, your model standing beside them in a live preview holding your sabers, over the dimmed match _(Sol)_
- Medals: the SJK team can give players medals (Early Tester, Early Contributor, Bug Hunter, JoF Clan). They grant nothing; they show as ribbon bars after the SJK emblem on the scoreboard, as medallions on the player card (X) and on the SJK UI's Players page, and in full with the date and the team's note on your Identity page. A medal new to you shows once in a pop-up on the main menu, or when you open the game menu in a match _(Sol)_
- Illuminate, a free power everyone has: the last entry of the Force wheel (or the force_illuminate bind) turns on a holocron that floats by your shoulder, slowly turning, and lights the way in dark maps. Only you see it; Settings > Game > Illuminate holocron takes it off the wheel _(Sol)_
- A new profile starts at mouse sensitivity 5 again; the move to stock sensitivity units turned it into 13.022 _(Sol)_
- SJK UI Character: the saber style is three buttons and the hilts a list you can scroll and click (two side by side for dual sabers); the Force page groups the powers as classic+ does (Neutral, your side, Lightsaber) with classic+'s round level marks numbered with their cost, each group in its colour (silver, blue, red, green), bought levels glowing, a hovered level charging up with a spark and its price previewed on the points bar, a ring sent out when you buy one, and a box at the bottom right shows the power under the mouse in big with what it does _(Sol)_
- Saber hilt lists come in the same order as in JoF EJK (the game's load order, not alphabetical), in every menu style _(Sol, after JoF EJK)_
- The SJK UI is now the default menu style. Profiles still on the old default (classic) switch to it once when you update; pick Classic or Modern again and it stays _(Sol)_
- First setup offers the menu style as its first row: SJK, Classic or Modern, and First setup stays open in the style you pick _(Sol)_
- The quicksetup console command is now only firstsetup, so q completes to quit alone _(Sol)_
- The camera stays locked behind you by default, as in JoF EJK (camera style EJK). Profiles still on the old default (SJK, the camera that trails you a little) switch to it once when you update; pick SJK again and it stays _(Sol, after JoF EJK)_
- First setup opens with Styles: the menu style, then the camera style _(Sol)_
- Grabbing a wall before a wall jump, your body now stays facing the wall while the camera looks around (JA+ servers keep the camera free) _(Sol)_
- Report a player: the game menu lists everyone on the server, a small scoreboard with scores, pings and who is a verified SJK player (SJK UI: Players; classic and modern: SJK > Report a player). Choose a player, a reason and a few words, and the report goes to the SJK team with who, where and when. Only verified SJK players can send reports; the hub limits how often anyone reports and how often one player can be reported _(Sol)_
- The console has the SJK look with the SJK UI (con_style auto, or con_style sjk with any menus): a full-width panel with a header, an input band and key hints; the rest of a command shows faintly as you type, and a scroll bar. Commands and cvars (F3) gets the SJK look too, its text selectable with the mouse; Ctrl+C copies the selection or the entry _(Sol)_
- Video screens play: shaders that show a video (videoMap), such as the screens some servers put on consoles, play it looping instead of a magenta checker _(Sol)_
- Ceiling lamps and light strips that only looked painted glow: declared lights without a glow layer take emission maps (regenerate the material pack to see them) _(Sol)_
- Footsteps on sand, snow and grass the map does not mark as such sound like sand, snow and grass, as on Siege Desert's dunes _(Sol)_
- World notes name the shader a server's remap shows on the surface, not only the map's own _(Sol)_

## 2026.1007.3 (Alpha) | 07/10/2026

Lumaya's free camera, peek and first-person saber body; the SJK UI gains its server
browser, scoreboard, loading screen and in-game menu; ground fog keeps its height,
parallax returns with a depth slider, and the chat font is evenly spaced.

- Ground fog keeps its height: distant fog no longer jumps up and down as you step between roofs and streets or jump. It lies on the map's own floors, and past the map's walls as one level bank _(Sol)_
- SJK UI loading screen: joining a server keeps the menu's map behind the server's name and a gold line that fills step by step, then fades to the destination's picture with the map's name, the server, its rules and message of the day and a gold load bar along the bottom; a failed join says why in the same place _(Sol)_
- SJK UI scoreboard: with the SJK UI's menus, Tab shows SJK's own scoreboard, its columns floating over the darkened game: the map, mode, time left and your place in gold on top, the teams side by side under their scores, duelists as facing cards with their records, pings as signal bars. Settings > Scoreboard > Scoreboard style offers Auto (the new default: SJK's with the SJK UI, else classic), SJK, Classic and Modern; profiles move to Auto once, and a style you pick after that stays _(Sol)_
- Parallax is back at a tenth of its old depth, with a Parallax depth slider (and a Parallax mapping switch) in Settings > Graphics > Image: 0 flat, 1 the full depth of the maps; the slider shows its change at once _(Sol)_
- First person with a saber shows your arms and body holding it, as the original game does, instead of a hilt floating alone; the head stays hidden, and dying or following someone goes back to the old view _(Lumaya, after JoF EJK)_
- Peek: `/peek <id or name> [seconds]` (or `/peek` on the player under your crosshair) watches another player for a few seconds from behind, as JoF EJK's peek does, with the camera kept out of walls; `/peek off` returns _(Lumaya, after JoF EJK)_
- Free camera: `/freecam` (bindable under Key bindings > Other) flies the camera away from your body as JoF EJK's fake noclip does, while you stand still on the server with the chat balloon; actions are held back while flying, and death, spectating, a vehicle, a disconnect or a map change end it _(Lumaya, after JoF EJK)_
- Even letter spacing in the chat font: most letters sat against the left of their space while C, G, O and a few others sat against the right, leaving holes inside words. The menu and classic HUD fonts had the same fault, less visibly _(Sol)_
- Bug reports and world notes carry the name you play under, so the SJK team knows who sent them; the hub adds it to your identity's name history like the names it sees on servers _(Sol)_
- SJK UI Servers: the server browser over the map. Favourites and the filters for empty, full and locked servers and the game type sit on the left; the list sorts by any column and shows each server's name in its colours, its mod and signal bars; the chosen server shows its map's picture, its numbers and who is playing. The keyboard reaches everything, and the password and address prompts match the rest. Searching finds servers by their name without its colour codes _(Sol)_
- SJK UI: coloured names read on its dark ground: black shows as grey, and dark blue, red, green and magenta are lighter with the same hue _(Sol)_
- SJK UI in-game menu: Escape in a match keeps the match on screen with the menu on an arc down its left side (Resume, Team, Vote, Character, Settings, Servers, Shot controls, Sol JK, Leave) and a card on the right with the server, map, mode and limits, your score and place, the time left and who is playing. Settings and Servers open in the SJK UI over the match, and Escape always goes back to the entry you came from _(Sol)_

## 2026.1007.2 (Alpha) | 07/10/2026

The SJK UI arrives (Settings > Interface > Menu style > SJK) with its main page, Settings,
Character and update pages over a camera tour of duel6; world notes reach the SJK team;
vector fonts everywhere, modern settings, a camera style choice and working screenshots.

- World notes reach the SJK team: aim at a wall, floor or door, press the inspect key (X) twice and write what is wrong. With the identity on, the note goes to the SJK hub with where you stood, what you aimed at and a small screenshot, and a centre print says it arrived. Your copy stays on your PC in SJK/notes.txt as before; cl_identity 0 sends nothing _(Sol)_
- The SJK UI begins (Settings > Interface > Menu style > SJK): SJK's own menus over the live map, in the site's Rajdhani and Exo 2 type. Its main page: the emblem in a turning holo ring with the menu on an arc round it, and the servers you joined last on the right, each joined with one click (the JoF server until you have joined any). The screens it has no look for yet open in their classic one _(Sol)_
- SJK UI Settings: every category down a lit rail on the left, switches, sliders and side-by-side choices under sub-headings, the setting you are on explained on the right (what it does, its default and range, when it applies), and the search always at the top. The key bindings are one of its categories, each action with its two keys as caps; the one waiting for your key turns gold _(Sol)_
- SJK UI Character: your character stands on the map itself, on the path below the tower in the map's own light, and every change shows on them at once. Your name is the title; the model grid, sabers (colour chips and sliders, the saber thrown out to float before you) and Force powers sit in a column beside them _(Sol)_
- SJK UI What's new, Update and Identity: the releases down a lit rail with the chosen one's notes beside them, and the update and identity pages in the same look _(Sol)_
- SJK UI text sits centred in its buttons, key caps, fields and rows (it was drawn a few pixels high, twice that at 4K) _(Sol)_
- Behind the SJK UI the camera tours mp/duel6: ten slow shots of the map (the tower from the gardens and from its foot, the wings' rooms, the temple stair) in a new order every start, with a soft fade between them _(Sol)_
- Screenshots work again: `screenshot`, the quick wheel's screenshot and world notes' pictures no longer fail with "Screenshot readback failed", and a second screenshot no longer freezes the game _(Sol)_
- Settings look modern (classic menus): on/off settings are switches you click, numbers are slim sliders with their value in a box, short choices (menu style, contrast, console style) sit side by side, longer lists open from a field, labels read as words instead of all capitals, long groups have sub-headings, and a changed setting shows a gold dot and a reset arrow _(Sol)_
- A camera style setting (Settings > Gameplay > Game options and First setup, `cg_cameraStyle`): SJK, the default, keeps the third-person camera easing after you as before; EJK locks it behind you with no lag when you move or turn, as JoF EJK plays with its strafe helper on _(Sol, after JoF EJK)_
- The game's own fonts are vector fonts: menus, chat and the classic HUD keep their look and layout but stay sharp at any size, with no bitmap left; the menu and HUD fonts are traced from the JoF HD fonts, the chat font is OCR-A set to the game's spacing with its accents, and the WSI logo in names is sharp too _(Sol, fonts traced from the JoF HD Fonts & Icons pack, OCR-A by John Sauter)_

## 2026.1007.1 (Alpha) | 07/10/2026

Report bugs from inside the game, bring your config over from another client, quick
wheels on Q and R, saber blades that stop at walls, and a credits page with everyone's
work.

- Report a bug: a button at the bottom of the screen while the game menu is open (and Escape > SJK > Report a bug) opens a panel where you describe the problem and send it to the SJK hub, signed with your SJK identity, with the map, build and server; with the classic menus the panel and the button take the classic look, a gold button on a red band and an in-game pop-up _(Sol)_
- The credits page lists everything: each person has a panel with their name in large letters, their role and counts, and folds that open their highlights and all their work, every feature and pull request by day, each unfolding into its commits, with pull requests and commits opening on GitHub; golden god rays turn down from the top and a sunburst turns behind the emblem, as on the website _(Sol)_
- Settings groups have icons (classic menus): every group down the left of Key bindings, Options, Graphics and Gameplay shows a grey metal disc in the quick wheels' style, lit while the group is open or pointed at _(Sol)_
- The console draws with a sharp vector font, JetBrains Mono, instead of the old blocky bitmap font, on the same grid; so do the FPS counter, vote text and kill feed _(Sol)_
- Saber blades stop at walls: the glow and core end where the blade meets a wall instead of shining through it, and reach full length again as it leaves, also on hilts that leave no wall marks _(Lumaya)_
- Configs from other clients load: exec runs a .cfg whose name or binds use accented letters (it said "not valid UTF-8"), and a jampconfig.cfg copied over SJK's config.cfg loads with its unbindall and unknown keys instead of losing all your settings _(Sol)_
- First setup opens at every start until you tick "Don't show at start", and you can drop a .cfg from another client (jampconfig.cfg, EternalJK, JA+) on the window to import your name, model, field of view and key bindings, ticking the ones you want _(Sol)_
- The key under Escape opens the console on every keyboard layout, including Hungarian where it types 0; where it types ^ (German), Shift+^ opens it _(Sol, after EternalJK)_
- Melee kicks play smoothly: a game joined from the menu now predicts kicks and saber attacks instead of starting them a round trip late, and on JA+ servers JA+'s own spin, back and backflip kicks hold you still instead of pulling you back with every snapshot _(Creyon, after JoF EternalJK)_
- The quick wheels show icons: grey metal discs in the style of the game's Force icons, the one you point at grows in an accent ring with its name in the middle _(Sol)_
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
