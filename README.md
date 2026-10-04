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
  regularly merges JKR's main branch, so JKR's work reaches SJK quickly.
- **What SJK adds.** Sol's changes are written as separate topic branches. Those
  that fit JKR are proposed upstream; once JKR accepts one, it simply becomes part
  of both. At the time of writing, SJK's additions include:
  - a classic menu style after the retail menus: main, profile, setup and
    controls pages, retail connect and loading screens, and no map behind the
    menu;
  - the optional retail game fonts, kept sharp at high resolutions;
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
  `jkr-dedicated`, `jkr-*`), and the client keeps JKR's configuration folder.
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

A legally obtained Jedi Academy installation is required. Point the client at its
`GameData` directory, containing `base/assets0.pk3` and the other retail PK3s.
Game data is not included in this repository.

## Client

```sh
./target/release/jkr-viewer /path/to/GameData
```

Connect directly to a server:

```sh
./target/release/jkr-viewer /path/to/GameData --connect 127.0.0.1:29070
```

The client also supports launch without arguments when it can locate an
installation or has a saved game-data path. Use the in-game menus for controls,
graphics, audio and player settings; Settings > GAME > Menu style switches
between the modern and the classic menus.

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
and contributors, under the same license.

OpenJK and TaystJK are compatibility references for Jedi Academy behavior. The
bundled Inter fonts retain their
[license](crates/jkr-viewer/assets/fonts/LICENSE.txt). Other dependencies retain
their respective licenses. The code license does not grant rights to retail
game assets or third-party PK3 content.

Star Wars, Jedi Knight and Jedi Academy are trademarks of their respective
owners. SJK is a fan project and is not affiliated with or endorsed by Lucasfilm,
Disney, Raven Software or Activision.
