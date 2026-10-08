# Client

Build instructions are in [development.md](development.md). SJK needs an installed
Jedi Academy `GameData` directory with the retail `base/assets*.pk3` files.

## Launch

The client program is `sjk` and the dedicated server `sjk-server` (`.exe` on
Windows). Developer and diagnostic environment variables are named `SJK_*`
(`SJK_TRACE_*`, `SJK_LAMP_*`, `SJK_GPU_*` and the like). On Windows both programs
carry SJK's icon and call themselves "Sol JK" and "Sol JK dedicated server" in
their version information; the client also sets the icon on its window (title bar
and taskbar on Windows, the window icon on X11; Wayland has none). See
[assets/branding](../assets/branding/README.md).

Put `sjk` (`sjk.exe` on Windows) inside the installed game's
`GameData` folder, beside `base/`, then launch it to open the main menu. A shortcut
can use any working directory. No path settings are required. Keep
`sjk-server` (`sjk-server.exe` on Windows) beside the client for Create game
and local `devmap`.

From that folder:

```sh
./sjk
```

Join a server directly:

```sh
./sjk --connect 127.0.0.1:29071
```

Without an explicit positional GameData argument, discovery checks these locations
in order and uses the first containing `base/assets0.pk3` and `base/assets3.pk3`:

1. `JKA_GAME_DATA`, if set and nonempty.
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
directory (`sjk /path/to/GameData maps/mp/ffa3.bsp`); it remains a direct
world/viewer launch, whereas no arguments opens the main menu. Demo playback
retains its explicit GameData argument.
See [launch.rs](../crates/sjk-viewer/src/launch.rs) and
[app_launch.rs](../crates/sjk-viewer/src/app_launch.rs).

The client has a server browser, player and settings screens, in-game menus,
console, HUD, screenshots, demo recording/playback and a Create game flow.
Presence here describes implemented surfaces; validation limits are in
[status.md](status.md).

Create game starts a child `sjk-server`, normally found beside the client.
Set `JKA_DEDICATED` to its executable path if installed elsewhere. The child
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
and in-game menus: `sjk` (default), the SJK UI; `modern` (or `0`); or `classic`
(or `1`), which is close to the retail multiplayer menus in layout and flow
without porting their `.menu` scripts. An unknown value falls back to `sjk`. The
retail 640x480 layout is fitted to the window height and centred.

SJK's own classic pages keep the retail look and add modern help inside it; the
rules are in [Classic+ menus](classic-plus.md).

Before 08/10/2026 SJK's default was `classic`, and `config.cfg` saves every
archived cvar, so every existing profile carries it. The console's start moves a
saved `classic` once to `sjk` and marks the profile (`ui_menuStyleDefaultVersion`
1, archived); a style chosen after that, `classic` included, stays, and a saved
`modern` was a choice and is left alone
([console_session.rs](../crates/sjk-viewer/src/console_session.rs)). The style is
also the first row of [First setup](#first-setup), under Styles with the camera
style.

The SJK UI is SJK's own menus drawn over the live map in SJK's type
([SJK UI](sjk-ui.md)). It has its main page: the emblem in a
turning holo ring with the menu on an arc round it (Play, Character, Settings,
Sol JK and Quit, the first, fourth and last opening pages of their own on the
ring), and the servers joined last in a column on the right, joined with one
click; and its Settings: every category down a lit rail, the rows with switches,
sliders and segments under sub-headings, the focused setting explained on the
right and the search at the top, the key bindings one of its categories; and
its Character, the character standing on the map's path below the tower beside
the form; What's new, Update and Identity in the same look; and its Servers, the
server browser with where the servers come from and which show on the left, the
sortable list in the middle and the chosen server's map, numbers and players on
the right; and its in-game menu, a compact arc of the game menu's entries over a
dark fade on the match's left, with a card of the match (server, map, mode and
limits, score and place, clock, players) on the right. Every other screen opens
in its classic+ version. A client started in the SJK UI stands on mp/duel6
instead of mp/ffa3.

The classic main menu has the retail pages, entries and order, with SJK's
Settings and SJK in place of retail's Controls and Setup:

- Main: Play and Profile on the left, Settings and SJK on the right, Exit below.
  Exit and Escape ask before quitting.
- Play: Solo Game, Join Server, Create Server, Play Demo and Rules. Solo Game
  and Create Server both open Create game, which hosts a local match with bots.
- Settings is retail's Controls and Setup as one screen of option panels with two
  tabs, KEY BINDINGS and OPTIONS (described below). It opens on OPTIONS.
- SJK lists SJK's own screens in the Play page's layout: Changelog, Credits and
  Update.
- Every sub-page repeats the navigation row (Play, Profile, Settings, SJK) and
  has Back and Exit.
