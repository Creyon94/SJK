# Client

Build instructions are in [development.md](development.md). JKR needs an installed
Jedi Academy `GameData` directory with the retail `base/assets*.pk3` files.

## Launch

In SJK the client program is `sjk` and the dedicated server `sjk-server`
(`.exe` on Windows). This page, shared with JKR, uses JKR's names `sjk-viewer`
and `sjk-dedicated`; the commands are otherwise the same. SJK reads
`JKA_GAME_DATA` and `JKA_DEDICATED` before JKR's `JKR_GAME_DATA` and
`JKR_DEDICATED`; developer and diagnostic variables (`JKR_TRACE_*`, `JKR_LAMP_*`,
`JKR_GPU_*` and the like) keep JKR's names. On Windows both programs carry SJK's
icon and call themselves "Sol JK" and "Sol JK dedicated server" in their version
information; the client also sets the icon on its window (title bar and taskbar
on Windows, the window icon on X11; Wayland has none). See
[assets/branding](../assets/branding/README.md).

Put `sjk-viewer` (`sjk-viewer.exe` on Windows) inside the installed game's
`GameData` folder, beside `base/`, then launch it to open the main menu. A shortcut
can use any working directory. No path settings are required. Keep
`sjk-dedicated` (`sjk-dedicated.exe` on Windows) beside the client for Create game
and local `devmap`.

From that folder:

```sh
./sjk-viewer
```

Join a server directly:

```sh
./sjk-viewer --connect 127.0.0.1:29071
```

Without an explicit positional GameData argument, discovery checks these locations
in order and uses the first containing `base/assets0.pk3` and `base/assets3.pk3`:

