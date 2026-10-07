# SJK UI

The SJK UI (`ui_menuStyle sjk`) is SJK's own menu style: SJK's menus redesigned
from the ground up, drawn over the live map in SJK's own type. It keeps the
feeling of Jedi Academy's menus (gold for what you choose, holo blue line-work,
the turning ring round the emblem) and drops their window frames and boxes. It
is SJK's default menu style since 08/10/2026 (Sol's call, before every screen
had its version); [classic+](classic-plus.md) stays a choice and keeps getting
fixes. A profile saved with the old default `classic` moves to `sjk` once
([client.md](client.md#menu-style)).

Status (07/10/2026): the main page, Settings (with the key bindings),
Character, What's new, Update, Identity, Servers (the server browser), the
loading screen, the scoreboard and the in-game menu are done. Every other
screen opens in its classic+ version (`MenuStyle::classic_screens`), which
covers the map as the classic style does.
Other styles: First setup's first row, Settings > Interface > Menu style, or
`ui_menuStyle classic`/`modern`; restart for the style's map behind the main page.

## Design

Sol chose the direction on 07/10/2026 from three mock-ups (a design canvas of
the main page in three directions, a Settings screen and the kit). The main page
is direction A, the ring: the emblem in a turning ring and the menu on an arc
round it, with the servers the player joined last in a column on its right, over
direction C's map (mp/duel6). (A first build combined A's ring with C's horizon
line and B's servers under Play; Sol went back to A the same day.)

Principles:

1. **Content floats on the world.** No boxes or window frames: structure comes
   from alignment, thin lines and dark fades at the screen's edges, over the
   live map.
2. **One memorable thing per screen.** On the main page, the ring and its gold
   arc, which turns to point at the chosen entry. Everything else stays quiet.
3. **Gold is the choice.** Gold marks what is chosen and the one main action of a
   screen; ember is only for leaving. Holo blue draws lines, never text that
   matters.
4. **Words, not capitals.** Navigation and labels are in sentence case.
5. **Keyboard first, pointer equal.** Every screen works with the arrows, Enter
   and Escape alone; hovering chooses, a click acts.
6. **One brand.** The colours and type are SJK's site's
   (`site/assets/style.css`), so the site, the release notes and the client
   read as one thing.

### Colours

Display values, like all 2D colours ([rendering.md](rendering.md)), in
`menu::sjk::color`:

| Token | Value | Use |
| --- | --- | --- |
| Space | `#060A14` | The ground and the fades; never pure black |
| Holo | `#A8CFFF` | Line-work: rules, ticks, outlines, the ring |
| Gold | `#E8B84A` | What is chosen; the screen's one main action |
| Gold bright | `#FFD97A` | Chosen text and marks |
| Text | `#E2E9F6` | Text |
| Muted | `#A9B5CB` | Secondary text |
| Quiet | `#8792AC` | An item that ends the session |
| Ember | `#FF7A3D` | Leaving: Quit |

### Type

Two bundled vector families, as on SJK's site, both under the SIL Open Font
License (licences beside the fonts in `crates/sjk-viewer/assets/fonts`):

- **Rajdhani** (SemiBold, Bold), the display family: navigation, titles, key caps,
  numbers. It lacks a few symbols (superscripts, fractions, ordinals, ¤, µ),
  which come from Exo 2 SemiBold.
- **Exo 2** (Regular, SemiBold), the body family: details, descriptions,
  everything else. Its static weights are instances of the variable font
  (`fonttools varLib.instancer`), since the rasterizer reads only a variable
  font's default instance.

Sizes are in pixels of a 1080-line screen and scale with the window's height:
the main page's entries 44 (chosen 68), server names 27, the player's name 26,
details 16 to 18, key hints 15 to 17; on Settings, its name 48, the categories
26, sub-headings 22, row names and descriptions 19, the detail's title 32 and
its facts 17.

Names keep their colour codes (`^1`...), drawn in a palette lifted to read on
the navy ground (`text::CodePalette::Legible`, which every text in the UI's
families uses): black shows as grey, and red, green, blue and magenta are
lighter with the same hue; the other colours are the game's.

`ui_gameFont` (classic game fonts) does not change the SJK UI's type; it keeps
applying to the classic screens and the in-game text.

Text is centred on its rectangle's middle line (`menu::sjk::text`): the
renderer sets a run's line box from the rectangle's top, so the helper places
it from each family's measured metrics, the middle of the capitals leaning a
quarter toward the lower case (0.489 of the size down the line box for
Rajdhani, 0.563 for Exo 2; a test checks them against the bundled fonts).
Every SJK UI text goes through it, so a control's text rectangle is simply the
control's own.

### Motion

The ring turns once every four minutes and the sunburst behind the emblem
slowly the other way; the gold arc eases towards the chosen entry (about 0.3 s);
the camera behind glides through its tour (The map behind); on the loading
screen a gold arc turns once every three seconds and the destination's
levelshot fades in (0.45 s). Nothing else moves on its own.

## The map behind

Behind the SJK UI the camera tours mp/duel6: ten authored shots
(`YAVIN_TRAINING_TOUR` in
[routes.rs](../crates/sjk-viewer/src/menu_backdrop/routes.rs), keyed by the map's
worldspawn message "Yavin Training Grounds"), each a 15-second glide from one
point to another while looking at a third, with a 0.9-second fade through the
UI's navy between them ([tour.rs](../crates/sjk-viewer/src/menu_backdrop/tour.rs)).
The tour opens on the map's own intermission view (the west wing's room looking
down the corridor at the tower), then plays the rest in a shuffled order that
changes every start and every round, never showing the same shot twice in a
row. The shots: the tower from three gardens, high; the east and south wings'
rooms; the tower from its foot; over the courtyard's wall at its base; the
south temple's stair; a diagonal path from the tower. They were placed with the
off-screen world shots and checked to glide through open air (the north wing's
view, full of leaves, and the west garden's, half a wall, were dropped).

On a map with a tour, a screen whose shot has a route is reached by fading out,
cutting to the shot and fading in, instead of flying there, since a tour's
shots are all over the map; every screen shows at once. The fade is a layer
over the world only (`ClientMenu::world_fade`), under the menus and the
console's pages. duel6 has two such shots, both on the south-west path from the
tower, between its stone benches, where the player's model stands
(`YAVIN_TRAINING_PLAYER`, floor at z 352, facing the camera): the player shot,
80 units off and turned so the model stands right of the screen's middle with
the tower behind; and the saber shot (`YAVIN_TRAINING_SABER`), a step closer,
the thrown saber floating between the camera and the model.

## Main page

[home.rs](../crates/sjk-viewer/src/menu/sjk/home.rs), drawn over the live map,
which is mp/duel6 when the client starts in the SJK UI
(`MenuStyle::boot_map`). It is laid out on a 16:9 frame of 1080-line pixels
centred in the window: a wider window shows more map at its sides, a narrower
one (4:3, 5:4) scales the frame down to its width.

- **Ring:** SJK's emblem (380 across) in a ring of tick marks, 312 in radius,
  centred at (620, 540). The ring is a light picture drawn by the emblem worker
  (`EmblemLayer::Ring`), tinted holo; a gold arc (24 degrees) on it points at the
  chosen entry, a dotted gold beam reaching towards it, and turns to the servers
  while they have the keyboard.
- **Arc:** the page's entries on an arc of radius 440 round the ring's right
  side, 16 degrees apart; the chosen one gold and larger, with a line saying
  what it opens. The pages:
  - Main: Play, Character (the player screen), Settings (the settings screen),
    Sol JK, Quit.
  - Play: Join a server (the browser), Create a game, Back.
  - Sol JK: What's new (the changelog), Update, Credits, Identity, Back.
  - Quit: Quit to desktop (ember when chosen), Stay. It opens on Stay.
  A page's name stands small over its first entry.
- **Recent servers:** a column on the right (its rule at x 1470) of the servers
  the player joined last, newest first, up to four, kept in
  `recent_servers.json` beside `favorites.json` ([recent.rs](../crates/sjk-viewer/src/menu/sjk/recent.rs)):
  a server is recorded when a join reaches the game, with its name and map,
  except a game this client hosts. Each shows its name (live from the server
  list, or as last seen), its players, map and ping while the server list has
  it, and when it was last played ("2 hours ago", "yesterday": relative, so no
  time zone is needed). Before any join, the column offers the JoF server
  (`135.125.145.49:29070`) under "Start here".
- **Corners:** the player, bottom left (a gold ring with their initial, their
  name with its colours, their model and blade); the keys of the page, bottom
  centre; the version and a newer release found, bottom right.

Keys: Up and Down move along the arc; Enter takes the entry; Right (or Tab)
moves to the servers, Up and Down choose one, Enter joins it, Left or Escape
returns to the arc. Escape on a page returns to the main page, on the entry that
opened it; on the main page it opens Quit's page. The pointer chooses by
hovering and acts with a click. A server with a password opens the browser's
password prompt.

## Character

[player_menu/sjk_view.rs](../crates/sjk-viewer/src/player_menu/sjk_view.rs): the
player screen, opened by the main page's Character. The model stands on duel6's
stage in the map's own light, holding the saber draft lit, and every change
shows on it at once (the modern screen's `menu_stage`). The screen is laid out
on the 16:9 frame, dark behind the form on the left and clear over the model.

- **Top:** the way back (Esc, "Main menu") and the player's name, with its
  colours, as the title; under it the pages as tabs, Character, Saber and
  Force, the one on show gold and underlined.
- **Character:** Name (a field; Enter types), Team colour, Search (a field that
  filters the grid), Model (‹ › steps through the grid), the model grid (eight
  icons a row, as many rows as fit, the current one ringed gold, scrolled by
  the wheel), then a species' head, torso, legs and skin colour and the hat and
  cape, two to a line.
- **Saber:** Style as three buttons (Single, Staff, Dual); the hilts as a list
  under a sub-heading (Hilt, or Staff), in two columns, six lines on show,
  scrolled by the wheel or its bar, the chosen one gold with a dot and kept in
  view (in the middle when the list first shows); for Dual two lists side by
  side, Right hand and Left hand. The hilts come in the game's load order, as
  JoF EJK lists them (`legacy_saber_load_order`). Under them the blade's colour
  as seven chips (the six stock colours and the custom one, ringed gold when
  chosen), its red, green and blue sliders (a digit types the number), and the
  second blade's for Dual. The camera cuts to the saber shot, where the model
  throws the saber to float and turn before it.
- **Force:** the mastery ("Jedi Master") and the points left of the rank's
  ("18 of 100 points left", and "not applied yet" while the draft differs)
  over a gold bar of the points left; the Light and Dark sides as two cards
  with their emblems; the powers in classic+'s groups under sub-headings,
  Neutral and Lightsaber down the left, the side's five on the right (the
  other side's left out, as on classic+): each its holocron, its name and its
  three levels as cells marked with what each costs (or "free"), the bought
  ones gold. Hovering a level previews buying up to it: its cells lit, the
  points it would take white at the bar's end (ember, cells and bar, when
  there are too few). A click on a level buys up to it, on the power's own
  level drops one. Then Start over, Discard and Apply (gold while there is
  something to apply).
- **Power box:** on the Force page, bottom right, the power under the pointer
  (or the keyboard) in big: its holocron, name, group, level, what it does in a
  line or two, the next level's price (or Mastered, Team games only...) and
  its levels' prices.
- **Caption:** beside the model, bottom right under a short gold rule, what the
  page shows of it: the model and its skin, the saber's style, hilt and blade,
  or (on the Force page with no power chosen) the side and rank.
- **Keys:** bottom right, the focused row's (Left Right change, Enter type or
  do it) and Tab with the next page's name.

The keys are the modern screen's: Up and Down choose a row (on the Saber and
Force pages in the order they show them: both hilt lists before the blades,
the Force groups' powers in their order, the other side's skipped), Left and
Right change it (a hilt list's choice moves one hilt), Enter types or acts, Tab
and `[` `]` change page, Escape returns to the main page (dropping an
unapplied Force draft, as before). The pointer: a click on a control acts (‹ ›
by the half it lands on, a chip picks its colour, a slider follows, a style
button, a hilt, a level), a click elsewhere on a row only chooses it, a click
on a tile picks that model. The SJK view's own targets (levels, style buttons,
hilts and the lists' wheel areas, tokens from 1000) go to
`PlayerMenu::sjk_pointer` before the modern screen's pointer.

It is the modern screen's state and controller with another view: the rows,
tiles, tabs and back key answer to the modern screen's tokens, each row
registering its control before the whole row so the token's rectangle is the
control's. Opened from a game, where there is no stage, the player screen shows
its classic pages and their preview instead.

## Sol JK's pages

What's new, Update and Identity, which the main page's Sol JK page opens (and
their console commands), have the SJK UI's look in this style: drawn in its
families over the map darkened as Settings is, each with the way back (Esc,
"Back") and its name at the top and its keys bottom right. They stay the
console's pages (their state, keys, pointer and opening and closing are as in
the other looks); `ViewerConsole::set_sjk_pages` picks the look and the console
overlay routes their text to the UI's families (`console_sjk_pages.rs`).

