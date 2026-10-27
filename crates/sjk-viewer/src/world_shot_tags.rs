//! World shots of tags ([`crate::tags`]) on `mp/duel6`: from where the client opens, the
//! crosshair's trace finds walls round about and the floor below, a tag goes on each as
//! `toy_tag` sticks one (upright on a wall, facing the player on the floor), and each is
//! seen from in front. The pictures come from the hub's `sjk_tags` pack in the folder
//! `SJK_TEST_TAGS` names (the hub's `assets/dist`), else the decals draw their missing
//! picture. Ignored like the other world shots.

use super::*;
use glam::Vec3;

/// `CONTENTS_SOLID`.
const SOLID: u32 = 1;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_tags_on_walls_and_floor() {
    on_big_stack(|| {
        // The hub's packs, mounted below the game data as a client's cache is.
        let cache = tempfile::tempdir().unwrap();
        if let Some(dist) = std::env::var_os("SJK_TEST_TAGS") {
            let assets = crate::sjk_packs::directory_in(cache.path());
            std::fs::create_dir_all(&assets).unwrap();
            std::fs::copy(
                std::path::Path::new(&dist).join("sjk_tags.pk3"),
                assets.join("sjk_tags.pk3"),
            )
            .unwrap();
            crate::sjk_packs::mount_at_start(cache.path());
        }
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1280, 720], None, &[]) else {
            return;
        };
        gpu.is_menu_world = false;
        let eye = gpu.camera_position;
        let trace = |gpu: &mut GpuState, to: Vec3| {
            let result = gpu.bsp.trace_box_with(
                &mut gpu.trace_scratch,
                eye.to_array(),
                to.to_array(),
                sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).unwrap(),
                SOLID,
            );
            (result.fraction < 1.0 && !result.start_solid)
                .then(|| {
                    result.plane.map(|plane| {
                        (
                            Vec3::from_array(result.end_position),
                            Vec3::from_array(plane.normal),
                        )
                    })
                })
                .flatten()
        };
        // Walls the eye sees within reach of a few steps, and the floor under it.
        let mut spots = Vec::new();
        for step in 0..8 {
            let yaw = (step as f32 * 45.0).to_radians();
            let forward = Vec3::new(yaw.cos(), yaw.sin(), 0.0);
            if let Some((at, normal)) = trace(&mut gpu, eye + forward * 700.0)
                && normal.z.abs() < 0.7
            {
                spots.push((at, normal, forward));
            }
        }
        spots.truncate(3);
        let down = Vec3::new(1.0, 0.0, -1.5).normalize();
        if let Some((at, normal)) = trace(&mut gpu, eye + down * 600.0) {
            spots.push((at, normal, down));
        }
        assert!(!spots.is_empty(), "the eye sees a wall or the floor");
        let mut images = Vec::new();
        for (index, (at, normal, forward)) in spots.iter().enumerate() {
            let tag = sjk_identity::PlacedTag {
                id: crate::tags::CATALOGUE[index * 4 % crate::tags::CATALOGUE.len()]
                    .id
                    .to_owned(),
                map: "mp/duel6".to_owned(),
                at: at.to_array(),
                normal: normal.to_array(),
                spin: crate::tags::spin_for(*normal, crate::tags::up_for(*normal, *forward)),
            };
            gpu.effect_aux.decals.set_tag(index, Some(&tag));
            // Seen from in front, a little back from where it was stuck.
            let back = *at - *forward * 110.0 + Vec3::Z * 10.0;
            let look = (*at - back).normalize();
            let yaw = look.y.atan2(look.x).to_degrees();
            let pitch = look.z.asin().to_degrees();
            aim(&mut gpu, back.to_array(), yaw, pitch);
            images.push(frame(&mut gpu, 6));
            println!(
                "tag {} at {at} facing {normal}: {} fragments",
                tag.id,
                gpu.effect_aux.decals.tag_polys()
            );
        }
        assert!(
            gpu.effect_aux.decals.tag_polys() > 0,
            "the tags were projected"
        );
        println!("{}", sheet(&images, 2, 960, "duel6-tags").display());
    });
}
