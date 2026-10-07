//! World shots of the player's own world notes (`world_notes.rs`): each note in
//! `<GameData>/SJK/notes.jsonl` seen again from where it was written, to check a
//! reported surface without starting the game. Ignored like the other world shots.
//!
//! `SJK_NOTES` picks notes by their line number (from 0) or by part of a map name
//! (`SJK_NOTES=12,13` or `SJK_NOTES=ffa1`); without it every note is shot. Each
//! note gives one sheet of four views: as written, then with a red point light (a
//! saber's) a little in front of the noted point; and the same two from halfway
//! closer, so a surface that dynamic light does not reach stands out. The player's
//! own renderer settings (`r_*` in `<GameData>/SJK/config.cfg`) apply, and
//! `SJK_NOTES_CVARS` (`name=value,name=value`) changes some for a comparison;
//! `SJK_NOTES_REMAP` (`old=new;old=new`) remaps shaders as a server would.

use super::*;
use crate::dynamic_lights::PointLight;

/// One note's view: where it was written from and what it hit.
struct NoteView {
    line: usize,
    map: String,
    eye: [f32; 3],
    yaw: f32,
    /// Degrees down, as the note's pose writes it.
    pitch_down: f32,
    hit: [f32; 3],
    normal: [f32; 3],
    shader: String,
}

fn read_notes(game_data: &std::path::Path) -> Vec<NoteView> {
    let path = game_data.join("SJK").join("notes.jsonl");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("no notes at {}", path.display());
        return Vec::new();
    };
    let vector = |value: &serde_json::Value| -> Option<[f32; 3]> {
        let items = value.as_array()?;
        Some([
            items.first()?.as_f64()? as f32,
            items.get(1)?.as_f64()? as f32,
            items.get(2)?.as_f64()? as f32,
        ])
    };
    let mut notes = Vec::new();
    for (line, row) in text.lines().enumerate() {
        let Ok(note) = serde_json::from_str::<serde_json::Value>(row) else {
            continue;
        };
        // "(x y z) : yaw pitch"
        let pose = note["pose"].as_str().unwrap_or_default();
        let numbers: Vec<f32> = pose
            .split(|c: char| c == '(' || c == ')' || c == ':' || c.is_whitespace())
            .filter_map(|part| part.parse().ok())
            .collect();
        let (Some(hit), Some(normal), [x, y, z, yaw, pitch_down]) = (
            vector(&note["hit"]),
            vector(&note["normal"]),
            numbers.as_slice(),
        ) else {
            continue;
        };
        notes.push(NoteView {
            line,
            map: note["map"].as_str().unwrap_or_default().to_owned(),
            eye: [*x, *y, *z],
            yaw: *yaw,
            pitch_down: *pitch_down,
            hit,
            normal,
            shader: note["surface"]["shader"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        });
    }
    notes
}

/// The player's renderer settings (`seta r_* "value"` in `<GameData>/SJK/config.cfg`),
/// so the shots are lit as the notes were seen.
fn renderer_settings(game_data: &std::path::Path) -> Vec<(String, String)> {
    let text =
        std::fs::read_to_string(game_data.join("SJK").join("config.cfg")).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            if words.next()? != "seta" {
                return None;
            }
            let name = words.next()?;
            let value = words.next()?.trim_matches('"');
            // Window and display settings stay the shot's own.
            let window = [
                "r_mode",
                "r_fullscreen",
                "r_custom",
                "r_resolution",
                "r_superSample",
            ]
            .iter()
            .any(|prefix| name.starts_with(prefix));
            (name.starts_with("r_") && !window).then(|| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

fn wanted(note: &NoteView, filter: &str) -> bool {
    filter.is_empty()
        || filter.split(',').any(|part| {
            let part = part.trim();
            part.parse::<usize>().map_or_else(
                |_| {
                    note.map
                        .to_ascii_lowercase()
                        .contains(&part.to_ascii_lowercase())
                },
                |line| line == note.line,
            )
        })
}

/// Every chosen note's four views, one sheet per note in `target/world-shots`.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn world_notes() {
    on_big_stack(|| {
        let game_data = PathBuf::from(
            std::env::var_os("JKA_GAME_DATA").expect("JKA_GAME_DATA names the GameData directory"),
        );
        let filter = std::env::var("SJK_NOTES").unwrap_or_default();
        let notes: Vec<NoteView> = read_notes(&game_data)
            .into_iter()
            .filter(|note| wanted(note, &filter))
            .collect();
        let mut settings = renderer_settings(&game_data);
        // `SJK_NOTES_CVARS=r_weather=1,r_weatherFog=2` overrides settings for a comparison.
        for pair in std::env::var("SJK_NOTES_CVARS")
            .unwrap_or_default()
            .split(',')
        {
            if let Some((name, value)) = pair.split_once('=') {
                settings.retain(|(known, _)| !known.eq_ignore_ascii_case(name));
                settings.push((name.to_owned(), value.to_owned()));
            }
        }
        let cvars: Vec<(&str, &str)> = settings
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        let mut maps: Vec<&str> = notes.iter().map(|note| note.map.as_str()).collect();
        maps.dedup();
        for map in maps {
            let Some((mut gpu, _profile)) = open(map, [1280, 720], None, &cvars) else {
                return;
            };
            // `SJK_NOTES_REMAP=old=new;old=new`: shader remaps as a server sends them,
            // made as the console's `remapShader` makes them.
            let remaps = std::env::var("SJK_NOTES_REMAP").unwrap_or_default();
            for pair in remaps.split(';').filter(|pair| !pair.is_empty()) {
                let Some((old, new)) = pair.split_once('=') else {
                    continue;
                };
                // The frame's own remap refresh applies it, as after the console command.
                let vfs = gpu.vfs.clone().expect("a map's files");
                if let Err(error) = gpu
                    .world_materials
                    .local_remap(&vfs, &gpu.shaders, old, new)
                {
                    eprintln!("remap {old} -> {new}: {error}");
                }
            }
            let _ = frame(&mut gpu, 30);
            for note in notes.iter().filter(|note| note.map == map) {
                let mut images = Vec::new();
                aim(&mut gpu, note.eye, note.yaw, -note.pitch_down);
                gpu.effect_aux.held_lights.clear();
                images.push(frame(&mut gpu, 8));
                // A saber's light (codemp `CG_AddSaberBlade`: red, twice the blade's
                // length) 48 units off the noted point.
                let origin: [f32; 3] =
                    std::array::from_fn(|axis| note.hit[axis] + note.normal[axis] * 48.0);
                gpu.effect_aux.held_lights.push(PointLight {
                    origin,
                    radius: 200.0,
                    color: [1.0, 0.2, 0.2],
                });
                images.push(frame(&mut gpu, 8));
                // Closer: halfway from the eye to the point.
                let near: [f32; 3] =
                    std::array::from_fn(|axis| (note.eye[axis] + note.hit[axis]) * 0.5);
                aim(&mut gpu, near, note.yaw, -note.pitch_down);
                images.push(frame(&mut gpu, 8));
                gpu.effect_aux.held_lights.clear();
                images.push(frame(&mut gpu, 8));
                let name = format!("note-{:02}", note.line);
                println!(
                    "{}: {} {} {}",
                    note.line,
                    map,
                    note.shader,
                    sheet(&images, 2, 960, &name).display()
                );
            }
        }
    });
}
