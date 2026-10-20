# JoF content

JoF content downloads the JoF community's maps, skins, hilts and packs from JoF's
own repository, the files JoF's launcher
([jof-launcher](https://github.com/JoFKypdurron/jof-launcher), by Kypdurron, MIT)
installs, and loads them in SJK. It is part of JoF tools: built with the `mod-jof`
feature, named `jof.*`, and its card is SJK UI in every menu style. Sol, a JoF
member, asked for it on 16/10/2026, after Kypdurron pointed Sol to the launcher.

**JoF's files, JoF's responsibility.** JoF chooses and publishes the files; SJK
hosts none of them and only downloads what a player picks, so it is not responsible
for their content. The card says so under its title.

**Off by default (Sol's rule).** Nothing loads until the player turns on
`jof.content`, and nothing downloads until they press Download on the card. Optional
groups (the event map pack, the cosmetics, the JKA HD Pack) are never chosen by
default.

## Where the files go

Into SJK's own JoF folder, `jof` beside SJK's settings (`GameData/SJK/jof`), or
`jof.contentFolder`. The files sit in its `base` folder, as the launcher puts them in
the game's `base`. SJK never writes to the game's installation.

While `jof.content` is on, that folder is one more content root in the search paths
([asset_search_paths.rs](../crates/sjk-viewer/src/asset_search_paths.rs),
`set_extra_root`), standing in for the launcher's files in the installation: inside
each game folder its PK3s sort by name among the installation's, as if they were in
the same folder (`sjk_vfs::pk3_search_order_merged`). So `jof/base` packs take the
place the launcher's layout gives them in `GameData/base`: above `assets0.pk3`, below a
player's own `zz_*.pk3` (Kushina's report, hub #100), and all below
`fs_basegame`/`fs_game`. The roots are read at every mount, so turning the switch on or
off, or a finished download, applies from the next map, with no restart. The JoF
packs also show in the asset browser, and server downloads (`cl_allowDownload`) see
them, so a map already in the JoF folder is not downloaded from the server again.

A file the game's `base` already holds with the manifest's MD5 (JoF's launcher put it
there) counts as here and is never fetched again. SJK leaves the game folder's files
alone: it never updates, moves or removes them.

## The repository

[sjk-jof-content](../crates/sjk-jof-content/src/lib.rs) reads JoF's manifest
(`getclouddata`), the same one the launcher reads
(`lib/services/cloud_service.dart`):

- **Groups.** Only `data.assetgroups` is used: each group has an id, name,
  description, a type (`REQUIRED`, `RECOMMENDED`, `OPTIONAL`; others such as
  `DEPRECATED` are left out) and its files. `clientgroups` (JoF's EternalJK client),
  `modgroups` (the JA+ and Tarascii client plugins, which SJK supports in its core) and
  `othergroups` are not read.
- **Files.** Each file has a name, a size, an MD5, a `localfolder` (where the repository
  serves it) and a `targetfolder`. `Base` files are fetched. A file for another folder
  (one cosmetics file targets `EternalJK`, JoF's own client) is listed as "for JoF's own
  client" and never fetched. A file whose name or folder is not one plain name (a
  separator, `..`, a drive, a character Windows refuses), that is not a `.pk3`, or that
  has no MD5 is left out, so the manifest can never name a path outside the folder.
- **Hosts** ([source.rs](../crates/sjk-jof-content/src/source.rs)), a table read by
  generic code: JoF Cloud (`cloud.jof.app`, an hourly copy that JoF runs to take load
  off the origin) and jofhub.eu (the origin). `jof.contentSource` is `auto` (JoF Cloud,
  then jofhub.eu, as the launcher does), `mirror` or `origin`. A file is
  `<host root>/<localfolder>/<name>`, each segment percent-encoded on its own, so names
  such as `{JoF}'s-GWing.pk3` work.
- **Checks** ([download.rs](../crates/sjk-jof-content/src/download.rs)). A file is
  written to `<name>.part` and renamed into place only when its size and MD5 match the
  manifest, so the game never mounts half a file. A stopped download carries on from
  its `.part` with an HTTP `Range` request. A file that fails on one host (missing, or an
  older copy that fails its MD5) is tried on the next. Requests name the client as
  `SJK/<version> (JoF content)`.
- **The disk** ([store.rs](../crates/sjk-jof-content/src/store.rs)). Each file is
  Installed (in SJK's folder, matching), In the game folder (matching), Outdated (in
  SJK's folder, not matching) or Missing. MD5s are kept in `jof/sjk-jof-content.json`
  with the size and modification time they were measured at, so only new or changed
  files are hashed again.
- **A sync** ([plan.rs](../crates/sjk-jof-content/src/plan.rs)) downloads the wanted
  files that are missing (a file is wanted by its own pick, else by its group's tick),
  smallest first. It deletes only files SJK itself
  downloaded (marked in the record): its copies of files no chosen group lists, and
  those the manifest no longer has or the game folder holds already. A PK3 put in the
  JoF folder by hand stays. It runs on its own thread
  ([job.rs](../crates/sjk-jof-content/src/job.rs)) and goes on while the card is closed
  or the player plays.
- **The game's folder is never SJK's.** A `jof.contentFolder` whose `base` is the
  game's own `base` is refused ("the JoF folder is the game's own folder").

## The card

`jof.downloads`, or Settings > General > JoF content (Simple) or Mods > JoF content (Advanced), Enter
([jof_content_card.rs](../crates/sjk-viewer/src/jof_content_card.rs)). A pop-up card
(1200 by 888, the kit's scrim and glass) in every menu style:

- **Top:** "JoF content" in gold, what it is, Close, and under them whose the files
  are (Sol's request, 16/10/2026): "JoF chooses and publishes these files and is
  responsible for them; SJK only downloads what you pick and is not responsible for
  their content."
- **Use JoF content:** the switch (`jof.content`), with what it does said plainly (off
  by default, nothing fetched until Download; on, loaded from the next map, off leaves
  the files unused on disk), the folder, and that the game folder's files are used as
  they are.
- **Groups:** one row per group, seven at a time (wheel and Page Up/Down for more): its
  tick (`jof.contentGroups`: `default` for the repository's own choice, else the ids
  chosen; Space, or a click on the box), name, type and description, and on the right
  what is here ("16 of 35 here · 3.3 GB to download", "All 157 here", "3 of 157 chosen
  · 75 MB to download", "Not chosen · 200 MB to remove") over its count and size.
  A group with only files for JoF's own client has no tick.
- **A group's files** (Sol's request, 16/10/2026: pick precisely what to take). Enter,
  Right or a click on a group's row shows its files: "‹ Groups / <name>" (a click, Esc,
  Left or Backspace goes back where the player was), the group's own row ("All of
  <name>", how many are chosen; its tick takes every file, or leaves them all out when
  every one was taken), then eight files at a time: tick, name, the repository's
  description, what a download does with it (To download, To update, Here, In game
  folder, To remove, Not chosen, JoF's client only) and its size. Space, Enter or a
  click picks a file alone. Picks against the group's tick are kept in
  `jof.contentFiles` (`+<id>` taken, `-<id>` left out); a pick that matches the tick
  again is forgotten, and the group's tick forgets them all.
- **Foot:** what a download would do, or the job's progress with a gold bar (asking
  for the list, checking the files already here, removing, downloading file n of m),
  how it ended, or why the repository could not be reached; Check again (F5), and the
  main button: Download <size>, Remove <size>, Up to date, or Stop while a job runs.
  Download stays dimmed while the switch is off.

Opening the card the first time in a run fetches the list and looks at the disk. The
first look hashes every matching file in the game's `base` once (about a minute for
10 GB on Sol's PC in a debug build); later looks use the record.

## Settings and commands

| Name | Default | What it does |
| --- | --- | --- |
| `jof.content` | 0 | Load the JoF folder's files (from the next map) |
| `jof.contentGroups` | `default` | Groups kept: `default`, `none`, or ids |
| `jof.contentFiles` | empty | Files picked against their group: `+<id>` taken, `-<id>` left out |
| `jof.contentSource` | `auto` | `auto`, `mirror` (JoF Cloud) or `origin` (jofhub.eu); Settings > Mods > JoF content host |
| `jof.contentFolder` | empty | The folder; empty is `jof` beside SJK's settings |
| `jof.downloads` | | Opens the card |

## Verification

- Unit tests: manifest reading and unsafe names (`manifest::tests`), host addresses
  (`source::tests`), the disk look and its MD5 record (`store::tests`), the plan
  (`plan::tests`), checked, resumed, refused and stopped downloads against a local
  HTTP server (`download::tests`), a whole sync (`job::tests`), the extra root's place
  in the search paths (`asset_search_paths::tests`), and the card's lines and button
  (`jof_content_card::tests`).
- Against the live repository on 16/10/2026 (`cargo test -p sjk-jof-content --test live
  -- --ignored`): both hosts gave the manifest (7 groups, no file left out) and
  `{JoF}UI.pk3`, checked by MD5. With Sol's `GameData`, the files JoF's launcher had
  installed were recognised (for example 118 of 157 skins, 16 of 35 essential maps).
- The card rendered off screen (`jof_content_card::tests::the_jof_content_card_renders_on_a_gpu`,
  1080p, with the switch off and on).
- Not yet verified: a full download of a group in the game, a JoF map loaded from the
  JoF folder on a JoF server, and the HD Pack's effect on SJK's material maps.
