# Client

Build instructions are in [development.md](development.md). JKR needs an installed
Jedi Academy `GameData` directory with the retail `base/assets*.pk3` files.

## Launch

Put `jkr-viewer` (`jkr-viewer.exe` on Windows) inside the installed game's
`GameData` folder, beside `base/`, then launch it to open the main menu. A shortcut
can use any working directory. No path settings are required. Keep
`jkr-dedicated` (`jkr-dedicated.exe` on Windows) beside the client for Create game
and local `devmap`.

From that folder:

```sh
./jkr-viewer
```

Join a server directly:

```sh
./jkr-viewer --connect 127.0.0.1:29071
```

Without an explicit positional GameData argument, discovery checks these locations
in order and uses the first containing `base/assets0.pk3` and `base/assets3.pk3`:

1. `JKR_GAME_DATA`, if set and nonempty.
2. The executable's directory, then its `GameData` subdirectory.
3. The saved `fs_gameData` setting.
4. The working directory, its `GameData` subdirectory, then its
   `Star Wars Jedi Knight - Jedi Academy/GameData` subdirectory.
5. The existing Linux Steam and `~/Games/Jedi Academy/GameData` locations.

Thus a drop-in installation wins over a saved location from another installation,
while an explicit environment or positional path still overrides it. Invalid
discovery candidates are skipped; an invalid explicit positional path is an error.
Discovery does not change the working directory or move any game data.

For a binary kept separately, `JKR_GAME_DATA=/path/to/GameData ./jkr-viewer` opens
the main menu. Positional launch also accepts a map path and optional player-model
directory (`jkr-viewer /path/to/GameData maps/mp/ffa3.bsp`); it remains a direct
world/viewer launch, whereas no arguments opens the main menu. Demo playback
retains its explicit GameData argument.
See [launch.rs](../crates/jkr-viewer/src/launch.rs) and
[app_launch.rs](../crates/jkr-viewer/src/app_launch.rs).

The client has a server browser, player and settings screens, in-game menus,
console, HUD, screenshots, demo recording/playback and a Create game flow.
Presence here describes implemented surfaces; validation limits are in
[status.md](status.md).

Create game starts a child `jkr-dedicated`, normally found beside the client.
Set `JKR_DEDICATED` to its executable path if installed elsewhere. The child
lifetime is managed by the client and defaults to local access; see
[local_server.rs](../crates/jkr-viewer/src/local_server.rs).

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
    Custom to character creation, APPLY on to lightsaber creation, Exit.
  - Character creation: species, skin tint swatches, the Head, Torso and Legs
    lists, Back and APPLY. Entering it from an ordinary character puts on the
    first species, as retail's Custom did.
  - Lightsaber creation: saber type, the hilt list (two for Dual Sabers), the
    six blade colour swatches (two rows for Dual), Apply, and Apply back to the
    main menu.

  Retail drew a live 3D model and a spinning saber. With no world behind the
  pages yet, the model's portrait and a drawn hilt and blade stand in, and the
  part lists show variant names where retail showed each variant's icon. The
  swatches are filled with the tint each `playerchoice.txt` entry sets, not its
  swatch image. Escape returns to the profile page, then to the menu.

Retail entries JKR has no screen for yet (Play Demo, Rules, Mods, Defaults) are
shown dimmed, and their description line says so.

Controls and Setup keep their group list down the left and show the chosen
group's items in the panel beside it, opening on Movement and Video as retail's
pages do. The items are the same settings and key bindings as the modern
screens, drawn the retail way: labels set against a column at retail `textalignx`,
the value after them, toggles as Yes/No, numbers as the retail slider (`menu/new`
art) with the value beside it, the focused item on the `menu_blendbox`
highlight, and the open group's entry in white.

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
Tab moves to the next group (on the key-binding groups, Left and Right do too). A key binding reads "A or B" (retail's `KEYBIND_OR`)
or `???` when unbound; Enter or a click waits for the new key, shown in red with
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
match. Its Join Red, Join Blue and Spectate buttons are left to the Join tab,
and the Force configuration button is not there yet. Controls and Setup open
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
band, bar) are converted to alpha at decode time. Animated retail stages (ring
rotation, scrolling glyphs, logo glint, the logo video) are drawn still. A
missing image falls back to JKR's own shapes. Retail assets are never bundled.

Outside a match the classic style draws no world. The main pages are opaque
over the retail background (the centre gap where retail played its logo video
stays dark), and the modern screens they open (Settings, key bindings, Player,
server browser, Create game) get the retail backdrop beneath them. The frame
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
in [loading.rs](../crates/jkr-viewer/src/menu/classic/loading.rs).

