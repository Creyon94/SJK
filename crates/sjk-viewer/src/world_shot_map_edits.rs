//! World shots of map edits ([`crate::map_edits`]): duel6's prop nearest its first spawn
//! point seen without edits, then with that prop hidden, the world surface ahead of the
//! camera hidden, a copy of the prop placed beside it and the floor under it remapped;
//! then the edits taken back. Ignored like the other world shots.

use super::*;
use crate::map_edits::document::{Document, Kind};
use crate::map_edits::panel::Tab;

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_prop_hidden_copied_and_floor_remapped() {
    on_big_stack(|| {
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1280, 720], None, &[]) else {
            return;
        };
        let (spawn, _) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
        let spawn = Vec3::from_array(spawn);
        let (entity, prop) = crate::static_models::spawns(&gpu.bsp)
            .into_iter()
            .min_by(|(_, a), (_, b)| {
                let distance =
                    |s: &crate::static_models::Spawn| Vec3::from_array(s.origin).distance(spawn);
                distance(a).total_cmp(&distance(b))
            })
            .expect("duel6 places props");
        let origin = Vec3::from_array(prop.origin);
        let camera = origin + Vec3::new(-220.0, -160.0, 120.0);
        let (yaw, pitch) = look(camera.to_array(), (origin + Vec3::Z * 24.0).to_array());
        aim(&mut gpu, camera.to_array(), yaw, pitch);
        let before = frame(&mut gpu, 40);

        // The surface under the prop and the one the camera looks at.
        let down = gpu
            .decal_surfaces
            .ray_hit(origin + Vec3::Z * 8.0, -Vec3::Z, 512.0)
            .expect("a floor under the prop");
        let forward = (origin - camera).normalize();
        let ahead = gpu
            .decal_surfaces
            .ray_hit(camera, forward, 4096.0)
            .expect("a surface ahead");
        let shader_of = |gpu: &GpuState, surface: usize| {
            let render = gpu.bsp.render();
            let index = render.surfaces()[surface].shader;
            gpu.bsp.shaders()[index].name_lossy().into_owned()
        };
        let floor = shader_of(&gpu, down.surface);
        let ahead_shader = shader_of(&gpu, ahead.surface);
        let mut document = Document::new("mp/duel6", 1);
        document.add(
            Kind::HideProp {
                entity,
                model: prop.model.clone(),
            },
            None,
        );
        if ahead.surface != down.surface {
            document.add(
                Kind::HideSurface {
                    surface: ahead.surface as u32,
                    shader: ahead_shader.clone(),
                },
                None,
            );
        }
        document.add(
            Kind::Place {
                model: prop.model.clone(),
                origin: (origin + Vec3::new(0.0, 96.0, 0.0)).to_array(),
                angles: [0.0, prop.angles[1] + 90.0, 0.0],
                scale: prop.scale,
            },
            None,
        );
        // Another of the map's own shaders, so it is surely in the game data.
        let target = gpu
            .bsp
            .shaders()
            .iter()
            .map(|shader| shader.name_lossy().into_owned())
            .find(|name| {
                name != &floor
                    && name.starts_with("textures/")
                    && !["sky", "nodraw", "caulk", "clip", "trigger", "system"]
                        .iter()
                        .any(|word| name.contains(word))
            })
            .expect("another shader");
        document.add(
            Kind::Remap {
                from: floor.clone(),
                to: target.clone(),
            },
            None,
        );
        gpu.shot_map_edits(Some(document));
        assert!(
            gpu.decal_surfaces
                .ray_hit(camera, forward, 4096.0)
                .is_none_or(|hit| hit.surface != ahead.surface || ahead.surface == down.surface),
            "a hidden surface is not picked"
        );
        let edited = frame(&mut gpu, 40);
        gpu.shot_map_edits(None);
        let after = frame(&mut gpu, 40);
        let changed = |a: &image::RgbaImage, b: &image::RgbaImage| {
            a.pixels()
                .zip(b.pixels())
                .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 24))
                .count()
        };
        println!(
            "prop {entity} {} at {origin:?}; hid surface {} ({ahead_shader}); floor {floor} -> {target}",
            prop.model, ahead.surface
        );
        println!(
            "pixels changed: edited {}, restored {}",
            changed(&before, &edited),
            changed(&before, &after)
        );
        println!(
            "{}",
            sheet(&[before, edited, after], 3, 960, "map-edits-duel6").display()
        );
    });
}

/// The map editor's panel over duel6: a prop selected under the crosshair, the Place page
/// searching for trees, the Edits page with a few edits, a surface clicked on the scene,
/// and a 4:3 window.
#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_map_editor_panel() {
    on_big_stack(|| {
        for (size, prefix) in [
            ([1920, 1080], "map-editor"),
            ([1440, 1080], "map-editor-4x3"),
        ] {
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &[]) else {
                return;
            };
            let (spawn, _) = assets::initial_camera(&gpu.bsp).expect("a spawn point");
            let spawn = Vec3::from_array(spawn);
            let (entity, prop) = crate::static_models::spawns(&gpu.bsp)
                .into_iter()
                .min_by(|(_, a), (_, b)| {
                    let distance = |s: &crate::static_models::Spawn| {
                        Vec3::from_array(s.origin).distance(spawn)
                    };
                    distance(a).total_cmp(&distance(b))
                })
                .expect("duel6 places props");
            let origin = Vec3::from_array(prop.origin);
            let camera = origin + Vec3::new(-220.0, -160.0, 120.0);
            let (yaw, pitch) = look(camera.to_array(), (origin + Vec3::Z * 24.0).to_array());
            aim(&mut gpu, camera.to_array(), yaw, pitch);
            if let Some(console) = gpu.console.as_mut() {
                console.close_for_connection();
            }
            let _ = frame(&mut gpu, 40);
            let mut document = Document::new("mp/duel6", 1);
            document.add(
                Kind::Place {
                    model: prop.model.clone(),
                    origin: (origin + Vec3::new(0.0, 96.0, 0.0)).to_array(),
                    angles: [0.0, 90.0, 0.0],
                    scale: [1.0; 3],
                },
                None,
            );
            document.add(
                Kind::HideSurface {
                    surface: 12,
                    shader: "textures/yavin/floor".to_owned(),
                },
                Some("192.0.2.1:29070".to_owned()),
            );
            document.add(
                Kind::Remap {
                    from: "textures/yavin/wall".to_owned(),
                    to: "textures/yavin/wall2".to_owned(),
                },
                None,
            );
            document.unpublished = true;
            gpu.shot_map_edits(Some(document));
            let _ = frame(&mut gpu, 10);
            gpu.shot_map_edit_panel(Tab::Selected, None);
            println!("prop {entity} {}", prop.model);
            println!(
                "{}",
                shoot(&mut gpu, 8, &format!("{prefix}-selected")).display()
            );
            if prefix != "map-editor" {
                continue;
            }
            gpu.shot_map_edit_panel(Tab::Place, None);
            gpu.shot_map_edit_search("yavin");
            println!("{}", shoot(&mut gpu, 4, "map-editor-place").display());
            gpu.shot_map_edit_panel(Tab::Edits, None);
            println!("{}", shoot(&mut gpu, 4, "map-editor-edits").display());
            // A click low on the left of the scene: the floor.
            gpu.shot_map_edit_panel(Tab::Selected, Some([300.0, 900.0]));
            println!("{}", shoot(&mut gpu, 4, "map-editor-surface").display());
        }
    });
}