- **What's new** ([changelog_sjk.rs](../crates/sjk-viewer/src/changelog_sjk.rs)):
  the releases down a lit rail on the left, newest first (name, then date or
  "Not released yet" in gold), the chosen one gold; beside them a reading
  column with its date and number of changes, its name, its introduction, then
  each change after a gold dot with its credit under it, scrolled by Page Up
  and Down or the wheel. Lines are wrapped by characters.
- **Update** ([update_panel_sjk.rs](../crates/sjk-viewer/src/update_panel_sjk.rs)):
  the version this is, what the check found as a headline over its detail, the
  download's progress as a gold bar with its percentage, and the page's actions
  as buttons, the one Enter takes gold (Install, Restart now, Check, Open
  release page), then Check again and Release notes when offered.
- **Identity** ([identity_panel_sjk.rs](../crates/sjk-viewer/src/identity_panel_sjk.rs)):
  the identity's state as a headline (the name the hub knows, or "Identity is
  off") over its lines, the switch sharing it with the hub, then while it is on
  the bio's field and Save (gold), Copy my key id and Use the official hub, and
  the known players here under a sub-heading, verified ones marked gold.

Credits keeps its own page.

## Settings

[sjk_view.rs](../crates/sjk-viewer/src/settings/sjk_view.rs) draws it, and
[settings.rs](../crates/sjk-viewer/src/menu/sjk/settings.rs) lists its
categories and opens, switches and closes it. The main page's Settings opens it
on the category last shown (Display the first time in a run), and First setup at
start opens it on First setup. It is laid out on the main page's 16:9 frame,
over the map darkened from the left (95 %) to the right (80 %), as the Settings
mock-up draws it.

