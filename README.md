# Sol JK (SJK)

**Sol JK**, or **SJK** for short, is Sol's flavor of
[JKR](https://github.com/Bishop-R/JKR), the Rust engine for **Star Wars Jedi
Knight: Jedi Academy** multiplayer created by Bishop. SJK follows JKR closely
and adds Sol's own changes on top: some are offered to JKR as pull requests,
others reflect a vision of the game that may stay specific to SJK.

Like JKR, SJK includes a graphical client and a dedicated server, with support
for the legacy protocol, PK3 content, maps, models, movement and combat. The
renderer uses wgpu. The client includes a server browser, console, configurable
controls, HUD, audio, screenshots and demo playback.

## SJK and JKR

- **Upstream.** JKR is developed by Bishop (Bishop-R) and its contributors. SJK
  merges JKR's main branch after reviewing it, so JKR's work reaches SJK.
- **What SJK adds.** Sol's changes are written as separate topic branches. Those
  that fit JKR are proposed upstream; once JKR accepts one, it simply becomes part
  of both. At the time of writing, SJK's additions include:
  - a classic menu style after the retail menus, the default in SJK: main,
    profile, setup, controls and server browser pages, retail connect and loading
    screens, the animated logo and glows, and no map behind the menu;
  - a classic scoreboard with client IDs, and HUDs drawn from the game's own
    menu files, so the retail HUD and custom HUD packs work;
  - JA+ support: the client identifies as the JA+ plugin, predicts JA+ movement,
    saber rules, `g_debugMelee`, the grapple and duel pass-through, and adds
    `serverconfig` and `pluginDisable`;
  - the optional retail game fonts on every retail text surface, tighter console
    and chat rows, and text that scales at high resolutions;
  - a renderer settings page for JKR's rendering options, sharp levelshots, and
    MOUSE1, MOUSE2 and ESC locked in the controls editor;
  - more reliable joining of public servers (lost handshake packets and lost
    gamestates are recovered) and support for older 72-bone player models;
  - an FPS cap that follows the monitor's refresh rate by default and holds its
    exact rate;
  - optional rend2-style material maps for world surfaces, with a local generator;
  - gameplay and presentation fixes (third-person camera, Force Speed afterimages,
    saber trails, death animations, key names for non-US keyboard layouts and
    more);
  - a personal `debug_panel` console command: an in-game checklist of the
    changes in this build and how to test them.
- **Names.** The crates and programs keep JKR's names (`jkr-viewer`,
  `jkr-dedicated`, `jkr-*`), and the client keeps JKR's configuration folder
  (`GameData/jkr/`).
  This keeps SJK easy to merge with JKR, and lets settings carry over between the
  two.

## Build

Install Rust 1.88 or newer, Cargo, a C/C++ compiler, CMake and pkg-config.
On Linux, development packages for ALSA, Wayland and XKB are required, along
with working Vulkan or OpenGL graphics drivers.

```sh
cargo build --release -p jkr-viewer -p jkr-dedicated
```

SJK is developed and tested mainly on Windows 11. JKR's verified platform is
Linux, and SJK keeps its Linux support, but SJK's own changes are not routinely
tested there.

## Game data

A legally obtained Jedi Academy installation is required. Put `jkr-viewer`
(`jkr-viewer.exe` on Windows) in its `GameData` directory, beside the `base`
folder containing `assets0.pk3` through `assets3.pk3`.
Game data is not included in this repository.

## Client

Launch the client from that folder or a shortcut to open the main menu. No
game-data path, environment variable or particular working directory is needed.
Put `jkr-dedicated` (`jkr-dedicated.exe` on Windows) beside it too for Create game
and local `devmap` support.

Connect directly to a server:

```sh
./jkr-viewer --connect 127.0.0.1:29070
```

If keeping the binary elsewhere, use `JKR_GAME_DATA=/path/to/GameData` or the
saved game-data setting; known installation locations are also checked. The
explicit positional form `jkr-viewer /path/to/GameData --connect HOST:PORT`
remains supported. See [client launch](docs/client.md#launch) for discovery order.
Use the in-game menus for controls, graphics, audio and player settings;
Settings > GAME > Menu style switches between the modern and the classic menus.

Settings and player-created files live in `GameData/jkr/`: `config.cfg`,
`marks.txt`, favorites, friends, screenshots, demos and optional chat logs.
Existing JKR user files are imported once without overwriting files already
there; the originals are retained. If that directory cannot be written, the
client uses its per-user folder instead. The console's `path` command shows the
active location. See [configuration and content](docs/client.md#configuration-and-content).

## Dedicated server

```sh
./target/release/jkr-dedicated \
  --game-data /path/to/GameData \
  --map mp/ffa3 \
  --bind 0.0.0.0:29070 \
  --hostname "SJK server"
```

Server configuration uses familiar cvars and commands, including `+set` launch
arguments. For example, append `+set g_gametype 0 +set fraglimit 20` for a
free-for-all match. UDP port 29070 must be reachable for remote players to join.

## Source layout

All source is under `crates/`:

- `jkr-viewer`: graphical client and platform integration.
- `jkr-dedicated`: dedicated server and game integration.
- `jkr-game-jka`: shared Jedi Academy game rules and movement.
- `jkr-client`, `jkr-network`, `jkr-protocol`: client state and legacy networking.
- `jkr-bsp`, `jkr-scene`, `jkr-runtime`: maps, scene data and world state.
- `jkr-materialgen`: offline generator of local material maps from installed
  textures (see [rendering](docs/rendering.md#generating-material-maps)).
- Remaining crates provide formats, content loading, collision, navigation,
  scripting, audio, UI and the console.

Legacy formats and game-specific behavior stay in compatibility modules;
engine services use their own data structures.

## Documentation

The [project wiki](docs/README.md) is JKR's, extended with SJK's changes. It
covers architecture, development, the client, the dedicated server, rendering
and networking. Start with [current status and priorities](docs/status.md) for
verified behavior and open work. [AGENTS.md](AGENTS.md) contains shared
contributor and AI guidance, including updating relevant documentation alongside
code changes.

## License and credits

GPL-2.0-only; see [LICENSE](LICENSE). SJK is a modified version of JKR: JKR is
the work of Bishop and its contributors, and SJK's changes are by Sol (Sol-Vulpes)
and contributors, under the same license. [CREDITS.md](CREDITS.md) lists who made
what. As section 2(a) of the license asks, SJK's changes to JKR's files, with
their authors and dates, are recorded in this repository's git history. Any SJK
binaries are built from the source published here, which is their corresponding
source.

OpenJK and TaystJK are compatibility references for Jedi Academy behavior. The
bundled Inter fonts retain their
[license](crates/jkr-viewer/assets/fonts/LICENSE.txt). Other dependencies retain
their respective licenses. The code license does not grant rights to retail
game assets or third-party PK3 content.

Star Wars, Jedi Knight and Jedi Academy are trademarks of their respective
owners. SJK is a fan project and is not affiliated with or endorsed by Lucasfilm,
Disney, Raven Software or Activision.
