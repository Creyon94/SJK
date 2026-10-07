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

Verification is the operator's alone (the hub's `verify` command or SM's SJK screen,
which list every key with its worn names); a player asks for nothing and sets nothing.

Nothing blocks a frame: the viewer compares settings and place with what the thread
was last told twice a second, and the scoreboard re-derives its marks only when the
hub's roster or its own rows change.

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
on it: its text, the map, the client build and the game server's address, signed with
the player's key. The hub keeps it until the operator removes it.

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
or why it refused, for example a quota) shows as a centre print. The hub checks
everything again and limits reports per key (3 a day, 20 once verified, 5 an hour, no
repeat within a day), per address (3 in 10 minutes) and overall (300 a day, 5000 kept),
so a troll with fresh keys gets little through and nothing that is not plain words. The
operator reads them with the hub's `reports` command or `/admin/v1/reports`.

With the classic menus (`ui_menuStyle classic`) the dialog and the button take the
classic+ look ([text_dialog_classic.rs](../crates/sjk-viewer/src/text_dialog_classic.rs),
[classic-plus.md](classic-plus.md#pages)): the in-game pop-up box with its title band,
the text in a retail list box, gold Send and Cancel, the description line under the box,
and a gold REPORT A BUG on retail's red band at the bottom of the canvas.

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