The profile pages are in
[player_menu/classic.rs](../crates/jkr-viewer/src/player_menu/classic.rs), with
entries and retail geometry in
[layout.rs](../crates/jkr-viewer/src/player_menu/classic/layout.rs), drawing in
[view.rs](../crates/jkr-viewer/src/player_menu/classic/view.rs) and pointer
routing in [pointer.rs](../crates/jkr-viewer/src/player_menu/classic/pointer.rs).
The main menu code is in [menu/classic.rs](../crates/jkr-viewer/src/menu/classic.rs): the
page tables are in [pages.rs](../crates/jkr-viewer/src/menu/classic/pages.rs),
types and geometry in [layout.rs](../crates/jkr-viewer/src/menu/classic/layout.rs)
and drawing in [view.rs](../crates/jkr-viewer/src/menu/classic/view.rs). The
option panels' frame is [panel.rs](../crates/jkr-viewer/src/menu/classic/panel.rs);
their items are drawn by
[settings/classic_view.rs](../crates/jkr-viewer/src/settings/classic_view.rs) and
[keybind_editor/classic_view.rs](../crates/jkr-viewer/src/keybind_editor/classic_view.rs).
The
in-game version is in
[ingame_menu/classic.rs](../crates/jkr-viewer/src/ingame_menu/classic.rs), with
[classic_view.rs](../crates/jkr-viewer/src/ingame_menu/classic_view.rs) and
[classic_actions.rs](../crates/jkr-viewer/src/ingame_menu/classic_actions.rs).
The artwork is loaded in [menu/art.rs](../crates/jkr-viewer/src/menu/art.rs) and
bound in [ui_renderer/art.rs](../crates/jkr-viewer/src/ui_renderer/art.rs). The
style is read in [style.rs](../crates/jkr-viewer/src/menu/style.rs). Shared exits
are in [destination.rs](../crates/jkr-viewer/src/menu/destination.rs).

Planned follow-ups, each a new page or screen module, following the retail
`ui/jamp` menus:

- Classic versions of the screens the classic pages still open in the modern
  style: Join Server (`joinserver`, `serverinfo`, `findplayer`, `password`,
  `createfavorite`), Create Server (`createserver`, `advancedcreateserver`),
  Solo Game (`quickgame`), and the in-game `ingame_playerforce`.
- Retail option items JKR has no setting for (video quality presets, colour
  depth, geometric and texture detail, EAX, languages) are left out of the
  panels, and the video restart confirmation is not needed.
- On the profile pages: a rendered 3D model and saber, part icons and tint
  images in character creation, and portraits for every model (the shared UI
  icon atlas holds 207, so species after the characters show none).
- The screens with no JKR equivalent yet: Play Demo (`demo`), Rules
  (`rules*`), Mods, Defaults, Add Bot (`ingame_addbot`), Siege objectives and
  voice chat, and the error page (`error`).
- The retail fonts (`ui_gameFont`, a separate change) and the animated art
  stages.

The player screen's Character and Saber pages write their cvars as soon as a
value changes. The Force page edits a draft instead: Apply writes `forcepowers`
once, in the stock format and legalized as before, Discard returns to the applied
profile, and leaving the screen drops unapplied changes. While a draft is pending,
the page's points line reads NOT APPLIED and the footer's Back cap says that
leaving drops it. Power icons (`gfx/mp/f_icon_*`) and side emblems
(`gfx/hud/mpi_jlight`, `gfx/hud/mpi_dklight`) come from the installed game data;
without them the page shows text only. They take icon-atlas cells of their own
after the HUD's, so the character grid keeps all 207 of its icon cells. See
[force.rs](../crates/jkr-viewer/src/player_menu/force.rs) and
[force_view.rs](../crates/jkr-viewer/src/player_menu/force_view.rs).

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

## Development maps

Run `devmap mp/ffa3` in the client console to start and join an owned local
FFA server with cheats enabled, no bots and no match limits. Other installed
maps work too, including `devmap t2_rancor`; `maps/` and `.bsp` are optional.
The command appears in console completion/help. It uses the same `jkr-dedicated`
binary lookup as Create game (`JKR_DEDICATED` overrides the adjacent binary).

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
[player_sprites.rs](../crates/jkr-viewer/src/player_sprites.rs) and
[pmove_talk.rs](../crates/jkr-game-jka/src/pmove_talk.rs).

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

