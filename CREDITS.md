# Credits

Sol JK (SJK) is a modified version of [JKR](https://github.com/Bishop-R/JKR).
This page records who made what. The git history is the authoritative record:
every commit carries its author, and SJK's own changes are kept as separate
topic branches merged into SJK's `main`.

## JKR, by Bishop

[Bishop (Bishop-R)](https://github.com/Bishop-R) created JKR, has developed it
continuously since, and wrote its initial source and most of its code: the Rust engine,
the wgpu renderer and its lighting, the native client and dedicated server,
protocol 26 networking, prediction and the shared Jedi Academy game rules,
content loading (PK3, BSP, models, shaders, sounds), the console, menus, HUD,
server browser, demos and the project wiki, over weeks of work before anyone
else joined. Sol had meanwhile set out to write a Jedi Academy client from
scratch; on learning of JKR the two talked, Sol began contributing, and Bishop
made JKR open source in October 2026.

Bishop's later pull requests to JKR include:

- renderer frame cost, frame submission and deferred lighting (#50, #79), fixture
  lighting and sky/terrain stability (#81), volumetric fringes (#84), the default
  visual profile (#85), sun-shadow banding (#4) and billboard orientation (#51);
- playable worlds through joins and map changes (#82), intermission (#83, which
  also keeps server prints out of chat in place of Sol's #58) and faster map
  loading (#89);
- console browsing, editing and colour-code input (#88, replacing Sol's #6 and
  #33, with Sol as co-author); manual values for every menu slider (#94,
  replacing Sol's #32);
- compact chat player actions (#92) and upright player status icons (#96),
  together replacing Sol's talk balloons and connection icons (#30);
- footsteps and authored animation sound cues (#90, replacing Sol's #41), legacy
  server pak references (#91), the leader display (#93), client devmap and server
  noclip (#95, replacing Sol's #31);
- drop-in installs and portable client storage (#103), distributable ZIPs (#104)
  and actor animation isolation (#112).

## Sol's contributions to JKR

Sol ([Sol-Vulpes](https://github.com/Sol-Vulpes)) has contributed to JKR since
October 2026: by early October 2026 Sol had opened 100 of
JKR's first 121 issues and pull requests (all 39 issues and 61 of the 82 pull
requests). Merged into JKR:

- #1 branch, commit and pull request rules; #5 the CI workflow
- #2 an 8 MiB main-thread stack for the Windows client
- #3 shared console prefix completion
- #29 the stun baton's fire sound; #42 short MP3 sounds ending in an ID3v1 tag;
  #43 looping sounds kept playing
- #40 the Force lightning and drain effect kept level
- #70 CRLF WGSL shader sources
- #97 old 72-bone humanoid models remapped as rd-vanilla does
- #100 chat kept in the console, one row per print line
- #101 text edits kept on their setting, no slider float noise
- #102 the server's cheat commands documented
- #105 `^8` orange and `^9` grey

Open pull requests to JKR, all included in SJK, cover the classic menu style,
classic scoreboard and game-data HUDs, retail game fonts, text spacing and
high-resolution scaling, JA+ support (plugin identity, `serverconfig`,
`g_debugMelee`, JA+ movement and saber rules, grapple, duel pass-through),
reliable joining, old models, material maps, FPS cap behaviour, key handling
(layout key names, dead keys, locked binds, capital key names), the renderer
settings page, sharp levelshots, saber trails, the third-person camera, Force
Speed afterimages, death animations and dust motes. See
[Sol's pull requests](https://github.com/Bishop-R/JKR/pulls?q=is%3Apr+author%3ASol-Vulpes).

## SJK only

Changes that stay in SJK, by Sol:

- the Sol JK name, README, credits and website;
- SJK's slider entry habits (type to open, Space steps, clicking away applies);
- the classic menu style as the default;
- the `debug_panel` console command, an in-game checklist of the changes in a
  build and how to test them.

## Tools

Many of Sol's changes, in JKR and SJK, were written with Claude (Anthropic) as a
coding assistant; such commits carry a `Co-Authored-By: Claude` trailer.

## References and third-party material

OpenJK (`codemp`), EternalJK, JoF EJK and TaystJK/jaPRO are compatibility
references for Jedi Academy behaviour; JKR and SJK reimplement that behaviour and
do not include their code. The bundled Inter fonts keep their
[license](crates/sjk-viewer/assets/fonts/LICENSE.txt), and other dependencies keep
their respective licenses.

Star Wars, Jedi Knight and Jedi Academy are trademarks of their respective
owners. SJK is a fan project and is not affiliated with or endorsed by Lucasfilm,
Disney, Raven Software or Activision. No retail game assets are included.