1. `JKA_GAME_DATA` (SJK; JKR's `JKR_GAME_DATA` is read when it is unset), if set
   and nonempty.
2. The executable's directory, then its `GameData` subdirectory.
3. The saved `fs_gameData` setting.
4. The working directory, its `GameData` subdirectory, then its
   `Star Wars Jedi Knight - Jedi Academy/GameData` subdirectory.
5. The existing Linux Steam and `~/Games/Jedi Academy/GameData` locations.

Thus a drop-in installation wins over a saved location from another installation,
while an explicit environment or positional path still overrides it. Invalid
discovery candidates are skipped; an invalid explicit positional path is an error.
Discovery does not change the working directory or move any game data.

For a binary kept separately, `JKA_GAME_DATA=/path/to/GameData ./sjk` opens
the main menu. Positional launch also accepts a map path and optional player-model
directory (`sjk-viewer /path/to/GameData maps/mp/ffa3.bsp`); it remains a direct
world/viewer launch, whereas no arguments opens the main menu. Demo playback
retains its explicit GameData argument.
See [launch.rs](../crates/sjk-viewer/src/launch.rs) and
[app_launch.rs](../crates/sjk-viewer/src/app_launch.rs).

The client has a server browser, player and settings screens, in-game menus,
console, HUD, screenshots, demo recording/playback and a Create game flow.
Presence here describes implemented surfaces; validation limits are in
[status.md](status.md).

Create game starts a child `sjk-dedicated`, normally found beside the client.
Set `JKA_DEDICATED` (or JKR's `JKR_DEDICATED`) to its executable path if installed
elsewhere. The child
lifetime is managed by the client and defaults to local access; see
[local_server.rs](../crates/sjk-viewer/src/local_server.rs).

Map previews (`levelshots/<map>.jpg`, `.tga` or `.png`) keep the resolution they
ship in, so an HD levelshot pack stays sharp in a large preview and on the
classic loading screen. They are decoded with their mip chain on a worker thread
([levelshot.rs](../crates/sjk-viewer/src/menu/levelshot.rs)) and drawn from
their own texture ([ui_renderer/levelshot.rs](../crates/sjk-viewer/src/ui_renderer/levelshot.rs)),
stretched to the 4:3 frame as the stock UI draws them. Images longer than 4096
pixels are reduced to that; decoded previews are cached up to 64 MiB.

## Menu style

`ui_menuStyle` (Settings, GAME tab, "Menu style") picks the layout of the main
and in-game menus: `modern` (default) or `classic`, which is close to the retail
multiplayer menus in layout and flow without porting their `.menu` scripts. The
retail 640x480 layout is fitted to the window height and centred.

SJK differs from JKR's documented behavior here: `classic` is the default, and
only `modern` (or `0`) selects the modern layout; an unknown value falls back to
`classic`.

The classic main menu has the retail pages, entries and order:

- Main: Play, Profile, Controls and Setup in two columns, Exit below. Exit and
  Escape ask before quitting.
- Play: Solo Game, Join Server, Create Server, Play Demo and Rules. Solo Game
  and Create Server both open Create game, which hosts a local match with bots.
- Controls and Setup are option panels, as retail's `controls.menu` and
  `setup.menu` are (described below).
- Every sub-page repeats the retail navigation row (Play, Profile, Controls,
  Setup) and has Back and Exit.
- Profile opens the retail profile pages (`player`, `player2`, `saber`), which
  edit the same drafts as the modern Player screen and write them at once:
  - Profile: name, team colour and the head grid (six 64-unit cells per row),
    Custom to character creation, APPLY on to lightsaber creation, Exit. SJK
    sets Custom beside a Force button (retail's in-game `configforce` art) to
    the right of the grid, with the Force profile at a glance under them
    (mastery, side, points left and the holocrons of the powers with a level,
    a pip per level), then JoF EJK's Cosmetics button and what is worn.
  - Character creation: species, skin tint swatches, the Head, Torso and Legs
    lists, Back and APPLY. Entering it from an ordinary character puts on the
    first species, as retail's Custom did.
  - Lightsaber creation: saber type, the hilt list (two for Dual Sabers), the
    six blade colour swatches (two rows for Dual), Apply, and Apply back to the
    main menu.
  - Force (SJK): retail's in-game `ingame_playerforce` window, here on both
    frames, editing the same draft as the modern Force tab. It keeps retail's
    frame, title band, gold mastery line, blue and red side bars and level
    stars, each numbered with what that level costs (`UI_DrawForceStars`:
    `forcestarN` once bought, `forcecircleN` before), and retail's templates
    down the left: the side's `forcecfg/light` or `forcecfg/dark` `.fcf`
    files from the game data (retail's Blademaster, Knight and others, and
    any pack's, under their own capitalised names) and the player's own,
    which come first and are tagged. A click or Left/Right loads a template
    into the draft, legalized at the draft's rank as `UI_ForceConfigHandle`
    does, and the row stays filled until the draft is edited; Name and Save
    Template write the draft to `forcecfg/<side>/<name>.fcf` in the client's
    user folder (beside `config.cfg`). The window is as wide as the in-game
    profile. SJK lays the powers out in two columns, the neutral powers over the saber skills and the chosen
    side's five beside them, each with its holocron (`gfx/mp/f_icon_*`), and
    adds Light and Dark cards with the side emblems, a points meter that shows
    a hovered star's price (red when the points left cannot pay it) and a
    panel for the hovered or focused power: its holocron, level, the next
    level's cost or why it cannot be bought (other side, team games only,
    Saber Attack 1 needed), every level's cost and what it has taken. A click
    on a power raises it a level and the right button lowers it, as retail's
    did; a click on a star sets that level (on the power's top star, one
    below). Left and Right step the focused power. Reset, Discard and Apply
    Powers act on the draft; the points line says "not applied" until it is
    written, Escape keeps the draft until the screen closes, and the profile
    page's APPLY (or Apply on lightsaber creation) writes a pending draft too.
    The main menu shows the window over the backdrop with the navigation row
    and Back.
  - Cosmetics (SJK, JoF EJK's `ingame_cosmetics`): the installed hats and
    capes side by side and the live model wearing them in a column on the
    right, the worn row filled.
    A click wears a piece and a second takes it off; the wheel and Left/Right
    move through a list and Enter wears. Show Cosmetics cycles `cg_cosmetics`
    (On, Only Me, Off), Remove All takes both off and Apply returns to the
    profile. An empty list says where cosmetics come from; when JoF's
    catalogue has pieces that are not installed the list ends with its note.
    See [Hats and capes](#hats-and-capes).

  Character creation shows the live model where retail drew its
  `ITEM_TYPE_MODEL`: the player's model, wearing their hat and cape, walking
  in place (`BOTH_WALK1`) while the view turns round it at retail's
  `model_rotation 50` (20 degrees a second), drawn without sabers as retail's
  was; the cosmetics window shows it standing (`BOTH_STAND1`) in a column
  beside the lists, where JoF EJK drew its model. The model's portrait shows
  until the first preview frame is drawn. Lightsaber creation shows it in
  the band under its boxes, where retail spun the bare hilt model, holding
  the draft's sabers lit in their style's stance, so hilts and blade colours
  are seen as they will be carried; a drawn hilt and blade stand in until
  then. See [model preview](rendering.md#classic-model-preview). The part
  lists show each variant's icon (`models/players/<species>/icon_<part>`,
  `.jpg`, `.png` or `.tga`) as retail did, its name when there is none, and
  the swatches are the species' tint base (`gfx/menus/players/<species>/`
  `*tintbase`) multiplied by each `playerchoice.txt` colour, as the swatch
  shaders draw them (a flat colour without the image). The species being
  edited is loaded into 64 cells of the UI icon atlas of its own (four rows,
  about 4 MiB), again when the species changes. Escape returns to the
  profile page, then to the menu. A click
  on a cell of the head grid, the part, tint and hilt lists or the blade
  swatches picks that cell; each list registers its own pointer region before
  its cells, since the menu canvas gives the pointer to the region registered
  last.

Join Server opens retail's join-server screen (`ui/jamp/joinserver.menu`) on
the same browser as the modern style, so the list, favourites, filters and
sorting carry over between styles. Labels and buttons are in capitals:

- GET NEW LIST and REFRESH LIST both fetch the master list again, as retail's
  `RefreshServers` behind both did.
- The selectors box: SOURCE (INTERNET or FAVORITES; Tab also switches),
  FILTER (retail's mod filter row is JKR's text filter over names and maps;
  `/` or a click starts typing, Escape ends), TYPE (game-type filter), and the
  VIEW EMPTY, VIEW FULL and VIEW LOCKED toggles (the archived
  `ui_browserShow*` cvars; VIEW LOCKED stands where retail's data rate was).
- The list: SERVER NAME, MAP NAME, PLYRS, TYPE and PING columns over retail's
  row bands and column frames, ten 26-unit rows, the sorted column filled and
  its header white with a ^ or v for the direction. Clicking a header sorts by
  it, again reverses it. TYPE adds JA+, JAPRO or MOD where JKR detects the
  server's mod, and `*P` for a locked server; favourites carry a gold `*`.
  The wheel and the scrollbar along the right edge scroll the list.
- The secondary row: CONNECT IP (where retail had NEW FAVORITE; JKR's direct
  connect), ADD FAVORITE or DEL. FAVORITE for the selected server, and SERVER
  INFO, a pop-up of the server's published settings and players (Escape or a
  click closes it). PASSWORD and FIND PLAYER are dimmed: JKR asks for the
  password when a locked server is joined, in a retail-style prompt, and has
  no player search yet.
- BACK returns to the Play page, EXIT to the quit page (not shown when the
  browser was opened from the game menu), JOIN joins the selected server (a
  double-click on a row too).

The status line under the list shows the fetch progress where retail showed
the refresh time, and the description line shows the hovered item's
description. The screen is in
[menu/classic/browser.rs](../crates/sjk-viewer/src/menu/classic/browser.rs).

Retail entries JKR has no screen for yet (Play Demo, Rules, Mods, Defaults) are
shown dimmed, and their description line says so.

Controls and Setup keep their group list down the left and show the chosen
group's items in the panel beside it, opening on Movement and Video as retail's
pages do. The items are the same settings and key bindings as the modern
screens, drawn the retail way: labels in capitals set against a column at retail
`textalignx`, the value after them, toggles as YES/NO, numbers as the retail slider
(`menu/new` art) with the value beside it, the focused item on the `menu_blendbox`
highlight, and the open group's entry in white. A group with more items than the
panel holds scrolls: the wheel over its items moves the list one item per notch
(three on a key-binding group), a thin bar shows the position and can be dragged,
and Up and Down keep the selected item in view.

- Setup: Video (resolution, display mode, sync, frame cap, field of view) and More
  Video (the rest of the VIDEO settings: marks, shadows, gun, readouts, gamma)
  split the VIDEO tab as retail splits its two video groups; Sound is AUDIO and
  Game Options is GAME. HUD, More HUD (the HUD+ tab) and Network follow as JKR
  additions.
- Controls: Movement, Interaction, Weapons and Other are the key-binding
  categories; Force Powers 1 holds the use/next/previous power and push, pull,
  speed and seeing binds, as retail's first Force page does, and Force Powers 2
  the rest. Mouse/Joystick shows the CONTROLS settings.

Up and Down move through the items, Left and Right (or Enter) change a value,
and typing or Enter on a number edits it exactly; clicking a slider sets it.
Tab moves to the next group (on the key-binding groups, Left and Right do too). A key binding reads "A OR B" (retail's `KEYBIND_OR`,
raised to capitals with the key names as retail's `BindingFromName` does) or `???`
when unbound; Enter or a click waits for the new key, shown in red with
retail's "Enter new key, or ESC to cancel, BACKSPACE to clear.", and Backspace
clears every key of the action. Escape closes the page to the main page.

The classic in-game bar's Setup and Controls open the same panels as retail's
`ingame_setup` and `ingame_controls` pop-ups: a box under the bar with the
group list and panel at their in-game positions and no navigation row, closing
back to the bar. Switching the Menu style (on Game Options) while a panel is
open continues on the modern settings screen.

The classic in-game menu (Escape during a match) is the retail top bar: About,
Join, Profile, Add Bot, Controls, Setup, Vote, Call Vote and Exit. Each opens a
pop-up under it or the matching screen. About shows the server info. Join picks
a team, or opens the class list in Siege. Vote is Yes/No. Call Vote opens the
call-vote lists. Exit offers Main Menu, Restart Match and Quit Program, each
with a Yes/No confirmation. Profile opens the retail in-game profile window
(`ingame_player`: name, team colour, head grid, Custom, Saber and the Force
summary, then `ingame_player2` and `ingame_saber`); its Apply returns to the
match. The Force box shows the side's emblem beside retail's mastery, side and
points lines and the known powers' holocrons, and its `configforce` button
opens the Force window; the Cosmetics button sits under Custom, as in JoF EJK.
Its Join Red, Join Blue and Spectate buttons are left to the Join tab. Controls and Setup open
the option panels described above. Siege swaps in Objectives and V Chat as retail does. Add Bot,
Objectives, V Chat and Restart Match are dimmed with a note, because the client
cannot add bots or restart a match it does not host. Left and Right move along
the bar; Escape closes a pop-up, then the menu. The JKR-only Server browser and
Shot controls entries are in the modern style only.

With the player's retail game data mounted, the classic menus draw its own
artwork: the backdrop, side glyph columns, ring, windows, logo, sub-page frames,
button glow, list glow (`menu_buttonback2`), slider bar and thumb
(`menu/new`), in-game bar and pop-up boxes from `gfx/menus`. The art is decoded
once on a worker thread the first time the classic style is used, and the UI
renderer uploads it into one texture per image, separate from the shared UI icon
atlas. Its bind group changes only between draw runs that need a different
texture, so layer order is kept. Additively blended retail images (glow, title
band, bar) are converted to alpha at decode time. A missing image falls back to
JKR's own shapes. Retail assets are never bundled.

The art moves as retail's shaders move it (`shaders/ui.shader`); the `.menu`
scripts themselves only swap pages at once and show or hide the glows. JKR's
main page plays `video/ja01` (`gfx/menus/videologo`) in its ring, as retail did.
SJK shows its own emblem there instead and does not read the video: the gold
starburst with the JK blade, centred in the centre window's opening and drawn
over the frames, 176 of the 640x480 canvas's units across (about 400 pixels at
1080 lines, 790 at 2160), with or without the retail art. Its orange core and
ring breathe (glow strength 0.08 to 0.45 every 4.2 seconds) and the blade's cyan
lights shimmer (0.30 ± 0.15 from two sines of 0.9 and 0.37 seconds), drawn as
additive glow layers over the still emblem. The emblem is bundled
([assets/branding](../assets/branding/README.md), drawn by
[menu/emblem.rs](../crates/sjk-viewer/src/menu/emblem.rs)) and decoded with its
mip chain on a worker thread when the first menu shows. The modern main page
shows it too, 128 units (pixels at 1080 lines) high above its title line and
aligned with the entry column; it shrinks in short windows. The ring
turns 5 degrees a second (`tcMod rotate 5`), the side glyph columns climb over
their `menu_side_text_b` backdrop (`tcMod scroll 0 0.025`), and a quarter of
`env_logo` drifts through the logo's translucent letters between an opaque and a
blended pass of the logo, as its three shader stages do. The button and list
glows, the title band and the in-game bar flicker: retail multiplies the screen
under them by four scrolling layers of `gfx/hud/static_menu`, which the renderer
reproduces by recomposing those small images with the noise on the CPU each frame
they are drawn, over the piece alone because the UI blends with alpha. The
motion clock and curves, the emblem's included, are in
[motion.rs](../crates/sjk-viewer/src/menu/art/motion.rs). The focused entry's
text pulses between white and 80% of it, as `Item_TextColor` does
(`PULSE_DIVISOR` 75 ms); the open group's or page's entry stays steady white.
Labels, buttons, titles and option values are in retail's capitals; descriptions,
typed text, vote-list names and the about values keep their case.

Outside a match the classic style draws no world. The main pages are opaque
over the retail background (in SJK the main page's emblem fills the centre gap,
the sub-pages' gap stays dark), and the modern screens they open (Settings, key
bindings, Player, Create game) get the retail backdrop beneath them; the classic
server browser draws its own. The frame
then clears instead of rendering the map, its secondary views and flares; the
boot map is still loaded, because the menu world is what joins build on, and
switching back to `modern` shows it again. Not loading it at all in the classic
style is a possible follow-up. Over a live match the in-game menu and the
screens it opens leave the game visible, as retail's do.

Joins and server map changes show retail's loading screens instead of the
modern gate. Until the gamestate arrives it is the connect screen
(`ui/jamp/connect.menu`, `UI_DrawConnectScreen`): `menu/art/unknownmap_mp`,
"Connecting to <address>" (or "Starting up..." when the client hosts the game)
and "Awaiting connection...", "Awaiting challenge..." or "Awaiting
gamestate...", following the join worker's phases. Then it is cgame's
information screen (`CG_DrawInformation`, `CG_LoadBar`): the map's
`levelshots/<map>` over the window (cropped top and bottom on a wide one, the
unknown-map art without a levelshot), "Loading... <what>" or "Awaiting
snapshot...", and the server's lines in retail order: host name, Pure Server,
message of the day, game name, the map's long name, cheats, game type, limits,
force rules and the game type's rules, worded from the player's `MP_INGAME`
strings. The LED bar along the bottom (`gfx/hud/mp_levelload`, `load_tick`,
`load_tick_cap`) has retail's nine ticks; JKR lights them from its own load
(gamestate, map parse, world build, world ready, session) rather than cgame's
registration steps. Colour codes in the host name are dropped. The gate stays
shut, the destination world is adopted only once it is built from the
session's own gamestate with the session in hand, and the player never walks a
preview world: the screen stays until the map is live. Escape or a click
cancels, as before. A failed join shows the connect screen with the reason; a
retail-style error page (`error.menu`) is not drawn yet. The loading screen is
in [loading.rs](../crates/sjk-viewer/src/menu/classic/loading.rs).

The profile pages are in
[player_menu/classic.rs](../crates/sjk-viewer/src/player_menu/classic.rs), with
entries and retail geometry in
[layout.rs](../crates/sjk-viewer/src/player_menu/classic/layout.rs), drawing in
[view.rs](../crates/sjk-viewer/src/player_menu/classic/view.rs) (the Force page
in [force_page.rs](../crates/sjk-viewer/src/player_menu/classic/force_page.rs),
the cosmetics window in
[cosmetics_page.rs](../crates/sjk-viewer/src/player_menu/classic/cosmetics_page.rs))
and pointer routing in
[pointer.rs](../crates/sjk-viewer/src/player_menu/classic/pointer.rs).
The main menu code is in [menu/classic.rs](../crates/sjk-viewer/src/menu/classic.rs): the
page tables are in [pages.rs](../crates/sjk-viewer/src/menu/classic/pages.rs),
types and geometry in [layout.rs](../crates/sjk-viewer/src/menu/classic/layout.rs)
and drawing in [view.rs](../crates/sjk-viewer/src/menu/classic/view.rs). The
option panels' frame is [panel.rs](../crates/sjk-viewer/src/menu/classic/panel.rs);
their items are drawn by
[settings/classic_view.rs](../crates/sjk-viewer/src/settings/classic_view.rs) and
[keybind_editor/classic_view.rs](../crates/sjk-viewer/src/keybind_editor/classic_view.rs).
The
in-game version is in
[ingame_menu/classic.rs](../crates/sjk-viewer/src/ingame_menu/classic.rs), with
[classic_view.rs](../crates/sjk-viewer/src/ingame_menu/classic_view.rs) and
[classic_actions.rs](../crates/sjk-viewer/src/ingame_menu/classic_actions.rs).
The artwork is loaded in [menu/art.rs](../crates/sjk-viewer/src/menu/art.rs) and
bound in [ui_renderer/art.rs](../crates/sjk-viewer/src/ui_renderer/art.rs). The
style is read in [style.rs](../crates/sjk-viewer/src/menu/style.rs). Shared exits
are in [destination.rs](../crates/sjk-viewer/src/menu/destination.rs).

Planned follow-ups, each a new page or screen module, following the retail
`ui/jamp` menus:

- Classic versions of the screens the classic pages still open in the modern
  style: Join Server's `findplayer` and `createfavorite` pop-ups, Create
  Server (`createserver`, `advancedcreateserver`),
  Solo Game (`quickgame`).
- Retail option items JKR has no setting for (video quality presets, colour
  depth, geometric and texture detail, EAX, languages) are left out of the
  panels, and the video restart confirmation is not needed.
- On the profile pages: portraits for every model (the atlas holds 207, so
  species after the characters show none).
- The screens with no JKR equivalent yet: Play Demo (`demo`), Rules
  (`rules*`), Mods, Defaults, Add Bot (`ingame_addbot`), Siege objectives and
  voice chat, and the error page (`error`).
- The retail fonts (`ui_gameFont`, a separate change), and the main page's
  hover captions (`*_undertext`, drawn in the `aurabesh` font).

The player screen's Character and Saber pages write their cvars as soon as a
value changes. The Force page edits a draft instead: Apply writes `forcepowers`
once, in the stock format and legalized as before, Discard returns to the applied
profile, and leaving the screen drops unapplied changes. While a draft is pending,
the page's points line reads NOT APPLIED and the footer's Back cap says that
leaving drops it. Power icons (`gfx/mp/f_icon_*`) and side emblems
(`gfx/hud/mpi_jlight`, `gfx/hud/mpi_dklight`) come from the installed game data;
without them the page shows text only. They take icon-atlas cells of their own
after the HUD's, so the character grid keeps all 207 of its icon cells. See
[force.rs](../crates/sjk-viewer/src/player_menu/force.rs) and
[force_view.rs](../crates/sjk-viewer/src/player_menu/force_view.rs).

## Player models

A player's or NPC's appearance (`models/players/<model>/<skin>`) loads its
`model.glm`, the skeleton the mesh names (`<name>.gla`), that skeleton's
`animation.cfg` and a skin; if the model cannot be loaded or built, the client
draws Kyle for that player instead
([player_assets.rs](../crates/sjk-viewer/src/player_assets.rs),
[actor_load.rs](../crates/sjk-viewer/src/actor_load.rs)). Files are read as
rd-vanilla and the retail cgame read them, so a model EternalJK draws and
animates is not swapped for Kyle:

- Skins are read with rd-vanilla's `CommaParse` loop (`RE_RegisterIndividualSkin`,
  `tr_skin.cpp`): tokens in surface/shader pairs, comments, missing commas, stray
  text and bytes outside UTF-8 change nothing about what loads, `tag_` entries
  are skipped, `_off` is stripped from surface names, the first entry for a
  surface wins and a skin keeps at most 128 entries
  ([skin.rs](../crates/sjk-model/src/skin.rs)).
- A skin never costs the model (`CG_RegisterClientModelname`, `cg_players.c`):
  when the requested skin is missing, has a missing part or names no surface,
  the model wears `model_default.skin`, and without one its surfaces' own
  shaders. A name is a three-part skin only when it has `|` and says `head`,
  `torso` and `lower`; the console notes each fallback
  ([player_skin.rs](../crates/sjk-viewer/src/player_skin.rs)).
- One leading slash on the mesh's skeleton name is dropped, as the filesystem
  drops it (`FS_FOpenFileRead`); vertex weights adding up past one are used as
  written (`G2_GetVertBoneWeight`); a mesh whose header bone count differs from
  its skeleton's loads when its bone references name skeleton bones.
- `animation.cfg` lines that name no animation, and sequences with no frames,
  are left out as `BG_ParseAnimationFile` leaves them; a table that then names
  nothing holds frame 0, as `G2_TransformBone` does.

Still drawn as Kyle: a model whose `model.glm` or skeleton is not installed or is
not version 6, a mesh referencing a bone past its skeleton, an `animation.cfg`
line naming an animation with other than five fields, and a model whose standing
animation lies past its skeleton's frames (rd-vanilla clamps such frames to 0).
SJK is more lenient than retail in two places: it keeps non-humanoid skeletons
and models without the hand, head or lumbar bolts the retail cgame checks for a
player, and a surface a skin does not name draws the mesh's own shader where
rd-vanilla draws its default shader.

SJK mounts `base` and, when set, `fs_basegame` and `fs_game`. EternalJK mounts its
own `EternalJK` folder by default (`fs_basegame EternalJK`), so skins shipped
there, such as `jedi/model_rgb.skin` in `japro-assets.pk3`, exist for EternalJK
and fall back to the default skin in SJK. Setting `fs_basegame EternalJK` and
restarting mounts it in SJK as well, together with that folder's menu, HUD and
string files.

The ignored test `player_model_scan` loads every installed model and skin through
this path without opening a window and compares each with what rd-vanilla and the
retail cgame would do:

```sh
JKA_GAME_DATA="/path/to/GameData" cargo test --release -p sjk-viewer \
    player_model_scan -- --ignored --nocapture
```

`JKA_MODEL_SCAN_GAMES=EternalJK` (comma-separated) mounts further folders above
`base` and reports only the models with files there. The table goes to
`target/parity-reports/player-models/`; see
[player_model_scan.rs](../crates/sjk-viewer/src/player_model_scan.rs).

## Hats and capes

SJK wears JoF EJK's free-choice cosmetics
([cosmetics.rs](../crates/sjk-viewer/src/cosmetics.rs)). A hat is any `.md3`
in `models/cosmetics/hats/`, a cape any in `models/cosmetics/capes/`
(`models/players/hats/` and `capes/` when the new folders are empty, as older
packs used them); names of JoF's catalogue are found in either. A name is at
most 13 letters, digits, `_` or `-` and does not start with a digit. JoF's
pack is `zzz_jof_cosmetics.pk3` in the `EternalJK` folder, which SJK mounts
only with `fs_basegame EternalJK` (see [Player models](#player-models)); a copy
in `base` works too.

What a player wears travels in the saber colour keys, as JoF EJK sends it:
`color1 "4santahat"` is blue blade 4 wearing the hat `santahat`, and `color2`
carries the cape. SJK's `color1`/`color2` are therefore text cvars read with
`atoi`; the profile writes them as `<colour><name>`, the saber page keeps the
name when it changes the colour, and the userinfo carries the name after the
digits only when it follows the rule above
([jof_cosmetics.rs](../crates/sjk-client/src/jof_cosmetics.rs)). Servers copy
the keys to the `c1`/`c2` clientinfo untouched (cut to 15 bytes), and other
clients' `atoi` reads only the colour. SJK reads another player's blade
colour with `atoi` too: before, `c1 "8santahat"` failed to parse and drew blue.

Each player's pieces are resolved when their actor is built or their
clientinfo changes: the names, the models (loaded mid-match like a
configstring model, on first use) and the fitting offset from
`settings/cosmetics/<hats|capes>/<name>.cosmetic`, JoF's and TaystJK's JSON of
per-model and per-skin `xOffset`/`yOffset`/`zOffset` (exact keys, else the
longest `prefix*` key; the skin's entry wins, the model's applies with
`"modelFallback": true`). Each evaluated pose then reads the `*head_top` and
`*back` bolts of the worn slots only. A piece is drawn with the body as
`CG_DrawCosmeticOnPlayer` places it: the bolt's axes, two units down its up
axis, plus the offset along the world axes, never on the dead, the
mind-tricked or a scaled model, and on the local first-person player only in
mirrors and portals. A piece this client does not have is not drawn.
`cg_cosmetics` (archived, 1) draws everyone's (1), only yours (2) or none (0).

`cosmetics hats` and `cosmetics capes` list the installed pieces, with a
number or a name they wear one (the same again takes it off), `cosmetics
clear` takes both off and `cosmetics visibility [off|on|onlyme]` shows or sets
`cg_cosmetics` ([command.rs](../crates/sjk-viewer/src/cosmetics/command.rs)).
The classic profile's Cosmetics window does the same, and the modern player
screen's Character page has Hat and Cape rows under its grid (None, then
each installed piece). The menu stage model wears what `color1` and `color2`
name, placed on the bolts of the pose it is skinned with like its hilts
([menu_stage/cosmetics.rs](../crates/sjk-viewer/src/menu_stage/cosmetics.rs)),
unless `cg_cosmetics` is 0.

jaPRO's race-unlock hats are drawn as JoF EJK's `CG_Player` draws them: a
player without a hat of their choosing (or one this client lacks) wears the
hat their `c5` clientinfo grants, the lowest of its bits (Santa hat,
Jack-o'-lantern, cap, fedora, Kringe Kap, sombrero, top hat, from
`models/players/hats/`, without fitting offsets), on servers that are neither
JA+ nor base. Bit 21 of `cg_stylePlayer` (2097152, jaPRO's
`JAPRO_STYLE_SEASONALCOSMETICS`; SJK reads no other bit) draws them on those
servers too and gives a player with no bits JoF's seasonal hat: a Santa hat
from 22 November to 7 January and a pumpkin on 31 October, by local date.
The player's own choice is `cp_cosmetics` (archived, userinfo), sent to
TaystJK/jaPRO servers, which grant it against the unlocks earned:
`cosmetics unlocks` lists the seven with the one worn and, where the server
sent its unlock table, what each takes ("requires mp/ffa3 jka in under 12.500
seconds"); `cosmetics unlocks <num>` wears that one alone, again takes it
off. Other servers answer "This server has no cosmetic unlocks."

## Animation sounds and voice variants

Footsteps and authored swing/spin sounds follow the evaluated lower/upper Ghoul2
frames and the model's `animevents.cfg`, including the shared skeleton table and
`include` directives. The client reads supported sound assets during appearance
loading and queues decoding on the audio worker; ordinary frame playback performs
no file reads. A world build shares the skeleton and event assets across matching
appearances. Active actors release their prefetched encoded bytes after registration.

Ground-contact events trace beneath the animated foot and select the authored
walk/run sound bank for the surface material. `cg_footsteps 0` mutes them. Frame
latches prevent repeated playback while an animation frame is held; absent actors,
teleports, backwards seeks and paused map changes reset the cursor. First-person
local actors use the same evaluated timing. This restores the blue-stance taunt's
spin sounds and authored melee/kick swing cues. Custom saber `spinSound` and
`swingSound1`–`3` override the standard animation samples.

Taunt, flourish and gloat voice choices advance per accepted event, with fallbacks
based on samples that actually resolved. The expanded taunt bank is used in FFA
as in TaystJK, rather than restricting ordinary FFA taunts to `taunt.wav`. Selection
is replay-stable but is not the legacy global random stream; repeats remain
possible. Animation selection, movement, saber timing and network events are
unchanged. Animation-driven effect/footprint marks and gameplay event actions
remain outside this audio adapter.

## Renderer settings

SJK gives JKR's own cvars (`jkr_*`) neutral engine names: rendering ones are
`r_*` (`r_sceneHdr`, `r_toneCurve`, `r_sceneBloom`, `r_superSample`,
`r_actorSunShadows`, `r_sunShadow*`, `r_dayNight`, `r_liveLighting`, ...), the
ground HUD is `cg_groundHud` and the dedicated server's are `g_npcNav` and
`g_stockRules`. Names rend2 or EternalJK use with another meaning are avoided.
The `jkr_*` names keep working as aliases, so JKR configs and commands still
apply, and `config.cfg` is saved under the new names; the full list is in
[cvar_renames.rs](../crates/sjk-viewer/src/cvar_renames.rs). This page otherwise
uses SJK's names.

These rendering cvars have their own settings page. The last row
of Settings > VIDEO, "Renderer", opens it, as JoF EJK's advanced renderer page
opens from its Video setup; Escape or Back returns to that row. With the
classic menu style, the Setup page's RENDERER entry (after NETWORK, in the main
menu and the in-game pop-up) opens the same page directly, and Escape or Back
returns to the Setup group that was open. The page has three tabs:

| Tab | Settings |
| --- | --- |
| IMAGE | HDR scene and exposure, eye adaptation (`r_autoExposure`) and its range in EV, filmic tone curve, bloom, dynamic glow (`r_DynamicGlow` 0-3) and its blur style (`r_dynamicGlowStyle`), FXAA, supersampling (`r_superSample`), soft particles, sunbeam dust (`r_dustMotes`), per-pixel model lighting, reflection probes (`r_cubeMapping`), floor mirrors (`r_floorReflections`), emission maps (`r_emissiveMaps`), their strength (`r_emissionStrength`) and glow halo (`r_emissiveGlow`) |
| LIGHTING | Sun and sky (`r_dayNight`), live lighting tier, time of day, day length, sunlight brightness, ambient fill and its corner shading, indirect boost, emission-map lights (`r_emissiveLights`), light shafts (`r_volumetrics`) and their clarity |
| SHADOWS | World and character sun shadows, shadow resolution, sharp and close cascade distances, filter taps, slit closing, contact shadows |

Rows marked "(restart)" are read when the client starts and apply after a
restart; "(next map)" applies when a map loads; the rest apply immediately.
Changing a value saves it like any other setting. Switches over numeric cvars
show ON/OFF and write 1/0. Defaults are unchanged (see
[Default visual profile](rendering.md#default-visual-profile)). Diagnostics such
as `r_dayDebug` stay console-only, as do the speeds and key of
[eye adaptation](rendering.md#eye-adaptation); the ground HUD stays on the HUD
tab, and exclusive fullscreen (`r_exclusiveFullscreen`) stays on VIDEO's
display-mode row. Eye adaptation holds still while this page is open, so
exposure changes made here show at once instead of being eased.
See [catalog.rs](../crates/sjk-viewer/src/settings/catalog.rs).

## Slider values

Every slider in Settings (including the classic Setup panels), the saber RGB
controls (including the second saber), and the Shot panel supports direct
numeric entry. Click its displayed value, select the row and press Enter, or
just type a number on the selected row: the draft starts with what was typed.
Enter applies it; Escape cancels. Left/Right, Home/End, Backspace and Delete
edit the draft. Space still steps a selected slider instead of opening entry.
Hovering does not move an edit to another setting. Text settings such as the
master server keep their edit on the row that opened it, and Enter writes that
setting.

SJK differs from JKR's documented behavior here: pressing anything else while a
numeric draft is open applies the draft when it is a valid number (an invalid
one is discarded) instead of always discarding it, typing on a selected slider
opens entry, Space steps rather than opening entry, and integer sliders round a
typed fraction (142.6 becomes 143) instead of refusing it.

Manual values respect the slider bounds but do not snap to its drag increment:
for example, the FPS cap accepts 142 and FOV accepts 97.5. A draft takes
digits, one decimal point (a comma counts as one), a minus sign only first and
only on sliders that go below zero (the FPS cap's `-1` is AUTO), and at most ten
characters; a held key does not repeat typed characters. Invalid or empty input
stays open with a red underline and does not change the setting. Dragging and
arrow adjustment outside editing retain their existing behavior. Values they
write are rounded to the step's decimals, so a 0.05-step slider stores 0.35
rather than 0.35000000000000003.

## Key bindings

Settings > Key bindings binds two keys per action: click a slot (or select it
and press Enter), then press a key or mouse button. A key already bound
elsewhere moves to the new action. Escape, or a click with the left or right
mouse button, cancels a pending capture without binding anything; the click is
consumed, so it activates nothing under the pointer. Retail
(`Item_Bind_HandleKey`) bound any key pressed while waiting, so a stray click
could take `+attack` off `MOUSE1`.

`MOUSE1`, `MOUSE2` and `ESCAPE` are locked in the editor: they cannot be
captured, a slot holding one cannot be rebound or cleared there, and locked
keys are drawn muted. Reset defaults restores them. The console's `bind` and
`unbind` commands are unrestricted. See
[keybind_editor.rs](../crates/sjk-viewer/src/keybind_editor.rs).

## Development maps

Run `devmap mp/ffa3` in the client console to start and join an owned local
FFA server with cheats enabled, no bots and no match limits. Other installed
maps work too, including `devmap t2_rancor`; `maps/` and `.bsp` are optional.
The command appears in console completion/help. It uses the same `sjk-dedicated`
binary lookup as Create game (`JKA_DEDICATED` overrides the adjacent binary).

This starts a fresh game on loopback, without master-server advertising. Once
launched, it replaces the current connection; it never asks a remote server to
change maps or allow cheats. Missing map names are reported before leaving the
current game. Disconnecting, cancelling the join, or exiting stops the owned
server. Ordinary Create game launches still leave cheats disabled.

The native server implements `noclip`, `give`, `setviewpos`, and `t_use` for
development. Run `noclip` again to return to ordinary movement; spawning again
clears it. Normal servers still require their own cheat permission. `god` remains
unimplemented on the native server.

## Talk balloons

Opening chat, the console or a menu sends the stock talk button and disables
other movement input while that keyboard catcher is active. Players carrying
the talk flag have a chatbubble over their heads; the connection-trouble icon
takes priority when the server marks a lost connection. These are upright frame
billboards using the existing [sprite orientation](rendering.md#billboard-icons).
Their texture opacity is preserved near walls even with soft particles enabled.
Your own bubble is visible in third person, not the first-person view. Mind-tricked
players, NPC talk flags and intermission do not show talk balloons. Siege voice
command icons remain unimplemented. See
[player_sprites.rs](../crates/sjk-viewer/src/player_sprites.rs) and
[pmove_talk.rs](../crates/sjk-game-jka/src/pmove_talk.rs).

## Joining and changing maps

The menu's FFA3 gate opens onto the prepared destination world. Map preparation
and connection run independently: when the world is ready first, you can walk,
jump and crouch locally while the connection finishes. A matching verified world
is reused when the server session becomes ready; joining does not build it twice.
The opening and crossing animation takes about 2.1 seconds once the destination
is ready. The gate stays closed while required content is unavailable. The
existing connection notice shows the server address/status and Cancel action
over the gate during joining; it disappears when the destination is entered.

On a server map change, gameplay pauses and a loading notice is shown over the
previous view until the destination and a fresh active snapshot are ready. The
client then adopts the server's spawn state. Match-end intermission uses the
server's normal camera, scoreboard and ready-to-exit controls. Player bodies and
vehicles are hidden there, matching codemp; scripted non-vehicle NPCs remain visible. Local gameplay
continuation on the old map is suspended while that feature is developed further.
The viewer no longer prepares an in-process native game for every loaded world.

CPU map preparation and GPU resource installation remain on background workers,
using the existing GPU context. Archive checksum inventory reads ZIP directories
without decompressing every asset. Matching same-map restarts can still reuse
the prepared world, and the gate adopts its already-built destination. The
waiting connection sends neutral commands during map loading and is kept separate
from the displayed old world, so new-map entities cannot appear in the wrong BSP.
Texture mip preparation and lamp extraction use bounded CPU workers; repeated
texture loads can reuse a bounded process-local mip cache. See
[load-time rendering preparation](rendering.md#load-time-texture-and-light-preparation)
for cache limits and unchanged output semantics. The gate animation and server
readiness still contribute to the time before play begins.

First entry through the gate before a server player exists retains movement-only
exploration with frozen brush collision. The gate's through-door view uses the
existing lightweight rendering path; full lighting begins when the destination
becomes the active world.

Server console output (`print`) goes exclusively to the console, including match
statistics, command replies and server announcements. It never enters chat history.
Each line of a print becomes its own console row, without the empty row a
server's closing newline would leave. Global, team and private chat retain their
conversation overlay, and are also kept in the console scrollback but not among
its notify lines, as stock cgame echoes chat with the `*` print prefix that
`CL_ConsolePrint` keeps out of the notify area. Center-print
gameplay notices keep their separate HUD presentation. The scoreboard continues
to use structured server scores rather than parsing printed statistics tables.

Chat remains connected to the real server during intermission, including
global/team/private composer messages and console/bound chat commands. Messages
received during background loading are retained; its loading notice hides the HUD. The scoreboard reserves a separate left column for messages and the
composer, temporarily overriding chat position/width while scores are visible.
Chat visibility/lifetime settings still apply. Typing captures gameplay input as
usual, and the ordinary chat layout returns after the scoreboard closes.

Escape opens the normal menu during exploration. Disconnect/cancel abandons the
pending connection and restores the retained main-menu world. Connection and asset
errors still show an error message. Downloads retain their existing policy and
limits; progress is available in the console. This does not eliminate disk,
shader, network or download latency, and direct command-line startup is separate
from the already-open menu's transition path.

Implementation: [resident worlds](../crates/sjk-viewer/src/resident_world.rs),
[early exploration](../crates/sjk-viewer/src/resident_walk.rs),
[world handoff](../crates/sjk-viewer/src/session_transition.rs) and
[gate destination](../crates/sjk-viewer/src/portal.rs).

Losing window focus releases held gameplay controls and discards gameplay actions
already queued for that frame. Synthetic key events on refocus cannot re-press a
held modifier such as Alt. This allows a saber throw already sent to the server
to finish normally after Alt+Tab.

### Chat player actions

Open the chat composer with your chat binding (`messagemode`), then click a
sender's name. The cursor is free while composing. The player menu offers:

- **whisper:** keeps the current draft and addresses the selected player using
  the stock `tell` command. Nothing is sent until Enter.
- **ignore:** hides that player's existing and incoming
  messages locally for the current map. Opening chat shows a hidden-message row
  whose name can be clicked to undo the ignore. It does not change server policy
  or suppress footsteps, saber effects, or other gameplay sounds.
- **friend:** saves a local name bookmark and adds a
  small five-point star to the left of that player's name. Bookmarks survive restarts in
  `chat-friends.txt`, beside `config.cfg`. Names ignore colour codes but otherwise
  match exactly; these are name bookmarks, not authenticated accounts.
- **copy:** copies the complete name, including its colour escapes.

The dropdown opens without a highlighted action. Hover follows the pointer;
keyboard navigation highlights only its current row until the pointer moves.
Arrow keys/Tab navigate the player menu; Enter selects and Escape dismisses it
without discarding the draft. The compact square-edged dropdown sits to the left
of chat, aligned with the clicked name and kept above the composer. It has only
four labels, no title or description, and never moves the conversation. If the
left margin is too narrow, it uses the right edge inside the chat lane to avoid
the scoreboard. Highlighted `ignore`/`friend` rows indicate active toggles; clicking
again undoes them. Name hover fits the visible username glyph bounds, excluding the star. Dropdown
row highlights use exactly the same rectangle as their clickable button. There is no
standing player-options hint or success notice. Opening the composer exposes history even when
passive chat is hidden with `cg_chatbox 0`.

The draft shares the console's UTF-8 caret and selection rules: arrows/Home/End,
Ctrl+arrows and Ctrl+Backspace/Delete, Shift-selection, Ctrl+A/C/X/V,
Ctrl+Insert to copy and Shift+Insert to paste. Click places the caret, drag selects,
and double-click selects a token. Selected text is highlighted; typing/pasting
replaces it. Clipboard text keeps colour escapes, strips controls and obeys the
existing chat byte limit. Pasting never sends a message. Dead keys type as in
the console (see below): `^` then a digit is a colour code, `^` then `e` is `ê`.

Actions use server-provided sender slots and current roster generations. Old
messages cannot address a replacement after an observed departure/name change;
an invalid whisper recipient leaves the draft open. Unattributed server messages
remain unclickable rather than guessing a destination from displayed text.
Legacy servers provide no authenticated account identity; unobserved same-name
slot reuse cannot be distinguished. No transport or protocol encoding changed.

The leader/opponent portrait and its name/score occupy the top-right corner,
with a 32-unit top margin and the existing 40-unit right margin at 1080p (scaled
with the HUD). Optional snapshot diagnostics, inventory and the automatically
positioned team overlay flow below that block. Explicit team-overlay coordinates
remain authoritative. Visibility and server-selected leader/opponent rules are
unchanged.

## Scoreboard styles

`cg_scoreboardStyle` (Settings, HUD+ tab, "Scoreboard style") picks the
scoreboard layout: `modern` (default), JKR's table beside the chat column, or
`classic`, the retail scoreboard as EternalJK-derived clients such as JoF EJK
draw it ([classic.rs](../crates/sjk-viewer/src/scoreboard/classic.rs), after
`CG_DrawOldScoreboard`/`CG_DrawClientScore` in `cg_scoreboard.c`):

- The header shows "Killed by" while you are dead, otherwise the player count
  (`cg_drawScoreboardPlayerCount`: 1 host name and counts, 2 counts only, 0 off;
  team games show "N vs. M", your team first), and below it your place ("2nd
  place (of 7) with 18", place in its retail colour) or the team lead.
- Columns are Name, Score, Ping, Time and, with `cg_showClientIDs` (on by
  default), the client ID; CTF shows Score, C, A, D, Ping and Time, and duels with
  a frag limit show wins/losses. Score shows score/deaths when `cg_scoreDeaths`
  provides deaths. Bots show `BOT` for their ping; clients still connecting show
  `-`, and connected clients the scores do not list yet show `N/A`.
- Team games list the leading team first over a translucent team band, then the
  spectators; free-for-all lists the players, then the spectators. Your row is
  highlighted in your rank's colour (1st blue, 2nd red, 3rd yellow, else grey) and
  is added at the bottom if the list does not reach it. Flag carriers show the
  flag icon before their row; at intermission, ready players are marked `READY`.
- Rows are 25 units of the 480-line screen, 15 units once more than 12 players
  are listed (always with `cg_smallScoreboard` and in CTF), and 12 units with the
  header moved up from 20 clients. `cg_drawScoreboardIcons` (on by default) shows
  each player's head icon (`models/players/<model>/icon_<skin>`), decoded when a
  model changes, two per frame at most.
- Positions follow the 640x480 screen fitted to the window height, spreading up
  to 1.25x horizontally on wide windows while text keeps its proportions. Names
  and headings are sized like retail `ergoec` text and numbers like `ocr_a`.

The classic board also fades in over 120 ms and out over 200 ms after release
(retail's fade time), rows glide to their new places when the order changes, pings
are coloured from green to red, and rows alternate a faint stripe. Nothing is
allocated per frame; the text and draw storage is reserved for 32 clients.

## HUD style

Settings > HUD > "HUD style" (`cg_hudStyle`) chooses JKR's `modern` or `classic`
layout or `game`, the status HUD of the game's own menu files: the original Jedi
Academy HUD, or a custom HUD pack that replaces `ui/hud.menu`. "Game HUD files"
(`cg_hudFiles`) names the menu list, `ui/jahud.txt` by default; `1` gives the
text-only HUD and EternalJK's `3`/`4` name its elegance and JoF HUD lists when
those files are installed. See
[game-data HUD](rendering.md#game-data-hud) for what is drawn.

## Configuration and content

The default writable client folder is `GameData/jkr/`, under the selected game
installation (SJK uses `GameData/SJK/` instead and imports JKR's `GameData/jkr/`
before the per-user folder; see [storage.rs](../crates/sjk-viewer/src/platform/storage.rs)). It is independent of the executable's location and working
directory. The client creates it automatically. Important files include:

| File or folder | Contents |
| --- | --- |
| `config.cfg` | Settings and key bindings |
| `marks.txt` | Marked map positions, views and notes |
| `favorites.json`, `chat-friends.txt` | Favorite servers and friend names |
| `jakey` | Persistent client identity key |
| `hud.json` | Optional custom HUD |
| `screenshots/`, `demos/` | Screenshots and recordings |
| `chatlogs/`, `qconsole.log` | Logs when enabled |

On first use, existing files from the previous per-user JKR folder are copied
into this folder. Root-level `.cfg` files and `configs/` are included. Files
already present in the destination win; the originals are never deleted. A
`.user-data-imported` marker prevents repeated imports, including restoration of
files subsequently deleted by the player. Imports skip source links and PK3s.
An incomplete import reports an error and can be retried without overwriting
completed files.

If `GameData/jkr/` cannot be written, the client uses its per-user folder:
`$XDG_CONFIG_HOME/jkr/` (otherwise `~/.config/jkr/`) on Linux, `%APPDATA%\jkr\`
on Windows, or `~/Library/Application Support/jkr/` on macOS. The chosen folder
is shared by all profile consumers for the entire session. Startup reports it;
the console's `path` command also lists it. The fallback uses the profile in
that per-user folder; the old and portable profiles are not continuously synced.
See [platform.rs](../crates/sjk-viewer/src/platform.rs) and
[storage.rs](../crates/sjk-viewer/src/platform/storage.rs).
Edit settings through the client, or edit the file while the client is stopped
so autosaving cannot overwrite your changes.

`com_maxfps` defaults to `-1` (AUTO in Settings > Video): frames are capped at the
refresh rate of the monitor holding the window, rounded to whole hertz and
re-read once a second, or at stock's 125 when the monitor reports none. `0` is
uncapped. The old default, 1000, saved in every existing profile, is reset to
AUTO once on first launch (marker `com_maxfpsDefaultVersion`); a cap chosen
afterwards is kept. The default is not saved to the configuration. On the slider
AUTO is the rail's left end: arrows step AUTO, 0, 25, 50 and so on, and typing
`-1` selects it. An uncapped
client saturates the GPU; screen recorders and streamers sharing it then skip
frames (OBS reported 83% skipped for encoding lag against an uncapped client at
4K). See [runtime_settings.rs](../crates/sjk-viewer/src/runtime_settings.rs).

The Video tab's Display mode row offers Windowed, Borderless fullscreen and,
where the windowing system supports it, Exclusive fullscreen (Wayland does not).
Stock `r_fullscreen` keeps its meaning, fullscreen on or off, and Alt+Enter still
toggles it. `r_exclusiveFullscreen` chooses the kind: 0 (default) is a borderless
window at the desktop size, 1 switches the monitor to the `r_resolution` video
mode. Stock JA's fullscreen is always the exclusive kind; JKR defaults to
borderless. Choosing Windowed leaves `r_exclusiveFullscreen` alone, so Alt+Enter
returns to the last fullscreen kind. Exclusive fullscreen without a monitor mode
of that size falls back to borderless.

Enter or a click on the Resolution row opens a list of the monitor's video-mode
sizes, grouped by aspect ratio with the monitor's own first and the size in use
highlighted. Windowed and borderless also list the classic presets that fit the
monitor and a custom `r_resolution`; borderless fullscreen always fills the
desktop and uses the size only when windowed. Left and Right step the row within
its aspect-ratio group. See [display.rs](../crates/sjk-viewer/src/settings/display.rs)
and [resolution.rs](../crates/sjk-viewer/src/settings/resolution.rs).

`fs_game`, `fs_basegame` and `fs_homepath` configure content search paths; restart
the client after changing them. Search precedence and shader protection are owned
by [asset_search_paths.rs](../crates/sjk-viewer/src/asset_search_paths.rs).

Downloaded content is stored separately from the retail installation and config.
On Linux the default is `$XDG_DATA_HOME/jkr/downloads/base`, falling back to
`~/.local/share/jkr/downloads/base`. `JKR_DOWNLOAD_HOME` overrides the download
root (the implementation appends `base`). See
[download_store.rs](../crates/sjk-viewer/src/download_store.rs).

Server reference lists use OpenJK's positional common-prefix rule: extra pak
names or checksums without a counterpart are ignored, including when one list
is empty. Download comparison and session cache selection share
[the compatibility parser](../crates/sjk-client/src/referenced_paks.rs), so a
connection cannot pass one check only to fail the other on list length.
Paired checksums, download paths and file contents remain validated; advertised
BSP checksums are still enforced. This does not change pure-server proofs.

References are not a mandatory client install manifest. When the server sets
`sv_allowDownload 0`, or the client sets `cl_allowDownload 0`, UDP transfers are
skipped and joining proceeds with available content. Missing directory names,
unsafe download names and retail packs never produce a download request.
Unavailable referenced archives do not abort world mounting; only locally
available checksum matches are selected from the cache. The actual map must
still exist and match its advertised BSP checksum. HTTP downloading remains
unsupported, and this policy does not disable pure-server admission checks.

## Key names and binds

`bind`, the controls editor and the config name keys as stock JA does on Windows:
by what the active keyboard layout prints on them. On a French AZERTY keyboard
`bind w +forward` is the key labelled W, and the key left of W is named `<`.
Character keys take the layout's unshifted character, including non-ASCII ones
such as `é`, `ù` or `²`; a dead key is named by its accent, so AZERTY's `^` key is
`^`. The digit row keeps `0`-`9` on every layout, keys without a character
(arrows, F-keys, keypad, modifiers) keep their stock names, and letter keys of
non-Latin layouts such as Cyrillic keep their US letter, so the default binds
still reach a key there. A release runs the binds of the name its press had.
Menu navigation keys (W/A/S/D beside the arrows) stay positional. See
[keys.rs](../crates/sjk-viewer/src/input/keys.rs) and
[key_names.rs](../crates/sjk-shell/src/key_names.rs).

Key names are shown in capitals, as retail's controls menu shows them
(`BindingFromName` upper-cases with `Q_strupr`): the key binding editor, the
vote prompt and the console's `bind`, `unbind` and `bindlist` output read `W`,
`SPACE`, `MOUSE1`. Only ASCII letters change, so layout names such as `é` keep
their character. `config.cfg` keeps the saved spelling (`bind "w" ...`), and
`bind` accepts names in any case. See
[key_names.rs](../crates/sjk-shell/src/key_names.rs).

## Colour codes

Text draws `^0` to `^9` as OpenJK's ten-entry colour table does: `^0`–`^7` are
the retail colours, `^8` is orange and `^9` grey (retail wrapped them onto black
and red). The table is `quake_color` in [text.rs](../crates/sjk-viewer/src/text.rs).

## Console styles

`con_style` (Settings, TEXT tab, "Console style") picks how the console looks:
`classic`, SJK's default, follows EternalJK's console (`cl_console.cpp`,
`cl_keys.cpp`); `modern` is JKR's console (Inter text on a tinted panel with a
header and key hints), unchanged. Only `modern` or `0` selects the modern
console; any other value, a typo included, gives `classic`. Input editing, mouse
selection, completion, the `]cmd` echo, history (Up and Down) and the F3 browser
work the same in both. See
[console_classic.rs](../crates/sjk-viewer/src/console_classic.rs),
[console_backdrop.rs](../crates/sjk-viewer/src/console_backdrop.rs) and
[console_options.rs](../crates/sjk-viewer/src/console_options.rs).

### Classic console

Text sits on a grid of character cells drawn with the console character set
(`gfx/2d/charsgrid_med`, so an HD replacement such as the JoF pack's is used),
without the glyph shadow other UI text has. The classic console loads the
character set whether `ui_gameFont` is on or not; without it the cells use Inter.
A cell is 8 by 16 pixels at 1080 lines and `con_scale 1`, grows with the window
height like the rest of the UI (with the console's 0.75 floor) and is rounded to
whole pixels. EternalJK's cells are 8 by 16 screen pixels times `con_scale`, so
`con_scale 0.5` at 2160 lines matches EternalJK at 4K with its default scale. A
row holds the screen width in cells minus two, and column `c` is drawn `c + 1`
cells from the left edge, as `Con_DrawSolidConsole` does.

The background is the game's `console` shader, read from the shader scripts and
the PK3s like any other, so a pack that overrides it wins as it does in
EternalJK: the retail one scrolls a star field (alpha blended) and adds the
pulsing Jedi Academy logo; a cosmetic pack such as JoF's draws an opaque picture
with scrolling stars over it. Each stage keeps its `blendFunc`, its `tcMod`
(`scroll`, `scale`, `rotate`, `transform`, `stretch`) and its `rgbGen` and
`alphaGen` (`wave`, `const`, `vertex`); other generators draw at full strength,
and an `animMap` shows its first frame. Below full height `alphaGen vertex` is
`con_opacity`, so the retail stars fade with it while an opaque first stage stays
opaque; at full height it is 1. Without the shader the console is a dark navy
panel. The background reaches `480 × fraction − 2` units of the 640×480 virtual
screen, with a bar two units tall in EternalJK's `console_color` (0.509, 0.609,
0.847) under it. `con_ratioFix` (1, archived, EternalJK's name and meaning) shows
the middle of the picture (`t` from `1 − k` to `k`, `k` = 4:3 over the screen's
aspect) when the console is half the screen or less on a wide screen, instead of
squashing the whole picture; set 0 for custom backgrounds made for a squashed
fit.

The classic console has its own 2D layer, drawn after every other 2D element,
text included, and its own text last, so menu, chat, HUD and frame-rate text
never shows through an opaque console.

| Key | Classic console |
| --- | --- |
| Console key (`cl_consoleKeys`, or the physical key with `cl_consoleUseScanCode`) | Opens to `con_height` (0.5) |
| Ctrl + console key | Opens full screen |
| Shift + console key | Opens a quarter of the screen |
| Shift+Escape | Opens to `con_height`; Escape closes |
| Page Up, Page Down, mouse wheel | Scroll back or forward two rows; ten with Ctrl |
| Ctrl+Home, Ctrl+End | Oldest row, newest row |
| Up, Down, keypad 8 and 2 (Num Lock off), Shift+wheel, Ctrl+P, Ctrl+N | Command history (32 commands) |
| Ctrl+L | Clear the scrollback |
| Insert | Toggle overstrike: typing replaces the character after the cursor |

A `toggleconsole` bind reopens at the last height a console key chose. The console
slides at `scr_conspeed` screens per second and is drawn from its first pixel
(the modern console waits until it is 8% open). A disconnected client whose menu
is closed shows the console full screen, as `Con_DrawConsole` does; the map
viewer without a menu does not.

Scrollback rows are drawn from three cells above the console's bottom edge up to
the top of the screen (`con_maxLines` is not used). Lines start white (`^7`),
error lines light red, and a colour code carries on into the rows a line wraps
onto. Words wrap as `CL_ConsolePrint` wraps them: a word that fits on a row but
not in what is left of the current one, or would end exactly at its edge, starts
the next row; a word longer than a row breaks at the edge (EternalJK breaks such
a word early, at an odd point); colour codes take no room. With `con_timestamps 2`
(EternalJK's layout) every row, wrapped ones included, starts with the local time
its line was written in grey, `HH:MM:SS` and a space, and the text wraps in the
columns after it; `1` also stamps the notify lines and `0` leaves no stamp column.
Scrolled back, a row of `^` every four columns in the bar colour sits under the
rows, and new output does not move the view.

The input row is two cells above the bottom edge: the local time in green in
columns 1 to 8, `]` in column 10, then the input as typed, colour codes shown
rather than applied, and a cursor that blinks every 256 ms, the character set's
underscore or, in overstrike mode, its block (Inter cells use `_` and a box). The
input scrolls sideways to keep the cursor on screen. In the bottom-right corner
the version line ends one cell from the edge, two and a half rows up, and the
local date and 12-hour time (`Sun Oct  4 10:52:10 PM`, as EternalJK prints
`asctime`) sit under it at the edge, both in the bar colour.

Closed, the console draws notify lines while a game, a demo or a map walk runs
and no menu has focus: of the last `con_notifylines` rows, those written within
`con_notifytime` seconds and not quiet (chat), from the top edge of the screen,
one cell plus `cl_conXOffset` pixels from the left. Dragging the mouse selects
scrollback text and Ctrl+C copies it, as in the modern console; the classic
console's selection does not include the time column.

Local time comes from the operating system's time zone rules, daylight saving
included ([local_time.rs](../crates/sjk-shell/src/local_time.rs)); the scrollback's
stamps, also the modern console's `[HH:MM:SS]` and the log file's, are local time
too. Differences from EternalJK besides those above: the input never runs past
the screen's right edge (EternalJK's can), and while the console is open a
printable console key types its character, as in SJK's modern console; Escape or
a non-printing `toggleconsole` key closes it.

## Useful console commands

Printable console shortcuts open the console but type normally once it is open;
Escape and non-text toggle bindings can still close it. On layouts with dead keys
(`^` on French AZERTY and German QWERTZ, `'` on US International), the console
line and the chat draft show a dead key at the caret at once and replace it with
what the platform composes on the next key, so typing reads as on a layout without
dead keys: `^` then `1` gives the colour code `^1`, `^` then `e` gives `ê`, and `^`
then Space gives `^`. Backspace removes only the shown `^`; Enter sends it. A dead
`^` that xkb composes into a superscript digit (`¹`) becomes `^1` again, since a
caret before a digit is a colour code. See
[dead_key.rs](../crates/sjk-viewer/src/input/dead_key.rs). The browser's search
field takes a dead `^` as a literal colour prefix. Opening or closing the console
clears the window's pending accent composition, so a dead toggle key such as `^`
on a German layout does not combine with the next letter.

`connect host:port`, `disconnect` and `reconnect` control the session.
`record`, `stoprecord`, `demo` and `playdemo` control demos.
`screenshot` and `screenshotJPEG` request captures; `condump filename` saves
console output. See [console registration](../crates/sjk-viewer/src/console_session.rs)
and [file commands](../crates/sjk-viewer/src/console_files.rs) for argument handling.

`tell <player> <message>` takes a slot number or, as in EternalJK, a name or a
unique part of one, ignoring case and colour codes; an exact name wins over longer
names containing it. The client sends the stock `tell <slot>` command; when no
player or several players match, it lists them and sends nothing. See
[console_tell.rs](../crates/sjk-viewer/src/console_tell.rs).

Held actions such as `+button12` work from binds, cfg files and the console. A
hold typed at the console lasts until its `-` command, as in the stock client:
opening the console, a menu or chat, or losing window focus, releases held keys
but not typed holds; `in_restart` and session changes release both. `+grapple` is
EternalJK's name for `+button12`, the JA+/JaPRO grapple hook; on a JA+ server
releasing it also taps `+use`, as EternalJK does. See
[input.rs](../crates/sjk-viewer/src/input.rs).

`serverconfig` lists a JA+ server's options from the `jp_cinfo` value in its
serverinfo (flip kick, roll fix mode, DFA variants, kata, ledge grab, alternate
dimension and the rest), as the JA+ client plugin and EternalJK print them
locally; on jaPRO/TaystJK it is forwarded to the server, which answers it, and
elsewhere it reports that the server runs neither. `pluginDisable` lists the
fifteen JA+ client-plugin features with `Allowed`/`Disallowed`, and
`pluginDisable <id>` toggles one bit of the archived userinfo cvar
`cp_pluginDisable` (a set bit disables the feature). Its default, 1536, disables
the holstered saber and ledge grab, which need JA+ animations JKR does not have.
JA+ and TaystJK/jaPRO servers receive it from the connect packet on, and a
toggle sends a userinfo update ([networking.md](networking.md)).
See [console_mod_commands.rs](../crates/sjk-viewer/src/console_mod_commands.rs).

`remapShader <old> <new>` draws every surface, model and effect using shader
`old` with shader `new` until the map changes, as EternalJK's command does;
`remapShader <old> <old>` restores it. `listRemaps` lists every remap with its
source (map, server or console), the server's time offset and whether it is in
effect, and `clearRemaps` (EternalJK's renderer command) removes them all until
the server sends new ones. The archived `cg_remaps` (EternalJK's name and default
2) chooses which remaps sent by the server apply: 0 none, 1 all but player-model
shaders, 2 all; the console's and the map's own always apply. Unlike EternalJK,
which latches it, a change applies at once. Settings > GAME has it as "Shader
remaps". See [Shader remaps](rendering.md#shader-remaps) and
[shader_remaps.rs](../crates/sjk-viewer/src/shader_remaps.rs).

The console input line has a caret, drawn as stock's underscore: Left and Right
move it, Ctrl+Left and Ctrl+Right by word, Home and End to either end, and Shift
with any of them selects. Backspace and Delete remove a character, or a word with
Ctrl; words end at anything but a letter or digit, so `cg_drawFPS` and
`127.0.0.1` are edited a piece at a time. Ctrl+A selects the line, Ctrl+X cuts,
and Ctrl+V or Shift+Insert pastes, replacing a selection. Typing inserts at the
caret, a long line scrolls sideways to keep it in view, and Enter runs the whole
line. Dragging the mouse over console output selects it, a double click selects
one whitespace-separated word (a whole `host:port`), Shift+click extends a
selection and a click elsewhere clears it; in the input line the mouse places the
caret and selects the same way. Ctrl+C (or Ctrl+Insert) copies selected output
without colour codes, else the selected input, else the whole input line, or the
last `viewpos` or `mark` answer when the line is empty. Up and Down stay history.
See [console_editing.rs](../crates/sjk-viewer/src/console_editing.rs) and
[console_selection.rs](../crates/sjk-viewer/src/console_selection.rs). The input
line and output rows are drawn and measured through one
[ConsoleText](../crates/sjk-viewer/src/console_text.rs) per text size, so the
caret, highlights and mouse hits follow the drawn glyphs at any size or letter
spacing.

Tab completes the command or cvar name being typed, after a leading `/` or `\` and
after the last `;`. A unique name completes with a trailing space; otherwise the
input extends to the longest shared prefix and the matching commands and cvars,
with cvar values, are listed as EternalJK prints them (`PrintMatches`,
`PrintCvarMatches`): a grey `Cmd` or `Cvar` label, the white name, a cvar's value
in grey quotes and the description in green. Up to 16 matches also show their
descriptions; longer listings end with the match count instead. Typed lines, and
the line a listing completes, are echoed into the scrollback as `]cmd`, the
prompt character straight before the text as in stock. Both are shell behaviour
and the same in either console style. Enter strips one leading `/` or
`\` from the line, then applies the same completion while `cl_allowEnterCompletion`
is set, without listing when the input is already a full name. Nothing strips a
slash on a command after `;`, so completing that command drops it. Command
boundaries follow the shell's quote and escape rules, and no completion happens
inside an open quote. Arguments are not completed. See
[shell_completion.rs](../crates/sjk-shell/src/shell_completion.rs).

F3 in the open console, or the bindable `consolebrowser` command, opens a browser of
every command and cvar with its description, and each cvar's value and default. Typing
searches names, then descriptions; Tab cycles All, Commands, Cvars and Changed (cvars
away from their default). Enter edits the selected cvar in place and applies it, or
starts a console line with the selected command; Delete restores a cvar's default;
Escape cancels an active edit first; otherwise Escape or F3 returns to the console.
The clickable Apply, Cancel and Filter controls follow the same actions as the
keyboard. Read-only cvars are listed but not edited. The
browser covers the whole frame: underlying menu shapes/text, chat and the FPS
counter are suppressed, including both font batches. See
[console_browser.rs](../crates/sjk-viewer/src/console_browser.rs).

## Third-person camera

The third-person camera follows codemp `CG_OffsetThirdPersonView`, with the
`cg_thirdPersonRange`, `cg_thirdPersonVertOffset`, `cg_thirdPersonAngle`,
`cg_thirdPersonPitchOffset`, `cg_thirdPersonCameraDamp` and
`cg_thirdPersonTargetDamp` cvars. The focus pitch, offset included, is capped
at 80 degrees, and fast yaw turns stiffen the camera damping. The target and
the camera then sweep an 8-unit cube against `MASK_CAMERACLIP` (solid, terrain
and player clip) through the world and solid brush entities such as lifts and
doors, as `CG_Trace` does; players do not block it. The prediction-error offset
moves the traced focus rather than the finished camera. `cg_thirdPersonAlpha`
and `cg_thirdPersonHorzOffset` are not implemented; stock multiplayer has no
automatic fade when the camera nears the player. See
[camera.rs](../crates/sjk-viewer/src/camera.rs).

For graphics controls and diagnostics, see [rendering.md](rendering.md).

## Text size and spacing

The Settings screen's TEXT tab holds four archived cvars. Menu text draws as
before at the defaults; the console's rows are closer together than before.

| Cvar | Default | Range | Effect |
| --- | --- | --- | --- |
| `ui_textScale` | 1 | 0.8 to 1.2 | Text size on menu screens and the in-game menu |
| `ui_letterSpacing` | 0 | -0.05 to 0.15 | Extra space after each letter in menus and the console, as a fraction of the text size |
| `con_scale` | 1 | above 0 (menu: 0.5 to 2) | Size of the whole console: text, margins and rows; the classic console's character cells |
| `con_lineSpacing` | 0.9 | 0.8 to 2 | Modern console history and notify row pitch as a multiple of the text size; at 0.8 descenders meet the next row's ascenders |

Menu text grows or shrinks about the centre of its line without moving the
layout, so the range is limited to what menu rows can hold; that style is
applied where retained text commands become glyph quads, see
[text/style.rs](../crates/sjk-viewer/src/text/style.rs). The console sizes its
own text with `con_scale` and puts the letter spacing into its layout
([console_view.rs](../crates/sjk-viewer/src/console_view.rs)), so anything that
measures console text, such as a caret, sees the spacing it is drawn with.
These settings do not affect chat, the scoreboard or the HUD.

Console defaults, compared with stock at 1080p: stock draws 8 x 16 px cells
whose capitals are 14 px tall, so its rows are 16 px apart and nearly touch.
JKR's console text is 14 px Inter: the size is its line box (ascent plus
descent, 1.21 em), with 8.4 px capitals. A pitch of 0.9 times the text size
gives 12.6 px rows at 1080p, where capitals fill two thirds of the pitch and the
deepest descender (`g`) still clears the next row's ascenders and brackets by
about 1 px. Below 0.82 they touch, so the range stops at 0.8; only accented
capitals can overlap the row above there. Rows closer than their line box
overlap: each row keeps its whole text box (and glyph shadow) for drawing, and
the bottom row's text ends at the input separator's margin. Selection bands and
pointer rows stay one pitch tall and centred on the text. Earlier builds read
`con_lineSpacing` as a multiple of a fixed 22 px pitch; such a saved value below
0.8 now clamps to 0.8, the tightest setting. `con_maxLines` defaults to 32 so
the default-height console fills with rows instead of stopping at the old 18.
Letter spacing stays 0: Inter's average advance relative to its x-height (0.89)
is already close to stock's cells (0.8), Inter's own size-specific tracking at
this size is +0.002 em, and tighter text would run digits and `il1` together.
See [console_options.rs](../crates/sjk-viewer/src/console_options.rs).

The chat box's wrapped body rows are one line box apart (18 px at 1080p and
`cg_chatBoxFontSize` 1; they were 27 px). Inter's capitals are 0.60 of that box,
the stock chat box's ratio (`ocr_a` capitals at scale 0.65 in rows 13 virtual
pixels apart, `CG_ChatBox_DrawStrings`), and descenders clear the next row by
0.18 of the box. A sender's name line advances 20 px to the body (was 26) and
messages are 8 px apart (were 10 after a named message, 14 otherwise); see
[chat/layout.rs](../crates/sjk-viewer/src/chat/layout.rs).