- **Top:** the way back (an Esc key cap, "Main menu", also a click target) and
  the screen's name, top left; the search pill, top right ("Find a setting", its
  `/` key, then the number found).
- **Rail:** the categories down a lit holo line on the left, each with its
  settings icon: First setup, Display, Graphics, Sound, Mouse, Key bindings,
  Gameplay, Interface, HUD, Scoreboard, Network. The one on show is gold with a
  gold bar on the line; none is lit while a search shows its results.
  - They are the classic+ Setup page's groups (`settings::Group` and tabs), but
    Graphics gathers the renderer's four tabs (image, lighting, shadows, weather)
    under their names (`Group::Graphics`).
  - Key bindings shows every action under its group's sub-heading (Movement,
    Interaction, Weapons, Force powers, Other), its picture before its name
    where it has one, its two keys as caps on the right (a dash for an empty
    one); the cap awaiting a key turns gold ("Press a key") and the detail
    column says what to press. The detail column names its keys, default key,
    console command and what else its keys do. The search finds actions by
    name, command, key or group. Enter, or a click on the row or its first cap,
    awaits its first key; a click on its second cap, its second; Backspace
    clears its keys; Escape cancels the wait. It is the classic+ list of the
    key-binding editor (`keybind_editor/sjk_view.rs`).
