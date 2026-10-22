//! Wall-normal facing for the local display copy, independent of server free look.
//!
//! OpenJK `codemp/game/bg_pmove.c:1662-1671` faces the grabbed wall using its
//! normal plus the rebound side's offset. JA+ may keep the approach yaw in the
//! simulation; the camera still needs the same wall-facing presentation as the
//! model. Use the shared reference calculation against current world/mover solids.

use sjk_client::pmove::{MovementCollision, MovementState, wall_hold_yaw};

/// `PMF_STUCK_TO_WALL` (`codemp/game/bg_public.h`).
const PMF_STUCK_TO_WALL: u16 = 16_384;

/// Align only the local display copy. Replay angles, deltas and velocity remain
/// owned by the predictor; a released wall or a missed wall check adds no hold.
pub(super) fn align(view: &mut MovementState, collision: &impl MovementCollision) {
    if view.movement_flags & PMF_STUCK_TO_WALL == 0 {
        return;
    }
    if let Some(yaw) = wall_hold_yaw(view.legs_anim, view.view_angles[1], view.origin, collision) {
        view.view_angles[1] = yaw;
        view.wall_view_locked = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_client::pmove::MovementTrace;

    /// An upright plane ahead, or a miss. Its hit normal is the reference input;
    /// no map/model data is embedded in this fixture.
    struct Wall(Option<[f32; 3]>);

    impl MovementCollision for Wall {
        fn trace(
            &self,
            _start: [f32; 3],
            _minimums: [f32; 3],
            _maximums: [f32; 3],
            end: [f32; 3],
            _content_mask: u32,
        ) -> MovementTrace {
            self.0.map_or_else(
                || MovementTrace::miss(end),
                |normal| MovementTrace {
                    fraction: 0.25,
                    plane_normal: normal,
                    ..MovementTrace::miss(end)
                },
            )
        }
    }

    #[test]
    fn oblique_grabs_present_the_reference_wall_yaw_for_every_side() {
        for step in [8, 7, 4, 3] {
            // codemp's forward, left, backward and right rebound/hold animations.
            for (rebound, hold, offset) in [
                (875, 879, 180.0),
                (876, 880, 90.0),
                (877, 881, 0.0),
                (878, 882, -90.0),
            ] {
                for animation in [rebound, hold] {
                    for (normal, normal_yaw) in [
                        ([-1.0, 0.0, 0.0], 180.0_f32),
                        ([-0.6, -0.8, 0.0], 233.130_1),
                    ] {
                        let expected = (normal_yaw + offset).rem_euclid(360.0);
                        let raw = MovementState {
                            command_time: step,
                            health: 100,
                            legs_anim: animation,
                            movement_flags: PMF_STUCK_TO_WALL,
                            view_angles: [12.0, expected + 30.0, 5.0],
                            delta_angles: [100, 200, 300],
                            velocity: [10.0, -20.0, 30.0],
                            ..Default::default()
                        };
                        let mut view = raw.clone();
                        align(&mut view, &Wall(Some(normal)));
                        let difference =
                            |yaw: f32| (yaw - expected + 180.0).rem_euclid(360.0) - 180.0;
                        assert!(difference(view.view_angles[1]).abs() < 0.001);
                        assert!(view.wall_view_locked);
                        assert_eq!((view.view_angles[0], view.view_angles[2]), (12.0, 5.0));
                        assert_eq!(view.delta_angles, raw.delta_angles);
                        assert_eq!(view.velocity, raw.velocity);
                        assert_eq!(raw.view_angles[1], expected + 30.0);
                        let mut authority = crate::input::view_authority::Authority::default();
                        let (mut pitch, mut yaw) = (0.0, 0.0);
                        authority.observe_prediction(&view, &mut pitch, &mut yaw);
                        assert!(difference(yaw.to_degrees()).abs() < 0.001);
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "release measurement; requires external retail assets in JKA_GAME_DATA"]
    fn release_wall_grab_view_cost() {
        use crate::movement_collision::BspMovementCollision;
        use glam::Vec3;
        use std::{hint::black_box, path::PathBuf, time::Instant};

        assert!(
            black_box(!cfg!(debug_assertions)),
            "run this measurement with --release"
        );
        let root = PathBuf::from(std::env::var_os("JKA_GAME_DATA").expect("JKA_GAME_DATA"));
        let mut files = sjk_vfs::VirtualFileSystem::new();
        for number in 0..4 {
            files
                .mount_pk3(root.join(format!("base/assets{number}.pk3")))
                .expect("retail archive");
        }
        let map = files
            .read("maps/mp/duel6.bsp")
            .unwrap()
            .expect("retail duel6");
        let bsp = sjk_bsp::Bsp::parse(&map.bytes).expect("BSP");
        let entities = sjk_entity::parse_entity_lump(bsp.entities()).expect("entities");
        let mut scratch = sjk_bsp::TraceScratch::default();
        let collision = BspMovementCollision::with_movers(&bsp, &mut scratch, &[]);
        let mut sample = None;
        'spawns: for entity in entities.iter().filter(|entity| {
            entity
                .classname()
                .is_some_and(|name| name.starts_with("info_player"))
        }) {
            let Some(origin) = entity.vector("origin").unwrap() else {
                continue;
            };
            for yaw in [0.0_f32, 90.0, 180.0, 270.0] {
                let angle = yaw.to_radians();
                let end = (Vec3::from_array(origin)
                    + Vec3::new(angle.cos(), angle.sin(), 0.0) * 4096.0)
                    .to_array();
                let hit = collision.trace(origin, [-15.0, -15.0, 0.0], [15.0, 15.0, 24.0], end, 1);
                if hit.fraction >= 1.0 || hit.start_solid || hit.plane_normal[2].abs() > 0.2 {
                    continue;
                }
                let normal = Vec3::from_array(hit.plane_normal);
                let raw = MovementState {
                    origin: (Vec3::from_array(hit.end_position) + normal).to_array(),
                    legs_anim: 875,
                    movement_flags: PMF_STUCK_TO_WALL,
                    view_angles: [12.0, (-normal.y).atan2(-normal.x).to_degrees() + 30.0, 0.0],
                    ..Default::default()
                };
                let mut view = raw.clone();
                align(&mut view, &collision);
                if view.wall_view_locked {
                    sample = Some(raw);
                    break 'spawns;
                }
            }
        }
        let raw = sample.expect("a reachable upright duel6 wall");
        for _ in 0..1000 {
            align(&mut raw.clone(), &collision);
        }
        let started = Instant::now();
        for _ in 0..10_000 {
            let mut view = raw.clone();
            align(black_box(&mut view), black_box(&collision));
            black_box(view.view_angles);
        }
        println!(
            "wall-grab display alignment: {:.3} us/call over 10000 warmed duel6 BSP traces; no movers or GPU",
            started.elapsed().as_secs_f64() * 1_000_000.0 / 10_000.0
        );
    }

    #[test]
    fn release_missing_walls_and_slopes_do_not_force_facing() {
        for (flags, wall) in [
            (0, Wall(Some([-1.0, 0.0, 0.0]))),
            (PMF_STUCK_TO_WALL, Wall(None)),
            (PMF_STUCK_TO_WALL, Wall(Some([-0.9, 0.0, 0.436]))),
        ] {
            let mut view = MovementState {
                legs_anim: 875,
                movement_flags: flags,
                view_angles: [12.0, 30.0, 0.0],
                ..Default::default()
            };
            align(&mut view, &wall);
            assert_eq!(view.view_angles, [12.0, 30.0, 0.0]);
            assert!(!view.wall_view_locked);
        }
    }
}
