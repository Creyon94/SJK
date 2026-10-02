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

`ui_menuStyle` (Settings, GAME tab, "Menu style") picks the layout of the main
and in-game menus: `modern` (default) or `classic`, which is close to the retail
multiplayer menus in layout and flow without porting their `.menu` scripts. The
retail 640x480 layout is fitted to the window height and centred.

The classic main menu has the retail pages, entries and order:

- Main: Play, Profile, Controls and Setup in two columns, Exit below. Exit and
  Escape ask before quitting.
- Play: Solo Game, Join Server, Create Server, Play Demo and Rules. Solo Game
  and Create Server both open Create game, which hosts a local match with bots.
- Controls: Movement, Interaction, Weapons, Force Powers 1 and 2 and Other open
  the key-binding editor on that tab (both Force pages on its one Force tab);
  Mouse/Joystick opens Settings on the CONTROLS tab. The editor opened this way
  closes back to the classic page.
- Setup: Video and More Video open Settings on VIDEO, Sound on AUDIO and Game
  Options on GAME; HUD and Network follow as JKR additions.
- Every sub-page repeats the retail navigation row (Play, Profile, Controls,
  Setup) and has Back and Exit. Profile opens the Player screen.

Retail entries JKR has no screen for yet (Play Demo, Rules, Mods, Defaults) are
shown dimmed, and their description line says so.

The classic in-game menu (Escape during a match) is the retail top bar: About,
Join, Profile, Add Bot, Controls, Setup, Vote, Call Vote and Exit. Each opens a
pop-up under it or the matching screen. About shows the server info. Join picks
a team, or opens the class list in Siege. Vote is Yes/No. Call Vote opens the
call-vote lists. Exit offers Main Menu, Restart Match and Quit Program, each
with a Yes/No confirmation. Profile, Controls and Setup open the Player screen
and Settings. Siege swaps in Objectives and V Chat as retail does. Add Bot,
Objectives, V Chat and Restart Match are dimmed with a note, because the client
cannot add bots or restart a match it does not host. Left and Right move along
the bar; Escape closes a pop-up, then the menu. The JKR-only Server browser and
Shot controls entries are in the modern style only.

With the player's retail game data mounted, the classic menus draw its own
artwork: the backdrop, side glyph columns, ring, windows, logo, sub-page frames,
button glow, in-game bar and pop-up boxes from `gfx/menus`. The art is decoded
once on a worker thread the first time the classic style is used, and the UI
renderer uploads it into one texture per image, separate from the shared UI icon
atlas. Its bind group changes only between draw runs that need a different
texture, so layer order is kept. Additively blended retail images (glow, title
band, bar) are converted to alpha at decode time. Animated retail stages (ring
rotation, scrolling glyphs, logo glint, the logo video) are drawn still. A
missing image falls back to JKR's own shapes. Retail assets are never bundled.

The code is in [menu/classic.rs](../crates/jkr-viewer/src/menu/classic.rs): the
page tables are in [pages.rs](../crates/jkr-viewer/src/menu/classic/pages.rs),
types and geometry in [layout.rs](../crates/jkr-viewer/src/menu/classic/layout.rs)
and drawing in [view.rs](../crates/jkr-viewer/src/menu/classic/view.rs). The
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
  Solo Game (`quickgame`), Profile (`player`, `player2`, `saber`), the
  controls and setup option panels, and the in-game `ingame_player`,
  `ingame_controls` and `ingame_setup`.
- The screens with no JKR equivalent yet: Play Demo (`demo`), Rules
  (`rules*`), Mods, Defaults, Add Bot (`ingame_addbot`), Siege objectives and
  voice chat, and the connect and error screens (`connect`, `error`).
- The retail fonts (`ui_gameFont`, a separate change) and the animated art
  stages.

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

## Chat and console text

Server text is split as in stock `codemp` cgame. `print` replies go to the
console and its notify lines (`con_notifytime`, `con_notifylines`), never the chat
box. Chat (`chat`, `tchat` and their location forms) goes to the chat box and is
also kept in the console scrollback, but not in the notify lines, like the stock
`*` print prefix. Centre prints stay on screen only. See
[server_commands.rs](../crates/jkr-viewer/src/server_commands.rs).

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

The console line and the chat field show a dead key (the `^` of French AZERTY or
German QWERTZ) at the caret as soon as it is pressed, and replace it with the
composed text when the next key arrives, so typing `^1` gives exactly `^1` and
the colour code previews as on a layout without dead keys. Enter, Backspace and
Tab keep a shown dead key as typed. A dead `^` that the system composes into a
superscript digit (`¹` through xkb on Linux) becomes `^` and the digit. See
[dead_key.rs](../crates/jkr-viewer/src/input/dead_key.rs).

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

The Settings screen's TEXT tab holds four archived cvars. Menu text draws as
before at the defaults; the console's rows are closer together than before.

| Cvar | Default | Range | Effect |
| --- | --- | --- | --- |
| `ui_textScale` | 1 | 0.8 to 1.2 | Text size on menu screens and the in-game menu |
| `ui_letterSpacing` | 0 | -0.05 to 0.15 | Extra space after each letter in menus and the console, as a fraction of the text size |
| `con_scale` | 1 | above 0 (menu: 0.5 to 2) | Size of the whole console: text, margins and rows |
| `con_lineSpacing` | 1.15 | 1 to 2 | Console history and notify row pitch as a multiple of the text size; 1 makes rows touch |

Menu text grows or shrinks about the centre of its line without moving the
layout, so the range is limited to what menu rows can hold; that style is
applied where retained text commands become glyph quads, see
[text/style.rs](../crates/jkr-viewer/src/text/style.rs). The console sizes its
own text with `con_scale` and puts the letter spacing into its layout
([console_view.rs](../crates/jkr-viewer/src/console_view.rs)), so anything that
measures console text, such as a caret, sees the spacing it is drawn with.
Chat, the scoreboard and the HUD are not affected.

Console defaults, compared with stock at 1080p: stock draws 8 x 16 px cells, so
its rows are 16 px apart. JKR's console text is 14 px Inter (11.6 px em, 8.4 px
capitals), and its old 22 px pitch was 1.9 em, loose for a log. A pitch of 1.15
times the text size gives 16.1 px at 1080p, stock's row pitch, and about 1.4 em
of leading. `con_maxLines` defaults to 32 so the default-height console fills
with rows (26 fit at 1080p) instead of stopping at the old 18. Letter spacing
stays 0: Inter's average advance relative to its x-height (0.89) is already
close to stock's cells (0.8), Inter's own size-specific tracking at this size
is +0.002 em, and tighter text would run digits and `il1` together. See
[console_options.rs](../crates/jkr-viewer/src/console_options.rs).
