# SJK UI

The SJK UI (`ui_menuStyle sjk`) is SJK's own menu style: SJK's menus redesigned
from the ground up, drawn over the live map in SJK's own type. It keeps the
feeling of Jedi Academy's menus (gold for what you choose, holo blue line-work,
the turning ring round the emblem) and drops their window frames and boxes. It
is being built screen by screen and becomes SJK's default once every screen has
its version; until then [classic+](classic-plus.md) stays the default and keeps
getting fixes.

Status (07/10/2026): the main page and Settings are done. Every other screen
(the key bindings among them) opens in its classic+ version
(`MenuStyle::classic_screens`), which covers the map as the classic style does.
To try it: Settings > Gameplay > Interface > Menu style > SJK, or
`ui_menuStyle sjk`; restart for the SJK UI's map behind the main page.

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

`ui_gameFont` (classic game fonts) does not change the SJK UI's type; it keeps
applying to the classic screens and the in-game text.

### Motion

The ring turns once every four minutes and the sunburst behind the emblem
slowly the other way; the gold arc eases towards the chosen entry (about 0.3 s);
the camera behind glides through its tour (The map behind). Nothing else moves
on its own.

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

On a map with a tour, a screen whose shot has a route (none yet on duel6) is
reached by fading out, cutting to the shot and fading in, instead of flying
there, since a tour's shots are all over the map; every screen shows at once.
The fade is a layer over the world only (`ClientMenu::world_fade`), under the
menus and the console's pages.

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
  - Key bindings opens the classic+ key bindings (until they get an SJK UI
    screen); leaving them comes back to the category they were opened from.
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

## Implementation

- `ui_menuStyle` has a third value, `sjk` (`menu::style::MenuStyle::Sjk`).
  `MenuStyle::classic_screens` is true for it, so every screen without an SJK UI
  version opens its classic one; the main page dispatches to `menu::sjk::home`.
  `ClientMenu::sjk_screen` says when one of the SJK UI's own screens is on show
  (the main page, or Settings without a picker open): the map is drawn under it
  (`classic_hides_world` leaves it out) and its text goes to the UI's families
  (`append_sjk_screen`).
- The controls are drawn by `menu::sjk::kit` (the band, switch, slider,
  segments, field, list, reset arrow, changed dot, sub-heading, search pill and
  lit rail), in frame pixels (`menu::sjk::Frame`).
- Settings is the classic+ panel's state with another view: the menu opens a
  category's rows as the classic+ panels do, keeps `classic_panel` empty and
  marks the SJK UI's screen open (`sjk::settings::SettingsPage`); the rows'
  results (`SettingsResult::Classic` for a category, `ClassicCycle` for Tab) are
  read as the rail's. Pointer tokens are the classic+ panel's (rows, values,
  segments, reset, list, search, scroll), the rail's categories its chrome
  tokens.
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
  client built without a window on duel6, the SJK UI over the touring camera.

## Plan

The next screens, in order; each gets snapshot tests before it replaces its
classic version:

1. The key bindings, as a category of Settings (the search then finds keys too).
2. The server browser, opened from Play's Join a server.
3. Character: the player screen, on the backdrop's player stage.
4. The in-game menu (its Setup and Controls then open Settings over the match).
5. The screens Sol JK's page opens (changelog, credits, update, identity) and
   the dialogs.

Once all of them are done, `sjk` becomes the default `ui_menuStyle`. The authored
camera routes, gate and player stage of the menu backdrop are mp/ffa3's; mp/duel6
needs its own before the screens that use them (the browser's gate, the player
stage) move to the SJK UI.
