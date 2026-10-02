# Client

Build instructions are in [development.md](development.md). JKR needs an installed
Jedi Academy `GameData` directory with the retail `base/assets*.pk3` files.

## Launch

Open the main menu using an explicit content environment:

```sh
JKR_GAME_DATA=/path/to/GameData ./target/release/jkr-viewer
```

Join a server directly:

```sh
./target/release/jkr-viewer /path/to/GameData --connect 127.0.0.1:29071
```

The no-argument launch also checks the saved game-data location and known install
locations. Positional launch accepts a map path and optional player-model directory;
it is a direct world/viewer launch, whereas no arguments opens the main menu.
See [launch.rs](../crates/jkr-viewer/src/launch.rs) and
[app_launch.rs](../crates/jkr-viewer/src/app_launch.rs).

The client has a server browser, player and settings screens, in-game menus,
console, HUD, screenshots, demo recording/playback and a Create game flow.
Presence here describes implemented surfaces; validation limits are in
[status.md](status.md).

Sliders in the settings screen and the saber RGB channels of the player screen
also take typed values: click the number right of the rail, press Enter on the
row or start typing digits while it is selected. Enter applies the number,
clamped to the slider's range and rounded to its step counted from the minimum;
Escape cancels, a click elsewhere applies it and Space still steps the slider.
See [slider_entry.rs](../crates/jkr-viewer/src/menu_widgets/slider_entry.rs) and
[the settings rules](../crates/jkr-viewer/src/settings/numeric.rs).

Create game starts a child `jkr-dedicated`, normally found beside the client.
Set `JKR_DEDICATED` to its executable path if installed elsewhere. The child
lifetime is managed by the client and defaults to local access; see
[local_server.rs](../crates/jkr-viewer/src/local_server.rs).

The player screen's Character and Saber pages write their cvars as soon as a
value changes. The Force page edits a draft instead: Apply writes `forcepowers`
once, in the stock format and legalized as before, Discard returns to the applied
profile, and leaving the screen drops unapplied changes. While a draft is pending,
the page's points line reads NOT APPLIED and the footer's Back cap says that
leaving drops it. Power icons (`gfx/mp/f_icon_*`) and side emblems
(`gfx/hud/mpi_jlight`, `gfx/hud/mpi_dklight`) come from the installed game data;
without them the page shows text only. See
[force.rs](../crates/jkr-viewer/src/player_menu/force.rs) and
[force_view.rs](../crates/jkr-viewer/src/player_menu/force_view.rs).

## Menu style

`ui_menuStyle` (Settings, GAME tab, "Menu style") picks the main-menu layout:
`modern` (default) or `classic`, which is close to the retail multiplayer menus
in layout and flow without porting their `.menu` scripts. The classic main menu
has the retail entries and order: Play, Profile, Controls and Setup in two
columns, Exit below. Play opens the retail "start playing" page (Join Server,
Create Server); Exit and Escape ask before quitting. The retail 640x480 layout is
fitted to the window height and centred. Only JKR's own shapes and text are
drawn, over the dimmed menu map.

Only the main menu has a classic version so far. Its entries open the modern
screens: the server browser, Create game, the Player screen, and Settings on the
CONTROLS tab (Controls) or the VIDEO tab (Setup). The code is in
[menu/classic.rs](../crates/jkr-viewer/src/menu/classic.rs): the pages and entries
are in [layout.rs](../crates/jkr-viewer/src/menu/classic/layout.rs), drawing is
in [view.rs](../crates/jkr-viewer/src/menu/classic/view.rs) and shared exits are
in [destination.rs](../crates/jkr-viewer/src/menu/destination.rs). The style is
read in [style.rs](../crates/jkr-viewer/src/menu/style.rs).

Planned follow-ups, each a new page or screen module beside the main menu,
following the retail `ui/jamp` menus:

- Start playing: Solo Game (`quickgame`), Play Demo (`demo`), Rules (`rules*`).
- Join Server (`joinserver`, `serverinfo`, `findplayer`, `password`,
  `createfavorite`) and Create Server (`createserver`, `advancedcreateserver`).
- Profile (`player`, `player2`, `saber`), Controls (`controls`), and Setup
  (`setup`: video, sound, game options, mods, defaults).
- The in-game menus (`ingame*`, `siege_class`) and the connect and error
  screens (`connect`, `error`).
- Optionally the player's own retail menu artwork (logo, window frames) from
  game data, with the current drawing as fallback. Retail assets are never
  bundled.

## Configuration and content

The Linux configuration is `$XDG_CONFIG_HOME/jkr/config.cfg`, falling back to
`~/.config/jkr/config.cfg`. The source also defines Windows `%APPDATA%` and macOS
Application Support locations in [platform.rs](../crates/jkr-viewer/src/platform.rs).
Edit settings through the client, or edit the file while the client is stopped
so autosaving cannot overwrite your changes.

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

## Useful console commands

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

## Talk balloon and player sprites

While the console, a menu or the chat field is open, each command carries
`BUTTON_TALK`, as stock `CL_CmdButtons` sends it. Movement then sets `EF_TALK` and
discards the player's other input, so a typing player stands still. Over players
with `EF_TALK` the client floats the chat icon, and over players the server flags
with `EF_CONNECTION` the connection icon instead, as `CG_PlayerSprites` does. The
siege voice-command icon is not drawn. Both are frame billboards, upright as
`RT_SPRITE` draws them (see
[rendering.md](rendering.md#billboard-icons)). See
[pmove_talk.rs](../crates/jkr-game-jka/src/pmove_talk.rs) and
[player_sprites.rs](../crates/jkr-viewer/src/player_sprites.rs).

## Text size and spacing

The Settings screen's TEXT tab holds four archived cvars; their defaults draw
text exactly as before.

| Cvar | Default | Range | Effect |
| --- | --- | --- | --- |
| `ui_textScale` | 1 | 0.8 to 1.2 | Text size on menu screens and the in-game menu |
| `ui_letterSpacing` | 0 | -0.05 to 0.15 | Extra space after each letter in menus and the console, as a fraction of the text size |
| `con_scale` | 1 | above 0 (menu: 0.5 to 2) | Size of the whole console: text, margins and rows |
| `con_lineSpacing` | 1 | 0.65 to 2 | Console history and notify row pitch; 0.65 makes rows touch |

Menu text grows or shrinks about the centre of its line without moving the
layout, so the range is limited to what menu rows can hold. Chat, the
scoreboard and the HUD are not affected. The style is applied where retained
text commands become glyph quads; see [text/style.rs](../crates/jkr-viewer/src/text/style.rs)
and [console_options.rs](../crates/jkr-viewer/src/console_options.rs).