Implementation: [resident worlds](../crates/jkr-viewer/src/resident_world.rs),
[early exploration](../crates/jkr-viewer/src/resident_walk.rs),
[world handoff](../crates/jkr-viewer/src/session_transition.rs) and
[gate destination](../crates/jkr-viewer/src/portal.rs).

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
draw it ([classic.rs](../crates/jkr-viewer/src/scoreboard/classic.rs), after
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

## Configuration and content

The default writable client folder is `GameData/jkr/`, under the selected game
installation. It is independent of the executable's location and working
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
See [platform.rs](../crates/jkr-viewer/src/platform.rs) and
[storage.rs](../crates/jkr-viewer/src/platform/storage.rs).
Edit settings through the client, or edit the file while the client is stopped
so autosaving cannot overwrite your changes.

`com_maxfps` defaults to `-1` (AUTO in Settings > Video): frames are capped at the
refresh rate of the monitor holding the window, rounded to whole hertz and
re-read once a second, or at stock's 125 when the monitor reports none. `0` is
uncapped. The old default, 1000, saved in every existing profile, is reset to
AUTO once on first launch (marker `jkr_maxfpsDefaultVersion`); a cap chosen
afterwards is kept. The default is not saved to the configuration. On the slider
AUTO is the rail's left end: arrows step AUTO, 0, 25, 50 and so on, and typing
`-1` selects it. An uncapped
client saturates the GPU; screen recorders and streamers sharing it then skip
frames (OBS reported 83% skipped for encoding lag against an uncapped client at
4K). See [runtime_settings.rs](../crates/jkr-viewer/src/runtime_settings.rs).

The Video tab's Display mode row offers Windowed, Borderless fullscreen and,
where the windowing system supports it, Exclusive fullscreen (Wayland does not).
Stock `r_fullscreen` keeps its meaning, fullscreen on or off, and Alt+Enter still
toggles it. `jkr_exclusiveFullscreen` chooses the kind: 0 (default) is a borderless
window at the desktop size, 1 switches the monitor to the `r_resolution` video
mode. Stock JA's fullscreen is always the exclusive kind; JKR defaults to
borderless. Choosing Windowed leaves `jkr_exclusiveFullscreen` alone, so Alt+Enter
returns to the last fullscreen kind. Exclusive fullscreen without a monitor mode
of that size falls back to borderless.

Enter or a click on the Resolution row opens a list of the monitor's video-mode
sizes, grouped by aspect ratio with the monitor's own first and the size in use
highlighted. Windowed and borderless also list the classic presets that fit the
monitor and a custom `r_resolution`; borderless fullscreen always fills the
desktop and uses the size only when windowed. Left and Right step the row within
its aspect-ratio group. See [display.rs](../crates/jkr-viewer/src/settings/display.rs)
and [resolution.rs](../crates/jkr-viewer/src/settings/resolution.rs).

`fs_game`, `fs_basegame` and `fs_homepath` configure content search paths; restart
the client after changing them. Search precedence and shader protection are owned
by [asset_search_paths.rs](../crates/jkr-viewer/src/asset_search_paths.rs).

Downloaded content is stored separately from the retail installation and config.
On Linux the default is `$XDG_DATA_HOME/jkr/downloads/base`, falling back to
`~/.local/share/jkr/downloads/base`. `JKR_DOWNLOAD_HOME` overrides the download
root (the implementation appends `base`). See
[download_store.rs](../crates/jkr-viewer/src/download_store.rs).

Server reference lists use OpenJK's positional common-prefix rule: extra pak
names or checksums without a counterpart are ignored, including when one list
is empty. Download comparison and session cache selection share
[the compatibility parser](../crates/jkr-client/src/referenced_paks.rs), so a
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
[keys.rs](../crates/jkr-viewer/src/input/keys.rs) and
[key_names.rs](../crates/jkr-shell/src/key_names.rs).

## Colour codes

Text draws `^0` to `^9` as OpenJK's ten-entry colour table does: `^0`–`^7` are
the retail colours, `^8` is orange and `^9` grey (retail wrapped them onto black
and red). The table is `quake_color` in [text.rs](../crates/jkr-viewer/src/text.rs).

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
[dead_key.rs](../crates/jkr-viewer/src/input/dead_key.rs). The browser's search
field takes a dead `^` as a literal colour prefix. Opening or closing the console
clears the window's pending accent composition, so a dead toggle key such as `^`
on a German layout does not combine with the next letter.

`connect host:port`, `disconnect` and `reconnect` control the session.
`record`, `stoprecord`, `demo` and `playdemo` control demos.
`screenshot` and `screenshotJPEG` request captures; `condump filename` saves
console output. See [console registration](../crates/jkr-viewer/src/console_session.rs)
and [file commands](../crates/jkr-viewer/src/console_files.rs) for argument handling.

`tell <player> <message>` takes a slot number or, as in EternalJK, a name or a
unique part of one, ignoring case and colour codes; an exact name wins over longer
names containing it. The client sends the stock `tell <slot>` command; when no
player or several players match, it lists them and sends nothing. See
[console_tell.rs](../crates/jkr-viewer/src/console_tell.rs).

Held actions such as `+button12` work from binds, cfg files and the console. A
hold typed at the console lasts until its `-` command, as in the stock client:
opening the console, a menu or chat, or losing window focus, releases held keys
but not typed holds; `in_restart` and session changes release both. `+grapple` is
EternalJK's name for `+button12`, the JA+/JaPRO grapple hook; on a JA+ server
releasing it also taps `+use`, as EternalJK does. See
[input.rs](../crates/jkr-viewer/src/input.rs).

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
See [console_mod_commands.rs](../crates/jkr-viewer/src/console_mod_commands.rs).

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
See [console_editing.rs](../crates/jkr-viewer/src/console_editing.rs) and
[console_selection.rs](../crates/jkr-viewer/src/console_selection.rs). The input
line and output rows are drawn and measured through one
[ConsoleText](../crates/jkr-viewer/src/console_text.rs) per text size, so the
caret, highlights and mouse hits follow the drawn glyphs at any size or letter
spacing.

Tab completes the command or cvar name being typed, after a leading `/` or `\` and
after the last `;`. A unique name completes with a trailing space; otherwise the
input extends to the longest shared prefix and the matching commands and cvars,
with cvar values, are listed. Up to 16 matches also show their descriptions;
longer listings end with the match count instead. Enter strips one leading `/` or
`\` from the line, then applies the same completion while `cl_allowEnterCompletion`
is set, without listing when the input is already a full name. Nothing strips a
slash on a command after `;`, so completing that command drops it. Command
boundaries follow the shell's quote and escape rules, and no completion happens
inside an open quote. Arguments are not completed. See
[shell_completion.rs](../crates/jkr-shell/src/shell_completion.rs).

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
[console_browser.rs](../crates/jkr-viewer/src/console_browser.rs).

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
[camera.rs](../crates/jkr-viewer/src/camera.rs).

For graphics controls and diagnostics, see [rendering.md](rendering.md).

## Text size and spacing

The Settings screen's TEXT tab holds four archived cvars. Menu text draws as
before at the defaults; the console's rows are closer together than before.

| Cvar | Default | Range | Effect |
| --- | --- | --- | --- |
| `ui_textScale` | 1 | 0.8 to 1.2 | Text size on menu screens and the in-game menu |
| `ui_letterSpacing` | 0 | -0.05 to 0.15 | Extra space after each letter in menus and the console, as a fraction of the text size |
| `con_scale` | 1 | above 0 (menu: 0.5 to 2) | Size of the whole console: text, margins and rows |
| `con_lineSpacing` | 0.9 | 0.8 to 2 | Console history and notify row pitch as a multiple of the text size; at 0.8 descenders meet the next row's ascenders |

Menu text grows or shrinks about the centre of its line without moving the
layout, so the range is limited to what menu rows can hold; that style is
applied where retained text commands become glyph quads, see
[text/style.rs](../crates/jkr-viewer/src/text/style.rs). The console sizes its
own text with `con_scale` and puts the letter spacing into its layout
([console_view.rs](../crates/jkr-viewer/src/console_view.rs)), so anything that
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
See [console_options.rs](../crates/jkr-viewer/src/console_options.rs).

The chat box's wrapped body rows are one line box apart (18 px at 1080p and
`cg_chatBoxFontSize` 1; they were 27 px). Inter's capitals are 0.60 of that box,
the stock chat box's ratio (`ocr_a` capitals at scale 0.65 in rows 13 virtual
pixels apart, `CG_ChatBox_DrawStrings`), and descenders clear the next row by
0.18 of the box. A sender's name line advances 20 px to the body (was 26) and
messages are 8 px apart (were 10 after a named message, 14 otherwise); see
[chat/layout.rs](../crates/jkr-viewer/src/chat/layout.rs).