- **Rows:** the open category's rows in a column (x 470 to 1270), 56 tall, under
  sub-headings (holo, with a rule after them): the name on the left, the control
  ending at 1226 and the reset arrow after it. Fourteen lines show; a longer
  category scrolls (wheel, scrollbar), keeping the focused row in view.
  - Controls, from the kit: a switch (gold with its knob right when on, On or
    Off after it); a slider (a gold-filled track and its number, which a click
    or a typed digit opens for typing); segments for a choice of up to three; a
    field with a caret for a longer list, the display mode, the resolution and
    the HUD (the last two open their full-screen pickers); a field for text.
  - The focused row has a soft band and a gold bar at its left; a row changed
    from its default a gold dot after its name, and when focused the reset arrow.
  - A list opens under its field (over it near the bottom), its choice in use
    marked with a gold dot; the controls under it are left out while it is open.
- **Detail:** on the right (x 1360, 464 wide), the focused setting: its
  category's icon (a search result's own group's) and its name, what it does,
  then its default, range or choices, when a change applies (gold), where a
  search found it and its console name (holo). First setup adds how to import
  another client's .cfg.
- **Keys:** bottom right, the keys of what has the keyboard: the focused row's
  (Enter switch, Left Right change, Enter choices...), Backspace for the default
  when it changed, Tab for the next category; a list's, a search's or a typed
  value's own while they are open.

Keys: Up and Down choose a row (Up from the first, or `/`, goes to the search);
Left and Right change it; Enter switches, opens a list or starts typing;
Backspace returns it to its default; Tab and `]` (`[` back) open the next
category, past Key bindings; Escape closes a list, clears a search, then returns
to the main page. Hovering a row chooses it; a click on a control acts, a click
on a row's name only chooses it (a switch flips from anywhere on its row); the
right button returns a row to its default.

The rows are a classic+ panel's (`settings::ClassicRows`), so the search over
every setting, the lists, the defaults and typed numbers are the classic+
panels' own; only the drawing and the frame are the SJK UI's. Changing Menu
style on the screen (it is on Interface) hands over to the classic+ panel of the
same group (Graphics: the renderer's image tab), or to the modern screen.

## Servers

[browser.rs](../crates/sjk-viewer/src/menu/sjk/browser.rs): the server browser,
opened by Play's Join a server (and by the in-game menu's), over the map darkened
as Settings is, on the main page's 16:9 frame. The camera keeps touring behind it.

- **Top:** the way back (Esc, "Main menu", or "Game menu" from a game), the
  screen's name ("Servers") and the search pill ("Find a server or map", `/`,
  then the number found). The search matches names without their colour codes,
  and map names.
- **Left:** where the servers come from down a lit rail, All servers and
  Favourites with how many of each answered; under "Show", switches for empty,
  full and locked servers and the game type as a cycler (All, then each type);
  at its foot Refresh the list and Join by address.
- **List:** a header line of sortable columns (Server, Mode, Map, Players,
  Ping; the sorted one gold with a sort mark after it, three stacked bars
  longest at the top when descending, at the bottom when ascending; a click
  sorts or flips it), then fourteen rows of 52 that scroll (wheel, scrollbar).
  A row: a gold dot for a favourite, the name in its colours (faded when
  nobody but bots plays), a gold padlock when it needs a password, the game type with
  the mod as a small tag (JA+, JAPRO, Mod; none for base), the map without
  `mp/`, players over capacity, four signal bars lit by the ping (four to 60 ms,
  three to 110, two to 180, one to 400) and the ping. The chosen row has the
  band and gold bar; favourites sort first. An empty list says why (the search,
  no favourite answered, the master server being asked, nothing answered, or
  the Show choices hiding everything).
