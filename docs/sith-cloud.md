# Sith cloud

The Sith cloud is the S community's own file server: their maps, skins and an
optional pack of unofficial SJK extras (emotes, skins and sounds). SJK chooses and
downloads from it on the same card as [JoF content](jof-content.md), with its own
settings (`sith.*`). Sol asked for it on 28/10/2026; the SJK Launcher's Sith cloud
tab reads the same server.

**The hosts' files, their responsibility.** The Sith cloud's hosts choose and publish
the files; SJK only downloads what a player picks. The card says so under its title.

**Off by default.** Nothing downloads until the player turns on `sith.cloud` and
presses Download on the card (`sith.downloads`, Settings > General or Advanced > Mods >
Sith cloud). Optional groups (SJK Extras) are never chosen by default.

## Where the files go

Straight into the game's `base`, where the SJK Launcher puts them, so the game,
EternalJK and SJK all load them; SJK reads `base` at every mount, so a finished
download loads from the next map. The job's root is `GameData` itself, so its record
of what it fetched, `GameData/sjk-jof-content.json`, is the one the launcher keeps:
both know which files came from the Sith cloud, and only those are ever removed
(a pick taken back, or a file the server no longer lists). Files a player put in
`base` by hand are never touched.

## The server

The server (`sith-cloud`, kept in the SJK Launcher's repository) serves a folder of PK3s over HTTP: `/manifest.json` in JoF's manifest
format (groups from the folder's sub-folders, each file's size and MD5) and the files
under `/repository/<folder>/<name>`. [sjk-jof-content](../crates/sjk-jof-content/src/lib.rs)
reads it as it reads JoF's repository, so the checks, resumable downloads and the
plan are the same.

SJK holds no address for it: the SJK hub names the server (`GET /v1/content-sources`,
asked before each list, from `cl_hubUrl`), so SJK can move it or switch it off for
everyone (the hub's empty list) without a release. Without a hub the card says it cannot
ask where the server is. `sith.cloudUrl` sets another server by hand and skips the hub.

## Settings

| Setting | Default | What it does |
|---|---|---|
| `sith.cloud` | 0 | Allow downloads from the Sith cloud |
| `sith.cloudGroups` | `default` | Groups kept: `default` (the server's recommended ones), `none`, or group ids |
| `sith.cloudFiles` | empty | Files picked one by one against their group (`+<id>`, `-<id>`), written by the card |
| `sith.cloudUrl` | empty | Another server's address; empty uses the one the SJK hub names |

The card is [jof_content_card.rs](../crates/sjk-viewer/src/jof_content_card.rs) with
the Sith cloud's words ([sith_cloud.rs](../crates/sjk-viewer/src/sith_cloud.rs)); like
JoF content it is built with the `mod-jof` feature, which brings `sjk-jof-content`.
