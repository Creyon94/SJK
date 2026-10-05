# Classic+ menus

SJK's classic menu style (`ui_menuStyle classic`, see
[client.md](client.md#menu-style)) follows the retail Jedi Academy multiplayer
menus: their pages, entries, artwork, colours and flow. Some SJK pages go
further. They keep everything a retail player recognises, and inside that frame
they add what a modern menu would offer: the numbers retail hid, a panel
explaining what is focused, previews, pictures, direct clicks on what is shown.
This is **classic+**: classic style, modernised. The classic Force page is the
reference example, with the cosmetics window and the creation pages' live model
and part icons.

This page holds the rules and the recipe, for new classic pages and for older
ones being reworked. It is SJK-only, like the classic menus' additions.

## What stays retail

A classic+ page must still read as the retail menu it stands for.

- **Canvas and places.** Pages are laid out on retail's 640x480 canvas, fitted
  to the window and centred ([`Placement`](../crates/sjk-viewer/src/menu/classic/layout.rs)).
  An item retail had keeps retail's rectangle (from the `ui/jamp/*.menu` item
  `rect`); a window keeps retail's window rectangle, and its items are written
  relative to it as retail wrote them. Pages work on both frames: the main
  menu's full page and the in-game window over a match.
- **Artwork.** The retail images come from the player's own game data
  ([`ArtPiece`](../crates/sjk-viewer/src/menu/art.rs)): backdrop and side
  columns, window boxes (`menu_box_ingame`), the title band (`menu_blendbox`),
  the glow behind a focused button (`menu_buttonback`), the highlight behind a
  hovered list row (`menu_blendbox2`), the red band (`menu_blendboxr`), slider
  and thumb, Force stars. Retail assets are never bundled, so every piece has a
  drawn fallback (`art.has(piece)` first).
- **Colours.** Retail's item colours, as named constants with their retail
  source in a comment:

  | Use | Colour | Retail source |
  | --- | --- | --- |
  | Titles, headings, captions | `.549 .854 1` | `forecolor` of titles (`LABEL`) |
  | Values, list text | `.615 .615 .956` | field `forecolor` (`VALUE`) |
  | Option labels | `.65 .65 1` | `setup.menu` items (`OPTION`) |
  | Buttons, key facts | `1 .682 0` | button text (`GOLD`) |
  | Focus | white, pulsing | `focusColor 1 1 1 1` (`focus_pulse`) |
  | Description line | `1 .682 0 .8` | `descColor` (`HINT`) |
  | Frames and rules | `.298 .305 .690` | `bordercolor` (`FRAME`) |
  | List boxes | `.66 .66 1 .25`, border `.66 .66 1` | creation lists (`LIST_BACK`, `LIST_BORDER`) |
  | Unavailable | `.5 .5 .5` | `disableColor` (`DISABLED`) |
  | Option panel | `0 0 .6 .5`, border `0 0 .6 1` | `setup_background` |

- **Text.** The retail fonts follow `ui_gameFont`. Labels, buttons and titles are
  in retail's capitals ([`Caps`](../crates/sjk-viewer/src/menu/classic/view.rs));
  descriptions, names and typed text keep their case.
- **Flow.** Retail's entries in retail's order, which keyboard focus follows;
  Apply, Back and Escape lead where retail's did; the description line at the
  bottom says what the hovered or focused item does. Classic and modern screens
  edit the same values, so switching style never loses anything.

## What the plus adds

Each rule names a page that already follows it.

1. **Show what retail hid.** The Force stars carry each level's cost; the points
   line reads "12 of 31" and "not applied" while the draft is pending; the
   player's own templates are tagged "yours" and the worn hat "worn".
2. **A detail panel for what is focused.** Under the Force page's side column a
   panel describes the hovered or focused power: holocron, level, the next
   level's cost or why it cannot be bought, every level's cost and what it has
   taken. With nothing focused it sums up the profile. Hover wins over keyboard
   focus, as the description line does.
