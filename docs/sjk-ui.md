# SJK UI

The SJK UI (`ui_menuStyle sjk`) is SJK's own menu style: SJK's menus redesigned
from the ground up, drawn over the live map in SJK's own type. It keeps the
feeling of Jedi Academy's menus (gold for what you choose, holo blue line-work,
the turning ring round the emblem) and drops their window frames and boxes. It
is being built screen by screen and becomes SJK's default once every screen has
its version; until then [classic+](classic-plus.md) stays the default and keeps
getting fixes.

Status (07/10/2026): the main page is done. Every other screen opens in its
classic+ version (`MenuStyle::classic_screens`), which covers the map as the
classic style does. To try it: Settings > Gameplay > Interface > Menu style >
SJK, or `ui_menuStyle sjk`; restart for the SJK UI's map behind the main page.

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
details 16 to 18, key hints 15 to 17.

`ui_gameFont` (classic game fonts) does not change the SJK UI's type; it keeps
applying to the classic screens and the in-game text.

### Motion

The ring turns once every four minutes and the sunburst behind the emblem
slowly the other way; the gold arc eases towards the chosen entry (about 0.3 s).
Nothing else moves on its own.

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

## Implementation

- `ui_menuStyle` has a third value, `sjk` (`menu::style::MenuStyle::Sjk`).
  `MenuStyle::classic_screens` is true for it, so every screen without an SJK UI
  version opens its classic one; the main page dispatches to `menu::sjk::home`.
  The map is drawn under the SJK UI's main page (`classic_hides_world` leaves it
  out).
- Fonts: `text::load_family` rasterizes a family's two faces into one atlas, as
  Inter's, taking glyphs a face lacks from its fallback family. The SJK UI's
  families load the first time the style is on, rasterized at 1.5x (glyphs 144
  pixels tall, sharp at 4K), into GPU layers kept by `game_font::GameFonts`.
  `MenuCanvas::set_family` tags each text run with its family
  (`menu_widgets::TextFamily`), and `append_text_families` routes the runs to
  their family's layer. Until the families are loaded the page draws in Inter.
- Snapshots: `menu_snapshot::sjk_home_snapshot` draws the page over the JoF HD
  wide levelshot of mp/duel6, each family from its own atlas.

## Plan

The next screens, in order; each gets snapshot tests in both fonts before it
replaces its classic version:

1. Settings: categories down a rail with their icons, left-aligned rows under
   sub-headings, the focused setting explained in a column on the right, search
   always at the top (the Settings mock-up).
2. The server browser, opened from Play's Join a server.
3. Character: the player screen, on the backdrop's player stage.
4. The in-game menu.
5. The screens Sol JK's page opens (changelog, credits, update, identity) and
   the dialogs.

Once all of them are done, `sjk` becomes the default `ui_menuStyle`. The authored
camera routes, gate and player stage of the menu backdrop are mp/ffa3's; mp/duel6
needs its own before the screens that use them (the browser's gate, the player
stage) move to the SJK UI.