- **Right:** the chosen server: its map's levelshot (cropped, never stretched:
  a square retail levelshot is a 4:3 picture), the map's name over its foot;
  the server's name in its colours (a second line goes on in the colour the
  first ended in); Mode, Players (with its bots), Ping and Mod (the server's
  own name for its mod once its status answers); its address, "Needs a
  password" in gold when it does, its limits in words ("30 frags, 20
  minutes"); Join (gold) and Add to favourites or Remove favourite; then
  "Playing now", its players with their colours and scores, bots muted, five
  then "and n more" past six.
- **Keys:** bottom right, what has the keyboard's; bottom left, how the list
  stands ("12 servers answered", "Asking the master server...").
- **Prompts:** the password and the address open a card over the darkened map
  (the list is left out under it, since text draws over every shape): its
  title, a line saying what to type, the field (the password as dots), the
  address's error in gold, Cancel and Join or Connect (gold, dimmed while the
  field is empty).

Keys: the list has the shared browser keys (Up and Down, Page Up and Down, Home
and End, Enter joins, F favourite, Tab between all servers and favourites, R
refresh, C an address, `/` search, 1 to 5 sort); Left moves to the left column,
where Up and Down choose, Enter shows a source or flips a switch, Left and Right
step the game type, and Right or Escape return to the list. While the search is
typed, Enter, Down or Tab return to the list (keeping it); Escape clears it.
Escape on the list clears a search first, then returns to the main page. The
pointer: a click on a row chooses it and a second within 0.4 s joins it; a
header sorts; a switch's row flips it; the game type steps back on its left
half and on on its right half.

Joining shows the SJK UI's loading screen (Loading); mp/duel6 has no gate to
fly through.

## Loading

[loading.rs](../crates/sjk-viewer/src/menu/sjk/loading.rs): what a join shows
from the browser, the main page's recent servers, an address or Create game
until the game is on screen, and a server's change of map. Its keys are the
classic screen's: Escape cancels (back to the browser, or to Create game for a
hosted game; on a server's change of map it leaves the server); after a failure
Escape, Enter or Space return to the browser. The pointer's target is the Esc
key cap at the bottom right only, not the whole screen as on the classic one,
so a stray click does not end a join.

- **Before the map is known** (an address typed or a recent server the list
  does not have; the server's info answer names the map within a second): the
  menu's map stays on show and its camera keeps touring, darkened towards the
  bottom and its left. A block at the bottom left: "Joining" small over the
  server's name large (Rajdhani 56, with its colours: the server list's or the
  recent servers' name, else the address), its address in holo, then a thin
  line 440 long that fills gold by step, and on the bottom row a slowly turning
  gold arc with the step in words. Esc "cancel" sits at the bottom right.
- **Once the map is known** (the browser row's map, the server's answer or the
  gamestate): the map's levelshot fades in over the whole window, never
  stretched (`levelshot::screen_fit`): one as wide as the window or narrower
  covers it, cut at its top and bottom (a square retail levelshot is a 4:3
  picture); a wider one (the HD packs' 2:1 shots, their title against the left
  edge) keeps its whole width with navy bands above and below. It is darkened a
  little all over, more towards the bottom and its left and a touch at the
  top. At the bottom left, from the top: "Joining", the map's own name from its
  worldspawn large (Rajdhani 124, 96 past 18 letters; the file name when it has
  none), its file name without `mp/` small, the server's name (Rajdhani 36, with its colours), then
  the classic screen's lines in words: the game type and limits ("Free for
  all, 30 frags, 20 minutes"; before the gamestate the server list's game type
  and players), the mod and the server's rules ("JA+ Mod v2.6 · Force mastery:
  Jedi Master · Saber only", "No Force powers", "Force-based teams", "Cheats
  on"), and the message of the day (up to two lines, without colour codes). The
  bottom row has the arc and the step, and a thin gold bar runs across the
  bottom of the frame.
