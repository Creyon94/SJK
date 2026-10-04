# Screenshots

The site's gallery shows the images listed in `manifest.json`, in order. It stays
hidden while the list is empty.

To add screenshots:

1. Take them in SJK with the `screenshotJPEG` console command (or bind it to a
   key, for example `bind F12 screenshotJPEG`); they are saved in
   `GameData/SJK/screenshots/`.
2. Copy the ones you want here, ideally as `.jpg` at 1920 px wide or less to keep
   the page fast (for example `classic-menu.jpg`).
3. List each one in `manifest.json` with a short caption:

```json
[
  { "file": "classic-menu.jpg", "caption": "The classic main menu" },
  { "file": "scoreboard.jpg", "caption": "Classic scoreboard with client IDs" }
]
```

4. Commit on `main`; the Pages workflow publishes `site/` automatically.

Only use your own SJK screenshots. Images of the game are fine to show, but do
not add extracted game files (textures, logos, fonts) to the site.
