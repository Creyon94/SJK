# Player identity and the SJK hub

SJK players can be recognised across servers without accounts or passwords. Each
install keeps an Ed25519 key; a small web service, the SJK hub, maps public keys to
the in-game names they have worn, an optional bio and a "verified" flag its operator
sets; and while a player is on a game server, their client tells the hub which slot
they are in, so other SJK clients can mark that player on the scoreboard and put a
gold verified badge after a verified player's nameplate. A player does nothing: the
key is made, registered and kept up to date on its own, under the name they play
with. The game works without the hub; the badges and profiles are an extra.

This page is the design and the current limits. The player-facing summary is
[client.md](client.md#identity).

## Pieces

| Piece | Where |
| --- | --- |
| Key file, signing, hub client, background service | [sjk-identity](../crates/sjk-identity/src/lib.rs) |
| Settings and live-session glue | [player_identity.rs](../crates/sjk-viewer/src/player_identity.rs), [identity_frame.rs](../crates/sjk-viewer/src/identity_frame.rs) |
| Scoreboard mark | [identity_mark.rs](../crates/sjk-viewer/src/scoreboard/identity_mark.rs) |
| Identity page, `identity` command | [identity_panel.rs](../crates/sjk-viewer/src/identity_panel.rs), [identity_command.rs](../crates/sjk-viewer/src/identity_command.rs) |
| Players page and player reports | [players.rs](../crates/sjk-viewer/src/ingame_menu/players.rs), [player_report.rs](../crates/sjk-viewer/src/player_report.rs) |
| Medals: catalogue, ribbons, pictures | [medals.rs](../crates/sjk-viewer/src/medals.rs), [medals/](../crates/sjk-viewer/src/medals/), [identity_panel_medals.rs](../crates/sjk-viewer/src/identity_panel_medals.rs) |
| New medal pop-up | [medal_popup.rs](../crates/sjk-viewer/src/medal_popup.rs) |
| The hub itself and its protocol | repository Sol-Vulpes/SJK-hub (`PROTOCOL.md`) |

The hub is a separate repository because it is deployed on its own schedule. The
client and hub each carry the protocol types; `PROTOCOL.md` has a signed-request test
vector that both test suites check, so a drift in either shows as a failing test.

## What happens

1. With `cl_identity` on (the default), the first start creates `identity.key` in
   the settings folder beside `config.cfg`. With it off, no key is made.
2. With `cl_hubUrl` set, a worker thread registers the key at the hub (a request
   signed by the key, which proves the client holds it) with the in-game name the
   player wears (the `name` setting), and fetches its profile. When the name changes
   it registers again with the new one. The hub keeps each key's worn names (20 most
   recent, with when it first and last saw each), and a profile's name is the one
   worn last unless the operator gave the key another. Nobody chooses a hub name, so
   two keys may wear one name: the name proves nothing, the key does.
3. While the client is in a live, non-local session, the thread repeats a *claim*
   every 45 seconds: "this key is in slot N of server S, shown as NAME". Claims
   live 90 seconds at the hub and are withdrawn when the player leaves or quits.
4. The thread reads the hub's list of claims for the server every 15 seconds. The
   scoreboard marks a row with SJK's emblem, in gold when verified, the player card
   shows the hub name, and a verified player's nameplate gets a gold badge after the
   name (`ui_renderer/verified_badge.rs`, read once a second), when a claim names
   that slot and its claimed name matches the name the game shows there (compared
   after lower-casing and dropping colour codes and symbols). The local player's own
   plate (`cg_nameplateSelf`) has the badge when their own key is verified.

Verification is the operator's alone (the hub's `verify` command or its operator API,
which list every key with its worn names); a player asks for nothing and sets nothing.

Nothing blocks a frame: the viewer compares settings and place with what the thread
was last told twice a second, and the scoreboard re-derives its marks only when the
hub's roster or its own rows change. While registered, the thread also reads the
player's own profile again every ten minutes, so the verified flag and medals the SJK
team changes reach a running client.

## What a badge proves

