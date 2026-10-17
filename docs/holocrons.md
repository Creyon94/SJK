# Holocron drops

A holocron is loot a player earns by playing SJK. The SJK hub rolls and stores them and
the client shows them. Since 12/10/2026 a holocron can be opened: it gives a sealed card
and credits ([cards.md](cards.md)), and nothing else (no power or setting). This page is the design, the decisions and what
is checked; the wire is the hub's `PROTOCOL.md` ("Holocrons") in Sol-Vulpes/SJK-hub,
decided together with the client and built in parallel. Players read the pop-up, the chat
lines and the Collection's Holocrons tab.

## Pieces

| Piece | Where |
| --- | --- |
| Wire types, the `active` claim flag, `GET /v1/holocrons`, drop events, staff requests | [wire.rs](../crates/sjk-identity/src/wire.rs), [hub.rs](../crates/sjk-identity/src/hub.rs), [staff.rs](../crates/sjk-identity/src/staff.rs) |
| Progress in the snapshot, `Service::set_active`, the worker's early profile reading | [service.rs](../crates/sjk-identity/src/service.rs) |
| Drops joined to the chat's messages, told to the worker when they are the player's own | [feed.rs](../crates/sjk-identity/src/feed.rs) |
| The one client list of tiers, the player's counts, recent holocrons, progress | [holocrons.rs](../crates/sjk-viewer/src/holocrons.rs) |
| What counts as actively playing, the input clock | [holocrons/activity.rs](../crates/sjk-viewer/src/holocrons/activity.rs), [holocrons_frame.rs](../crates/sjk-viewer/src/holocrons_frame.rs), input hooks in [viewer_app.rs](../crates/sjk-viewer/src/viewer_app.rs) |
| The drop pop-up, its ceremony and look | [holocron_popup.rs](../crates/sjk-viewer/src/holocron_popup.rs), [holocron_popup/view.rs](../crates/sjk-viewer/src/holocron_popup/view.rs) |
| `holocrons_seen.txt` | [holocrons/seen.rs](../crates/sjk-viewer/src/holocrons/seen.rs) |
| The chat lines' words and the tier gem | [holocrons/line.rs](../crates/sjk-viewer/src/holocrons/line.rs), [holocrons/gem.rs](../crates/sjk-viewer/src/holocrons/gem.rs), [sjk_chat_look.rs](../crates/sjk-viewer/src/sjk_chat_look.rs) |
| The tiers' pictures in the UI atlas | [holocrons/icons.rs](../crates/sjk-viewer/src/holocrons/icons.rs), [ui_renderer/icons.rs](../crates/sjk-viewer/src/ui_renderer/icons.rs), [hud/icons.rs](../crates/sjk-viewer/src/hud/icons.rs) |
| The Holocrons tab (the page, its drawing, the console side) | [holocrons_panel.rs](../crates/sjk-viewer/src/holocrons_panel.rs), [holocrons_panel_view.rs](../crates/sjk-viewer/src/holocrons_panel_view.rs), [console_holocrons_page.rs](../crates/sjk-viewer/src/console_holocrons_page.rs) |
| The loot table (the page, the odds worked out) | [holocrons_panel_loot.rs](../crates/sjk-viewer/src/holocrons_panel_loot.rs), [cards/loot.rs](../crates/sjk-viewer/src/cards/loot.rs) |
| The tab's 3D holocron, the camera's shot behind it | [holocrons/stage.rs](../crates/sjk-viewer/src/holocrons/stage.rs), [menu_backdrop/routes.rs](../crates/sjk-viewer/src/menu_backdrop/routes.rs) |
| `debug_holocron` | [holocrons/rehearsal.rs](../crates/sjk-viewer/src/holocrons/rehearsal.rs) |
| Give holocron and Remove on the Staff page | [staff_panel.rs](../crates/sjk-viewer/src/staff_panel.rs), [staff_panel_view.rs](../crates/sjk-viewer/src/staff_panel_view.rs) |
| The hub's rolls, caps and storage | repository Sol-Vulpes/SJK-hub (`PROTOCOL.md`, `src/holocrons.rs`) |