3. **Hover previews, a click commits.** Hovering a star shows its price on the
   points meter (white, or red when the points left cannot pay it); nothing
   changes until the click. Lists highlight the row under the pointer.
4. **Direct manipulation.** A click picks the cell it lands on (a head, a part,
   a tint, a hilt, a template); a click on a star sets that level and the right
   button lowers it; the wheel scrolls the list under the pointer; numbers can
   be typed (the option panels' sliders).
5. **Pictures from the game data.** Holocrons and side emblems, part icons, the
   live model (character creation, cosmetics, lightsaber creation), levelshots.
   When a picture is missing the page shows text and stays usable.
6. **Explain empty and unavailable states.** "No templates for this side.";
   an empty cosmetics list says where cosmetics come from; a power that cannot
   be bought is grey and the panel says why.
7. **Group inside the retail window.** Column headings over a thin `FRAME` rule,
   side cards, a second column: structure is added within the window retail had,
   not by growing it past retail's frame.
8. **Retail items keep their places.** Additions go where retail drew decoration
   or nothing (the Force page's detail panel, the creation pages' preview where
   retail's model stood). A retail control never loses part of its target.
9. **Cheap and robust.** Nothing needs the retail art; frames do not allocate
   (`label_fmt` with `format_args!`, fixed storage); pictures decode on worker
   threads; the page works with the keyboard alone.

## Layout conventions

Values are canvas units of the 640x480 canvas.

| Element | Convention |
| --- | --- |
| Title band | `menu_blendbox` art, `LABEL` text 12-15 semibold, centred |
| Heading | `LABEL` 12 semibold over a 1-unit `FRAME` rule; a side band for a side's column |
| List box | `LIST_BACK` fill, `LIST_BORDER` border (`FOCUS` while active), 16-unit rows |
| List row | `VALUE` 11-12; chosen row filled; hovered or focused row on `menu_blendbox2`; a tag (`GOLD` 9-10 semibold) at the right end |
| Scrollbar | 3 units wide, 5 in from the box's right edge, only when the list is longer than the box |
| Button | `GOLD` 14-17 semibold, centred; `menu_buttonback` glow and `FOCUS` when focused; `DISABLED` when it cannot act |
| Text field | "Name: value"; an empty one shows a prompt at 70% alpha; underlined while typing |
| Detail panel | `ink(0.45)` fill, `FRAME` border; a 44-unit picture at the top left; title `GOLD` 14 semibold; lines `VALUE` 11-12, 13-14 units apart |
| Description line | `HINT` 12-13, centred on retail's description position |

## Code recipe

The profile pages show the pattern
([player_menu/classic](../crates/sjk-viewer/src/player_menu/classic.rs)):

- **Entries** ([layout.rs](../crates/sjk-viewer/src/player_menu/classic/layout.rs)):
  an `Item` enum with retail `label()` and `hint()` (`descText`); `items(page,
  frame)` gives each page's entries in focus order; `rect(item, page, frame)` its
  retail rectangle, window-relative for windows; tests check that every entry
  lies inside its window and that none overlap.
- **Drawing** ([view.rs](../crates/sjk-viewer/src/player_menu/classic/view.rs),
  one module per larger page such as
  [force_page.rs](../crates/sjk-viewer/src/player_menu/classic/force_page.rs)):
  `entries()` registers each entry's pointer region before drawing its cells,
  because the canvas gives the pointer to the region registered last; the entry
  under the pointer (or one of its cells, `sub_hovered`) supplies the
  description and the detail panel.
- **Pointer** ([pointer.rs](../crates/sjk-viewer/src/player_menu/classic/pointer.rs)):
  tokens decode into a `Target`; hovering a cell focuses the entry that owns it;
  the wheel scrolls the list under it; the secondary button is the "less" action.
  Token ranges are constants with a round-trip test, and the cells of a long list
  are numbered by their place on screen, not by their index in the list.
- **Keyboard**: Up, Down and Tab move through the focus order; Left and Right
  step the focused value; Enter activates; Escape goes back one level. A text
  field takes every key while typing and ends on Enter (keep) or Escape.
- **Pictures**: retail images are `ArtPiece`s (add a piece in
  [menu/art.rs](../crates/sjk-viewer/src/menu/art.rs)); icons from the game data
  use ranges of the UI icon atlas (`FORCE_ICON_FIRST`, `PART_ICON_FIRST`, ...)
  filled off-thread; large pictures get a texture of their own
  (`LEVELSHOT_TEXTURE`, `PREVIEW_TEXTURE`).
- **Module documentation** names the retail menu and its files, and says what
  SJK adds ("SJK adds the side cards, ...").

The option panels (Setup, Controls and the renderer page) are drawn by
[menu/classic/panel.rs](../crates/sjk-viewer/src/menu/classic/panel.rs), with
their rows by the settings screen and the key-binding editor
([settings/classic_view.rs](../crates/sjk-viewer/src/settings/classic_view.rs),
[keybind_editor/classic_view.rs](../crates/sjk-viewer/src/keybind_editor/classic_view.rs)).
`PanelPlace::detail` draws their detail box from a `Detail` (title, value, two
lines, facts, console name); `label_marked` and `changed_mark` draw the rows'
marks. What each setting does is in
[settings/help.rs](../crates/sjk-viewer/src/settings/help.rs), which a test keeps
complete: a new setting needs its line there.

## Seeing a page without the game

[menu_snapshot.rs](../crates/sjk-viewer/src/menu_snapshot.rs) draws menu screens
into PNGs on the CPU, with the installation's retail art and the menu font, so a
layout can be checked without opening a window:

```sh
JKA_GAME_DATA="/path/to/GameData" cargo test --release -p sjk-viewer \
    menu_snapshot -- --ignored --nocapture
```

The pictures go to `target/menu-snapshots/`. They approximate the UI renderer
(no rounded corners, art without its motion, Inter text only, no atlas icons),
which is enough to catch overlaps, cut-off labels and empty space. Add a screen
to the test when building a page.

## Checklist

- The retail menu this page stands for is named, and its items keep retail's
  places, words, colours and order.
- Everything added answers a player's question (what does it cost, what does it
  do, what will it look like, why is it grey) or saves a step (a click, a search).
- The focused or hovered item explains itself, in the description line or a
  detail panel.
- It works without the retail art, with the keyboard alone, on both frames, and
  at 4:3 and wide windows.
- Layout and token tests, and a look at it in the menu snapshots; no game window
  is opened to verify it.
- [client.md](client.md#menu-style) describes what the page shows,
  [status.md](status.md) records the verification, the `debug_panel` list says
  how to test it, and [CREDITS.md](../CREDITS.md) names notable work.

## Pages

| Page | Retail menu | What the plus adds |
| --- | --- | --- |
| Force | `ingame_playerforce` | Side cards, cost-numbered stars, points meter with hover preview, holocrons, two columns, detail panel; on the main menu too |
| Cosmetics | JoF EJK's `ingame_cosmetics` | Hats and capes side by side, live model, worn tags, explained empty lists |
| Character creation | `player2`, `ingame_player2` | Live model where retail's stood, part icons, tinted swatches |
| Lightsaber creation | `saber`, `ingame_saber` | The model holding the lit sabers |
| Setup, Controls | `setup.menu`, `controls.menu`, `ingame_setup`, `ingame_controls` | A detail box for the focused setting or binding (what it does, default, range, when it applies, console name, keys shared with other actions); changed and applies-later marks; Backspace or the right button for the default; key hints |
| Renderer | none (SJK's renderer settings) | The renderer settings as a `setup.menu`-style page with IMAGE, LIGHTING and SHADOWS groups, on both frames, with the same panels |