- Profile opens the retail profile pages (`player`, `player2`, `saber`), which
  edit the same drafts as the modern Player screen and write them at once:
  - Profile: name, team colour and the head grid (six 64-unit cells per row),
    Custom to character creation, APPLY on to lightsaber creation, Exit. SJK
    sets Custom beside a Force button (retail's in-game `configforce` art) to
    the right of the grid, with the Force profile at a glance under them
    (mastery, side, points left and the holocrons of the powers with a level,
    a pip per level; a Jedi with more powers than fit gets smaller holocrons,
    never a missing one), then JoF EJK's Cosmetics button and what is worn. SJK
    also adds a model search right of Team Color: Enter or a click starts
    typing, and the grid then lists only the models of the team whose
    `model/skin` name (a species' model name) contains every typed word,
    ignoring case, with the count at the field's end; Enter or a click
    elsewhere keeps the search, Escape clears it, and it is cleared when the
    screen opens. The description line names the model under the pointer.
    Retail's Character Model, The Force and Saber bars overhung their boxes by
    two units (the in-game one also stood above its box with its title off
    centre); SJK sets each flush on its box with the title centred, and the
    in-game APPLY on the bottom band it sits on.
  - Character creation: species, skin tint swatches, the Head, Torso and Legs
    lists, Back and APPLY. Entering it from an ordinary character puts on the
    first species, as retail's Custom did.
  - Lightsaber creation: saber type, the hilt list (two for Dual Sabers; in
    every menu style the hilts come in the game's load order, as EternalJK and
    JoF EJK list them: the highest-priority pk3 first, each file's hilts in
    order, not sorted), the
    six blade colour swatches (two rows for Dual), Apply, and Apply back to the
    main menu. SJK adds JoF EJK's custom colour: a red, green and blue slider
    per saber (`ingame_saber.menu`'s "RGB Color Creation"), right of the live
    sabers in the lower box under a "Custom color" heading with a chip of the
    colour (framed white while it is the saber's own), or under each saber's
    swatches in the in-game window. The sliders show the blade as it is drawn,
    a stock colour's own tint included; a click or drag on a bar, the wheel
    over it or Left/Right (steps of 5) makes the colour the saber's own:
    `color1`/`color2` 6 (`SABER_RGB`) and the tint in `cp_sbRGB1`/`cp_sbRGB2`
    (`r | g << 8 | b << 16`), as JoF EJK's `UI_UpdateSaberColor` writes them.
    A swatch brings a stock colour back.
  - Force: retail's in-game `ingame_playerforce` window, here on both
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
    side's five beside them, each with its holocron (`gfx/mp/f_icon_*`), whole
    whenever the power can be bought (a team power outside team games, or
    Saber Defend or Throw without Attack, keeps its holocron at 55% and its
    stars a readable grey, where retail's near-black hid their costs), and
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
  - Cosmetics (JoF EJK's `ingame_cosmetics`): the installed hats and
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
  until the first preview frame is drawn. Lightsaber creation shows the
  sabers alone in the band under its boxes, as retail spun the bare hilt
  model (`isSaber` items, `Item_Model_Paint`): lit, laid on their side and
  turning about their length at retail's `model_rotation 20` (50 degrees a
  second, from `model_angle 180`), each centred on its whole length so a
  staff fills the band as a single saber does, the second of a pair below
  the first; a drawn hilt and blade stand in until the first frame. See
  [model preview](rendering.md#classic-model-preview). The part
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
  FILTER (retail's mod filter row is the client's text filter over names and maps;
  `/` or a click starts typing, Escape ends), TYPE (game-type filter), and the
  VIEW EMPTY, VIEW FULL and VIEW LOCKED toggles (the archived
  `ui_browserShow*` cvars; VIEW LOCKED stands where retail's data rate was).
- The list: SERVER NAME, MAP NAME, PLYRS, TYPE and PING columns over retail's
  row bands and column frames, ten 26-unit rows, the sorted column filled and
  its header white with a ^ or v for the direction. Clicking a header sorts by
  it, again reverses it. TYPE adds JA+, JAPRO or MOD where the client detects the
  server's mod, and `*P` for a locked server; favourites carry a gold `*`.
  The wheel and the scrollbar along the right edge scroll the list.
- The secondary row: CONNECT IP (where retail had NEW FAVORITE; the client's
  direct connect), ADD FAVORITE or DEL. FAVORITE for the selected server, and SERVER
  INFO, a pop-up of the server's published settings and players (Escape or a
  click closes it). PASSWORD and FIND PLAYER are dimmed: the client asks for the
  password when a locked server is joined, in a retail-style prompt, and has
  no player search yet.
- BACK returns to the Play page, EXIT to the quit page (not shown when the
  browser was opened from the game menu), JOIN joins the selected server (a
  double-click on a row too).

The status line under the list shows the fetch progress where retail showed
the refresh time, and the description line shows the hovered item's
description. The screen is in
[menu/classic/browser.rs](../crates/sjk-viewer/src/menu/classic/browser.rs).

Retail entries SJK has no screen for yet (Play Demo, Rules) are shown dimmed,
and their description line says so; retail's Mods and Defaults are left out of
Settings (Backspace restores one setting's default).

Settings' two tabs sit on the panel's title band, KEY BINDINGS on the left and
OPTIONS on the right; a click switches, and Tab walks through the groups of both
tabs in turn. Each tab keeps its group list down the left and shows the chosen
group's items in the panel beside it, opening on First setup (OPTIONS) and Movement
(KEY BINDINGS). The items are the same settings and key bindings as the modern
screens, laid out the retail way (labels set against a column at retail
`textalignx`, the value after them, the open group's entry in white) with
classic+'s own controls, below. A group with more items than the
panel holds scrolls: the wheel over its items moves the list one item per notch
(three on a key-binding group), a thin bar along the panel's right edge shows the
position and can be dragged, and Up and Down keep the selected item in view.

SJK makes these panels [classic+](classic-plus.md): a search field and the items
fill the panel's upper part (10 rows on the main menu, 8 in game, where retail's
taller panel held 15 and 12) and a detail box under them describes the focused
item. For a setting
it shows the full name and value, what the setting does
([help.rs](../crates/sjk-viewer/src/settings/help.rs), one or two lines for
every setting), its default, its range (or how many choices it has), "applies
after a restart" or "applies on the next map", and its console name. For a key
binding it shows the keys, the console command, which other actions those keys
also do, and the default key. Row labels drop their bracketed notes and their
"(restart)" or "(next map)", which the detail box says instead; a setting that
applies later has a gold `*` after its label, and a row changed from its default
(a binding off its default key) has a gold dot after its label. Backspace, the
right button or the focused row's gold reset arrow (at the row's right end)
returns the setting to its default (resolution and display mode excepted). The
description line names the keys of the focused row, and the detail box's title
carries the icon of the setting's group (or of a binding's category).

Classic+ draws the items with its own controls on retail's blue and gold
(Sol's request, 07/10/2026; [classic-plus.md](classic-plus.md)), instead of
retail's YES/NO text and `menu/new` slider art:

- Labels are in sentence case ("Menu text size"), in retail's option blue; values
  are brighter. The focused row's label turns white over a soft band with a gold
  bar at its left end. Headings, tabs and group names keep retail's capitals.
- An on/off setting is a switch: a pill, gold with its knob right when on, dark
  with its knob left when off, and On or Off after it. A click on its row, Enter,
  Space, Left or Right flips it.
- A number is a slim rail filled gold up to a round knob (ringed while the row is
  focused), with its value centred in a small frame after it. Clicking or dragging
  along the row sets it, Left and Right step it, typing or Enter edits it exactly.
- A choice of two or three values (menu style, menu contrast, console style,
  scoreboard style) shows them all side by side, the one in use filled gold; a
  click on one sets it, Left and Right step.
- A longer list (the menu accent), the display mode, the resolution and the HUD
  show their value in a field with a small caret; a click or Enter opens the list.
  A text setting shows its text in a field, typed in place.
- The longer groups are divided by sub-headings: Game options (items and models,
  effects, camera and prediction), Interface (menus, console), HUD (layout,
  crosshair, names and cards, nameplates, readouts) and First setup (styles,
  display, aim, sound, HUD, nameplates, Force shells, SJK).

Retail split video and the Force binds over two pages because a page held few
items; classic+ panels scroll and explain the focused item, so SJK shows each as
one group and regroups the modern GAME, HUD, HUD+ and TEXT tabs by subject
([settings/groups.rs](../crates/sjk-viewer/src/settings/groups.rs)):

- OPTIONS lists First setup, Graphics, Sound and Gameplay (Sol's grouping,
  06/10/2026). First setup is the FIRST SETUP tab's rows as a group (see
  [First setup](#first-setup)); Sound is AUDIO. Graphics and Gameplay open pages
  of their own in the same layout, their groups down the left, the OPTIONS tab
  still marked, and Back (or Escape) returning to OPTIONS; in game they are the
  same pop-up.
  - Graphics: Video is the whole VIDEO tab (resolution, display, frame rate,
    field of view, marks, shadows, gamma); Image, Lighting and Shadows are the
    renderer settings' tabs; Weather is the renderer settings' WEATHER tab
    (weather, density, quality, forced weather, ground fog, clouds; see
    [Weather](rendering.md#weather)).
  - Gameplay: Mouse is the CONTROLS settings (retail's Mouse/Joystick, moved
    here from the key bindings). Game Options holds the gameplay rows (simple
    items, forced models, saber and speed trails, aura shell, shader remaps,
    third-person camera style and damping, prediction smoothing). Interface
    gathers the menus' and console's look (menu style, accent, contrast, game
    fonts, menu text size and
    spacing, console style, text size and line spacing); HUD the HUD style, files
    and scale, status, weapon bar, crosshair and its size, names, nameplates,
    timer, speedometer, team overlay, lagometer, chat and ground readout;
    Scoreboard its style, client numbers, head icons and small rows. Network
    follows.
- KEY BINDINGS: every binding is in one list, under the headings Movement,
  Interaction, Weapons, Force powers (retail's two Force pages as one) and Other.
  The categories down the left jump to their heading, and the one holding the
  selected action is marked as the list scrolls. The weapon rows name their
  weapon (Saber / melee, Blaster pistol, ... Explosives) rather than `weapon N`.

Every group down the left (both tabs, Graphics and Gameplay, main menu and in-game
pop-up) has an icon at the left end of its row, opposite the label: SJK's own grey
metal discs in the quick wheels' style, generated by Sol
([assets/settings](../crates/sjk-viewer/assets/settings/README.md),
`Entry::icon`). The open or hovered group's icon is at full strength, the others
dimmed.

The search field on the panel's first row finds across the whole tab. `/`, Up
from the first row, or a click gives it the keyboard; typing filters as it goes,
with the number found at the row's end; Enter, Down or Tab go to the results and
Escape clears the text, then leaves the field (an Escape in the panel clears a
search before it leaves). On OPTIONS it searches every option of every group,
the renderer's included, by name, console name, description and group
([settings/search.rs](../crates/sjk-viewer/src/settings/search.rs)), and shows
the results under their groups' headings, the detail box adding "In <group>";
opening a group clears the search. On KEY BINDINGS it finds actions by name,
console command, category or bound key (`space` finds Jump).

The two tabs' searches reach each other: while text is typed, the description line
says how many entries of the other tab match it ("3 key bindings match too: click the
KEY BINDINGS tab"), and a search with no result here names them in the list. Clicking
the other tab (or Tab, or `[` and `]`, across the tab boundary) carries the search text
to it, so one query walks every setting and binding
([menu/classic.rs](../crates/sjk-viewer/src/menu/classic.rs), `sync_cross_search`).

A list (a choice of more than three values, the display mode) does not change
on a click or Enter: they open a dropdown under the value, a rounded list over a
soft shadow with the value in use in gold and marked with a dot, and nothing
changes until a choice is applied with Enter or a click. Up and Down move in it;
Escape, Backspace or a click elsewhere close it unchanged. Left and Right still
step the value directly. The rows the dropdown covers leave their values out
while it is open, since text draws over every shape.

Key bindings carry the retail HUD picture of what they select or use
([keybind_editor/icons.rs](../crates/sjk-viewer/src/keybind_editor/icons.rs)):
each weapon's `gfx/hud/w_icon_*`, each Force power's holocron
(`gfx/mp/f_icon_*`), the items' `i_icon_*` (bacta, seeker, sentry, force field,
binoculars), the saber toggle and style, and the team menu's flag. They sit in
a column between the labels and the keys and, larger, in the detail box. They
are read from the player's game data on a worker thread when the key bindings
first open, into two atlas rows of their own; a picture the game data lacks is
left out.

Up and Down move through the items, Left and Right change a value, Enter flips a
switch or opens a list's dropdown, and typing or Enter on a number edits it exactly; clicking
a slider sets it. Tab moves to the next group (on KEY BINDINGS, Left and Right
do too). A key binding reads "A OR B" (retail's `KEYBIND_OR`,
raised to capitals with the key names as retail's `BindingFromName` does) or `???`
when unbound; Enter or a click waits for the new key, shown in red with
retail's "Enter new key, or ESC to cancel, BACKSPACE to clear.", and Backspace
clears every key of the action. Escape closes the page to the main page (the
Graphics and Gameplay pages to OPTIONS).

The classic in-game bar's Settings opens the same panels as retail's
`ingame_setup` and `ingame_controls` pop-ups, with the two tabs on the pop-up's
title band: a box under the bar with the group list and panel at their in-game
positions and no navigation row, closing back to the bar. Switching the Menu style (on Interface) while a panel is
open continues on the modern settings screen.

The classic in-game menu (Escape during a match) is the retail top bar: About,
Join, Profile, Add Bot, Settings (retail's Controls and Setup), Vote, Call Vote
and Exit, after SJK's own narrower SJK button at its left end. Each opens a pop-up under it or the matching screen. SJK's pop-up holds
SJK's own screens, for now the [changelog page](#changelog-page)
([ingame_menu/sjk.rs](../crates/sjk-viewer/src/ingame_menu/sjk.rs) lists them);
the modern game menu has the same pop-up as its SJK row, before Server info.
About shows the server info. Join picks
a team, or opens the class list in Siege; in a team game Team Red and Team Blue
carry their flag (`gfx/hud/mpi_rflag`, `mpi_bflag`) and the team's player count. Vote is Yes/No. Call Vote opens the
call-vote lists. Exit offers Main Menu, Restart Match and Quit Program, each
with a Yes/No confirmation. Profile opens the retail in-game profile window
(`ingame_player`: name, team colour, head grid, Custom, Saber and the Force
summary, then `ingame_player2` and `ingame_saber`); its Apply returns to the
match. The Force box shows the side's emblem beside retail's mastery, side and
points lines and the known powers' holocrons, and its `configforce` button
opens the Force window; the Cosmetics button sits under Custom, as in JoF EJK.
Its Join Red, Join Blue and Spectate buttons are left to the Join tab. Settings opens
the option panels described above. Siege swaps in Objectives and V Chat as retail does. Add Bot,
Objectives, V Chat and Restart Match are dimmed with a note, because the client
cannot add bots or restart a match it does not host. Left and Right move along
the bar; Escape closes a pop-up, then the menu. The Server browser and Shot
controls entries, which retail did not have, are in the modern style and the SJK
UI only.

With the player's retail game data mounted, the classic menus draw its own
artwork: the backdrop, side glyph columns, ring, windows, logo, sub-page frames,
button glow, list glow (`menu_buttonback2`), slider bar and thumb
(`menu/new`), in-game bar and pop-up boxes from `gfx/menus`. The art is decoded
once on a worker thread the first time the classic style is used, and the UI
renderer uploads it into one texture per image, separate from the shared UI icon
atlas. Its bind group changes only between draw runs that need a different
texture, so layer order is kept. Additively blended retail images (glow, title
band, bar) are converted to alpha at decode time. A missing image falls back to
the client's own shapes. Retail assets are never bundled.

The art moves as retail's shaders move it (`shaders/ui.shader`); the `.menu`
scripts themselves only swap pages at once and show or hide the glows. Retail's
main page plays `video/ja01` (`gfx/menus/videologo`) in its ring; SJK shows its
own emblem there instead and does not read the video: the gold
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
over the retail background (the main page's emblem fills the centre gap,
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
`load_tick_cap`) has retail's nine ticks; the client lights them from its own load
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
- Retail option items SJK has no setting for (video quality presets, colour
  depth, geometric and texture detail, EAX, languages) are left out of the
  panels, and the video restart confirmation is not needed.
- On the profile pages: portraits for every model (the atlas holds 207, so
  species after the characters show none).
- The screens with no SJK equivalent yet: Play Demo (`demo`), Rules
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

The Character page's grid (and the classic head grid) can list far more models
than those 207 cells: an installation with community packs lists over 900. The
cells are therefore a cache ([model_icons.rs](../crates/sjk-viewer/src/player_menu/model_icons.rs)):
a tile asks for its icon when it is drawn, a worker thread decodes it (without
the shared image cache, so the whole catalogue is never held at full size) into
a free cell or the one drawn least recently, and tiles on screen keep theirs.
Before SJK did this, every model past the 207th was a black tile. A model whose
icon file cannot be decoded shows its name in the tile instead. Tiles answer to
the pointer by their place on screen, so a long list never runs into other
controls' pointer tokens. The Search row under Team colour (Enter to type)
filters the grid as the classic profile's search does (see above).

## Player models

A player's or NPC's appearance (`models/players/<model>/<skin>`) loads its
`model.glm`, the skeleton the mesh names (`<name>.gla`), that skeleton's
`animation.cfg` and a skin; if the model cannot be loaded or built, the client
draws Kyle for that player instead
([player_assets.rs](../crates/sjk-viewer/src/player_assets.rs),
[actor_load.rs](../crates/sjk-viewer/src/actor_load.rs)). That includes a model
changed during the match (`/model`, or a new player in a used slot), as
`CG_LoadClientInfo` falls back to `DEFAULT_MODEL`: the player is not left in
their previous model, and the log says `client N Kyle in place of <model>`. A
model that failed is not loaded again until the next map
([clientinfo_refresh.rs](../crates/sjk-viewer/src/clientinfo_refresh.rs)). Files are read as
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
([cosmetics.rs](../crates/sjk-viewer/src/cosmetics.rs)): every player wears
whichever hat and cape they like. A hat is any `.md3`
in `models/cosmetics/hats/`, a cape any in `models/cosmetics/capes/`
(`models/players/hats/` and `capes/` when the new folders are empty, as older
packs used them); names of JoF's catalogue are found in either. A name is at
most 13 letters, digits, `_` or `-` and does not start with a digit. JoF's
launcher installs its pack (`zzz_jof_cosmetics.pk3`) in the `EternalJK` folder:
SJK mounts every PK3 there that has models in `models/cosmetics/hats/` or
`capes/`, or JoF's client pictures in `gfx/jof/` (`jofclient-assets.pk3`, which
also holds the sounds JoF servers play, see [Force wheel](#force-wheel)),
below all other content, without mounting the rest of that folder
(its menus, HUD and strings come only with `fs_basegame EternalJK`, see
[Player models](#player-models)); the log names each pack. A pack in `base`
works too ([asset_search_paths.rs](../crates/sjk-viewer/src/asset_search_paths.rs)).

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

Every installed hat and cape is open to everyone: nothing is unlocked or
granted by a server. jaPRO's race-unlock hats (the `c5` clientinfo,
`cp_cosmetics`, `cosmetics unlocks`) are left out on purpose.

## EternalJK animation fixes

Players' animations are remapped before they are shown, as EternalJK's `CG_Player`
does: without a saber in hand, two-handed, dual and staff runs and walks play as the
ordinary ones; the old Bryar's `BOTH_STAND1` (servers without `g_fixWeaponAttackAnim`,
such as JoF's, fire it that way, arm hanging between shots) shows as `BOTH_ATTACK2`
with the arm raised; the concussion rifle's `BOTH_ATTACK2` shows as `BOTH_ATTACK3`.
NPCs are left as the server sends them.

## Wall grab facing

A player grabbing a wall to jump off it (Force jump 3, pushing into a wall in the
air: `BOTH_FORCEWALLREBOUND_*`, then `BOTH_FORCEWALLHOLD_*` while the hold lasts)
is drawn facing the wall the way the animation needs, whatever their view does
([wall_hold_pose.rs](../crates/sjk-viewer/src/wall_hold_pose.rs)). `CG_G2PlayerAngles`
turns the body with the view yaw. A stock server turns that yaw to the wall during
the hold (`PM_AdjustAngleForWallJump`), so the camera locks too, but a JA+ server
leaves the view free there ([networking](networking.md#server-dialect-movement-rules)),
and looking around turned the body off the wall, as if it floated. While the
legs play a rebound the model's yaw is the stock server's facing instead
(`vectoyaw(wall normal) + yawAdjust`, from the same 128-unit check along the
view, [`wall_hold_yaw`](../crates/sjk-game-jka/src/pmove_wall_moves.rs)); a frame
whose check misses keeps the rebound's last facing, and the kick off returns the
body to the view. It applies to the local player (from the predicted state) and to
other players alike. Only the model changes: the camera stays free on JA+ and
locked on a stock server, as each server computes it, and the kick off still goes
straight back from the side the view picks (`checkDir`), so on JA+ it follows
where the player looks, not the wall. The check sees the world's brushes only: a
wall on a mover keeps the body on the view.

## Dismemberment and disintegration

Both follow EternalJK's cgame (`cg_ents.c`, `cg_players.c`) and rd-vanilla's renderer.

- **Cut-off limbs.** A server with `g_dismember` above 0 sends each cut limb as its
  own entity. With `cg_dismember` 1 (no heads or waists) or 2 and up (everything) the
  client copies the owner's model for the limb, draws only the limb and its cap, and
  turns the limb off on the owner with the stump's cap on; a cut right arm, right hand
  or waist takes the weapon with it. Both cuts smoke, and a flying limb trails smoke.
  A body left behind keeps the missing limbs; the player is whole again once alive.
  `cg_dismember` defaults to 2 (EternalJK: 0), so a server that enables
  dismemberment shows every cut limb (0 shows none, and a config that saved 0
  keeps it). `g_dismember` (0 to 100) set in the
  console is handed to games this client hosts (Create game and `devmap`).
- **Disintegration** (`EF_DISINTEGRATION`): a disruptor kill, or a corpse shot or cut
  until it gives way, freezes the pose and burns the body away from the hit point:
  the model is eaten away with a blackened edge and `gfx/effects/burn` glows on what
  remains, with `disruptor/death_smoke` for the first second. A player is gone after
  1.5 s; a body draws until the server removes it. The stage program does rd-vanilla's
  per-vertex work (`RB_CalcDisintegrateColors`, `RB_CalcDisintegrateVertDeform`).
  It is always on, with no cvar, as in EternalJK and OpenJK (`CG_Player` checks only
  `EF_DISINTEGRATION`). Its colours are final: real-time sun light and fullbright
  leave them as they are, as the stage's colour generators do (`entity_control.x` 3).

Model cap surfaces (`*_cap_*`) are carried in every actor mesh, hidden until a cut.

## Third-person camera

Third-person framing follows OpenJK multiplayer `CG_OffsetThirdPersonView`.
The camera uses a four-unit collision hull against solid/terrain/player-clip
surfaces and presented inline models, including moving doors and platforms and
the permanent ones the server sends only in its baselines, such as the `misc_bsp`
map pieces a server places in a map's void: stock `CG_BuildSolidList` adds them
from `cg_permanents` (`cg_predict.c`), so the camera stops at their walls. The
crosshair name trace uses the same list, so such a wall hides a player behind it.
Player and vehicle bodies do not obstruct this camera trace. When geometry
collapses the camera onto its target, the view uses the intended forward direction
instead of constructing an undefined look-at matrix.

Pitch limits, pitch-offset direction and turn-dependent damping follow the
multiplayer reference. How fast the camera closes the gap follows EternalJK's
`cg_cameraFPS` (default 125, as EternalJK): the damping cvars apply per frame of
that rate whatever the real frame rate, and the ideal point's own movement is
compensated (EternalJK `CG_DampPosition`), so the camera stays close behind a
fast-moving player. As in EternalJK the damping is timed by the predicted
player's command time, the clock the focus moves on; timed by the presentation
clock it stuttered. Stock multiplayer applies them once per 50 ms step, against
8 ms per step at the default 125, so the same damping value closes the gap in
steps 6.25 times shorter; `cg_cameraFPS` below 15 restores the stock behaviour.
View changes, teleports, followed-player changes and mounting/dismounting reset
the presentation history. The ordinary range, height, angle and damping cvars
remain available; `cg_thirdPersonHorzOffset` controls the stock sideways offset.
`cg_cameraStyle` chooses whether the damping applies ([Camera style](#camera-style)).

The decaying prediction error (`cg_errorDecay`) moves the third-person camera's
focus before its collision and damping traces, as `CG_CalcViewValues`
(`cg_view.c`) adds it to the view origin before `CG_OffsetThirdPersonView`. A
correction next to a wall is therefore traced like any other focus movement
instead of shifting the finished camera into the wall. First-person and other
views still add it to the finished view origin.

Vehicle appearances cache their `.veh` camera settings when loaded. Mounted
views use the authored range, height, pitch and sideways offsets, including the
pitch-dependent and fighter-strafe adjustments. Vehicle targets follow immediately
and camera damping follows the stock vehicle policy. These are presentation
changes; movement, vehicle physics and protocol serialization are unchanged.

The locally piloted vehicle is drawn at the same predicted origin and angles as
its movement, following `CG_AddPacketEntities` and `CG_CalcEntityLerpPositions`.
Its rider seat uses that same root with the current animated driver bolt. This
avoids the camera advancing ahead while the mount and rider trail behind on
snapshot interpolation. Remote vehicles, passengers without local vehicle
prediction, and demo playback retain their snapshot presentation. The camera's
animated seat is evaluated every rendered frame. Prediction-error smoothing uses
the vehicle root while piloting, avoiding false corrections from rider animation.
The movement collision adapter excludes the piloted vehicle's own snapshot body
and its owned objects, matching the stock skip/ownership rules; dismounted and
foreign vehicles remain solid.
Snapshot replay also retains the same ride's local vehicle timers, including turbo
expiry and recharge, while refreshing its networked state. A different vehicle,
pilot or definition starts with fresh local state. This prevents exhausted boost
input from predicting a new burst after every snapshot.

The decaying prediction error moves the camera's focus before the
collision traces, as `CG_CalcViewValues` adds it to the view origin before
`CG_OffsetThirdPersonView`, rather than shifting the finished camera, which could
put it inside a wall. `cg_thirdPersonAlpha` is not implemented; stock multiplayer
has no automatic fade when the camera nears the player. See
[camera.rs](../crates/sjk-viewer/src/camera.rs) and
[camera_motion.rs](../crates/sjk-viewer/src/camera_motion.rs).

### Camera style

`cg_cameraStyle` (archived; Settings > Gameplay > Game options, "Camera and
prediction", and First setup's second row, under Styles) picks how the camera
follows the player ([camera_style.rs](../crates/sjk-viewer/src/camera_style.rs)):

| Value | Camera |
| --- | --- |
| `ejk` (default) | Sits at its ideal place every frame: no camera or target damping, whatever the damping cvars say. |
| `sjk` | Eases towards its ideal place with `cg_thirdPersonCameraDamp` and `cg_thirdPersonTargetDamp`, as described above, so it trails a moving or turning player a little. |

An unknown value selects `ejk`. `ejk` became the default on 08/10/2026 (Sol's
choice; `sjk` was the default before). Every profile had saved `sjk`, so the
console's start moves a saved `sjk` once to `ejk` and marks the profile
(`cg_cameraStyleDefaultVersion`); an `sjk` picked after that stays.

`ejk` is JoF EJK's camera as Sol plays it (07/10/2026). The camera code is the same
in both clients: EternalJK's damping with `cg_cameraFPS 125`, range 80, height 16
and damping 0.3 and 0.5 are SJK's defaults too. But EternalJK switches both
dampings off while a strafe helper style is drawn (`cg_strafeHelper` bits 0-3 or
13; `CG_UpdateThirdPersonTargetDamp` and `CG_UpdateThirdPersonCameraDamp` in
`cg_view.c`), and Sol's JoF EJK profile has one on (`cg_strafeHelper 2242`), so
there the camera never trails. Range, height, angles, collision, vehicle framing
and the first/third person choice are the same in both styles; Sol's JoF EJK
profile also sets `cg_thirdPersonRange 100`, which SJK leaves to that cvar. SJK's
`sjk` style does not apply EternalJK's strafe helper rule.

## Animation sounds and voice variants

Footsteps and authored swing/spin sounds follow the evaluated lower/upper Ghoul2
frames and the model's `animevents.cfg`, including the shared skeleton table and
`include` directives. The client reads supported sound assets during appearance
loading and queues decoding on the audio worker; ordinary frame playback performs
no file reads. A world build shares the skeleton and event assets across matching
appearances. Active actors release their prefetched encoded bytes after registration.

Ground-contact events trace beneath the animated foot and select the authored
walk/run sound bank for the surface material. Ground the map gives no material
(no `MATERIAL_*` bits, which codemp plays as stone) takes its bank from the hit
shader's file name in SJK (`footsteps::material_from_name`: sand but not sandstone,
snow, grass, gravel, mud, dirt, carpet, wood or plank): retail leaves much of its
sand untagged, such as `mp/siege_desert`'s `siege/siege2sand`, where Sol asked for
sand steps (world note, 07/10/2026). The collision trace reports the brush side's
or patch's shader for this (`CollisionTrace::shader`). `cg_footsteps 0` mutes them. Frame
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

SJK's own cvars have engine names: rendering ones are `r_*` (`r_sceneHdr`,
`r_toneCurve`, `r_sceneBloom`, `r_superSample`, `r_actorSunShadows`,
`r_sunShadow*`, `r_dayNight`, `r_liveLighting`, ...), the ground HUD is
`cg_groundHud` and the dedicated server's are `g_npcNav` and `g_stockRules`.
Names rend2 or EternalJK use with another meaning are avoided.

These rendering cvars have their own settings page. The last row
of Settings > VIDEO, "Renderer", opens it, as JoF EJK's advanced renderer page
opens from its Video setup; Escape or Back returns to that row. With the
classic menu style its tabs are groups of the Setup page's GRAPHICS page (in the
main menu and the in-game pop-up), after Video. The page has four tabs:

| Tab | Settings |
| --- | --- |
| IMAGE | HDR scene and exposure, eye adaptation (`r_autoExposure`) and its range in EV, filmic tone curve, bloom, dynamic glow (`r_DynamicGlow` 0-3) and its blur style (`r_dynamicGlowStyle`), FXAA, supersampling (`r_superSample`), soft particles, sunbeam dust (`r_dustMotes`), per-pixel model lighting, reflection probes (`r_cubeMapping`), floor mirrors (`r_floorReflections`), parallax mapping (`r_parallaxMapping`) and its depth (`r_parallaxStrength`), emission maps (`r_emissiveMaps`), their strength (`r_emissionStrength`) and glow halo (`r_emissiveGlow`) |
| LIGHTING | Sun and sky (`r_dayNight`), live lighting tier, time of day, day length, sunlight brightness, ambient fill and its corner shading, indirect boost, emission-map lights (`r_emissiveLights`), light shafts (`r_volumetrics`) and their clarity |
| SHADOWS | World and character sun shadows, shadow resolution, sharp and close cascade distances, filter taps, slit closing, contact shadows |
| WEATHER | Weather (`r_weather`), its density (`r_weatherDensity`), quality (`r_weatherQuality`), forced weather (`r_weatherForce`), ground fog (`r_weatherFog`) and volumetric clouds (`r_clouds`); see [Weather](rendering.md#weather) |

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

## First setup

The modern settings screen's last tab, FIRST SETUP (called Quick setup before
06/10/2026)
([quick.rs](../crates/sjk-viewer/src/settings/quick.rs)), gathers the settings worth
choosing on a first start: under a Styles heading, the menu style (SJK, Classic
or Modern) and the camera style (EJK or SJK, [Camera style](#camera-style));
then resolution, display mode, vsync, field of view, mouse
sensitivity and inversion, always run, effects and music volume, the HUD look and
scale, the crosshair, the nameplates and their bars, Force bar and power icons, the
Force aura and the combined Protect+Absorb shell, and the update and identity
opt-ins, then its own "Don't show at start" row (`ui_hideFirstSetup`), ending in a
Key bindings row. Its other rows are the catalogue's own, looked up by cvar, so a
change there is the same change the other tabs make; it is the last tab so the
other tabs keep their numbers.

With the classic menus the same rows are the first group of the
Setup page, FIRST SETUP (`Group::Quick`, [groups.rs](../crates/sjk-viewer/src/settings/groups.rs)),
drawn as a classic+ option panel like the others, with search, descriptions and
defaults; the SJK UI shows them as the first category of its Settings, and the
modern style as the FIRST SETUP tab. Picking another menu style on its first row
keeps First setup on show in the new style: the classic Setup page's FIRST SETUP
group, the SJK UI's First setup category or the modern FIRST SETUP tab (not
Interface, which also has the row).

At every start, the first time the main menu is up and the menu style is known, it
opens in the active style, until the player ticks "Don't show at start"
(`ui_hideFirstSetup` 1, archived); Escape leaves it for that run. (Before
07/10/2026 it opened once, on the first start, marked by `ui_quickSetup`, which is
no longer read.) The `firstsetup` console command opens it too, over the main
menu or from a running game, in the active style
([first_setup.rs](../crates/sjk-viewer/src/menu/first_setup.rs)). Its old name
`quicksetup` was removed on 08/10/2026, with no hidden alias, so `quit` is the only
command a `q` completes to. Not yet run in a game window.

### Importing from another client

A `.cfg` file dropped on the window (winit's `DroppedFile`), at any time, opens the
Import page over whatever is on show
([config_import.rs](../crates/sjk-viewer/src/config_import.rs),
[config_import_panel.rs](../crates/sjk-viewer/src/config_import_panel.rs)); the
First setup screen says so. `firstsetup import <path>` does the same from the
console (a path's words may be left unquoted; inside quotes a backslash escapes,
so quoted Windows paths need forward slashes), and `firstsetup import` alone
explains how. The file is read, never run: only `seta`/`set`/`sets`/`setu`, a
bare `name`, `model` or `cg_fov`, `bind`, `unbind` and `unbindall` count, `//`
comments are skipped, and a file that is not UTF-8 is read as Latin-1, as the
legacy game wrote names. Files over 1 MB, other extensions, and configs holding
none of the parts are refused with the reason.

The page lists the parts found, each ticked at first: Name (`name`), Model
(`model`, with `char_color_red`/`_green`/`_blue` when the file sets them), Field
of view (`cg_fov`) and Key bindings. Space, Left/Right or a click ticks a row;
Enter copies the ticked parts into the profile and shows what changed (also logged
to the console). A file that clears every binding first (`unbindall`, as a saved
`jampconfig.cfg` does) replaces the player's table: the keys the binding editor
locks keep their binding, and SJK's own actions get their default key back when
the file left both the action and its key free. A file without `unbindall` only
adds its keys. Bindings on keys SJK does not know are left out and counted. The
page is drawn like the Update page in both menu styles.

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

Pressing anything else while a numeric draft is open applies the draft when it
is a valid number and discards an invalid one, and integer sliders round a typed
fraction (142.6 becomes 143).

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
and press Enter), then press a key or mouse button, or turn the mouse wheel to
bind `MWHEELUP` or `MWHEELDOWN` (a notch only scrolls the list when no slot is
waiting; trackpad deltas under half a notch do not bind). A key already bound
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
other movement input while that keyboard catcher is active. As in EternalJK, so
does a window you alt-tab away from or minimise, so others see you are away:
`cl_unfocusedChatbox` and `cl_minimizedChatbox` (archived, both 1) turn that off. Players carrying
the talk flag have a chatbubble over their heads; the connection-trouble icon
takes priority when the server marks a lost connection. These are upright frame
billboards using the existing [sprite orientation](rendering.md#billboard-icons).
Their texture opacity is preserved near walls even with soft particles enabled.
Your own bubble is visible in third person, not the first-person view. Mind-tricked
players, NPC talk flags and intermission do not show talk balloons. The balloons,
pickup icons and hook ropes have their own share of the effect particle pool, so a
scene full of effects no longer hides every balloon at once. Siege voice command
icons remain unimplemented. See
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
gameplay notices keep their separate HUD presentation. As `CG_DrawCenterString`
does, a centre-print row longer than 50 characters wraps at its last space; retail
counts bytes of its one-byte code page, so the limit counts characters, and a name
with `×` or `é` (two bytes in UTF-8) in a duel challenge wraps like any other. The scoreboard continues
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

### Timed player peek

`/peek <client ID|name> [seconds]` temporarily follows a received player from
behind, with wall collision, as JoF EJKSol does. The default is 5 seconds,
maximum 60. Names ignore colours/case; an unambiguous fragment also works.
Quote names containing spaces, for example `/peek "Reborn Master" 5`.
Empty or colour-only queries are rejected. As in JoF, scope and crosshair
overlays still use the local player aim. Monitoring only received entities
respects server PVS, but may show surroundings outside your line of sight.
`/peek` alone uses the crosshair player; `/peek off` or a duration of 0 cancels.
Local flight (`fakenoclip` or `freecam`) ends a peek; stop flight before peeking.
The local body stays at its player position and the view gun is suppressed.
The camera cancels when the timer ends, the target slot/name changes, or the
map changes. Missing snapshot data pauses the view; it resumes if data returns
before expiry. Players outside the
received snapshot cannot be watched. Player input and server authority remain
unchanged. The command is bindable under Settings > Key bindings > Other.

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

`cg_scoreboardStyle` (Settings > Scoreboard > "Scoreboard style") picks the
scoreboard layout:

- `auto`, the default of a new profile: the SJK UI's scoreboard while
  `ui_menuStyle` is `sjk`, the classic one with any other menu style (what SJK
  showed before the choice existed);
- `sjk`, the SJK UI's scoreboard whatever the menus
  ([SJK UI](sjk-ui.md#scoreboard)): columns floating over the darkened game in
  the UI's type, the teams side by side, the duelists as facing cards;
- `classic`, the retail scoreboard as EternalJK-derived clients such as JoF EJK
  draw it (below);
- `modern` (or `0`), the modern table beside the chat column.

A mistyped value gives the classic board. `config.cfg` saves every archived
setting, so every profile from before `auto` existed had the old default
`classic` saved: the console's start moves a saved `classic` once to `auto`
(`cg_scoreboardStyleDefaultVersion`), which draws the same classic board unless
the menus are the SJK UI. A look chosen after that stays. The classic board
([classic.rs](../crates/sjk-viewer/src/scoreboard/classic.rs), after
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

`cg_hudStyle` chooses `game`, the status HUD of the game's own menu files (the
original Jedi Academy HUD, or a custom HUD pack that replaces `ui/hud.menu`),
or SJK's own `modern`, `classic` or `radial` layout. The default is `game`, the
classic HUD, and a saved `cg_hudStyle` is kept. `cg_hudFiles`
names the menu list, `ui/jahud.txt` by default; `1` gives the text-only HUD and
EternalJK's `3`/`4` name its elegance and JoF HUD lists when those files are
installed. `cg_hudPack` names the PK3 whose HUD to use when several replace
`ui/hud.menu` (`assets1.pk3` is the original); empty, the one installed last
wins, as in the game. See [game-data HUD](rendering.md#game-data-hud) for what
is drawn.

In SJK's own layouts, health and armour over the maximum (125 health at spawn, up
to 199 armour from a large shield picked up at 99) show on their meter: the meter
holds one maximum, and what lies over it is a thinner band down the middle in a
deeper shade of the meter's colour, from the start, as on the nameplates. The
game-data HUD draws the original pictures and does not.

Settings > HUD > "HUD look" shows the HUD in use; Left and Right step through
every HUD that can be used and Enter (or a click) opens the HUD picker
([hud_picker.rs](../crates/sjk-viewer/src/settings/hud_picker.rs)): the list on
the left, a picture of the highlighted HUD on the right, drawn for a sample
player over a retail levelshot, and its PK3 or list file under it, the HUD in
use marked. The list holds each installed PK3 that ships `ui/hud.menu` in
install order (the game's own archives as "Jedi Academy", others by file name
without a `zz_` load-order prefix), every other `ui/*hud*.txt` list that
describes a HUD, the text-only HUD, and SJK's classic and modern layouts, which
have a note instead of a picture. Arrows, the wheel or the pointer highlight;
Enter or a click uses the HUD and closes; Escape closes. The picker takes
retail's colours and highlight art on the classic menus and the theme's on the
modern ones. "Game HUD files" stays on the tab for lists the picker does not
find.

## Nameplates

`cg_nameplate` (on by default) draws MMO-style nameplates over players
([nameplate.rs](../crates/sjk-viewer/src/hud/nameplate.rs); layout in
[nameplate_math.rs](../crates/sjk-viewer/src/hud/nameplate_math.rs)): 2D HUD shapes
and text at the projected point 8 units above the head of the player's box (decoded
from `entityState_t::solid`, so a crouching player's plate drops). Names come from the
chat roster with their colour codes and are drawn in the classic HUD font (Inter when
that font is not loaded), whatever `cg_classicHudFont` says.

- **Far:** only the name, small and dim. Plates shrink with distance (to 60% at the
  range) and fade over the last quarter of `cg_nameplateRange` (3000 units).
- **Near:** inside `cg_nameplateNear` (1000 units) the name rises and a plate fades
  in under it, framed in red or blue in team games: the shield bar on top, then
  health, then Force. The bars take the HUD's own colours (the layout's
  `armor_ratio`, `health_ratio` and `force_ratio` widgets: the Radial HUD's green
  shield, red health and blue Force; the retail green, red and light blue for the
  game-data HUD, which draws pictures). Health is always that red, whatever is left.
- **Empty shield:** a player's shield bar is always there; known to be empty it is
  drawn broken, grey dashes on the dark track.
- **Overheal and overshield:** health and shield go to twice the maximum (125 at
  spawn; 199 shield from a large shield picked up at 99). The bar shows 0 to the
  maximum; what lies over it is a second, inner band in a deeper shade of the same
  colour, from the left (125 health: a full red bar with a deep red first quarter).
  Both decay a point a second back to the maximum, as the server does.
- **Unsure:** a health or shield estimate whose range spans 60 or more (a player
  first seen, or back from long out of view) dims and gets a yellow "?" over it until
  a pain, a shield hit or another clue narrows it.
- **Uncertainty:** an estimated bar is a range ([estimate.rs](../crates/sjk-viewer/src/hud/estimate.rs)):
  the lowest and highest the value can be and the best guess. The bar fills to the
  guess, and a grey haze, thickest at the guess and fading out towards either bound,
  covers the uncertain stretch, so a loose estimate reads as a blurred edge and an
  exact one as a sharp edge.
- `cg_nameplateWeapon` (on): the held weapon's icon (the weapon selection row's art,
  the staff and dual saber icons for those styles) left of the plate, or left of
  the name when there is no plate. A saber's is ringed and haloed in its stance's
  colour (`fireflag`, which carries `fd.saberAnimLevel`; the Radial HUD's colours:
  fast blue, medium yellow, strong red, dual green, staff magenta), dimmed while
  the blade is put away.
- **Verified badge:** a player the SJK hub's operator vouches for (see
  [Identity](#identity)) gets a gold seal with a white tick after the name, drawn at
  start into an icon cell ([verified_badge.rs](../crates/sjk-viewer/src/ui_renderer/verified_badge.rs)).
  Which slots are verified is read from the identity service once a second.
- `cg_nameplateSelf` (off): your own plate over your head in third person, with your
  real health, shield and Force (the server sends you those), your weapon and your
  badge when your key is verified. In first person it is not drawn: it would sit in
  the camera.
- **Modes (V):** the `nameplates` command, bound to V by default (Controls > Other >
  "Nameplate mode"; an existing config gets it when V is free), cycles off, names
  only, target, everyone, and round again, showing the mode's name near the top of the
  screen for a moment; `nameplates off|names|target|all` picks one. Target shows bars
  (and the weapon and power icons) only on the player you aim at, kept for three
  seconds after the crosshair leaves them, and on your duel opponent. Names only is
  just the names. The modes set `cg_nameplate` and `cg_nameplateBars` (0 names, 3
  target, 2 everyone); 1 (allies only) stays reachable in the settings.
- `cg_nameplateBars`: 0 none, 1 allies only, 2 everyone (default), 3 target and duel
  opponent. `cg_nameplateScale`
  sets the text size, `cg_nameplateForce` the Force bar, `cg_nameplatePredict` the
  estimated health and shield, `cg_nameplateWalls` shows
  players behind walls at 35% opacity instead of fading them (the same BSP trace as
  before, from the rendered eye; changes ease over 120 ms), and `cg_nameplateNpcs`
  adds NPC plates (class name, health; at most 16; vehicles skipped).
- `cg_nameplateIcons` (on): up close, a row of the holocron icons of the Force powers a
  player has on sits over the name, at most four, dark side first (lightning, grip,
  drain, rage, then protect, absorb, speed, heal, team force, mind trick, sight). They
  come from `forcePowersActive`, so every player's powers are known without a request;
  jump, push, pull and the saber powers are left out because they last a moment. The
  pictures are the Force bar's, loaded with the map.
- `cg_nameplateDebug` logs, every two seconds, what the server sends about each other
  player (health, `tinfo`, active powers, weapon and stance) and the estimates (health,
  armour and Force as `guess[low..high]`) to `logs\last-client.log`, with the regen
  pace in use and where it came from, the server's Force-related info keys, whether
  its pain events carry real health, and your own real Force next to the estimate
  for yourself.
- While a menu is open the plates hide: their text is in the classic stream, which
  draws over the menus' text.

Where the numbers come from, and what is not known:

- **Teammates** in team games: health and shield from the team overlay's `tinfo`
  command, exact. The client asks for it (`teamoverlay` userinfo) whenever nameplate
  bars are on, not only with `cg_drawTeamOverlay`.
- **NPCs** (and any entity whose `entityState_t::health`/`maxhealth` the server
  sets): exact. The SJK server sets them for NPCs, breakables and emplaced guns, not
  for clients; stock JKA does not either.
- **Everyone else's health and shield** are estimated
  ([vitals_estimate.rs](../crates/sjk-viewer/src/hud/vitals_estimate.rs)) once per
  snapshot (every one, so no event is missed) from what the server sends everyone,
  with its own rules:
  - spawn at a quarter over the maximum health with a quarter of it as shield (125
    and 25; 100 and none in Duel), learnt from your own spawns for mods that change
    it; health and shield over the maximum lose a point a second;
  - `EV_PAIN` carries the health left after a hit of ten or more (at most every
    700 ms): exact. `EV_SHIELD_HIT` carries the shield a hit took: exact.
    `EV_SABER_HIT` sizes the damage (under 5, under 20, more); a missile striking
    (`EV_MISSILE_HIT`) is a hit of unknown size, except JA+'s grapple hook (the only
    missile fired as `WP_STUN_BATON`), which does exactly one point. With no pain after a hit outside the
    700 ms, it was under ten. A hit with no shield flash met no shield;
  - when you hit someone, the server tells you their health and shield before the
    hit (`PERS_ATTACKEE_ARMOR`), matched to them when only one player fits (your
    saber's victim, your duel opponent, or the only estimate that allows the values);
  - falls (`EV_FALL`/`EV_ROLL`, exact), medpack and shield pickups
    (`EV_ITEM_PICKUP`), Force heal (its sound at the player: 5 to 25), drain healing
    the drainer, deaths (`EF_DEAD`, `EV_OBITUARY`) and the respawn after one; a spawn
    with no death seen toggles `EF_TELEPORT_BIT`, as a teleport does, so it only
    raises the high bound;
  - private duels (`EV_PRIVATE_DUEL`, 1 at the start, 0 at the end): on JA+ both
    duellists are set to 100 health and 100 shield when it starts, and the winner to
    100 and 25 when it ends, whatever they had left; on stock JKA the start changes
    nothing and the winner is healed to the maximum. A duel ending within a second of
    a death was won; one ending otherwise (called off, too far apart) changes
    nothing. The start values are learnt from your own duels once you have had one,
    for servers that set others (jaPRO's `g_duelStartHealth`);
  - JA+'s chat protection: a player with the chat balloon up (`EF_TALK`, whatever
    opened it) takes no damage unless in the middle of an action (a saber swing,
    its transitions or a special, a kick, a punch or a melee grab, read from the
    saber move and the torso and legs animations). On JA+ a hit seen on such a player
    without a pain or a shield flash takes nothing, and a fall takes nothing; a pain
    or a shield flash still counts, since the server only sends one when damage landed;
  - a player first seen, or back after time out of view, gets a wide range that the
    next pain closes.

  Servers can hide the pain values: JAPro's `g_stopHealthESP` (off by default) sends a
  fixed 50 or no pain event at all, and scrambles `PERS_ATTACKEE_ARMOR`. At 2 it plays
  the pain sound instead (`EV_ENTITY_SOUND` of `*pain25` to `*pain100`, chosen by the
  health left), which bounds the health to its quarter: 25 or less, 50 or less, 75 or
  less, or more. Your own pain
  events are checked against your real health: until one has been, pain values are
  trusted but their absence proves nothing; if they do not match (or every pain says
  50), they are ignored and a pain only says ten or more landed. JA+ is closed source
  and not verified.
- **Force** is never sent for other players, so it is estimated
  ([force_estimate.rs](../crates/sjk-viewer/src/hud/force_estimate.rs)) from their
  entity state with the server's own rules: a full pool at spawn, a point per
  regen pace (measured from your own pool while you idle, which the server does send,
  else `g_forceRegenTime` from the server's info string, else 200 ms; six times as
  fast with the boon) while no power but drain is on and no saber is thrown or in a
  special move, the price of each power when it switches on, protect, absorb, grip
  and lightning running costs, force jumps, push, pull and saber throw at their
  price, and being drained (`EV_FORCE_DRAINED`). Power levels are not sent, so the
  low bound pays each power at its dearest level and the high bound at its cheapest
  (the guess at level 3); a force jump costs anything up to its price; until the pace
  is measured the bounds refill a little slower and faster than the guess. A power
  starting proves the pool held its price, which raises the low bound. It misses
  saber blocks in some mods and anything that changes costs. It refills while a
  player idles, so the range closes within about twenty seconds.

`cg_drawPlayerNames` keeps TaystJK's plain overhead names (0 off, 1 names, 2 adds a
health strip, text only, off by default); they are hidden while nameplates are on.
`cg_drawFriend` draws the ally marker for either. At most 32 players and 16 NPCs are
tagged and nothing is allocated per frame. Not tested in game yet; the estimates
are unit-tested against synthetic snapshots only.

## Version label

The client draws its build label small at the top centre of every frame, in
menus and in play, so a screenshot says which build it shows:
`SJK <version> · <dd/mm/yyyy HH:MM> · <commit>`, the release version (a date
version such as `2026.1005.1`, or `dev` for a local build), the source commit's date and time on the 24-hour
clock, and its short hash ([version_overlay.rs](../crates/sjk-viewer/src/version_overlay.rs),
[build_info.rs](../crates/sjk-viewer/src/build_info.rs)). The top centre is free in
the stock and game-data HUDs (gauges at the bottom corners, the FPS counter and
timer at the top right, notify and vote lines at the top left). It is left out
while the console is open, which shows the version in its own corner.
`cg_drawVersion 0` (Settings > HUD > "Version and date") hides it. The log names
the same build at startup (`build: SJK ...`). How the version is decided is in
[SJK conventions](sjk.md#version).

## Changelog page

The main menu's Changelog entry (modern list; SJK > CHANGELOG on the classic
page), the in-game SJK pop-up's Changelog and the
`changelog` console command show every SJK release from [CHANGELOG.md](../CHANGELOG.md), built into the client
([changelog.rs](../crates/sjk-viewer/src/changelog.rs), parsed by
[changelog_data.rs](../crates/sjk-viewer/src/changelog_data.rs)). Releases are
listed newest first on the left, "Unreleased" (the changes on `main` since the
last release) on top; the right pane shows the selected release's introduction
and its changes, wrapped to the pane, each followed by its credit in capitals.
Up and Down (or a click) pick a release; Page Up, Page Down, the wheel over the
pane and its scrollbar scroll it; Escape closes. Like the `debug_panel` list the
page lives in the console and is drawn in its place, so it opens over the menus
and in a match, and closes the console with itself when it opened it. With
`ui_menuStyle classic` it takes the classic+ look of the command browser's
pop-up ([changelog_classic.rs](../crates/sjk-viewer/src/changelog_classic.rs)):
the in-game pop-up box and title band, a retail list box of releases, a detail
box with the release and its credits in gold, a Close button and the
description line, in the menus' retail font. How the file is kept is in
[SJK conventions](sjk.md#changelog).

## Updates

The client looks for a newer SJK release when it starts and from the Update page
([update.rs](../crates/sjk-viewer/src/update.rs) does the work,
[update_panel.rs](../crates/sjk-viewer/src/update_panel.rs) draws the page).

- `cl_autoUpdate` (default 1; Settings > Network > Check for updates at start)
  asks `api.github.com/repos/Sol-Vulpes/SJK/releases/latest` once at start-up on
  a worker thread. A newer version shows on the main menus' version line
  ("update 2026.1010.1 available"). The check sends the request GitHub needs and
  nothing else; turn it off to send none.
- Main menu > Update (classic: SJK > UPDATE) or the
  `update` command opens the page. It shows the state and offers Install (Enter),
  Check again (C) and Release notes (N); Escape closes. Opening it with nothing
  checked yet checks at once.
- Install downloads `SJK-<version>-<platform>.zip`, refuses it unless its SHA-256
  is the one in the release's `SJK-<version>-SHA256SUMS-<platform>.txt`, writes the
  ZIP's files beside the running program as `<name>.new` and renames each running
  file to `<name>.old` before the new one takes its name (a Windows program can be
  renamed while it runs, not overwritten). A failed step puts the old files back.
  Only plain file names are taken from the ZIP. The `.old` files are deleted at
  the next start. Settings in `GameData/SJK/` are not touched.
- The new version starts when the client exits, after it saved its settings:
  "Restart now" on the page quits, or the player keeps playing and restarts later.
- Nothing is installed without Install. When the program's folder is not writable
  (a Program Files install) the page offers the release page instead.
- A local build says `dev`, which has no release number to compare, so it reports
  that and never offers an update. `cl_updateAs 2026.1001.1` makes it check as if
  it were that release, to try the page without a release build.

## Credits page

The main menu's Credits entry (modern list; SJK > CREDITS on the classic page), the in-game SJK pop-up's Credits and the
`credits` console command show who makes Sol JK, from
[credits.txt](../crates/sjk-viewer/assets/credits.txt), built into the client
([credits.rs](../crates/sjk-viewer/src/credits.rs), parsed by
[credits_data.rs](../crates/sjk-viewer/src/credits_data.rs)), and everyone's
work from [credits_history.txt](../crates/sjk-viewer/assets/credits_history.txt),
which `scripts/credits_history.py` writes from git and GitHub. Each section of
the file (Sol JK, Contributors, Origins, ...) is a heading with its people.

- Everyone with a history (Sol, Bishop, Creyon, lumaya) has a panel across the
  column: the name large (the first section's larger still), the role, chips
  counting their changes, pull requests and commits, their links, and two folds.
  HIGHLIGHTS holds the card's own lines; ALL WORK (ALL PULL REQUESTS when all of
  it is pull requests) lists every feature and pull request newest first under
  the day it was merged into main, each row with its pull request's number
  (JKR #48, SJK #23) and how many commits it has. A row of several commits
  unfolds into them: hash, subject, and who made it when that is not the owner.
- Every fold is closed when the page opens. A fold's rows fade in one after the
  other as it opens; EXPAND ALL in the footer opens every fold at once and then
  reads COLLAPSE ALL.
- Clicking a pull request's number opens it on GitHub, clicking a commit (or a
  one-commit row) opens the commit on SJK's GitHub, a GitHub handle opens that
  profile and a link its address (`update::open_page`, https only). They brighten
  under the pointer and take clicks only while the page shows them whole, up to
  84 targets a frame, under the menu canvas's limit.
- The rest (Claude, the fonts, the reference clients) sit on cards in rows of up
  to three with their lines shown.

The page is animated from the clock alone. Golden god rays turn slowly down from
above the top of the screen, two sets against each other so the shafts shimmer
where they cross, sparks rise through them, and SJK's emblem breathes in front of
two sunbursts turning opposite ways, above a title a glint crosses: the look of
SJK's site (`site/assets/effects.js`). The rays are two light pictures the emblem
module draws on its worker at start (`EmblemLayer::Sunburst` and `Godrays` in
[emblem.rs](../crates/sjk-viewer/src/menu/emblem.rs)), added as light by the UI
renderer's additive pipeline and turned by their texture coordinates
(`emblem::rays`). Panels rise into place as the page opens and their edges glow
in turn. With the classic menus it takes retail's gold and blue and the menus'
retail font. Arrow keys, Page Up and Page Down, Space, the wheel and the
scrollbar at the right edge scroll it, gliding to the place asked; Escape, Enter
or CLOSE closes it. Like the changelog it lives in the console, so it opens over
the menus and in a match. How the files are kept is in
[SJK conventions](sjk.md#credits).

## Identity

SJK keeps an identity key and can mark other SJK players on a server with SJK's
emblem on the scoreboard and in a card beside them; the design, limits and privacy
are in [identity.md](identity.md).

- `cl_identity` (default 1; Settings > Network > SJK identity) makes the key
  (`identity.key` beside `config.cfg`) the first time it is on and lets the client
  talk to the hub. Off sends nothing and makes no key.
- `cl_hubUrl` (default `https://sjk.dfox.app`; Settings > Network > SJK hub) is the
  hub's `https://` address. Empty means no hub, so nothing is sent.
- The Identity page is where a player sets all of this up without a command: main
  menu > SJK > IDENTITY (classic menus), the in-game SJK menu, or the `identity`
  command ([identity_panel.rs](../crates/sjk-viewer/src/identity_panel.rs)). It shows the
  key id, the file to back up, whether the hub's operator vouches for the player, the
  hub's status and the players it knows on the current server, and has
  - a switch for `cl_identity` ("Share my identity with the SJK hub", ON or OFF);
  - an optional bio field (500 characters at most), saved to the hub with Save or Enter.
    There is no name field: the hub takes the name the player plays under (`name`), and
    the page shows it with up to three earlier names. A new player has nothing to do;
  - "Copy my key id" (for the operator to verify the player);
  - "Use the official hub", shown only while `cl_hubUrl` is another address (a hub tried
    on this PC, say): it sets `cl_hubUrl` back to `https://sjk.dfox.app`. A saved
    `cl_hubUrl` wins over the default, so an old address stays until it is changed.

  With the classic menus (`ui_menuStyle classic`) the page takes the classic+ look
  ([identity_panel_classic.rs](../crates/sjk-viewer/src/identity_panel_classic.rs)): the
  retail pop-up and title band, option rows with the highlight band, text fields in list
  boxes, gold buttons that glow, and a description line under the box.

  Tab, Shift+Tab and the arrow keys move between the controls, Enter or Space works the
  switch and the buttons, letters type into the focused field (Ctrl+V pastes) and Escape
  closes; the pointer works too. The bio field follows the hub's copy until the player
  types in it. The modern main menu has no entry (its list is full); reach it from the
  in-game menu or the command there. The commands remain: `identity bio <text>` changes
  the bio (`identity name` says the name is the one played under), `identity key` prints
  the key id and file,
  `identity who [slot]` lists known players (with a slot, that player's bio).
- The scoreboard (every style) draws SJK's emblem at the end of the name of a
  player the hub knows (the SJK UI's right after the name), in gold when the
  hub's operator vouches for them, then up to three ribbon bars for the medals the
  SJK team gave them ([identity.md](identity.md#medals)); fewer bars show where the
  name would run short of room. It trusts
  a claim only when the claimed name matches the name the game shows in that slot.
- The Identity page lists the player's own medals: in the modern look a Medals panel
  right of the page's card, in the SJK UI a column right of its own, each with its
  picture, name, description, the date it was given (`dd/mm/yyyy`) and the team's note;
  the classic+ box names them in its status with their medallions. With none, a line
  says how medals come. A medal new to the client shows once in a pop-up on the main
  menu or when the game menu opens ([identity.md](identity.md#medals)).
- Back up `identity.key`: losing it loses the identity.

### Player card

Look at a player, keeping the view steady, and a card appears beside their head
([player_card.rs](../crates/sjk-viewer/src/hud/player_card.rs)). It shows what the
server already publishes to every client (name with its colour codes, model and its
head icon, saber hilts with their blade colours, the hat and cape worn, duel record or
bot skill) and, when the hub knows
the player, SJK's emblem, their hub name, a gold VERIFIED and the medallions of the
medals the SJK team gave them ([identity.md](identity.md#medals)); a pinned card names
the medals under them, a repeatable one with its count. It adds nothing a
glance at the scoreboard would not: no health, Force or position.

- `cg_playerCard` (default 1; Settings > HUD+ > Player card) turns it on.
- `cg_playerCardDelay` (default 1.5; Settings > HUD+ > Card delay) is the seconds
  the crosshair must stay on the player before the card fades in. Turning the view
  more than 6 degrees, or losing the player for over 0.3 seconds, starts the wait
  again and the card fades out.
- `inspect` (Settings > Key bindings > Interaction > Inspect player; bound to X by default,
  or `bind <key> inspect`) pins the card to the player under the crosshair at once, with
  no wait. It stays when you look away or the player moves, follows them (kept inside the
  screen), and a second press hides it; it also drops when the player leaves or the map
  changes. After hiding it, the card does not return until the crosshair leaves that
  player. Behind you, the card is not drawn but stays pinned.
- World notes: with no card pinned and no player under the crosshair, `inspect`
  selects the world surface under the crosshair, or the mover whose bounds the view ray
  meets first, and names it in a centre print (shader, BSP surface, lightmap or vertex
  lighting, distance). When a remap (the map's, the server's or a local `remapShader`)
  draws another shader over the surface, the print, `notes.txt` and `notes.jsonl`
  (`surface.remapped_to`) name it too (`<shader> remapped to <target>`): on a server
  what the player saw is often the remap's target (08/10/2026). The hub still gets the
  map's own shader. While selected, and while its note is written, the surface's
  triangle edges (a mover's bounds) are traced over the view in flickering green dots
  (at most 600 a frame) over a green scanline shade of its triangles (horizontal strips
  with a brighter wave running down them, at most 800 a frame; cut where the surface
  passes behind the eye, so a floor underfoot keeps its visible part). Escape before the second press drops the selection (it does not
  open the game menu then). A second press within 20 seconds opens the text dialog
  ([text_dialog.rs](../crates/sjk-viewer/src/text_dialog.rs)): a panel in the middle of
  the screen with the subject, a text box of six wrapped lines (any printable text, up to
  500 characters), a character count, Send and Cancel; Tab moves between them. With the
  classic menus it takes the classic+ look of the in-game pop-ups, eight lines in a
  retail list box ([identity.md](identity.md#bug-reports)). As it is typed the text
  keeps the bug reports' alphabet (letters, digits, spaces and `. , ! ? ' - : ( )`), and
  Send needs 3 characters with 2 letters. Enter or
  Send appends the note to `notes.jsonl` (one JSON object: map, UTC time,
  `setviewpos`, camera pose, hit point and normal, draw surface, collision flags,
  nearest map entity, screenshot path) and a readable line to `notes.txt` in the config
  directory, and takes a silent JPEG of the same view (`note_<unix seconds>.jpg` in the
  screenshots folder; JPEG screenshots drop the alpha channel, which the encoder
  refused before, so `screenshotjpeg` works too). With `cl_identity` on, the note and a
  smaller copy of the screenshot also go to the SJK hub for the SJK team, and a centre
  print says whether it took them ([identity.md](identity.md#world-notes)). Escape or
  Cancel drops it. [world_notes.rs](../crates/sjk-viewer/src/world_notes.rs).
- The crosshair scan that names players under the crosshair finds the target. The card
  is anchored in the world beside the top of the player's box (the box height the server
  sends, as the nameplates use), a body width clear of them in screen pixels, so it
  neither floats above nor overlaps the player at any range. It goes to the player's left
  near the right edge of the screen and hides under the scoreboard, menus, the console
  and intermission.
- The head icon is the scoreboard's (`models/players/<model>/icon_<skin>`, one atlas cell
  per client slot), resolved when a card is shown even with the scoreboard closed. Hats and
  capes are models without pictures, so they are named in text (`c1`, `c2` after the colour
  digits).
- A hub bio is not shown yet.

## Force wheel

With the `game` and `classic` HUD styles, `forcenext`/`forceprev` show JoF
EternalJK's Force wheel: retail's Force selection bar (`CG_DrawForceSelect`),
the selected power large in the middle above the HUD with up to three
neighbours on each side and its name under it, for 1.4 seconds as in retail.
It scales with `cg_hudScale`; the `modern` style keeps its list of names. On a
JoF JA+ server that grants them, the wheel also holds Stasis and Repulse (after
Sense) and Dash (before Speed), from spare `forcePowersKnown` bits, as in JoF EJK
([force_wheel.rs](../crates/sjk-client/src/force_wheel.rs)). They are never sent
as the selected power: with one selected, `+useforce` engages Stasis (the
usercmd button the server reads) or sends `force_repulse` or `force_dash` once
per press, and the selection stays until the server takes the ability away. The
binds `force_dash`, `+force_stasis` and `force_repulse` (Settings > Key
bindings > Force powers) work without the wheel; `+force_stasis` does nothing
where Stasis is not granted. On JA+, merc mode shows Lightning as the
flamethrower until the player is seen using real lightning. Repulse, Dash and
the flamethrower use JoF EJK's pictures from `EternalJK/jofclient-assets.pk3`;
Stasis has none and shows Jump's. Power names are retail's (Dark Rage, Sense).
SJK's own [Illuminate](#illuminate) is the wheel's last entry on every server.
See [hud/force_wheel.rs](../crates/sjk-viewer/src/hud/force_wheel.rs).

## Illuminate

Illuminate is a free power every player has, SJK's own and not a game power: a
holocron that floats by the player's left shoulder, turning slowly and bobbing,
with a warm point light (300 units) that lights the way in dark maps. Only this
client sees it; no server knows of it and other players, SJK ones included, do
not see it. It is the Force wheel's last entry (and the modern list's), with its
own holocron icon; `+useforce` on it turns the holocron on or off, and the
`force_illuminate` command does the same from a bind (Settings > Key bindings >
Force powers). It is never sent as the selected power: the client sets its
`forcePowersKnown` bit itself ([`client_known`](../crates/sjk-client/src/force_wheel.rs))
and swallows `+useforce` on it. Where the player has no Force at all,
`forcenext` still walks the inventory, as in stock, and only the bind reaches it.

`cg_illuminate` (archived, default 1; Settings > Game > Illuminate holocron) puts
it on the wheel; 0 takes it off and puts the holocron out. The holocron appears
and goes out over 0.3 seconds, trails a moving player by at most 20 units and
jumps with a teleport. It goes out while the player is dead, spectating,
following someone or at the intermission, and comes back at the player. In first
person only its light shows (the cube is drawn in mirrors, like the body). It is
on or off for the client's run, not saved. Its light is added first each frame,
so a full light list never drops it, but `r_dynamiclight 0` puts it out with the
others.

The cube, its two pictures (lit metal, and the emblem alone for the glowing
stage), its shader and the wheel icon are bundled and mounted below all game
data, so a PK3 with the same paths replaces them. Sol generated the art; see
[assets/holocron](../crates/sjk-viewer/assets/holocron/README.md) and
[illuminate.rs](../crates/sjk-viewer/src/illuminate.rs).

## Quick wheels

SJK's quick wheels ([quick_wheel.rs](../crates/sjk-viewer/src/quick_wheel.rs)):
hold the key, a ring of choices opens in the middle of the screen, move the mouse
towards one (it grows inside a ring of the menu accent colour and its name shows in the
middle) and let go to run it. The choices are round icons in the style of the game's
neutral Force icons (SJK's own, generated by Sol; [assets/wheel](../crates/sjk-viewer/assets/wheel/README.md));
the game menu and first setup show their name on a plain disc. Letting
go with the mouse still near the middle, or Escape, runs nothing; opening the
console, chat or a menu closes the wheel. While a wheel is open the mouse moves its
pointer instead of the view; the movement keys keep working. A dot marks the
choices already in effect.

| Wheel | Bind | Choices |
| --- | --- | --- |
| `general` | `+wheel general`, Q by default | Third person, nameplates (cycles), HUD on/off, screenshot, cosmetics, AFK, game menu, first setup |
| `weather` | `+wheel weather`, R by default | The map's own weather, drizzle, rain, storm, snow (`r_weatherForce`), ground fog everywhere (`r_weatherFog` 1/2), clouds, weather on/off |

Both are in Settings > Key bindings > Other; the default keys are bound only where
a profile has nothing on Q or R. A choice is the console command it stands for, so
it does exactly what typing it would.
## Configuration and content

The default writable client folder is `GameData/SJK/`, under the selected game
installation (see [storage.rs](../crates/sjk-viewer/src/platform/storage.rs)). It
is independent of the executable's location and working directory. The client creates it automatically. Important files include:

| File or folder | Contents |
| --- | --- |
| `config.cfg` | Settings and key bindings |
| `marks.txt` | Marked map positions, views and notes |
| `favorites.json`, `chat-friends.txt` | Favorite servers and friend names |
| `jakey` | Persistent client identity key |
| `hud.json` | Optional custom HUD |
| `screenshots/`, `demos/` | Screenshots and recordings |
| `chatlogs/`, `qconsole.log` | Logs when enabled |

`config.cfg` and the scripts `exec` runs are read as UTF-8, or, when a file is not
valid UTF-8, one character per byte (Latin-1), as Jedi Academy and EternalJK save
names and chat binds; a leading byte-order mark (Notepad) is dropped
([config.rs](../crates/sjk-shell/src/config.rs) `decode_config_text`). So another
client's config runs with `exec`, and a `jampconfig.cfg` copied over `config.cfg`
loads: its `unbindall` and `unbind` lines are accepted and bindings on keys SJK has
no name for are dropped. Any other line still stops the load (and saving, so the
file is not overwritten). The [Import page](#importing-from-another-client) is the
way that takes only the name, model, field of view and bindings.

If `GameData/SJK/` cannot be written, the client uses its per-user folder:
`%APPDATA%\SJK\` on Windows, `~/Library/Application Support/SJK/` on macOS, or
`$XDG_CONFIG_HOME/SJK/` (otherwise `~/.config/SJK/`) elsewhere. The chosen folder
is shared by all profile consumers for the entire session. Startup reports it;
the console's `path` command also lists it. The fallback uses the profile in
that per-user folder; the per-user and portable profiles are not synced.
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

A window without focus has caps of its own: `com_maxfpsUnfocused` (SJK's default
30; EternalJK's 0 keeps the normal cap) and `com_maxfpsMinimized` (50), where 0
uses `com_maxfps`. They replace the normal cap while they apply, minimized first,
so an alt-tabbed client does not render a 4K scene at full rate.

`snd_mute_losefocus` (archived, 1, EternalJK's name and default; Settings > Sound
> Mute in background) silences every sound, music included, while the window is
unfocused or minimized. EternalJK pauses its sound device; SJK sets the effects
and music gains to zero and keeps mixing, so music and loops carry on silently and
come back where they are. The change is applied from the focus and minimize
events themselves, since a minimized window may draw no frame; see
[console_window_options.rs](../crates/sjk-viewer/src/console_window_options.rs).

The Video tab's Display mode row offers Windowed, Borderless fullscreen and,
where the windowing system supports it, Exclusive fullscreen (Wayland does not).
Stock `r_fullscreen` keeps its meaning, fullscreen on or off, and Alt+Enter still
toggles it. `r_exclusiveFullscreen` chooses the kind: 0 (default) is a borderless
window at the desktop size, 1 switches the monitor to the `r_resolution` video
mode. Stock JA's fullscreen is always the exclusive kind. Choosing Windowed leaves `r_exclusiveFullscreen` alone, so Alt+Enter
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
The default is `%LOCALAPPDATA%\SJK\downloads\base` on Windows,
`~/Library/Application Support/SJK/downloads/base` on macOS, and elsewhere
`$XDG_DATA_HOME/SJK/downloads/base`, falling back to
`~/.local/share/SJK/downloads/base`. `SJK_DOWNLOAD_HOME` overrides the download
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

| Value | Look |
| --- | --- |
| `auto` (default) | `sjk` while the menus are the SJK UI (`ui_menuStyle sjk`), else `classic` |
| `sjk` | The SJK UI's deck ([SJK UI console](#sjk-ui-console)) |
| `classic` | EternalJK's console (`cl_console.cpp`, `cl_keys.cpp`) |
| `modern` (or `0`) | The modern console (Inter text on a tinted panel with a header and key hints) |

A mistyped value acts as `auto`, as do `horizon` and `dock`, two SJK UI designs
removed on 08/10/2026 once Sol chose the deck. Every profile had saved the old default
`classic`, so a saved `classic` moves once to `auto` (`con_styleDefaultVersion`,
as `cg_scoreboardStyle` did): it still gives the classic console unless the
menus are the SJK UI; a `classic` chosen afterwards stays. Input editing, mouse
selection, completion, the `]cmd` echo, history (Up and Down) and the F3 browser
work the same in every look; the classic console and the SJK UI's deck share
EternalJK's keys below. See
[console_classic.rs](../crates/sjk-viewer/src/console_classic.rs),
[console_sjk.rs](../crates/sjk-viewer/src/console_sjk.rs),
[console_backdrop.rs](../crates/sjk-viewer/src/console_backdrop.rs) and
[console_options.rs](../crates/sjk-viewer/src/console_options.rs).

### Classic console

Text sits on a grid of character cells drawn with the console font, without the
glyph shadow other UI text has. The console font is JetBrains Mono, a bundled
vector font rasterized once at startup like Inter
([console_font.rs](../crates/sjk-viewer/src/text/console_font.rs)), in place of
retail's `gfx/2d/charsgrid_med` bitmap, whose 8 by 16 texel glyphs looked heavy
and blocky magnified at 1440p and 4K (SJK draws no bitmap fonts:
[sjk.md](sjk.md#fonts)). It keeps the character set's cell: every character
advances half the line height, and the font's ascent and descent are centred in
the cell. The retail character set had no Windows-1252 typographic characters
(`€`, `’`, `‘`, `…`, bytes 0x80..=0x9E); the classic console still gives them no
cell, like colour codes, so rows close up as in EternalJK, while the text keeps
them. The classic console loads the font whether `ui_gameFont` is on or not; if it
cannot be loaded the cells use Inter.
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
| Console key (the key under Escape, or `cl_consoleKeys` with `cl_consoleUseScanCode 0`) | Opens to `con_height` (0.5) |
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
rather than applied, and a cursor that blinks every 256 ms: a bar on the cell's
bottom rows or, in overstrike mode, a block, both solid shapes in the retail
character set's cursor proportions (Inter cells use `_` and a box). The
input scrolls sideways to keep the cursor on screen. In the bottom-right corner
the version line ends one cell from the edge, two and a half rows up, and the
local day, date and time sit under it at the edge, both in the bar colour. Where
EternalJK prints `asctime` on a 12-hour clock (`Sun Oct  4 10:52:10 PM`), SJK writes
the date day first on the 24-hour clock (`Sun 04/10/2026 22:52:10`; see
[SJK conventions](sjk.md#dates-and-times)).

Closed, the console draws notify lines while a game, a demo or a map walk runs
and no menu has focus: of the last `con_notifylines` rows, those written within
`con_notifytime` seconds and not quiet (chat), from the top edge of the screen,
one cell plus `cl_conXOffset` pixels from the left. Dragging the mouse selects
scrollback text and Ctrl+C copies it, as in the modern console; the classic
console's selection does not include the time column.

### SJK UI console

`sjk`, the deck, is the classic console with another skin
([console_sjk.rs](../crates/sjk-viewer/src/console_sjk.rs)): the same grid of
JetBrains Mono cells, word wrap, scrollback, history, keys, overstrike, mouse
selection and notify lines, drawn in the SJK UI's colours
([sjk-ui.md](sjk-ui.md#colours)) without the `console` shader. Rows are a fifth
further apart than the classic console's (19 pixels at 1080 lines), the cells
centred in them; text starts in the SJK UI's text colour, error lines in a soft
red, time stamps quiet, and colour codes use its legible palette. The input row
starts with a gold `›`, the caret is a thin gold bar (a gold block in overstrike
mode), and while the caret ends a unique command or cvar name being typed the
rest of the name shows faintly after it, as Tab would complete it. Scrolled back,
the bottom row says how many newer rows there are; a thin bar right of the rows
shows the part on screen, gold while scrolled back. Labels are in the SJK UI's
families (Rajdhani and Exo 2), drawn on the console's own layer so nothing shows
through; they load with the look, and the console font stands in until they
have. After Ctrl+C the header says how many characters were copied. Below full
height the panel follows `con_opacity`. The notify lines stay the classic ones.

The deck is a full-width navy panel (the world shows faintly through): a header
with "Console" underlined gold, the version, the date and the time; the input
on a band with the SJK UI's gold bar; key hints (Tab, Up and Down, Page Up and
Down, F3, Ctrl+C); a gold rail fading into a holo line along its bottom edge.

A click on the F3 hint opens the command browser, which is drawn in the SJK
UI's look with the deck (see below). Verified by unit tests (the looks' names,
`auto` and the retired names, the saved value's move, the grid, caret and ghost
completion) and by off-screen frames of every look over mp/duel6 at 1080 lines
and 4K (`world_shot::tests::duel6_console_styles`); not yet used in a running
client.

Local time comes from the operating system's time zone rules, daylight saving
included ([local_time.rs](../crates/sjk-shell/src/local_time.rs)); the scrollback's
stamps, also the modern console's `[HH:MM:SS]` and the log file's, are local time
too. Differences from EternalJK besides those above: the input never runs past
the screen's right edge (EternalJK's can).

## Console socket

`cl_consoleSocket <port>` lets a program on the same computer read the console
and run commands, as JoF EJK's setting of the same name does; apps written for
it (Sol's Archive, a chat quote bot) work unchanged. It is off (`0`) by default.
`cl_consoleSocketPassword` is required: the socket does not open without one and
starts as soon as one is set, so a web page that writes to a loopback port cannot
run console commands (JoF EJK allowed an empty password). Both cvars are archived,
the password in plain text like `password`. The `consolesocket` command explains
the feature and shows its state.

```
cl_consoleSocketPassword mySecret123
cl_consoleSocket 29071
```

The protocol is lines of text on `127.0.0.1:<port>`, which only this computer can
reach. The app sends the password as its first line (anything else closes the
connection and nothing runs) and is answered `cl_consoleSocket: authenticated`.
Then every console line the app has not seen is streamed to it as it prints, ended
by `
`, with `^n` colour codes; lines printed before it authenticated are not
sent. Each line the app sends runs as a console command, as if typed (`say hi`
chats). Text uses Windows-1252 bytes, like chat on the wire
([legacy_text.rs](../crates/sjk-protocol/src/legacy_text.rs)); text with a
character outside it is sent as UTF-8.

Chat lines carry the stock `0x19` separator before the colon (`Name^7<0x19>: hi`),
which is how an app tells chat from other output; the console shows the line
without it, so the socket is given the server's raw text instead
(`push_chat_line` in [console_socket.rs](../crates/sjk-viewer/src/console_socket.rs)).

Up to 4 apps connect at once, a command line is at most 1023 characters (a longer
one is discarded whole), and an app that stops reading is dropped after 256 KB of
unread output; the game never waits for it. The listener, framing and limits are
[console_socket.rs](../crates/sjk-shell/src/console_socket.rs) in `sjk-shell`; the
viewer feeds it once per frame from `run_console_command_buffer`. Covered by unit
tests over real loopback connections (authentication, limits, chat separator,
command execution); not yet run against a game session or Sol's Archive itself.

## Useful console commands

The console key opens and closes either console style and never types its
character, as in EternalJK (`IN_TranslateSDLToJKKey` and `IN_IsConsoleKey` turn
it into `A_CONSOLE`). By default (`cl_consoleUseScanCode 1`, EternalJK's) it is
the physical key under Escape on every layout, so Hungarian, where that key types
`0`, has one; on layouts where it types `^` (German) it needs Shift, leaving `^`
for colour codes, and `cl_consoleShiftRequirement` can require Shift elsewhere.
Profiles that saved the earlier default 0 are moved to 1 once
(`cl_consoleKeyDefaultVersion`). With `cl_consoleUseScanCode 0` the console key is
a `cl_consoleKeys` character such as `~` or `²` instead. Holding it toggles once. Escape and Shift+Escape close
the console too, and so does a key bound to `toggleconsole` that prints nothing;
a printable key bound to it types while the console is open
([console_keyboard.rs](../crates/sjk-viewer/src/console_keyboard.rs)). The chat
composer still types `~` when the console is closed. On layouts with dead keys
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

Alt codes type as in other Windows programs: hold Alt, type a number on the
numeric keypad and release Alt. A number starting with 0 is a Windows-1252 code
(`Alt+0248` gives `ø`), any other a code page 437 one (`Alt+21` gives `§`,
`Alt+130` gives `é`), counted modulo 256; a code naming a control character types
nothing. They work wherever text is typed (the console line and browser search,
the chat draft, menu fields) and not during play, where the keypad keeps its
bindings; the keypad digits of a code reach no field, so the classic console's
keypad 8/2 do not walk the history. AltGr and Ctrl+Alt do not compose. Windows
composes these itself, but winit drops the resulting character, so the client
composes them with the same rules
([alt_code.rs](../crates/sjk-viewer/src/input/alt_code.rs)). Codes without a
leading 0 always use code page 437, the US OEM page, even where Windows uses
another OEM page (850 in much of Western Europe).

`r_we <command>` runs one weather command on the current map, as the original
renderer's `r_we` does: `r_we heavyrain`, `r_we snow`, `r_we heavyrainfog`,
`r_we constantwind ( 0 -800 0 )`, `r_we clear` (an unknown command lists them).
It adds to the map's own weather until the next map; see
[Weather](rendering.md#weather).

`connect host:port`, `disconnect` and `reconnect` control the session.
`record`, `stoprecord`, `demo` and `playdemo` control demos.
`screenshot` and `screenshotJPEG` request captures; `condump filename` saves
console output. A capture copies the final swapchain image into a buffer that is
mapped over the next frames; frames keep running meanwhile, and the readback stays
in [screenshot.rs](../crates/sjk-viewer/src/screenshot.rs)'s state until its own map
callback lands (dropping it earlier aborts the map). See [console registration](../crates/sjk-viewer/src/console_session.rs)
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

`flipkick` is JoF EJK's flip-kick bind: one press starts a run of jump taps, jump
held in one user command and released in the next, overriding a held jump key and
letting go of it when the run ends. EJK counts frames; SJK counts user commands,
which it makes every 8 ms, EJK's 125 fps frame
([networking.md](networking.md#user-commands-and-move-packets)), so one frame is
one command and `cg_fkDuration` (default 50) lasts 0.4 s.
`cg_fkFirstJumpDuration` holds the first jump for that many frames and
`cg_fkSecondJumpDelay` starts the second jump at that frame (both 0). A server
forbids it with bit 7 (`RESTRICT_FLIPKICKBIND`) of serverinfo `restricts`. See
[flip_kick.rs](../crates/sjk-viewer/src/input/flip_kick.rs).

`serverconfig` lists a JA+ server's options from the `jp_cinfo` value in its
serverinfo (flip kick, roll fix mode, DFA variants, kata, ledge grab, alternate
dimension and the rest), as the JA+ client plugin and EternalJK print them
locally; on jaPRO/TaystJK it is forwarded to the server, which answers it, and
elsewhere it reports that the server runs neither. `pluginDisable` lists the
fifteen JA+ client-plugin features with `Allowed`/`Disallowed`, and
`pluginDisable <id>` toggles one bit of the archived userinfo cvar
`cp_pluginDisable` (a set bit disables the feature). Its default, 1536, disables
the holstered saber and ledge grab, which need JA+ animations SJK does not have.
JA+ and TaystJK/jaPRO servers receive it from the connect packet on, and a
toggle sends a userinfo update ([networking.md](networking.md)).
See [console_mod_commands.rs](../crates/sjk-viewer/src/console_mod_commands.rs).

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
On Windows the clipboard is reached through PowerShell with UTF-8 in both
directions ([clipboard.rs](../crates/sjk-viewer/src/clipboard.rs)), so pasted or
copied symbols such as `€`, `’` or `×` keep their characters, including under
PowerShell's Constrained Language Mode (with `clip` as a last resort for copy). A
copy waits up to 1.5 s for the tool to finish and reports failure if it fails; a
paste waits for a copy still being set.
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
counter are suppressed, including every font batch. See
[console_browser.rs](../crates/sjk-viewer/src/console_browser.rs).

With the classic console (`con_style classic`) the browser is drawn classic+
([classic-plus.md](classic-plus.md)): the in-game pop-up's retail box and title
band over the dimmed screen, the four filters and Edit (Insert, Apply), Default,
Filter and Close as retail gold buttons with the `menu_buttonback` glow, a retail
list box whose selected row sits on `menu_blendbox2` (a changed cvar's value in
gold with its default beside it), a detail box with the selected entry's kind,
value, default and whole description, and the description line under the box.
It uses the menus' retail font when `ui_gameFont` is on, and works without the
retail art. Keys, pointer and wheel act as in the modern look
([console_browser_classic.rs](../crates/sjk-viewer/src/console_browser_classic.rs)).

With the SJK UI's console (`con_style sjk`, or `auto` with the SJK UI's
menus) the browser has the SJK UI's look, laid out as its
Settings over the darkened frame
([console_browser_sjk.rs](../crates/sjk-viewer/src/console_browser_sjk.rs)): Esc
"Back", "Commands and cvars" and the search pill at the top; the four filters
(All, Commands, Cvars, Changed) down a lit rail with how many entries each shows
for the search; the entries (name, description, the value at the right and a
gold dot when it differs from the default; a command's kind); and the chosen
entry's detail column: its kind, name, value, default and whole description,
then Edit value (Insert for a command, Apply while typing), Default and Copy.
The detail's name, value, default and description can be selected with the
mouse: a drag selects across lines, a double click selects a word
([text_select.rs](../crates/sjk-viewer/src/text_select.rs)); the selection lasts
while the same entry is shown. In every look Ctrl+C (or Ctrl+Insert) copies the
selected text, or else the chosen entry as a console line (a command's name, a
cvar's `name value`, the value quoted when empty or spaced), and the footer says
what was copied. Mouse selection is the SJK UI's look's only; the modern and
classic+ looks copy the chosen entry.

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
The modern console's text is 14 px Inter: the size is its line box (ascent plus
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

## Vehicle and creature assets

Vehicle appearances use the shared multiplayer `.veh` parser, including its
comment handling, model/skin selection and camera fields. Example definitions
inside block comments cannot replace the real vehicle model.

Vehicle projectile trails resolve their vehicle-weapon index through `.vwp`
definitions in model-registration order. Their authored EFX and optional rigid
models are preloaded when the vehicle is registered; an effect-only laser does
not acquire the ordinary rocket model. Custom vehicle flight-loop sounds remain
a separate audio gap.

Community NPCs need their model PK3 mounted by both the server and client. An NPC
definition alone cannot supply a missing mesh. SJK does not distribute those
packs. The animation-error isolation described in [rendering.md](rendering.md#actor-animation-failures)
protects other actors from malformed custom clips, but does not repair the clip.

## Shader remap controls

Server map recolors and material replacements are enabled by default. SJK
defaults to `cg_remaps 2`, EternalJK's default, which includes player-texture
configstring remaps; `cg_remaps 1` is TaystJK's default policy
excluding them, and `cg_remaps 0` disables server remaps.
Settings > GAME > "Shader remaps" sets the same cvar and applies at once.
A map's own worldspawn remaps always apply. `listRemaps` lists the map's, the
enabled server and the temporary local remaps in the order they were applied, each
labelled `map:`, `server:` or `local:`, and marks those a later remap overrides.
`remapShader <old> <new>` replaces a shader locally for the loaded map; remapping
it to itself restores the original. The latest remap of a shader wins, so a later
server remap replaces a local one. `clearRemaps`, EternalJK's renderer command,
drops every active remap whatever its origin, the map's included, without telling
the server; the world draws its own shaders until the server sends new remaps (a
shader-state change or a `remapShader` command) or a new map loads. As in
EternalJK, destination time offsets are kept. See [rendering](rendering.md#server-shader-remaps)
for scope and remaining limitations. These controls do not edit map geometry.