A badge says "a registered SJK key claimed this slot under this name". It does not
prove the player in that slot holds the key, because stock and JA+/JoF servers
publish only fixed fields of a player's userinfo to other clients (SJK's own server
does the same, `bridge_userinfo.rs`), so nothing in-band can carry a proof. The
hub limits the damage: a claim fails while another key holds the same slot under
the same name, display names that normalise to another's are refused, and the claimed
name must match what the game shows. The worst a false claim does is label someone
else's slot with the claimant's own profile; it cannot take another key's name or
verified flag. Badges are for recognition, not for granting anything.

A confirmed tier (SJK's own server adding a hub-signed ticket to the player string)
is planned, not implemented.

## Privacy

With `cl_identity` on and `cl_hubUrl` set the hub receives the player's public key
and in-game name at start and whenever the name changes, the game server address,
slot and in-game name for as long as they play, and sees their IP address. Claims
are deleted 90 seconds after they stop being repeated; profiles and the worn-name
history stay until the operator removes them. With either setting off the client
sends nothing. Since 06/10/2026 `cl_hubUrl` defaults to `https://sjk.dfox.app` so players
set nothing: a default install makes a key and tells that hub where it plays. The
Identity page, the setting's help and the changelog say what is sent and that
`cl_identity 0` stops it.

A bug report (Escape, SJK, Report a bug) is sent only when the player presses Enter
on it: its text, the map, the client build, the game server's address and the in-game
name the player wears, signed with the player's key. The hub keeps it until the operator removes it.

A player report (Escape, Players, or Escape, SJK, Report a player) is sent only by a
verified player who chooses a player and a reason and presses Send: the reason and its few
words, the reported player's slot and the name the game shows for them, the key the hub's
presence list shows in that slot (if any), the server's address and name, the map, the
build, the match clock and the in-game name the reporter wears, signed with the reporter's
key. The hub keeps it until the operator removes it.

A world note (`inspect` twice on the world, [client.md](client.md#player-card)) is sent
only when the player sends it, with `cl_identity` on: its text, the map, the build, the
game server's address, the in-game name the player wears, where the player stood (`setviewpos`), the aimed point, shader,
surface and entity, and a smaller copy of its screenshot (at most 1280 x 720), signed
with the player's key. The note dialog says so. The hub keeps it until the operator
removes it; the full note and screenshot stay on the player's PC either way.

## Bug reports

[bug_report.rs](../crates/sjk-viewer/src/bug_report.rs) and
[report.rs](../crates/sjk-identity/src/report.rs). While the game menu is open, a Report a
bug button sits centred at the bottom of the screen (also Escape, SJK, Report a bug); it
closes the game menu and opens the text dialog
([text_dialog.rs](../crates/sjk-viewer/src/text_dialog.rs)): a panel in the middle of the
screen with a text box, a character count, Send and Cancel (Escape cancels too). As it is
typed or pasted, only letters and digits of any
script, spaces and `. , ! ? ' - : ( )` are kept (line breaks become spaces), up to 600
characters. Enter or Send checks the hub's rules (a refusal is shown in the panel, which
stays open) (10 to 600 characters, a few real words, no
long run of one character) and hands the report to the identity service, which sends
`POST /v1/report` signed with the player's key; the outcome (the hub's report number,
or why it refused, for example a quota) shows as a centre print. The report carries the
in-game name the player wears (the service fills it from the `name` it already sends the
hub); the hub keeps it with the report and adds it to the key's worn names. The hub checks
everything again and limits reports per key (3 a day, 20 once verified, 5 an hour, no
repeat within a day), per address (3 in 10 minutes) and overall (300 a day, 5000 kept),
so a troll with fresh keys gets little through and nothing that is not plain words. The
operator reads them with the hub's `reports` command or `/admin/v1/reports`.

## Player reports

[players.rs](../crates/sjk-viewer/src/ingame_menu/players.rs) and
[player_report.rs](../crates/sjk-viewer/src/player_report.rs). The game menu's Players page
is a small scoreboard of everyone on the server: each connected client's name, side, score
and ping (the client asks the server for scores every two seconds while it shows) and what
the hub knows of them (an SJK player, or a verified one, from the presence list), sixteen a
page with More players... and Back under them. The SJK UI draws it as a table with the
chosen player's card on the right (main page > Players); the classic and modern menus list
it in the SJK pop-up's Report a player. Enter on a player opens the Report page: seven
reasons (cheating, harassment or hate, griefing, exploiting a bug, an offensive name, spam
or advertising, something else) and Back. A reason closes the menu and opens the text
dialog for a few words (10 to 300 characters, the bug reports' alphabet and noise rules);
Send hands the report to the identity service, which sends `POST /v1/player-report`
signed with the player's key, and the outcome (the hub's number, or why it refused) shows
as a centre print. The player is copied when chosen, so the report names them as they were.

Only a verified SJK player may report (`player_identity::report_gate`): with the identity
off, the hub not answering yet, an unverified key or a game on this PC, the Players page
says why under its title (the classic and modern pages in their first line), and the Report
page's reasons are dimmed, Back carrying the reason; yourself and bots cannot be reported
either. The service refuses an unverified key before anything is sent, and the hub checks
everything again: verified keys only, a live claim of the reporter on that server, nobody
reports themselves, 3 an hour and 10 a day per key, one report per key about one player a
day (the same server and name, or the same key), 20 a day about one player from everyone,
3 in 10 minutes per address, 300 a day and 5000 kept overall. The hub names the reported key
from its own live claim on that slot when the names match, else takes the client's. The
operator reads them with the hub's `player-reports` command or `/admin/v1/player-reports`
(also by reported key) and decides what to do.

## World notes

[world_notes.rs](../crates/sjk-viewer/src/world_notes.rs). A world note
([client.md](client.md#player-card)) is kept on the player's PC as before and, with the
identity on, also goes to the hub for the SJK team. The note dialog keeps the bug
reports' alphabet as it is typed, and Send checks the hub's note rules (3 to 500
characters, at least 2 letters, no long run of one character). The identity service
sends `POST /v1/note` with the text and the selection: map, build, server, `view` (the
`setviewpos` x, y, z and yaw), the aimed point and normal, shader, BSP surface,
lighting, distance and the entity's class name (a name outside the hub's alphabet is
left out rather than the note refused), and the in-game name the player wears, as for a
report. The hub answers with the note's number, which a
centre print and the console show, or why it refused. The screenshot writer also makes a
smaller JPEG of the same pixels (`capture::preview_jpeg`: fitted within 1280 x 720,
quality lowered until under 380 KiB) and hands it to the service, which sends it with
`PUT /v1/note/<id>/image` once the hub took the note; a picture that fails leaves the
note as it is. The hub limits notes per key (20 a day, 200 once verified, 30 an hour, no
repeat of the same text about the same shader within a day), per address (40 notes and
pictures in 10 minutes) and overall (1000 a day, 3000 kept), and takes a picture only
from the note's sender, once, within 10 minutes. The operator lists notes with their
pictures with the hub's `notes` command or `/admin/v1/notes`.

With the classic menus (`ui_menuStyle classic`) the dialog and the button take the
classic+ look ([text_dialog_classic.rs](../crates/sjk-viewer/src/text_dialog_classic.rs),
[classic-plus.md](classic-plus.md#pages)): the in-game pop-up box with its title band,
the text in a retail list box, gold Send and Cancel, the description line under the box,
and a gold REPORT A BUG on retail's red band at the bottom of the canvas.

## Medals

A medal is recognition the SJK team gives a player by hand: for testing SJK early,
contributing to its code, finding bugs, or belonging to the JoF clan. A medal grants
nothing: no setting, cosmetic, power or right comes with it, on any server. Players do
not ask for medals or choose them; the SJK team gives them.

| Medal | Id | For | Given again |
| --- | --- | --- | --- |
| Early Tester | `early_tester` | Helped test SJK in its early days. | no |
| Early Contributor | `early_contributor` | Contributed to SJK's code in its early days. | no |
| Bug Hunter | `bug_hunter` | Found bugs that got fixed. | yes, with a count ("Bug Hunter x2") |
| JoF Clan | `jof_clan` | A member of the JoF clan. | no |

The hub lists a key's medals in its profile (`"medals":[{"id","count","awarded","note"}]`:
the count, when it was last given and a short note from the team, often empty) and in its
presence entry (`id` and `count` only), so the scoreboard needs no request per player.
Lists come in the table's order; a client shows only the ids it knows and older hubs
send none. The catalogue is [medals.rs](../crates/sjk-viewer/src/medals.rs): each medal's
name, description, ribbon colours and two pictures in `assets/medals` (`<id>.png`, the
whole medal on its ribbon, 512 square; `<id>_small.png`, the medallion alone, 128
square, for anything under about 64 pixels). A new medal is one entry there and two
pictures; new art is a file replacement. The JoF Clan picture is provisional.

Where they show:

- The scoreboard, every style: up to three small ribbon bars after the SJK emblem, on
  rows whose claim the emblem trusts (the claimed name matches the name the game shows).
  They are coloured rectangles drawn from the catalogue (Early Tester amber with black
  stripes, Early Contributor navy with a white centre stripe, Bug Hunter emerald and JoF
  Clan crimson with black edges), sized after the emblem; they take room from the name,
  never the columns, and fewer show where the name would keep less than half its room
  (the SJK UI: a third). Derived only when the roster or the rows change.
- The player card (`inspect`): the medallions in a row under the hub name; a pinned card
  names them too, with a repeatable one's count.
- The SJK UI's Players page: the chosen player's card lists their medallions and names.
- The Identity page: the player's own medals, each with its whole picture, name and
  count, description, the date it was given (`dd/mm/yyyy`) and the team's note; with
  none, a line saying the SJK team gives medals for testing, contributing and more.
- The new medal pop-up ([medal_popup.rs](../crates/sjk-viewer/src/medal_popup.rs)): the
  first time the client sees a medal in the player's own profile, or a repeatable one's
  count rise, it shows it once, large, with its name, description, date and note;
  Enter, Escape, Space or a click closes it, and several show one after another. It
  opens on the main menu, or when the game menu opens in a match, never over play: a
  medal that arrives during a match is announced once by a centre print pointing to the
  game menu. While it shows, the menu under it is neither drawn nor given input. What was
  shown is kept in `medals_seen.txt` beside `identity.key` (the key id, then one
  `<id> <count>` a line; ids the build does not know are kept), so each medal and each
  new count shows once per identity, and an install that already held medals when it
  first read its profile still shows them.
- Not on nameplates.

Medals are public: anyone can read a key's profile and the presence list of a server,
so a player's medals, counts, dates and notes are visible to everyone, as their hub
name and verified flag are. The client sends nothing about medals.

## Settings and commands

- `cl_identity` (default 1; Settings > Network > SJK identity) turns the feature on.
- `cl_hubUrl` (default `https://sjk.dfox.app`; Settings > Network > SJK hub) is the hub's address.
  It must be `https://host[:port]` with no path; plain `http://` is accepted for
  localhost only.
- The Identity page (main menu > SJK > IDENTITY, the in-game SJK menu, or the `identity`
  command) shows what the hub knows: the name worn now and up to three earlier ones,
  whether the key is verified, the key file's location and the players the hub knows here.
  Its controls are optional: the on/off switch, a bio field with Save, a button that copies
  the key id. The words do the same without it: `identity bio <text>` sets the bio,
  `identity key` shows the key id and file, `identity who [slot]` lists the players the hub
  knows here (with a slot, their bio). `identity name` explains that the name is the one
  played under (`/name`).

## The key file

`identity.key` holds the private key as two lines (`SJK-IDENTITY-1` and the base64url
seed). It is created once and never overwritten; a damaged file is reported and
left alone, and the feature stays off until it is restored, because replacing it
would silently end that identity. The player must back it up: it cannot be
recovered, and copying it to another PC makes that PC the same identity. On Unix it
is created readable by its owner only; on Windows it relies on the folder's default
permissions.

## Planned, not built

The hub is meant to grow: a signed asset manifest, music and video, and private
chat. None of that exists. A main-menu entry for the page is also not added (the
main menu's rows are tight); the page opens from the in-game SJK menu and the
`identity` command.
