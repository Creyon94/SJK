# Cards and credits: what a holocron holds

**Status (12/10/2026): opening is built, cards are sealed.** Opening a holocron
gives one *sealed* card and some credits. A sealed card has its rarity, finish and
edition but no face yet: the faces come with the first set of cards, and every
sealed card is revealed into that set then. Credits are kept and shown; nothing
spends them yet. [Holocron drops](holocrons.md) give the holocrons. The wire is
the hub's `PROTOCOL.md` ("Holocrons", "Opening") in Sol-Vulpes/SJK-hub.

Collecting cards grants nothing: no power, setting or right, as with medals
([identity.md](identity.md#medals)). There is no trading and never any real money
in the system. Hats and capes stay free for everyone and are never sold or dropped
([client.md](client.md#hats-and-capes)).

## Decisions

| Question | Decision |
| --- | --- |
| What opening gives | One card and credits per holocron (Sol, 12/10/2026). |
| Cards before the art exists | The card is sealed: rarity, finish and edition are rolled at opening; the face is chosen when the first set is out (Sol, 12/10/2026). |
| Where a sealed card is revealed | Into the first set only, so Alpha cards stay launch-set cards (my pick, Sol's go-ahead). |
| What the holocron's tier does | A higher tier raises the odds of rarer cards, better finishes and more credits ([Odds](#odds)). |
| Finishes | Normal, Holo, Solar (the rarest: an animated sunlit shimmer). |
| Editions | A stamp apart from the finish: **Alpha Edition** while the alpha lasts. |
| Credits' sign | The galactic credit's, the slanted seven crossed by two bars (Sol's pick, from Wikipedia); drawn from shapes. |
| What credits will buy | Droppable cosmetics, as a fix for bad luck ([Credits](#credits)); not built. |
| Duplicates | Cards keep a count ("x3"). A droppable cosmetic dropped again turns into credits (planned with the shop). |
| Drop rates | Start stingy; buffing later is easy on the hub and nerfing is not. |
| Money, trading | None. Free, non-commercial, no trading. |

## Opening

All decided by the hub, in one transaction:

1. `POST /v1/holocrons/open` with `{"ids":[..]}` (1 to 20 of the player's own
   holocrons not opened yet, signed). For each the hub rolls the card's rarity and
   finish and the credits, stores the sealed copy (with the current edition, the
   holocron and its tier) and a line in the credits ledger, and marks the holocron
   opened. All or nothing: 404 `not_found` (not the player's), 409
   `holocron_opened` (opened already, from another PC of the person's), 400
   `bad_ids`.
2. It answers what each brought (`opened`: holocron, tier, card's id, rarity,
   finish, edition, credits) and the person's profile after.
3. The profile carries `credits` (the balance) and `cards` (the copies, one entry per
   card, rarity, finish and edition, with a count, the first and last time; `card` is
   null while sealed). A holocron's entry carries `opened`; `holocron_counts` counts
   only the unopened ones, so a client shows what can still be opened.

Opening counts toward no cap, and an opened holocron still counts toward the day's
drop caps. Removing an opened holocron (staff) leaves its card and credits.

## Odds

The hub's tables (`src/cards.rs` there); the holocron's tier picks the row.

Rarity of the card:

| Holocron | Common | Uncommon | Rare | Legendary |
| --- | --- | --- | --- | --- |
| Uncommon | 70 % | 24 % | 5.5 % | 0.5 % |
| Rare | 50 % | 33 % | 15 % | 2 % |
| Legendary | 30 % | 38 % | 26 % | 6 % |
| Mythical | 10 % | 30 % | 45 % | 15 % |

Finish of the copy:

| Holocron | Normal | Holo | Solar |
| --- | --- | --- | --- |
| Uncommon | 97 % | 3 % | 0 % |
| Rare | 90 % | 10 % | 0 % |
| Legendary | 70 % | 29 % | 1 % |
| Mythical | 40 % | 55 % | 5 % |

Credits, any whole number in the range, equally likely:

| Holocron | Credits | On average |
| --- | --- | --- |
| Uncommon | 10 to 20 | 15 |
| Rare | 30 to 50 | 40 |
| Legendary | 80 to 120 | 100 |
| Mythical | 250 to 350 | 300 |

About 35 credits a holocron on average, so at most about 280 a day under the
holocrons' daily cap of 8. The client shows the ranges
([cards.rs](../crates/sjk-viewer/src/cards.rs) `CREDITS`, the hub's copy, tested);
the hub decides. Rolls use the operating system's entropy, as drops do.

## Credits

Credits are a ledger at the hub (`credit_ledger`: each change with its reason,
`holocron` or `staff`, and what it refers to); the balance is its sum and never
goes below 0. Staff correct a balance with `POST /v1/staff/credits` (logged), the
operator with `give-credits`. The client has no staff control for it yet.

**Planned: the shop (not built, Sol's direction 12/10/2026).** Credits buy
droppable cosmetics that are not tied to an achievement or a medal, as a fix for
bad luck: a player who plays a lot and never gets the one thing they want can buy
it. The rules agreed:

- Only what can drop is for sale. Medal blades, achievement rewards, hats and capes
  never are.
- A price follows the item's odds, not a list: the credits an unlucky player has
  earned by the time 80 % of players would have dropped it (about 35 credits a
  holocron times `1.6 / p` holocrons for an item dropping at `p` per holocron). An
  item at 0.5 % costs about 11 000 credits, some 40 days of capped play. Buffing an
  item's odds lowers its price by itself.
- A droppable item dropped again turns into credits.
- Mythical items are for sale too; the formula keeps them the dearest.
- Cards stay luck only at first.

No player text promises the shop until it exists.

## In the client

| Piece | Where |
| --- | --- |
| Wire types (`credits`, `cards`, `opened`, the opening's answer), the request, the outcome | [wire.rs](../crates/sjk-identity/src/wire.rs), [hub.rs](../crates/sjk-identity/src/hub.rs), [service.rs](../crates/sjk-identity/src/service.rs) (`Service::open_holocrons`, `Snapshot::opening`) |
| Rarities, finishes, editions, the credits' ranges, a profile's stacks | [cards.rs](../crates/sjk-viewer/src/cards.rs) |
| A card drawn from shapes, the finishes' light | [cards/look.rs](../crates/sjk-viewer/src/cards/look.rs) |
| The credits' mark | [cards/mark.rs](../crates/sjk-viewer/src/cards/mark.rs) |
| Open, Open all and the ceremony on the Holocrons tab | [holocrons_panel_opening.rs](../crates/sjk-viewer/src/holocrons_panel_opening.rs), [holocrons_panel.rs](../crates/sjk-viewer/src/holocrons_panel.rs), [holocrons_panel_view.rs](../crates/sjk-viewer/src/holocrons_panel_view.rs) |
| The Collection's Cards tab | [collection_cards.rs](../crates/sjk-viewer/src/collection_cards.rs) |
| `debug_open` | [cards/rehearsal.rs](../crates/sjk-viewer/src/cards/rehearsal.rs) |

- **Rarities' colours:** Common silver `#B4C0D4`, then the holocron tiers' green, blue
  and purple for Uncommon, Rare and Legendary.
- **A card** is drawn, not a picture: a dark plate edged in its rarity's colour, the
  holocron's gem at its heart in a ring, the rarity's name above, "Sealed" under the
  gem and the edition stamped at the foot. Holo lays slanted bands of the rainbow
  drifting across it with a glint; Solar a warm wash, a corona of arcs turning round
  the gem, a band of sunlight passing over, embers at the foot and a breathing gold
  edge. The light is cut to the card: the UI renderer cuts strokes to the clip
  rectangle since this change ([ui_renderer.rs](../crates/sjk-viewer/src/ui_renderer.rs)
  `clip_polygon`), as it always cut rectangles.
- **The credits' mark** is the sign's outline in shapes: the crossbar and four short
  bars as rectangles, the slanting leg one stroke cut level at the bar's top and the
  foot. Eight commands, sharp at every size.
- **The Holocrons tab:** the player's credits stand at the panel's top right, the
  chosen tier says what opening one gives ("Opening it gives a sealed card and 30 to
  50 credits."), and under the holocron's caption **Open** (the oldest of the tier
  chosen, Enter) and **Open all (n)** (up to twenty of all tiers, the oldest first,
  A). Opened holocrons stay in the recent list, dimmed, marked "Opened".
- **The ceremony** is drawn by the tab over its own darkened page (text draws over
  every shape, so the page under it is not drawn): while the hub answers the tier's
  gem pulses ("Opening...", 15 seconds at most, then "The SJK hub did not answer");
  then for each holocron the burst in its tier's light (rings, turning rays, the gem
  flaring away), the card turning in, the credits counting up beside the mark, and
  its words: "Rare card, Holo", "Alpha Edition. From a Rare Holocron", what the finish
  is, that it is sealed, "1 of 3", Next or Done. Enter, Space, Right or a click skip
  the entrance, then go on; Escape ends it. Openings of more than three play a
  shorter entrance each. A Solar finish or a Legendary card plays the medals'
  fanfare, another card the unlocks' chime. A refusal says why ("That holocron was
  already opened."; a hub from before opening: "This SJK hub cannot open holocrons
  yet.") and the profile is read again soon.
- **The Cards tab** (the Collection's seventh): "N cards" with the line that they are
  sealed until the first set, the credits at the right of the headline, up to twelve
  kinds (six by two, the best first, each with "x3" and its finish; more are counted
  under them), and the kind chosen up close on the right with its title, edition,
  how many and when the first was found, what its finish is and that it is sealed.
  Left and Right choose. The tab's count in the row is the cards held.
- **The player card's** rarest holocron gem counts opened holocrons too, as the
  counts now hold only the unopened ones.
- `debug_open <tier|all> [x<count>] [normal|holo|solar]` plays the ceremony with
  made-up cards on the Holocrons tab; nothing is sent or kept.

## Planned, not built

1. The first set: the card catalogue at the hub (`cards/<set>.json`: id, set,
   rarity, the optional `community` flag), `GET /v1/cards`, and the reveal (a sealed
   copy picks a card of its rarity from the first set). The faces come from an asset
   pack, `sjk_cards`: `cards/<id>.card` (strict JSON like a blade-skin file: `name`
   up to 40 characters, `kind` `place`/`personality`/`game`/`community`, `lore` up
   to 400 characters, `art` a PNG or JPEG up to 1024 on a side). A card whose file is
   missing shows "Unknown card" with its id.
2. The Cards tab as a binder by set with progress ("Places 7 of 12"), missing cards
   as dark outlines.
3. The shop and the duplicate payout ([Credits](#credits)); a staff control for
   credits on the Staff page.
4. Set completion feeding achievements the hub counts.

Suggested first sets, about 20 to 30 cards: Places (planets, the Jedi Academy, the
game's maps), Personalities (figures of the lore), Jedi Academy (Force powers,
saber forms, hilts, moments of the game), Community (players and servers, with their
consent).

## The art and the rights

Sol makes the pictures, with AI tools, in one shared style. Cards of the lore draw
on Lucasfilm and Disney properties and Jedi Academy itself, and the credits' sign is
Lucasfilm's, so the system stays strictly non-commercial. AI-made art is generally
not protected by copyright, so card packs cannot carry the "all rights reserved"
claim the blade skins' art does; the credits say the pictures are AI-made. Community
cards use a player's name and picture, or a stylised emblem, only with their consent,
never an AI likeness of a real person's face, and staff can take one down at the
player's request.

## Privacy

A person's cards and credits are public like their medals: anyone can read a
profile. The hub stores which copies a person holds and when each was obtained, and
every change to their credits with its reason.

## Checked and not checked

Verified (12/10/2026, Windows 11): the hub's tests on branch `feat/holocron-opening`
(odds tables, rolls, the API's opening of one and twenty, refusals that change
nothing, merges moving cards and credits, linked keys, staff credits); the client's
`cargo test --locked --workspace` (the wire, the worker's opening and refusals, the
lists, stacks and credits' words, the mark's and a card's fixed commands, the
rehearsal's words, the tab's keys, the ceremony's flow, timeout and room on the
canvas in three windows, the Cards tab's states and room, the renderer's cut of a
stroke). World shots `world_shot::cards_shots` (off-screen renders on duel6, made-up
data) were looked at: the tab with Open and the credits, a Holo card's entrance
moment by moment, each finish, the wait, a refusal, the Cards tab at 1080p and 4:3
and with none. Not tried: against the real hub (it is not deployed), in a game, the
keys and clicks in a running client, the motion (the shots are stills).
