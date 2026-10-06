# SJK conventions

SJK is JKR with Sol's changes on top. The rest of the wiki, [AGENTS.md](../AGENTS.md)
and [development.md](development.md) are shared with JKR and apply unchanged;
this page holds only the rules that exist because SJK is a separate project.

## Branches

- `main` of [Sol-Vulpes/SJK](https://github.com/Sol-Vulpes/SJK) is SJK: JKR's
  `main` plus SJK's topic branches. It is updated by merging, never rebased.
  JKR's `main` is merged in after Sol has reviewed the incoming changes.
- A change meant for JKR starts from JKR's current `main` and follows the JKR
  [branch and pull request rules](development.md#branches-commits-and-pull-requests).
  Its pull request comes from a fork of JKR; SJK is not one, so no pull request
  can be opened from it. The branch is merged into SJK's `main` when Sol wants it
  there, accepted upstream or not.
- A change only for SJK is named `personal/<topic>`, starts from SJK's `main` and
  is merged back without a pull request.

Because a JKR pull request merged into SJK's `main` brings its wiki changes with
it, SJK's `status.md` holds sections for changes JKR has not merged yet. Leave
them in place: the upstream merge adds the same text and resolves cleanly.

## Names

- Everything a player sees says SJK: the programs `sjk` and `sjk-server`, the
  release ZIPs, the `GameData/SJK/` folder and messages.
- New settings get neutral engine names (`r_*`, `cg_*`, `cl_*`, ...), never a
  `jkr_` or `sjk_` prefix, and must not collide with EternalJK or rend2 settings.
  JKR's old `jkr_*` names are aliases in the client's
  [cvar_renames.rs](../crates/sjk-viewer/src/cvar_renames.rs) and the server's
  [cvars/mod.rs](../crates/sjk-dedicated/src/cvars/mod.rs) (`RENAMED`).
- The crates are SJK's too: `crates/sjk-*`, packages `sjk-*` (`cargo build -p
  sjk-viewer -p sjk-dedicated`) and Rust paths `sjk_*`. JKR's code uses `jkr-*` for
  the same crates; [sjk_names.py](../scripts/sjk_names.py) holds the mapping.
- SJK's emblem (Sol's) is its picture: the classic and modern main menus, the
  programs' and window's icons, the README, release notes and site. Every image
  of it is generated from one original by the scripts in
  [assets/branding](../assets/branding/README.md); regenerate them rather than
  editing one by hand.
- What stays JKR's on purpose: the old names inside the alias tables, the
  `GameData/jkr` import, `JKR_*` environment variables, the dedicated server's
  `jkr_server.cfg`, and "JKR" meaning Bishop's project.
- Public text ([README](../README.md), [CREDITS.md](../CREDITS.md), the site)
  presents SJK as Sol's: Sol develops it and sets its direction, and Sol comes
  first (Sol's rule, 06/10/2026). It names Sol and Bishop rather than using
  pronouns, and credits JKR, the engine SJK is built on, to Bishop and its
  contributors. Keep CREDITS.md current when SJK gains notable
  work.

## Defaults

A new profile starts with Sol's own choices where JKR's defaults differ. Defaults
apply only to settings a `config.cfg` has not saved: an existing profile keeps its
values and nothing is migrated. Where each is documented:

| Setting | SJK | JKR | See |
| --- | --- | --- | --- |
| `ui_menuStyle` | `classic` | `modern` | [Menu style](client.md#menu-style) |
| `ui_gameFont` | on | off | [UI ownership](rendering.md#ui-ownership) |
| `cg_scoreboardStyle` | `classic` | `modern` | [Scoreboard styles](client.md#scoreboard-styles) |
| `cg_drawTimer`, `cg_drawTeamOverlay` | on | off | [status.md](status.md#gameplay-and-interface-defaults-sjk-only) |
| `cg_dismember` | 2 | 0 | [Dismemberment](client.md#dismemberment-and-disintegration) |
| `snaps` | 120 (slider to 125) | 40 (slider to 60) | [status.md](status.md#gameplay-and-interface-defaults-sjk-only) |
| `com_maxfpsUnfocused` | 30 | 0 | [Configuration](client.md#configuration-and-content) |
| `cl_maxpackets` | 125 | 63 (unused) | [User commands and move packets](networking.md#user-commands-and-move-packets) |
| rendering profile | noon, bloom, dust, material maps | 11:00, off | [Default visual profile](rendering.md#default-visual-profile) |

The HUD look (`cg_hudStyle game`) and the console (`con_style classic`) are SJK
defaults too, documented with their pages.

## Copyright notice

[NOTICE](../NOTICE) is SJK's copyright notice: Sol-Vulpes, Bishop-R and the JKR
contributors, under GPL-2.0-only. The client
([notice.rs](../crates/sjk-viewer/src/notice.rs), printed to its log and console
by `app_launch.rs`) and the dedicated server (`NOTICE` in its `main.rs`) announce
the copyright and the absence of warranty at startup, so GPLv2 section 2(c)
requires modified versions to keep announcing them. Keep the two copies of the
lines in step. SJK packages put `NOTICE` first in `LICENSES-SJK.txt`. The SJK
names and emblem are not licensed with the code (see `NOTICE`).

## Version

One file decides the programs' version: [build_version.rs](../scripts/build_version.rs),
which both programs' `build.rs` include. Releases are numbered by date and a
counter, `YYYY.MMDD.N`: the first release of 05/10/2026 is `2026.1005.1`, a second
one that day `2026.1005.2` (Sol's choice, 05/10/2026). The form sorts, is a valid
Cargo and Windows version, and is tagged `sjk-v2026.1005.1`; people see the date
itself day first in the label below. A release build gets the version of its tag
through `SJK_VERSION`, which the release workflow sets; any other build says `dev`,
so a local build never passes for a release. The Cargo package version (`0.1.0`)
is not the release number. The source commit's short hash and committer
time come from git when the source is a checkout, else from `SJK_COMMIT` and
`SJK_COMMIT_TIME`, else they are left out. A build is redone when `HEAD` moves;
uncommitted edits keep the last commit's label. The client shows
`SJK <version> · <dd/mm/yyyy HH:MM> · <commit>` at the top of the screen
([client.md](client.md#version-label)) and in its log, the menus and console show
`SJK <version>`, and both startup notices carry the version.

## Dates and times

Dates that players see, and dates in SJK's own text (release notes, Discord posts,
SJK's own pages), are written day first, `dd/mm/yyyy`, and times on the 24-hour
clock, `HH:MM`, with no AM or PM (Sol's rule, 05/10/2026). The version label and the
classic console's clock follow it. Machine formats are exempt: git tags, ISO 8601
timestamps in logs and JSON, and file names meant to sort. Pages shared with JKR
keep their own style.

## Merging from JKR

SJK's names differ from JKR's, so JKR's changes are merged with
[sjk_names.py](../scripts/sjk_names.py), never with a plain `git merge`:

```sh
python scripts/sjk_names.py merge upstream/main     # or a JKR pull request branch
python scripts/sjk_names.py continue                # after resolving any conflicts
```

It translates the JKR commit and the merge base to SJK's names with the same rules
that renamed SJK, merges three ways and records the JKR commit as the merge's
second parent. A JKR change therefore conflicts only where it would have without
the renames. Renaming another setting means adding its pair to an alias table and
running `python scripts/sjk_names.py apply` on SJK in the same change, so SJK and
every later translation agree. Check `cargo build --locked` after a merge that
changed dependencies.

## Automation

| Workflow | Runs on | Does |
| --- | --- | --- |
| [CI](../.github/workflows/ci.yml) | Pull requests, pushes to `main` | Formatting, workspace build and tests (shared with JKR) |
| [Pages](../.github/workflows/pages.yml) | Pushes to `main` changing `site/` or the workflow | Publishes `site/` to https://sol-vulpes.github.io/SJK/ |
| [SJK release](../.github/workflows/release.yml) | Tags `sjk-v<version>` | Builds and publishes the release ZIPs |

Release tags exist only on SJK and are created on Sol's request. The release
title carries the stage, "Sol JK 2026.1005.1 (Alpha)" (`STAGE` in the workflow).
Alphas are not marked as pre-releases, so the site's download link
(`releases/latest`) finds them; their name and notes say "Alpha" instead.

## Changelog

[CHANGELOG.md](../CHANGELOG.md) lists every release, newest first, each change
closed by its credit: who made it (Sol, Bishop) and "after <client>" when it
follows another client's behaviour (EternalJK, JoF EJK). The client builds the
file in and shows it on its changelog page (main menu > Changelog, or the
`changelog` command; [client.md](client.md#changelog-page)), and its tests reject
a change without a credit, a date that is not `dd/mm/yyyy` or a non-ASCII
character. The merge that brings a player-visible change into `main` adds its line
under "Unreleased"; tagging a release renames that section to
`<version> | <dd/mm/yyyy>` and its lines become the release notes' "New since"
list. The Discord changelog posts are made from the same file.

## Credits

[credits.txt](../crates/sjk-viewer/assets/credits.txt) feeds the client's credits
page ([client.md](client.md#credits-page)); [CREDITS.md](../CREDITS.md) stays
the full written record. Sol develops SJK alone since October 2026; Bishop is
credited for JKR, the engine and renderer SJK is built on.

Credits are kept for everything (Sol's rule, 06/10/2026): every merge into
`main` that brings someone's work updates credits.txt, CREDITS.md and the
changelog line's credit in the same change. A person gets a card in the merge
that brings their first change (Creyon with SJK pull request #2), and their card
gains lines, with the pull request numbers, as they keep contributing; Sol's own
card gains a line when a notable feature lands. Cards carry the person's GitHub
handle (`github:`, opens their profile when clicked) and may carry `link:`
addresses (https only, optionally `Label | https://...`). Sections are free: a
later "Supporters" section is a `== Supporters` heading and its cards, with no
code. The tests reject a card without a role, an unknown key, a link that is not
https or a non-ASCII character.

## Debug panel

The `debug_panel` console command lists SJK's changes and how to test them, from
[debug_panel.txt](../crates/sjk-viewer/assets/debug_panel.txt). It is personal to
SJK and never part of a JKR pull request. Update it in the merge that brings a
change into SJK's `main`.