## Tiers

Four tiers, the same ids, names and order in the hub's `src/holocrons.rs` and in the
client's [holocrons.rs](../crates/sjk-viewer/src/holocrons.rs); a test on each side pins
the list. A client shows only the tiers it knows.

| Id | Name | Colour | Odds |
| --- | --- | --- | --- |
| `uncommon` | Uncommon Holocron | green `#2FC77A` | 60% (600 per mille) |
| `rare` | Rare Holocron | blue `#2E7BFF` | 28% (280) |
| `legendary` | Legendary Holocron | purple `#A64DFF` | 10.5% (105) |
| `mythical` | Mythical Holocron | gold `#FFC933` | 1.5% (15) |

The colours are none of the game's `^n` codes, in the game's palette or the SJK UI's
lifted one (at least 0.28 away in RGB, tested), and the four are at least 0.4 apart from
one another for players who tell hues apart poorly. Mythical gold is near the SJK chat's
own gold (`#F5C756`, 0.14 away) because both are gold; a drop line stands apart from a
message by its gem, its wording and having no sender column. Each tier's picture is
`gfx/sjk/holocron_<tier>.png`, bundled with the client and mounted below all game data
(the list in [illuminate.rs](../crates/sjk-viewer/src/illuminate.rs)); where it is missing
the tier draws as a gem made of shapes ([holocrons/gem.rs](../crates/sjk-viewer/src/holocrons/gem.rs)).

