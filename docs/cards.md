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
| Editions | A stamp apart from the finish: **Alpha Edition** while the alpha lasts. Every other collectible carries one too ([Editions](unlockables.md#editions)). |
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

**Staff only for now.** Since 12/10/2026 (Sol's request, while opening is worked on)
the hub lets only staff open holocrons unless its operator opens it to everyone
(`--opening everyone`, or `SJK_HUB_OPENING=everyone`, on the hub; no client release is
needed). Anyone else gets 403 `opening_staff_only` and keeps their holocrons. The
progress answer (`GET /v1/holocrons`) says `can_open`; the Holocrons tab hides Open and
Open all, and Enter and A do nothing, when it is false (or not read yet), and says
"Opening comes soon: keep your holocrons, they will wait." in their place.

## Odds

The hub's tables (`src/cards.rs` there); the holocron's tier picks the row. Players see
them, with the chances per holocron found, on the Holocrons tab's loot table
([holocrons.md](holocrons.md#loot-table)); the client's copy (`cards::ODDS`,
`cards::CREDITS`) is pinned to these by a test.

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

## Faces and the first card

Since 12/10/2026 (Sol's first card, "Toxiee and Sol on JoF Memorial", Sol lore 001,
Legendary, its art by "Sol (AI-assisted)": the names in the title follow the picture,
Toxiee on the left) a sealed copy can become a real card. A card's `artist` line says
how its art was made; the footer reads "Art: <artist>".

- **Where cards are made:** Sol keeps each card's picture and words in `card_import/`
  at the repository's root, which git ignores (`card_import/README.txt` says the format):
  `cards/card1.png` (1020 x 850, the 6:5 art window, framed as it will show) and
  `cards/card1.txt` (`number`, `collection`, `kind`, `rarity`, `title`, `art`,
  `description`, optional `artist`), the full paintings in `sources/`, a white canvas
  and a frame overlay to frame new ones in `templates/`.
- **How they reach players:** the hub's `scripts/import_cards.py <card_import>` writes
  them into the hub's `sjk_cards` pack sources (`cards/<id>.card` JSON and
  `cards/<id>.jpg`, id `<set>_<NNN>` such as `sol_lore_001`), and its
  `scripts/build_packs.py` builds `sjk_cards.pk3` and the hub's catalogue
  `cards.json` (id, set, number, rarity). The hub serves the pack like the blade skins'
  (`GET /v1/assets`), so every SJK client downloads it into `assets/` beside
  `identity.key` and mounts it; adding a card is a new pack and a hub restart, no client
  release.
- **The hub picks:** opening a holocron rolls the copy's rarity and finish as before,
  then picks a card of that rarity from the catalogue (evenly); a rarity with no card
  yet stays sealed. `POST /v1/cards/reveal` gives the person's sealed copies of a
  rarity that has cards a card each (oldest first). Both follow who may open
  holocrons (staff only for now).
- **Each card has one rarity** (Sol lore 001 is Legendary): the first set needs cards
  of every rarity before every copy can be revealed.

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
| The faces from the packs, their pictures decoded on a worker | [cards/faces.rs](../crates/sjk-viewer/src/cards/faces.rs), mounted by [sjk_packs.rs](../crates/sjk-viewer/src/sjk_packs.rs) |
| The pictures' textures | [ui_renderer/card_art.rs](../crates/sjk-viewer/src/ui_renderer/card_art.rs) |
| Reveal (`Service::reveal_cards`, `Snapshot::reveal`) | [service.rs](../crates/sjk-identity/src/service.rs), [player_identity.rs](../crates/sjk-viewer/src/player_identity.rs) |

- **Rarities' colours:** Common silver `#B4C0D4`, then the holocron tiers' green, blue
  and purple for Uncommon, Rare and Legendary.
- **A card** is drawn, not a picture: a dark plate edged in its rarity's colour, the
  holocron's gem at its heart in a ring, the rarity's name above, "Sealed" under the
  gem and the edition stamped at the foot. Holo lays slanted bands of the rainbow
  drifting across it with a glint; Solar a warm wash, a corona of arcs turning round
  the gem, a band of sunlight passing over, embers at the foot and a breathing gold
  edge. The light is cut to the card, rounded corners included: the finish draws
  under a rounded clip (`DrawCommand::PushRoundedClip`), and the UI renderer cuts
  strokes to it, each corner as four chords of its arc
  ([ui_renderer.rs](../crates/sjk-viewer/src/ui_renderer.rs) `clip_polygon`); the
  finish's washes take the card's corner radius themselves. The light stays inside the
  rarity's edge, its slanting strokes reach far past the card so their square ends never
  show, and Holo's sweep crosses the card in about four seconds, eased, then waits off
  it for about three before the next pass (Solar's sun the same, slower).
- **Tilt (13/10/2026, Sol's request):** while the pointer rests on a card of the Cards
  tab (the one up close or one of the grid), the card turns towards it in perspective,
  up to about 9 degrees at its edges, the side under the pointer receding as if pressed;
  its light follows the pointer across it (Holo's bands, Solar's sun) and a soft glare
  sits under the pointer. It eases in and out in about a tenth of a second. The card is
  drawn as usual under `DrawCommand::PushTransform` (a projective transform the UI
  renderer applies to every shape vertex and text glyph drawn inside it, after clipping;
  `look::tilt_matrix`).
- **A revealed card** shows its face as a trading card: its name and its rarity's
  symbol ([sjk-ui.md](sjk-ui.md#symbols)) on top, its picture in a gold-edged 6:5 window (dark until the
  picture is decoded and uploaded), a strip with its kind and set ("Community ·
  Sol lore") and its edition and finish, its lore in a box (cut into lines at spaces,
  seven at most), and at the foot its number in its set ("001 / 001", counting the
  set's cards the packs hold), its artist and its rarity. The plate, the rarity's edge
  and the finishes' light are the sealed card's. A card narrower than 200 frame pixels
  shows the picture without words, and below 300 only the name. A copy whose face is
  not in the packs reads "Unknown card <id>" and keeps its sealed back.
- **The pictures** are decoded on a worker thread when the packs are mounted, cut to
  6:5 about their middle if they are not, squeezed into a 1024 square with its mip
  chain and drawn back into the 6:5 window (which undoes the squeeze); the renderer
  uploads two a frame. 512 faces at most.
- **The credits' mark** is the sign's outline in shapes: the crossbar and four short
  bars as rectangles, the slanting leg one stroke cut level at the bar's top and the
  foot. Eight commands, sharp at every size.
- **The Holocrons tab:** the player's credits stand at the panel's top right, the
  chosen tier says what opening one gives ("Opening it gives a sealed card and 30 to
  50 credits."), and under the holocron's caption **Open** (the oldest of the tier
  chosen, Enter) and **Open all (n)** (up to twenty of all tiers, the oldest first,
  A). Opened holocrons stay in the recent list, dimmed, marked "Opened".
- **The ceremony** is drawn by the tab over its own page, which is not drawn under it
  (text draws over every shape). Since 17/10/2026 it opens the holocron itself, as a
  loot crate opens
  ([holocrons_panel_unseal.rs](../crates/sjk-viewer/src/holocrons_panel_unseal.rs)):
  - **While the hub answers** (15 seconds at most, then "The SJK hub did not answer")
    the tab's 3D holocron ([holocrons.md](holocrons.md#the-holocrons-tab)), now the one
    asked for in its own look, glides in 0.6 seconds from its place on the page to the
    middle of the window, humming round and breathing its light, with "Opening..." under
    it. The world shows round it through a vignette: four straight fades from the
    window's edges to a clear box about it (the UI renderer's arcs are strokes, not
    fills, and take no clip, so rings could not make it).
  - **The unseal** (1.7 seconds): it spins up (to about 42 radians a second more),
    swells, shakes a few pixels at the end and its light grows to three and a half
    times the tier's; rays of the tier's colour break out of its seams, more, longer and
    faster as it charges, its name above it; then it pops and is gone in a flash of the
    tier's colour with a white heart, which fades over the strip in 0.45 seconds. It
    builds on the Force jump's charge (`jumpbuild`) and cracks open on the game's own
    holocron pick-up sound (`sound/player/holocron.wav`). The motion
    (`holocrons::stage::Motion`: where, turn, swell, light, shown) is worked out from
    the ceremony's clock alone and read by the world each frame; gone, the holocron
    grows back from nothing in its place when the ceremony ends.
  - **The crate spin** (6 seconds; since 14/10/2026): a strip of what its tier can drop,
    cards and droppable shaders, runs past a gold marker with a soft tick a tile
    (`menuroam`) and slows over 88 % of it to a stop on what the hub rolled, the last
    tile taking about two seconds; the strip is made up round the real prize and the
    stop lands anywhere up to 45 % of a tile from its middle, a close call near its
    edge but always inside it (the strip never shows a near miss that was not). Each
    tile stands over a band of its colour, as a loot crate's, and at speed its colour
    streaks across it and trails it. At rest (`sub_select`) the prize lifts and lights
    up in its colour, breathing, the others dim to a third, and a grand prize (a
    Legendary or Solar card, a Legendary or Mythical shader) turns slow rays behind it.
  - **The entrance:** the burst in its tier's light (rings, turning rays, the gem
    flaring away), the card turning in, the credits counting up beside the mark, and
    its words: "Rare card, Holo", "Alpha Edition. From a Rare Holocron", what the finish
    is, that it is sealed, "1 of 3", Next or Done. A shader instead of a card shows its
    blade framed in its tier's colour, its name, "Common saber shader. From a Rare
    Holocron", what it looks like and how many the player holds. A Solar finish or a
    Legendary card plays the medals' fanfare, another card the unlocks' chime.
  - **Opening more than three**
    ([holocrons_panel_batch.rs](../crates/sjk-viewer/src/holocrons_panel_batch.rs)):
    the first holocron cracks open ("12 holocrons"), then their strips run five at a
    time, stacked, each named by its holocron's tier at its left and marked in its
    colour, each stopping 0.35 seconds after the one above (3.6 seconds for the top);
    the next five follow on their own ("Holocrons 6 to 10 of 12"). The stacked tiles are
    light: a plate washed from below in the prize's colour, a card's rarity symbol
    (ringed in gold for Solar, a rainbow under it for Holo) or a shader's blade. A grand
    prize plays the fanfare as its strip stops. Then everything found is laid out, the
    finest first, ten to a row, each coming in on its beat, with its name and its
    credits or how many the player holds, the credits they brought together, and Done.
  - **Grand finds** are told to everyone: a hub that sends the feed's `finds` (hub branch
    `feat/grand-finds`, not deployed yet) relays a Legendary or Solar card, or a
    Legendary or Mythical shader, to every SJK player as a chat line, "Sol opened a
    Mythical Holocron: Phoenix blade!" ([holocrons.md](holocrons.md#chat-lines)).
  - Enter, Space, Right or a click skip to the burst (to the strips' stops, then the
    next five, then the finds, for many), finish the entrance, then go on; Escape ends
    it (what was opened is the hub's already). A refusal says why ("That holocron was
    already opened."; a hub from before opening: "This SJK hub cannot open holocrons
    yet.") and the profile is read again soon.
- **The Cards tab** (the Collection's seventh): "N cards" and a line about sealed cards
  while there are some, the credits at the right of the headline, up to twelve kinds
  (six by two, the best first, each with "x3" and its finish; more are counted under
  them), and the kind chosen up close on the right, 470 frame pixels wide so a face's
  lore reads at 1080p (290 before faces), with its name, rarity, finish, edition, how
  many and when the first was found, and its set and number (a sealed one: what its
  finish is and that it is sealed). **Reveal N sealed cards** (or R) stands under the
  grid when the hub lets the player open holocrons and the packs have faces of a sealed
  copy's rarity; it says "Revealing..." until the hub answers, then a line says how
  many were revealed or why not. Left and Right choose. The tab's count in the row is
  the cards held.
- **The ceremony** shows a card the hub picked with its face, bigger (400 wide) and
  higher, its name as the title, "Legendary, Holo. Alpha Edition. From a Mythical
  Holocron" and its set and number in place of the sealed line.
- **The player card's** rarest holocron gem counts opened holocrons too, as the
  counts now hold only the unopened ones.
- `debug_open <tier|all> [x<count>] [normal|holo|solar]` plays the ceremony with
  made-up cards on the Holocrons tab; nothing is sent or kept. A rarity the packs have
  faces of shows one of them, as the hub would pick.

## Planned, not built

1. The rest of the first set: cards of every rarity (only Sol lore 001, Legendary,
   exists), so every sealed copy can be revealed; a reveal ceremony (the copies turn
   over one by one, as an opening's cards turn in).
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

The crate opening (17/10/2026, Windows 11): unit tests for the 3D holocron's motion
(glide, hum carried into the unseal, spin-up, swell, light, shake bounds, gone in the
flash), its handover (the one asked for, then the one opened, back in its place after),
the strip's stop always inside the prize's tile in both strip sizes and its speed curve,
the finds' order and grand ones, the batch's moments and skips, Open all's flow, and every
moment of a single opening (every tenth of a second) and of twenty at once (every
quarter) fitting the canvas in three windows. World shot `duel6_sjk_crate_open`
(off-screen, duel6, made-up prizes, no blade-skin packs so shader tiles show plain
blades) was looked at: the wait, the unseal early, charged and flashing, the strip out
of the flash, fast, crawling and at rest, a Solar Legendary card and a Mythical shader at
rest, and a batch of twelve cracking open, running, stopping, at rest, its last round and
its finds coming in and laid out. Not tried: in a running client (no motion seen, no
sounds heard: the cue files are checked to exist in the retail `assets0.pk3` only), with
the hub's packs, at 4K or 4:3 in a shot (the canvas test covers them).