- **Steps** (`loading::Step`): Asking the server, Connecting, Receiving the
  game state, Joining the game (the gamestate is in), then once the session is
  in hand Loading <map>, Preparing the graphics and Entering the game; a
  download in progress shows its own line instead ("Downloading x.pk3: 120 /
  900 KiB"). The line and bar fill one seventh a step and never run back within
  a join: on the menu's map the browser's map is first built as a preview,
  which the session's own build replaces, so the world's steps count only for
  the session's world.
- **A failure** keeps the layout of the state it happened in, with "Could not
  join" (or "Connection lost" once the session was in hand) over the name, the
  reason in words on the bottom row instead of the step, no line or bar, and
  Esc "back to servers".
- **A server's change of map**: "Next map" over the server's name. Until the
  new gamestate names the map, the block shows (the classic screen keeps the
  last map's levelshot meanwhile; this one does not), and Esc says "leave the
  server".
- **Starting a hosted game**: "Starting your game" over "Your game" (or the
  game's own name once its gamestate is in), without an address.

The world under the screen: on the menu's map the world keeps rendering under
it (the join's destination loads beside it, as for the classic style) until
the destination's levelshot has faded in, then it is left out
(`ClientMenu::sjk_loading_hides_world`, read by
`menu_backdrop::classic_hides_world`). On a server's world (a change of map, or
the joined world installed before the game takes over) the world is always
left out and the screen draws on the UI's navy ground; the levelshot fades in
over it.

## Scoreboard

[scoreboard/sjk.rs](../crates/sjk-viewer/src/scoreboard/sjk.rs): the scoreboard
held with Tab in a match (and shown at intermission), `cg_scoreboardStyle sjk`.
Its default, `auto`, shows it while `ui_menuStyle` is `sjk` and the classic
scoreboard otherwise; a profile's saved old default (`classic`) moves to it
once, a style chosen after that stays ([client.md](client.md#scoreboard-styles)). No panel: the columns float over
the game, which is darkened by the UI's navy (a third everywhere, deeper behind
the header and, from the chat column's edge, behind the columns). It fades in
and out as the classic board does, settling into place as it opens.

- **Header line:** across the top, as the other screens' top bar: the map
  without `mp/` (Rajdhani 48), then the mode and its limits in words ("Free for
  all · 30 frags · 20 minutes", as the browser words them); on the right the
  time left in a timed match ("12:12 left", else "3:12 played") and your place,
  the screen's one memorable thing, in gold ("3rd", "of 14" after it, "of 14,
  tied" on a tie; "Spectating" when you watch). Team games say "Your team leads
  by 2", "trails by", "Tied at 5" (or which team leads, for a spectator). Under
  the map, "Killed by" and the name while you are dead.
- **Where:** the columns start at x 680 of the frame, right of the chat column
  the scoreboard keeps for messages and the composer (`scoreboard::layout`, 640
  pixels of a 1080-line window), and end at 1824.
- **Rows:** under muted column labels and a thin holo rule, one row each,
  52 tall at most, with a hairline under it: the place (Rajdhani; ties share
  it), the name with its colours (Exo 2), SJK's emblem right after it for a
  player the hub knows (gold when vouched for, as elsewhere), at intermission a
  gold "Ready", then the score, the deaths when `cg_scoredeaths` counts them,
  the minutes played, and the ping as the browser's signal bars and the number.
  Your row has the UI's band with its gold bar, and its place, score, bars and
  ping in gold. Bots and players still joining are muted, with "Bot", "Joining"
  or "Connecting" for their ping.
- **Free for all** (and Holocron, Jedi Master): one list by place; once rows
  would be thinner than 34 pixels (past 22 players) two lists side by side, 16
  each at 32.
- **Team games:** the two teams side by side, red left and blue right, each
  under its head: the team's score in Rajdhani 76 (muted while it trails), the
  team's name in its muted colour with how many play, and a thin rule in that
  colour. Capture modes show score, captures, assists and defends (no minutes,
  for room) and a carried flag's icon before the name. A team too long for
  rows of 26 pixels ends with "and n more", keeping your row.
- **Duel and power duel:** the duelists (`CS_CLIENT_DUELISTS`, else the
  players not spectating) as two facing cards about the board's middle with
  "vs" between them: the name (Rajdhani 44), the score large (112, gold for
  you), the record and health in words ("3 wins · 1 loss · 87 health · 25
  shield"; your health and shield from your snapshot, an opponent's health when
  the server shares it, `g_showDuelHealths`), over a rule, gold under your
  card. A power duel's pair stack two smaller cards facing the lone duelist.
  Under them "Waiting to duel": everyone else in turn, with their record.
- **Spectators:** one line at the bottom, "Watching" then their names without
  colours.

A full server fits the board's canvas (320 text runs, 1024 draw commands) in
every mode, at 1080 lines and 4K (unit tests). The look draws in the UI's
families, which load for it even with another menu style when it is chosen on
its own; their metrics place what follows a measured run (the emblem after a
name, the header's right side).

## In-game menu

[sjk_view.rs](../crates/sjk-viewer/src/ingame_menu/sjk_view.rs) draws it and
[sjk_actions.rs](../crates/sjk-viewer/src/ingame_menu/sjk_actions.rs) holds what
its entries do. Escape in a match opens it; the match keeps drawing, under a
dark fade from the left edge (deep there, clear by the middle), on the main
page's 16:9 frame.

- **Arc:** a compact version of the main page's: the page's entries on an arc
  whose centre lies off the frame's left edge (radius 580, the middle entry at x
  440), 68 apart (50 for eleven or twelve, 36 for more, smaller type), the
  chosen one gold and larger with a line under it saying what it opens; a page's
  name stands small over its first entry. Beside them a lit holo rail with a
  gold mark that eases to the chosen entry, and inside the curve SJK's emblem
  in its turning ring. Leaving entries are ember when chosen; Back and Stay are
  quiet, as is the main page's Leave.
- **Pages:**
  - Main: Resume, Team, Players, Vote, Character, Settings, Servers, Shot
    controls, Sol JK, Leave.
  - Players: a small scoreboard in place of the arc ([identity.md](identity.md#player-reports)):
    the title and how many are on the server (or why the player cannot report, in
    ember), then a table from x 340 to 1330 under a darker fade: a team's colour
    mark, the name in its colours, score, ping ("bot" for a bot) and what the SJK
    hub knows ("Verified" in gold, "SJK" in holo, "You"), 37 apart, sixteen a page
    with More players... and Back; the chosen row has a gold band and edge. The
    card on the right becomes the chosen player's: name, slot and side, what the
    hub knows (key and hub name), score and ping, and "Enter: report this player"
    in gold, or why not after an ember mark. Enter opens Report.
  - Report a player: the seven reasons on the arc with what each covers, and Back;
    the player's card stays on the right. Without the right to report, the
    reasons are dimmed and passed over, and Back's line says why. A reason opens
    the text dialog; Escape returns to the player's row.
  - Team: Join the game and Spectate, or in a team game Auto-join, Red team,
    Blue team (each with its colour and players) and Spectate; the side the
    player is on is quiet ("Your team", "You are watching") and passed over by
    the keys. In Siege, Team opens the class list (the shared rows, a row's
    detail after its label).
  - Vote: Vote yes and Vote no (the vote on, with its counts, under them;
    passed over while none is on), Call a vote; without a vote on, Vote opens
    Call a vote directly, whose lists (map, game type, kick, warmup, limits)
    are the shared call-vote lists. Voting or calling a vote returns to the
    match, as retail's pop-ups do.
  - Sol JK: What's new, Credits, Identity, Report a bug, Report a player (the
    Players page).
  - Leave: Leave the server, Quit to desktop (ember when chosen), Stay. It opens on Stay, since its rows act at once.
- **Match card:** on the right (x 1360, 464 wide) over its own fade: the
  server's name with its colours and its address, a rule, the map without
  `mp/`, its mode and limits in words ("FFA, 30 frags, 20 minutes"), then the
  numbers: the player's score and place ("3rd", "of 14"; "Tied" when tied) or,
  in a team game, both teams' scores (the player's marked "Your team") and the
  player's score, and the clock (time left of the time limit, or played); under
  a rule how many play, watch and fit, and "You are spectating" for a
  spectator. It reads the live session each frame and the server's info once
  a second; without a server (a map explored alone) it hides.
- **Keys:** bottom centre: Up Down choose, Enter open (report on Players, write
  on Report a player), Esc resume (back on a page).

Escape on a page returns to the main page on the entry that opened it (a
call-vote list to its row of Call a vote); on the main page it resumes. The
pointer chooses by hovering and acts with a click. Character, Settings and
Servers hand over to their screens over the match and come back on their
entry: Settings and Servers are the SJK UI's, opened on the category last shown
and with "Game menu" as their way back; Character opens the classic pages, as
the player screen does in a game (there is no stage there). Shot controls
opens the shot panel in its own look. Server info and Controls have no entries:
the card shows the server and Settings holds the key bindings. The Report a bug
button the other looks put at the bottom is left out; Sol JK's page has it.

## Implementation

- `ui_menuStyle` has a third value, `sjk` (`menu::style::MenuStyle::Sjk`).
  `MenuStyle::classic_screens` is true for it, so every screen without an SJK UI
  version opens its classic one; the main page dispatches to `menu::sjk::home`.
  `ClientMenu::sjk_screen` says when one of the SJK UI's own screens is on show
  (the main page, Settings without a picker open, Character, Servers, the
  loading screen): the map is drawn under it (the loading screen leaves it out
  itself when it has to)
  (`classic_hides_world` leaves it out) and its text goes to the UI's families
  (`append_sjk_screen`).
- The controls are drawn by `menu::sjk::kit` (the band, switch, slider,
  segments, field, list, reset arrow, changed dot, sub-heading, search pill,
  lit rail, cycler, colour chips, button and rank pips), in frame pixels
  (`menu::sjk::Frame`).
- The player screen's style: `PlayerMenu::set_sjk` (from `set_menu_art`) draws
  the modern screen in the SJK UI's view; `ClientMenu::sjk_screen` covers the
  player phase then, so the map and the stage model show under it.
- Settings is the classic+ panel's state with another view: the menu opens a
  category's rows as the classic+ panels do, keeps `classic_panel` empty and
  marks the SJK UI's screen open (`sjk::settings::SettingsPage`); the rows'
  results (`SettingsResult::Classic` for a category, `ClassicCycle` for Tab) are
  read as the rail's. Pointer tokens are the classic+ panel's (rows, values,
  segments, reset, list, search, scroll), the rail's categories its chrome
  tokens.
- Servers is the browser's state with another view: `ServerBrowser` and its
  pointer tokens (rows, headers, the tabs for the sources, the filter strip's
  for Show, the prompts') are the other styles'. Its keys go first to
  `ClientMenu::sjk_browser_key`, which handles the search, the left column
  (`sjk::browser::BrowserPage`) and Left, and leaves the rest to the shared
  `browser_key`; its pointer to `sjk_browser_pointer` before the shared one.
  The chosen server's levelshot comes through the create-game screen's cache
  (`CreateGame::service_levelshot_for`, which also gives its size).
- The loading screen is the classic loading screen's state with another view:
  `classic::loading::ClassicLoading` (fed by the join, `set_game` and
  `sync_classic_loading`) also keeps the session's world stage and whether the
  session is in hand (`set_world`), a count of joins (`generation`), whether
  the load named its map (`named_map`) and the gamestate's facts in words
  (`sjk::loading::Facts`). The screen's own memory
  (`sjk::loading::LoadingPage`) is the furthest step of the join and the
  levelshot's fade. Its levelshot comes through the same cache, serviced by
  `upload_menu_images` as for the classic screen.
- The scoreboard is the scoreboard module's state with another look:
  `style::ScoreboardStyle::Sjk` (resolved from `cg_scoreboardStyle` and
  `ui_menuStyle`) builds its rows with `scoreboard::sjk::build` on the board's
  own canvas, through the HUD's path (`scoreboard::append_overlay`), not the
  menu's; the text goes to the UI's families (`append_text_families`), or Inter
  until they load. `game_font::prepare` loads the families when the scoreboard
  wants them too. The ping bars are the browser's (`browser::signal`).
- The in-game menu keeps the in-game menu's pages, rows and actions
  (`ingame_menu::InGameMenu`, `Page`): `InGameMenu::is_sjk` picks the SJK UI's
  look (`is_classic` no longer covers it), `sjk_view::prepare` writes the rows
  it words its own way (Main, Team, Vote, Sol JK, Leave) with their hints, and
  the shared rows (Siege, the call-vote lists) keep theirs, a "label  /
  detail" row splitting into label and hint. `GpuState::activate_sjk_ui_row`
  acts on its own rows before the shared actions; `back_or_close_game_menu`
  goes to `sjk_view::parent` (the entry that opened the page). The keys pass
  over rows that cannot be taken (`InGameMenu::sjk_step`); a screen handed
  over to returns on its entry (`InGameMenu::return_row`, read by
  `MenuAction::ReturnToGameMenu`). The match card (`sjk_view::Card`) is
  refreshed from the session by `GpuState::refresh_game_menu_card`. Settings
  opened from it is `ClientMenu::open_sjk_settings_from_game`; Settings' and
  Key bindings' top bar take their way back from `settings::Rail::back`. The 2D
  pass now also runs while the game menu is open without a session
  (`frame_overlays.rs`), which an explored map's game menu needed too.
- Fonts: `text::load_family` rasterizes a family's two faces into one atlas, as
  Inter's, taking glyphs a face lacks from its fallback family. The SJK UI's
  families load the first time the style is on, rasterized at 1.5x (glyphs 144
  pixels tall, sharp at 4K), into GPU layers kept by `game_font::GameFonts`.
  `MenuCanvas::set_family` tags each text run with its family
  (`menu_widgets::TextFamily`), and `append_text_families` routes the runs to
  their family's layer. Until the families are loaded the page draws in Inter.
- Snapshots: `menu_snapshot::sjk_home_snapshot` and `sjk_settings_snapshot` draw
  the screens over the JoF HD wide levelshot of mp/duel6, each family from its
  own atlas. `world_shot::tests::duel6_sjk_menu` renders the real frames: the
  client built without a window on duel6, the SJK UI over the touring camera;
  `duel6_sjk_browser` the browser on made-up servers (one answered status),
  a search and the password prompt; `duel6_sjk_loading` the loading screen on
  a made-up join of the JoF server (before the map is known, loading mp/ffa3,
  failed with and without the map known, a server's change of map on the navy
  ground);
  `duel6_sjk_scoreboard` the scoreboard on
  made-up matches (`scoreboard::shot`: free for all with 14 and with 30
  players, capture the flag, a duel, a power duel, and a 4:3 window), drawn
  without a server;
  `duel6_sjk_ingame` the in-game menu over
  duel6 on a made-up match (`Card::for_shot`, `InGameMenu::sjk_for_shot`):
  the main page, Team in a CTF, a vote on, the installed maps to vote for,
  Leave, a spectator's card and Settings opened from it.

## Plan

The next screens, in order; each gets snapshot tests before it replaces its
classic version:

1. The dialogs (the report box, the import page) and Credits.
2. Settings' search finding key bindings too (it finds settings; Key bindings'
   finds keys).
3. A player screen for the in-game menu without the stage (it opens the
   classic pages for now).
4. Create a game.

`sjk` became the default `ui_menuStyle` before these were done. mp/duel6 has
its tour, player stage and saber shot; it has no gate, which mp/ffa3's browser
flies through on a join.
