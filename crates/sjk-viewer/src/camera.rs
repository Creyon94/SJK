//! Multiplayer third-person view: stock framing, collision and vehicle overrides.
//! Input angles/player prediction remain untouched; only the rendered view changes.

use super::{GpuState, movement_collision};
use crate::console::ViewerConsole;
use glam::{Quat, Vec3};
use sjk_runtime::EntityId;

#[path = "camera_motion.rs"]
mod motion;
#[path = "camera_vehicle.rs"]
mod vehicle;
pub(crate) use motion::State;
pub(crate) use vehicle::Profile as VehicleProfile;

/// Keep the local actor at its predicted root with yaw-only orientation.
pub(crate) fn local_actor_root(
    first_person_view: Vec3,
    view_height: f32,
    yaw_radians: f32,
) -> ([f32; 3], [f32; 4]) {
    (
        (first_person_view - Vec3::Z * view_height).to_array(),
        Quat::from_rotation_z(yaw_radians).to_array(),
    )
}

/// `cg_cameraFPS` as the camera reads it: a float cvar, EternalJK's default 125
/// when there is no console. Below [`motion::CAMERA_MIN_FPS`] (0 included) the
/// stock per-50 ms damping applies.
fn camera_fps(console: Option<&ViewerConsole>) -> f32 {
    float_cvar(console, "cg_cameraFPS", 125.0)
}

/// A float cvar from the console, or `fallback` without one (or of another type).
fn float_cvar(console: Option<&ViewerConsole>, name: &str, fallback: f32) -> f32 {
    console
        .and_then(|c| c.float_cvar(name))
        .map_or(fallback, |v| v as f32)
}

/// `CG_OffsetThirdPersonView` and its two collision/damping stages, from OpenJK codemp.
///
/// `focus_offset` is the decaying prediction error. Stock adds it to the view
/// origin before `CG_OffsetThirdPersonView` (`CG_CalcViewValues`), so it moves the
/// focus and is traced with it rather than shifting the finished camera into a wall.
pub(crate) fn damped_third_person(
    state: &mut GpuState,
    focus_offset: Vec3,
    _delta_seconds: f32,
    presentation_time: i64,
) -> (Vec3, Vec3) {
    let cvar = |name: &str, fallback: f32| float_cvar(state.console.as_ref(), name, fallback);
    let mut horizontal = cvar("cg_thirdPersonHorzOffset", 0.0);
    let camera_damp = cvar("cg_thirdPersonCameraDamp", 0.3);
    let target_damp = cvar("cg_thirdPersonTargetDamp", 0.5);
    let camera_fps = camera_fps(state.console.as_ref());
    let fallback = [
        cvar("cg_thirdPersonAngle", 0.0),
        cvar("cg_thirdPersonPitchOffset", 0.0),
        cvar("cg_thirdPersonRange", 80.0),
        cvar("cg_thirdPersonVertOffset", 16.0),
    ];
    let mut framing = state
        .console
        .as_mut()
        .and_then(|c| c.director.camera(fallback))
        .unwrap_or(fallback);
    let snapshot = state
        .live_session
        .as_ref()
        .map(|s| s.latest_snapshot())
        .or_else(|| {
            state
                .demo_session
                .as_ref()
                .map(|s| s.snapshot_at_or_before(presentation_time as i32))
        });
    let riding = snapshot.map_or(0, |s| s.player.vehicle_entity_num());
    let vehicle = (riding != 0)
        .then(|| {
            let id = EntityId::new(u64::from(riding) + 1);
            state
                .actor_meshes
                .iter()
                .find(|m| m.entity_id == Some(id))
                .map(|m| &m.preview)
        })
        .flatten();
    let mut unrestrained = false;
    if let (Some(snapshot), Some(vehicle)) = (snapshot, vehicle) {
        if let Some(profile) = vehicle.vehicle_camera {
            let strafe = state
                .local_prediction
                .vehicle_camera_strafe()
                .unwrap_or_else(|| {
                    snapshot
                        .vehicle_player
                        .as_ref()
                        .and_then(|p| p.raw_field(91))
                        .unwrap_or(0) as i32
                });
            profile.apply(
                &mut framing,
                &mut horizontal,
                snapshot.player.view_angles()[0],
                -state.camera_pitch.to_degrees(),
                strafe,
            );
        }
        let game = state
            .live_session
            .as_ref()
            .map(|s| s.game_state())
            .or_else(|| state.demo_session.as_ref().map(|s| s.game_state()));
        unrestrained = vehicle.vehicle == Some(crate::vehicle_assets::VehicleKind::Fighter)
            && game
                .and_then(|g| g.config_string(1))
                .and_then(|info| sjk_protocol::info_value(info, b"bg_fighterAltControl"))
                .is_some_and(|v| sjk_game_jka::userinfo::atoi(v) != 0);
    }
    let held = held_camera_yaw(state, presentation_time);
    let (yaw, pitch) = if let Some(yaw) = held {
        framing[2] = crate::actor_world_submission::monster_hold::HELD_CAMERA_RANGE;
        (yaw.to_radians(), 0.0)
    } else if snapshot.is_some_and(|s| (s.player.stats[0] as i32) <= 0) {
        (
            snapshot.unwrap().player.stats[6] as i32 as f32 * std::f32::consts::PI / 180.0,
            state.camera_pitch,
        )
    } else {
        // Stock pitch offsets are down-positive, the renderer's pitch is up-positive.
        (
            state.camera_yaw + framing[0].to_radians(),
            state.camera_pitch - framing[1].to_radians(),
        )
    };
    let identity = snapshot.map_or((0, riding, 0), |s| {
        (s.player.client_num(), riding, s.player.entity_flags() & 8)
    });
    let hyperspace = snapshot
        .and_then(|s| s.vehicle_player.as_ref())
        .is_some_and(|p| {
            let start = p.vehicle_fields().hyperspace_time;
            start != 0 && presentation_time - i64::from(start) < 4000
        });
    // EternalJK times its damping by the predicted player's command time
    // (`cg.predictedPlayerState.commandTime`), the clock the focus moves on.
    // Timing it by the presentation clock instead made the focus move in one
    // frame while that clock barely advanced, and the camera stuttered.
    let eternal = camera_fps >= motion::CAMERA_MIN_FPS;
    let time = match state.local_prediction.predicted_state() {
        Some(predicted) if eternal => i64::from(predicted.command_time),
        _ => presentation_time,
    };
    let frame = motion::Frame {
        focus: state.camera_position + focus_offset,
        yaw,
        pitch,
        range: framing[2],
        vertical: framing[3],
        horizontal,
        camera_damp,
        target_damp,
        time,
        identity,
        unrestrained,
        hyperspace,
        camera_fps,
    };
    // Movers are evaluated at the same presentation time as their drawn geometry.
    // Packed player/vehicle bodies are excluded by stock MASK_CAMERACLIP.
    let game = state
        .live_session
        .as_ref()
        .map(sjk_client::ClientSession::game_state)
        .or_else(|| {
            state
                .demo_session
                .as_ref()
                .map(crate::demo_playback::Session::game_state)
        });
    let solids = game.zip(crate::first_person_view::presented_snapshot(
        state.live_session.as_ref(),
        state.demo_session.as_ref(),
        presentation_time as i32,
    ));
    state.third_person_camera.update(frame, |start, end| {
        movement_collision::camera_trace(
            &state.bsp,
            &mut state.trace_scratch,
            solids,
            presentation_time as i32,
            start,
            end,
        )
    })
}

