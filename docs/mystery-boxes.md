# Mystery boxes

Boxes SJK's creator places on maps, which any SJK player opens for credits (Sol's idea,
27/10/2026). The hub keeps where each box stands, takes the price, rolls what is inside
and pays it; the client draws the boxes on the map file they were placed on, opens the one
the player faces with `+use`, and shows what came out to the opener and to the SJK players
around. Code: [mystery_box.rs](../crates/sjk-viewer/src/mystery_box.rs), the hub calls in
[sjk-identity's mystery_box.rs](../crates/sjk-identity/src/mystery_box.rs).

No game server knows of a box. It is drawn by SJK clients only, has no collision (players
walk through it), and players on other clients see someone pressing Use at nothing.

The box is SJK's own (bundled, `assets/mystery_box`, made by
[mystery_box_model.py](../scripts/mystery_box_model.py)): a long wooden chest bound in
iron, 60 x 26 x 24 units, a pale question mark glowing (pulsing) on each half of its two
long sides, and a lid hinged at the back that swings open while it rolls and closes once
what came out is taken.

Words stand in the world over every box, in the headline font (Fira Sans Condensed, as
centre prints), for every SJK player who sees it (hidden behind
walls, fading out past 1000 units, gone past 1400): "MYSTERY BOX" and its price (or "Free
roll: press Use" for a player holding one), "Rolling..." while it rolls, then what came
out ("JACKPOT!", "DOUBLE!", "CREDITS BACK" or "EMPTY", and the credits; the opener's also
says to press Use to take it).

## Opening one

Standing at a box (within 80 units across the floor and 96 up or down) and facing it
(within 60 degrees), the player is told once "Mystery box: press Use to open it (100
credits)". `+use` asks the hub to open it; the hub refuses without the price, within the
cooldown (one box every 3 seconds a person) or when the box is gone, and the refusal shows
in the centre of the screen. An opened box plays a crate sliding open and a little music
box tune (SJK's own, bundled: `assets/mystery_box`, made by
[mystery_box_assets.py](../scripts/mystery_box_assets.py)) while it rattles and things
rise out of it and roll over it: the game's weapons and the four prizes, changing fast,
then slower, like a wound-down spring. At 5.5 seconds, on the tune's last chord, the roll
stops on what came out and plays a chime (credits back), a fanfare (the jackpot) or a
locked door (empty):

| Outcome | Stops on |
| --- | --- |
| `empty` | a handful of bolts |
| `refund` | a blue crystal |
| `double` | a green crystal |
| `jackpot` | a red crystal |

The opener is told what came out in the centre of the screen. It floats over the box,
turning slowly, until they take it with `+use` at the box (a sparkle; a chat line gives
the credits left) or for 10 seconds, when it is taken for them. Meanwhile the box is busy:
another player's `+use` there says someone is opening it. The hub pays at the opening
itself, so walking away loses nothing. The other SJK players on the server see and hear
the same roll through the hub's toy event (`mystery_box`, `open_<box>_<outcome>`); they
see what came out float for the 10 seconds, as they are not told when the opener takes it.

Every sound plays at the box on a sound source of the box's own (`0x5200_0000` and its id),
never an entity's: the audio moves an entity's sounds to where it is, so box sounds once
followed the player on slot 0 (bug report #185).

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

Some of its sounds are retail (the crate sliding open, the chime, the fanfare, the locked
door, the box hunt's alarm); the tune and the take are SJK's own.

## Box hunt

Players opt in with the Box hunt toy (`toy_boxhunt`, Collection > Toys, the quick wheel;
look toy `box_hunt`); with it off a player is never the mark and is told nothing of the
hunt. Among the players of a server with it on (at least 2), the hub picks a mark at
random for 3 minutes. The mark is told in the centre of the screen ("You are the mark!"),
the other hunters by a chat line, and they see "MARK" in red over the mark's head
(hidden behind walls). Every hunter, the mark too, hears the game's alarm
(`sound/effects/mpalarm.wav`, Sol's pick, 28/10/2026; with "Mystery box sounds") as a new
mark is picked, and a small card at the top centre of the screen names the mark (in
the headline font, "BOX HUNT · THE MARK" over the name) or, red, tells the player "YOU
ARE THE MARK"; its red edge flashes for 2 seconds as it arrives, and it stays until the
mark is hunted or the hub has no mark or another one. The card and the alarm are
[hunt.rs](../crates/sjk-viewer/src/mystery_box/hunt.rs) and
[label.rs](../crates/sjk-viewer/src/mystery_box/label.rs); the card draws with the words
in the world, so it is hidden with them (menus, console, scoreboard, intermission). A hunter who kills the mark wins a free mystery box roll: the
next box they open costs nothing (the centre says "press Use for your free roll" at a
box). The hunt then pauses a minute before the next mark.

The hub never sees kills, so the mark's own client says who killed it
(`POST /v1/box-hunt/killed`, from the game's obituary): a hunter cannot claim a kill it
did not make, only the mark could fail to tell. A hunter keeps at most 3 free rolls and
wins at most 5 a day. The client reads the hunt (`POST /v1/box-hunt`) every 20 seconds
while the toy is on or a box stands on the map, and at once after the hub's toy events
(`box_hunt`: `mark_<slot>`, `hunted_<slot>_<won>`). All the numbers are the hub's,
PROVISIONAL.

## Limits

- The hub cannot tell where a player stands: the reach is the client's.
- No other bonuses or penalties yet: an outcome only pays credits. Effects (toys played on
  the opener, items) are Sol's plan for later and need a client release for each new kind.
- Two friends with the toy on could take turns being killed for free rolls; the 3-minute
  mark, the pause and the 5 a day cap how much that pays.
- Verified: unit tests (the pick of the faced box, which boxes stand on a map file and
  server, the roll slowing to its prize, the stop, the take and the take after 10
  seconds, the lid, the words over a box, the hunt's words and reads, the result lines;
  on the hub, the mark picked among hunters only, the kill counted only from the mark
  and for a hunter, the pause, the free roll spent; on the hub, the odds, placing and
  removing as the creator only, the price, the roll, the cooldown and another server's
  box). Not tried in a game; the hub side is deployed (27/10/2026).
