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
  JKR's old `jkr_*` names are aliases in
  [cvar_renames.rs](../crates/sjk-viewer/src/cvar_renames.rs).
- Crates, modules and source identifiers keep JKR's names (`sjk-viewer`,
  `jkr-*`), so JKR's changes keep merging cleanly.
- Public text ([README](../README.md), [CREDITS.md](../CREDITS.md), the site)
  names Sol and Bishop rather than using pronouns, and credits JKR's work to
  Bishop and its contributors. Keep CREDITS.md current when SJK gains notable
  work.

## Automation

| Workflow | Runs on | Does |
| --- | --- | --- |
| [CI](../.github/workflows/ci.yml) | Pull requests, pushes to `main` | Formatting, workspace build and tests (shared with JKR) |
| [Pages](../.github/workflows/pages.yml) | Pushes to `main` changing `site/` or the workflow | Publishes `site/` to https://sol-vulpes.github.io/SJK/ |
| [SJK release](../.github/workflows/release.yml) | Tags `sjk-v<version>` | Builds and publishes the release ZIPs |

Release tags exist only on SJK and are created by Sol. Alphas are not marked as
pre-releases, so the site's download link (`releases/latest`) finds them; their
name and notes say "Alpha" instead.

## Debug panel

The `debug_panel` console command lists SJK's changes and how to test them, from
[debug_panel.txt](../crates/sjk-viewer/assets/debug_panel.txt). It is personal to
SJK and never part of a JKR pull request. Update it in the merge that brings a
change into SJK's `main`.