/// Held local player's facing from the snapshot currently being presented.
fn held_camera_yaw(state: &GpuState, presentation_time: i64) -> Option<f32> {
    let snapshot = crate::first_person_view::presented_snapshot(
        state.live_session.as_ref(),
        state.demo_session.as_ref(),
        presentation_time as i32,
    )?;
    let world = state
        .demo_session
        .as_ref()
        .map_or(&state.live_world, crate::demo_playback::Session::world);
    crate::actor_world_submission::monster_hold::camera_yaw(world, snapshot, presentation_time)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(focus: Vec3, time: i64, camera_fps: f32) -> motion::Frame {
        motion::Frame {
            focus,
            yaw: 0.3,
            pitch: 0.1,
            range: 80.0,
            vertical: 16.0,
            horizontal: 0.0,
            camera_damp: 0.3,
            target_damp: 0.5,
            time,
            identity: (0, 0, 0),
            unrestrained: false,
            hyperspace: false,
            camera_fps,
        }
    }

    /// Camera position after a 50 ms step with the focus moved.
    fn second_position(camera_fps: f32) -> Vec3 {
        let mut state = State::default();
        state.update(frame(Vec3::ZERO, 1_000, camera_fps), |_, end| end);
        let moved = Vec3::new(40.0, 10.0, 0.0);
        state
            .update(frame(moved, 1_050, camera_fps), |_, end| end)
            .0
    }

    #[test]
    fn camera_fps_is_read_as_a_float_and_zero_selects_the_stock_path() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        // Registered as a float: the camera's float read sees the default.
        assert_eq!(camera_fps(Some(&console)), 125.0);
        assert_eq!(camera_fps(None), 125.0);
        assert!(camera_fps(Some(&console)) >= motion::CAMERA_MIN_FPS);

        assert!(console.set_cvar("cg_cameraFPS", "0"));
        let zero = camera_fps(Some(&console));
        assert_eq!(
            zero, 0.0,
            "an integer-registered cvar would read back as the fallback"
        );
        assert!(zero < motion::CAMERA_MIN_FPS);
        assert!(console.set_cvar("cg_cameraFPS", "60.5"));
        assert_eq!(camera_fps(Some(&console)), 60.5);

        // The value read from the console drives the damping: 0 matches the
        // stock path and differs from the default EternalJK path.
        assert!(console.set_cvar("cg_cameraFPS", "0"));
        let stock = second_position(camera_fps(Some(&console)));
        assert_eq!(stock, second_position(0.0));
        let eternal = second_position(camera_fps(None));
        assert!((eternal - stock).length() > 0.1);
    }
}
