# Mystery boxes

Boxes SJK's creator places on maps, which any SJK player opens for credits (Sol's idea,
27/10/2026). The hub keeps where each box stands, takes the price, rolls what is inside
and pays it; the client draws the boxes on the map file they were placed on, opens the one
the player faces with `+use`, and shows what came out to the opener and to the SJK players
around. Code: [mystery_box.rs](../crates/sjk-viewer/src/mystery_box.rs), the hub calls in
[sjk-identity's mystery_box.rs](../crates/sjk-identity/src/mystery_box.rs).

No game server knows of a box. It is drawn by SJK clients only, has no collision (players
walk through it), and players on other clients see someone pressing Use at nothing.

## Opening one

Standing at a box (within 80 units across the floor and 96 up or down) and facing it
(within 60 degrees), the player is told once "Mystery box: press Use to open it (100
credits)". `+use` asks the hub to open it; the hub refuses without the price, within the
cooldown (one box every 3 seconds a person) or when the box is gone, and the refusal shows
in the centre of the screen. An opened box hops and turns for 1.6 seconds and plays a crate
sliding open, then a chime (credits back), a fanfare (the jackpot) or a locked door (empty);
the result shows in the centre and a chat line gives the credits left. The other SJK players
on the server see and hear the same box open through the hub's toy event
(`mystery_box`, `open_<box>_<outcome>`).

Settings > Sound > Toys, "Mystery box sounds" (`s_toyMysteryBoxSound`), silences them.

The price and the odds are the hub's (`GET /v1/mystery-boxes`), PROVISIONAL, Sol's to tune:

| Outcome | Chance | Pays |
| --- | --- | --- |
| `empty` | 50 % | nothing |
| `refund` | 30 % | the price back |
| `double` | 15 % | twice the price |
| `jackpot` | 5 % | five times the price |

On average a box pays back 85 of its 100 credits. A newer hub's outcome the client does
not know is shown by its credits alone.

## Placing one (SJK's creator)

| Command | Does |
| --- | --- |
| `mysterybox_place [server]` | Place a box where you aim, facing you: on every server running this map file, or with `server` on this one only |
| `mysterybox_remove [id]` | Remove the box nearest you (within 256 units), or the one with that id |
| `mysterybox_list` | The boxes on this map, their ids and distance, and the price (anyone) |

The map editor's Place page (`mapedit_panel`) shows the creator Mystery box and Remove
box beside Place model; Mystery box follows the editor's scope (All servers or This
server). Placing needs the map's BSP checksum (as map edits do, `sv_mapChecksum`): the box
stands in that exact map file only. The hub takes the creator's request only; every
placing and removal goes to the staff log. The client reads the hub's list once registered,
every 10 minutes, and at once after a placing, a removal or a box that is gone.

The box is a retail crate (`models/map_objects/imperial/crate_banded.md3`) until it gets a
model of its own; its sounds are retail too.

## Limits

- The hub cannot tell where a player stands: the reach is the client's.
- No other bonuses or penalties yet: an outcome only pays credits. Effects (toys played on
  the opener, items) are Sol's plan for later and need a client release for each new kind.
- Verified: unit tests (the pick of the faced box, which boxes stand on a map file and
  server, the hop and its sounds, the result lines; on the hub, the odds, placing and
  removing as the creator only, the price, the roll, the cooldown and another server's
  box). Not tried in a game, nor against the live hub (the hub's `mystery-box` branch is
  not deployed).