The Holocrons tab's tier rows carry the tiers' rarity effects (10/10/2026): from Rare up
a breathing frame, Legendary's sheen, Mythical's running spark and motes, faint for a
tier not held ([unlockables.md](unlockables.md#rarity-effects), off with
`ui_rarityEffects 0`).

## Loot table

Every chance a holocron brings, as players see it on the Holocrons tab's loot table
(12/10/2026, Sol's request; Loot table or L). The client works it out from the hub's
tables, which `GET /v1/holocrons` sends under `loot` (14/10/2026, [`loot_odds.rs`](../crates/sjk-viewer/src/loot_odds.rs)):
the tiers' odds, the card's odds and credits ([cards.md](cards.md#odds)) and the shader
odds. Until the hub's first answer, or from a hub older than them, it shows its own copy
(`loot_odds::BUILT_IN`); tables that do not fit what the client can show (other tiers,
rarities or finishes, shares not summing to a thousand) are ignored the same way. Tier
odds read elsewhere ("10.5% of drops") come from the same tables. A card's rarity and its finish are rolled apart, so a kind is the two chances
multiplied; "per holocron found" weighs each tier by how often it drops. Since 14/10/2026 an
opening may bring a saber shader instead of the card and its credits (30% of Uncommon
holocrons to 50% of Mythical ones, [unlockables.md](unlockables.md)): the card columns of a
tier are the card's odds when it holds one, the "any holocron" shares are of the cards
found, and the credits' averages and the second table are per holocron found, shaders
counted (`cards::loot::card_in`). Since 17/10/2026 (prepared, not deployed) one opening in
ten may bring a pile of kyber shards instead ([cards.md](cards.md#shards-and-the-shop)): the
tables' `shard_pile` and `shards`, a "Kyber shards" column (the chance and the range, the
average under it, a dash from a hub without piles), and the card's share counts them too.
The figures below are with the piles.

Per holocron:

| Holocron | Drop chance | Common | Uncommon | Rare | Legendary | Normal | Holo | Solar | Credits (average) | Kyber shards (average) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Uncommon | 60% | 70% | 24% | 5.5% | 0.5% | 97% | 3% | 0% | 10 to 20 (9) | 10%: 5 to 15 (1.0) |
| Rare | 28% | 50% | 33% | 15% | 2% | 90% | 10% | 0% | 30 to 50 (20) | 10%: 10 to 25 (1.8) |
| Legendary | 10.5% | 30% | 38% | 26% | 6% | 70% | 29% | 1% | 80 to 120 (45) | 10%: 25 to 50 (3.8) |
| Mythical | 1.5% | 10% | 30% | 45% | 15% | 40% | 55% | 5% | 250 to 350 (120) | 10%: 100 to 200 (15.0) |
| Any holocron found | 100% | 60.9% | 27.5% | 10.1% | 1.51% | 92.3% | 7.56% | 0.14% | 10 to 350 (18) | 10%: 5 to 200 (1.7) |

Every card, per holocron found:

| Card | Normal | Holo | Solar |
| --- | --- | --- | --- |
| Common (60.9%) | 31.8% (1 in 3) | 1.9% (1 in 53) | 0.0172% (1 in 5 822) |
| Uncommon (27.5%) | 13.9% (1 in 7) | 1.34% (1 in 75) | 0.027% (1 in 3 710) |
| Rare (10.1%) | 4.78% (1 in 21) | 0.774% (1 in 129) | 0.0258% (1 in 3 878) |
| Legendary (1.51%) | 0.661% (1 in 151) | 0.165% (1 in 606) | 0.0073% (1 in 13 633) |

The page: over the darkened tab, the title, what it is and the hub's pace ("One holocron
about every 30 minutes of active play on a server, at most 8 a day", once the hub has said;
else "Holocrons are found by playing on a server."); "Per holocron", the first table, each
tier's row in its colour with its picture, the row for any holocron found in gold under a
rule, credits as the range with the average under it, a chance of none quiet; "Every card,
per holocron found", the second table as a grid of cells in the rarity's colour, each its
share and "1 in N" (left out above one in two); "How it rolls", notes: the rolls made
apart, credits and piles of shards rolled evenly, the rarest card ("a Legendary Solar:
about 1 in 13 633 holocrons", worked out), how the card is picked among its rarity's (sealed while the rarity has none), gifts given and not found. Back
(or Escape, L, Enter) goes back to the tab. Shares read with three figures at most, halves
rounded up (`cards::loot::percent`). A change to the hub's tables needs the client's copy
changed with it: until then the page shows the old odds.

## Earning

- Every claim to the hub carries `"active": true` only when the player is actively
  playing. The client says so when it is in a live, non-local game of someone else's, in
  the player's own view (not spectating or following), mid-match (not at the intermission
  or the scoreboard freeze), out of the menus (the game menu, the console, a dialog, a
  pop-up or the main menu) and had input (a key or mouse button pressed, the wheel, the
  mouse moving) in the last two minutes. A claim without the flag, which an older client
  sends, earns nothing; `active: false` is left out of the body altogether.
- On each claim renewal with `active: true` the hub adds the time since its last claim of
  the key, at most 60 seconds, to the key's active time, and `last_tick` moves on with every
  claim. The client renews every 45 seconds, so a renewal counts in full and sending claims
  faster counts only real elapsed time. At 1800 seconds the hub drops a holocron and keeps
  the rest. Progress persists in the hub's database across sessions. A hub started with
  `--holocron-every <secs>` drops sooner, for tests; the caps stay.
- The tier is rolled by the hub from a system random source.
- Caps, all the hub's: at most 8 drops in a rolling 24 hours per key (staff gifts excluded);
  at most 1 mythical per key in 24 hours (another becomes legendary); at most 16 per
  address in 24 hours across all keys. A drop a cap holds back waits (the active time stays
  at 1800) and happens at the first renewal after the cap frees; nothing is lost.
- A staff gift (`source: "staff"`) counts toward no cap and is not announced.

The client reads the progress with `GET /v1/holocrons` (`progress_secs`, `every_secs`, the
day's `today` and `daily_cap`) at the registration, every five minutes, soon after a drop
of its own and when a screen asks ([`holocrons::refresh`](../crates/sjk-viewer/src/holocrons.rs),
at most once every 30 seconds). A hub from before holocrons refuses it; it is asked again
in an hour and the identity's status never shows it. The remaining time is as the hub last
said and does not count down by itself.

## What the hub stores, and privacy

The hub stores each key's active play time (a total carried toward the next holocron) and
the time of its last claim, and every holocron dropped for a key: its tier, time, source
(`play` or `staff`), a staff note and the address (or a hash of it) it dropped from, for
the per-address cap. The profile lists a key's counts and its recent holocrons publicly,
as it does its medals, and legendary and mythical drops are told to every SJK client in
the feed with the key's hub name. Uncommon and rare ones reach only their owner. Old
clients ignore all of it. [identity.md](identity.md#privacy) says what a default install
sends.

## In the client

### The pop-up

A holocron new to the client shows once, large, on a darkened screen in its tier's colour:
"New holocron" (and "1 of 3"), where it came from (dropped while playing, or a gift from
the SJK team), its name, when it was found (`dd/mm/yyyy HH:MM`, UTC), how rare it is and,
for a gift, the team's note. It follows the medal pop-up
([identity.md](identity.md#medals), [medal_popup.rs](../crates/sjk-viewer/src/medal_popup.rs)):

- It opens on the main menu, or when the game menu opens in a match, never over play. A
  holocron that arrives during a match is announced once by a card at the top of the
  screen, the achievements' ([identity.md](identity.md#achievements),
  [unlock_toast.rs](../crates/sjk-viewer/src/unlock_toast.rs); since 10/10/2026, Sol's
  request, in place of a centre print): "New holocron", "Open the game menu to see it", the
  name in the tier's light, "Found while playing: 28% of drops are Rare" (or "A gift from
  the SJK team"), the tier's icon (its gem without one) in a ring that sweeps round, and the
  burst, sparks and edge in the tier's colour, with the achievements' chime
  (`cg_achievementSound`). It takes no input and leaves after about six seconds; one
  arrival brings at most three cards (a reinstalled PC finding twenty shows three), and the
  pop-up shows them all. When the pop-up opens it takes back the holocron cards still
  showing or waiting. While it shows, the menu under it is neither drawn nor given input. A
  medal on show or waiting goes first.
- Each arrives in a ceremony of the medal's timing and light in the tier's colour (it comes
  down into place, rings and sparks run out, a ring of ticks turns, a glow breathes while it
  waits, the words fade up one group after another) with the multiplayer game's fanfare
  (`music/goodsmall.mp3`, the medals' cue; silent without game data). Enter, Space, Right,
  Escape or a click finish the entrance, then take Next (Close on the last). The look is the
  SJK UI's in its families, Inter until they load, and the same under the classic menus.
- Everything is draw-list shapes of a fixed number: no allocation a frame, and a canvas of
  320 commands holds the busiest moment.
- `holocrons_seen.txt` in the settings folder, beside `identity.key`, holds the key id and
  the highest holocron number shown, written when a holocron's button is taken. A number
  above it is new. With no file for the key (a reinstall, a new PC) everything the profile
  holds is new, and the newest 20 show once, oldest first. A holocron of a tier the client
  does not know is left out and never counted as shown. Holocron numbers only rise at the
  hub, so a staff removal never brings an old one back.
- The identity worker reads the own profile again soon (at most once every 30 seconds) when
  a drop for the player's own key arrives in the feed, and the pop-up sees the new holocron
  within half a second of the profile arriving.

### Chat lines

A drop is one SJK chat line in the tier's colour with a small gem where the verified tick
goes, in all three places the SJK chat shows ([hub-chat.md](hub-chat.md#how-a-line-looks)):

- The game's chat feed and the docked chat: `Sol found a Legendary Holocron!` (the hub
  name without colour codes, "Someone" when it has none), and for the player's own
  `You found an Uncommon Holocron.`; a `!` ends the two grand tiers' lines, a `.` the others.
  There is no sender column, no colon and no tick.
- The chat page: the name in its column, the gem, then `found a Legendary Holocron!`, with
  no colon; a tier the page does not know reads `found a holocron.`. A name with the JoF
  clan's tag keeps its emblem before it there ([identity.md](identity.md#clan-tags)); the
  feed and the dock word the name inside the sentence, so they draw no emblem.
- A muted player's drops are not shown, and a drop of an unknown tier is left out of the
  feed and the dock. A drop is not a message, though it shares the feed's numbers and is
  kept with the chat's last 200.
- Legendary and mythical drops are delivered to every reader and uncommon and rare ones to
  their owner only, by the hub; a fresh reader gets only the drops of the last 60 seconds,
  so a joiner is not replayed history. With `cl_sjkChat 0` no line is kept, but the
  player's own drop still makes the worker read the profile.
- **Grand finds** (since 17/10/2026, with a hub that sends the feed's `finds`): when a
  holocron someone opened gives a Legendary or Solar card, or a Legendary or Mythical
  shader, every SJK player gets a line like a drop's, in the holocron's colour with its
  gem: `Sol opened a Mythical Holocron: Phoenix blade!`, `Fox opened an Uncommon
  Holocron: a Legendary card!`, `You opened a Rare Holocron: an Uncommon card, Solar!`
  (the shader's name plain and at most 40 characters, the card's rarity and its finish
  unless plain). The identity service relays a find as a drop line whose mark carries
  the prize (`DropMark::find`, [feed.rs](../crates/sjk-identity/src/feed.rs)); the
  words are [holocrons/line.rs](../crates/sjk-viewer/src/holocrons/line.rs)'s. Muted
  players' finds and unknown tiers are left out as drops are.

### Staff

The Staff page ([identity.md](identity.md#staff)) has a Holocrons section under the
players: the chosen player's counts, the four tiers to choose (the choice ringed in its
colour), Give (with the note field, `StaffRequest::HolocronGive`, `POST
/v1/staff/holocron-give`) and the player's four newest holocrons with Remove
(`StaffRequest::HolocronRemove`, `POST /v1/staff/holocron-remove`). The hub's answer is the
target's profile and replaces the chosen player's, and the player's own when it is theirs.
To make room the player list shows 10 players instead of 14.

### `debug_holocron`

`debug_holocron <tier|all> [x<count>]` (alone it lists the tiers and the odds) rehearses
drops offline: `all` queues one of each tier, `rare x3` three of one (1 to 20). They go
through the same queue, card (in a match), pop-up, ceremony and sound, and the first one's
chat line goes into the game's feed as the player's own. The console closes so the pop-up
shows at once on a menu, or the cards over play in a match. Nothing is sent to the hub, and rehearsed holocrons (numbered from
4,000,000,000) are never written to `holocrons_seen.txt`.

### The Holocrons tab

The Collection's sixth tab in the SJK UI ([sjk-ui.md](sjk-ui.md#collection); the Profile
screen's eighth until the Collection was made, the same day); with the classic menus the
Profile page's See holocrons or the `holocrons` command open the same page on its own, in
the SJK UI's look as the Collection page does. Left, the world is left
clear for one 3D holocron; right, over a dark panel, the four tiers (their picture, name in
the tier's colour, how many are held or a dash while the hub has not answered, and the odds
of a drop), what the chosen tier is, "Next holocron in about N minutes of play" with a bar
and "n of 8 today", the newest ten holocrons (tier, `dd/mm/yyyy HH:MM` in UTC, and for a gift
"Gift from the SJK team" with the note) and, when the identity is off, no hub is set or the
hub has not answered, the reason ("Holocrons need the SJK identity"). Since 12/10/2026 the
counts are the holocrons not opened yet, the player's credits (and since 17/10/2026 their
kyber shards) stand at the panel's top right, the chosen tier says what opening one gives,
Open and Open all (and Shop, while the hub lets the player open, [cards.md](cards.md#in-the-client))
stand under the holocron's caption and opened holocrons are marked in the recent list; opening and its
ceremony are in [cards.md](cards.md#in-the-client). While the hub keeps opening to staff
(`can_open` false, [cards.md](cards.md#opening)) a line says opening comes soon in place of
the buttons.

- **Keys and pointer:** Up and Down (or Left and Right, Tab and Shift+Tab, the wheel, 1 to
  4, Home and End) choose the tier; Escape goes back; a click on a row chooses it; Ctrl+Tab
  is the Profile screen's. Enter opens the oldest holocron of the tier chosen, A opens up
  to twenty (both only when the hub lets the player open). L, or Loot table at the right of what the chosen tier holds, shows the
  [loot table](#loot-table); S, or Shop, the shop ([cards.md](cards.md#the-shop)).
- **The 3D holocron** is a normal object of the world pass, not a stage of the Character
  tab: an instance of the chosen look's model and a point light in the tier's colour
  ([holocrons/stage.rs](../crates/sjk-viewer/src/holocrons/stage.rs)), floating before the
  camera on the ray through the clear left of the 16:9 frame (`STAGE_AT`), 64 units ahead,
  scaled to a fixed size on screen in every window shape. It turns and bobs as Illuminate's
  does (`illuminate::idle_rotation`, `idle_bob`). While a holocron is opened it is that
  one: it glides to the middle of the window, spins up and is gone in a flash, then grows
  back in its place ([cards.md](cards.md#in-the-client), `holocrons::stage::Motion`). A new tier shrinks the old look away in
  0.12 s and grows the new one in 0.32 s. While the tab shows, the backdrop camera parks on
  its own shot (`Shot::Holocrons`, a route on duel6, where the toured camera cuts to it;
  other maps keep the main vantage), and the classic menu style stops hiding the world
  under it. Over a match (the game menu's Profile screen) it floats before the game's
  camera. The Character tab's stage was not used because that stage is the player model's
  (one actor with a pose, sabers and cosmetics) and is not drawn in a match, while the tab
  needs a rigid model that swaps by tier and works in both.
- **A tier of its own** is a model of its own. A surface's shader is named by its model
  file, so the shared `holocron.md3` (shader `models/sjk/holocron`) would draw the base look
  for every tier whatever shader was given. `scripts/holocron_assets.py` writes
  `holocron_<tier>.md3` (the same cube, surface shader `models/sjk/holocron_<tier>`), they
  are mounted with the other files and loaded with the map's rigid models; a test checks
  each names its tier's shader and is the base cube otherwise. Illuminate's holocron is
  unchanged.
- **A tier held none of** (or not known) shows the locked look and not nothing, so all four
  can be seen: `holocron_locked.md3` with its shader `models/sjk/holocron_locked`, the face
  drained of colour and dark, no glow, lit by a light a fifth as strong in the tier's colour;
  a padlock stands over the tier's name and by its row's count, and the caption says "None
  held yet: shown dimmed".
- The hub's counts, list and progress are copied from the identity's snapshot twice a second
  while the page is open (`Data::of`), never per frame, and the hub is asked for fresh
  progress when the page opens (at most every 30 seconds).

## For other screens: the client's list

[holocrons.rs](../crates/sjk-viewer/src/holocrons.rs) is the one list the rest of the
client reads; the Collection's Holocrons tab is built on it.

- `TIERS: [Tier; COUNT]`, `tiers()`, `Tier::from_id` / `by_id(id)`, `colour(id)`, `name(id)`;
  a `Tier` has `id`, `name`, `colour` (`colour_alpha(a)`), `per_mille`, `odds` text, `index`,
  `icon_path()`, `article()` and `is_announced()`.
- `entries(list)` (known tiers only, in the hub's order, an `Entry` has `id`, `tier`,
  `dropped`, `gift`, `note` and `when()`) and `counts_of(counts)` read a profile's holocrons;
  the Holocrons tab reads the own profile and `snapshot.holocrons` (the progress) once
  in `holocrons_panel::Data::of`.
- `refresh()` (ask the hub again when a tab opens)
  and `next_text(&progress)` ("Next holocron in about 20 minutes of play", or "Daily limit
  reached: 8 of 8 holocrons today").
- `icons::texture(index)` is the atlas cell of a tier's picture, uploaded with the HUD's
  pictures; `hud.holocron_icons` says which loaded (`icons::is_loaded(index)` for the pages the
  console draws). `gem::draw(list, centre, half, colour, alpha, rows)` draws the fallback.
- `Tier::light` is the colour of the tier's point light and `Tier::about()` what the tab says of
  the tier. The 3D holocron's models are `stage::TIER_MODELS` and `stage::LOCKED_MODEL`.

## Checked and not checked

Verified (10/10/2026, Windows 11, `cargo test --locked --workspace`): the wire types parse
(profiles with and without the new fields, drop events, the progress answer); claims carry
`active` only when true; the worker reads progress after registering and every five
minutes, treats a hub without holocrons as cosmetic and reads the profile and progress
soon at most twice a minute after an own drop; the feed merges drops with messages in id
order and tells the worker once per own drop; staff give and remove send the protocol's
fields; the catalogue, its colours and the pop-up's queue, order, first-read cap, seen file,
rehearsal, ceremony and layout (every tier, with and without a picture, a note, in five
window shapes, in the families and in Inter); the three chat views' lines; the staff page's
requests; the activity rules against player states. Not tried: any of it against a running
hub (`crates/sjk-identity/tests/hub_e2e.rs` has an ignored test that needs one), in a game,
or with the art of the tiers (the pop-up draws the gem until the pictures are mounted); the
input hooks and the 500 ms comparison are not covered by a test of their own.

The Holocrons tab (10/10/2026): unit tests for its keys, wheel and clicks, the stage request
for the chosen tier (and the locked look), the data from every identity state, the twice a second
read, every state at 1080p, 4K, 4:3 and 21:9 on the Profile screen and on its own, the 3D
holocron's placement in those windows and its swap, and the tier models' and shaders'
mounting (`illuminate::tests::every_tier_has_a_cube_naming_its_own_shader`). The world shots
`world_shot::holocrons_tab::duel6_sjk_holocrons` and `duel6_sjk_holocrons_windows` (off-screen
renders on duel6 with made-up holdings) were rendered and looked at on this machine: the tab
in the SJK UI with each tier chosen, a tier held none of, the identity off, and 4:3, 21:9, 4K,
`ui_textScale 1.2` and the classic style. Not tried: on another map than duel6, with real hub
data, the keys and clicks in a running client, or the swap's motion (the shots are stills).

Over a match (10/10/2026, Sol's report: the holocron sat too far right in a game): it is
placed from the view the frame renders (`GpuState::frame_view`: the third-person camera
too) with that view's own vertical field. It took the renderer's vertical field for the
horizontal `cg_fov` and narrowed it again, so its offset to the left came out about three
quarters of what it should (and it floated before the eye, not the third-person camera).
The unit test projects it back through the view at 90 and 64 degrees in four windows, and
the world shot `duel6_sjk_collection` renders it from the game menu at a match's field.

## Planned, not built

What opened holocrons will also give: the cards' faces ([cards.md](cards.md#planned-not-built)).
The shop credits buy from and the kyber shards are prepared, not deployed
([cards.md](cards.md#shards-and-the-shop)).
Saber shaders drop already
([unlockables.md](unlockables.md#holocron-shader-drops)).
