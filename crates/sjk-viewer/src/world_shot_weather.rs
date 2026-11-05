//! World shots of a storm on `T2_Rogue` at ultra (`r_weatherForce 3`, `r_weatherQuality
//! 3`, `r_weatherFog 2`, as Sol set it there on 05/11/2026): the streets from the start,
//! their puddles mirroring the world ([`crate::weather::mirror`]) and, for comparison,
//! the same views with the puddles' mirror off (the sky alone, as before it). Ignored
//! like the other world shots. With `SJK_GPU_PHASES=1` the `gpu-phases` lines print the
//! wet pass's time (`rain-wet`, the mirror's copy included) for each half.

use super::*;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn t2_rogue_storm_puddles() {
    on_big_stack(|| {
        let cvars = [
            ("r_weatherForce", "3"),
            ("r_weatherQuality", "3"),
            ("r_weatherFog", "2"),
        ];
        let Some((mut gpu, _profile)) = open("maps/t2_rogue.bsp", [1600, 900], None, &cvars) else {
            return;
        };
        gpu.is_menu_world = false;
        // The map's start (info_player_start at 1872 -1984 -552, facing +Y), at eye height.
        let eye = [1872.0, -1984.0, -510.0];
        let views: [(f32, f32); 4] = [(90.0, -12.0), (90.0, -30.0), (0.0, -20.0), (200.0, -20.0)];
        let mut images = Vec::new();
        for mirror in [true, false] {
            gpu.weather.set_mirror_allowed(mirror);
            eprintln!("perf: puddle mirror {}", if mirror { "on" } else { "off" });
            for (index, (yaw, pitch)) in views.iter().enumerate() {
                aim(&mut gpu, eye, *yaw, *pitch);
                // The cover and the far cover survey on workers: give them time.
                let frames = if images.is_empty() { 240 } else { 96 };
                let image = frame(&mut gpu, frames);
                let name = format!(
                    "t2_rogue_storm_{index}_{}",
                    if mirror { "mirror" } else { "sky" }
                );
                image
                    .save(directory().join(format!("{name}.png")))
                    .expect("write the shot");
                images.push(image);
            }
        }
        let sheet = sheet(&images, 4, 640, "t2_rogue_storm_puddles");
        println!("{}", sheet.display());
    });
}
