# Player identity and the SJK hub

SJK players can be recognised across servers without accounts or passwords. Each
install keeps an Ed25519 key; a small web service, the SJK hub, maps public keys to
a display name, a bio and a "verified" flag its operator sets; and while a player is
on a game server, their client tells the hub which slot they are in, so other SJK
clients can put a badge on that scoreboard row. The game works without the hub; the
badges and profiles are an extra.

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
   signed by the key, which proves the client holds it) and fetches its profile.
3. While the client is in a live, non-local session, the thread repeats a *claim*
   every 45 seconds: "this key is in slot N of server S, shown as NAME". Claims
   live 90 seconds at the hub and are withdrawn when the player leaves or quits.
4. The thread reads the hub's list of claims for the server every 15 seconds. The
   scoreboard marks a row with SJK's emblem, in gold when verified, and the player
   card shows the hub name, when a claim names that slot and its claimed name
   matches the name the game shows there (compared after lower-casing and dropping
   colour codes and symbols).

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

With `cl_identity` on and `cl_hubUrl` set the hub receives the player's public key,
the game server address, slot and in-game name for as long as they play, and sees
their IP address. Claims are deleted 90 seconds after they stop being repeated;
profiles stay until the operator removes them. With either setting off the client
sends nothing. Since 06/10/2026 `cl_hubUrl` defaults to `https://sjk.dfox.app` so players
set nothing: a default install makes a key and tells that hub where it plays. The
Identity page, the setting's help and the changelog say what is sent and that
`cl_identity 0` stops it.

## Settings and commands

- `cl_identity` (default 1; Settings > Network > SJK identity) turns the feature on.
- `cl_hubUrl` (default `https://sjk.dfox.app`; Settings > Network > SJK hub) is the hub's address.
  It must be `https://host[:port]` with no path; plain `http://` is accepted for
  localhost only.
- `identity` opens the Identity page (also the in-game SJK menu). `identity name
  <text>` and `identity bio <text>` set the profile at the hub, `identity key` shows
  the key id and file, `identity who [slot]` lists the players the hub knows here
  (with a slot, their bio).

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
