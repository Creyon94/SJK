//! Illuminate, SJK's own Force-wheel entry ([`sjk_client::force_wheel::ILLUMINATE`]):
//! a holocron that floats by the local player's left shoulder, turning slowly, and
//! lights the way with a warm point light. It is this client's alone: no server
//! knows of it and no other player sees it. `+useforce` on its wheel entry, or the
//! `force_illuminate` command, turns it on and off; `cg_illuminate 0` takes it off
//! the wheel and puts it out. In first person only its light shows (the cube is
//! drawn for mirrors, like the body).
//!
//! Its model, pictures and shader are bundled ([`FILES`], made by
//! `scripts/holocron_assets.py`) and mounted below all game data, so a PK3 with
//! the same paths replaces them.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use crate::dynamic_lights::PointLight;
use glam::{Quat, Vec3};
use sjk_protocol::PlayerState;
use sjk_vfs::{VfsError, VirtualFileSystem};
use std::time::Instant;

/// Archived; 1 (default) puts Illuminate on the Force wheel.
pub(crate) const CVAR: &str = "cg_illuminate";
pub(crate) const MODEL: &str = "models/sjk/holocron.md3";
/// The wheel's picture (`gfx/sjk/force_illuminate.png`).
pub(crate) const ICON: &str = "gfx/sjk/force_illuminate";

/// The bundled files at their game paths.
const FILES: [(&str, &[u8]); 5] = [
    (MODEL, include_bytes!("../assets/holocron/holocron.md3")),
    (
        "models/sjk/holocron.jpg",
        include_bytes!("../assets/holocron/holocron.jpg"),
    ),
    (
        "models/sjk/holocron_glow.jpg",
        include_bytes!("../assets/holocron/holocron_glow.jpg"),
    ),
    (
        "gfx/sjk/force_illuminate.png",
        include_bytes!("../assets/holocron/force_illuminate.png"),
    ),
    (
        "shaders/sjk_holocron.shader",
        include_bytes!("../assets/holocron/holocron.shader"),
    ),
];

/// Where the holocron floats, from the eye in the view's yaw frame (x forward,
/// y left): a little behind and to the left, just under eye height.
const OFFSET: Vec3 = Vec3::new(-6.0, 18.0, -4.0);
/// How far it trails behind a moving player at most, and the jump past which it
/// is placed at once (a teleport, a respawn).
const MAX_LAG: f32 = 20.0;
const SNAP: f32 = 96.0;
/// Rate of its follow, per second.
const FOLLOW: f32 = 12.0;
/// Seconds to appear or go out.
const FADE: f32 = 0.3;
/// Its bob, up and down, and the bob's period in seconds.
const BOB: f32 = 1.0;
const BOB_PERIOD: f32 = 2.8;
/// Its turn about the vertical, radians per second, and its tilt, so that the
/// top shows as it turns.
const SPIN: f32 = 0.6;
const TILT: [f32; 2] = [0.38, 0.28];
/// The light: reach in units and a warm white.
const RADIUS: f32 = 300.0;
const COLOR: [f32; 3] = [1.5, 1.3, 1.0];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), VfsError> {
    vfs.mount_memory("SJK holocron", FILES.iter().copied())?;
    Ok(())
}

/// Whether the holocron can show for `player`: alive, playing, not watching
/// someone else and not at the intermission.
fn playing(player: &PlayerState) -> bool {
    // `pmtype_t`: PM_SPECTATOR 4, PM_DEAD 5, PM_INTERMISSION 7, PM_SPINTERMISSION 8.
    player.health() > 0
        && !matches!(player.movement_type(), 4 | 5 | 7 | 8)
        && player.movement_flags() & 4096 == 0
}

/// Where and how the holocron is drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pose {
    pub(crate) position: Vec3,
    pub(crate) rotation: Quat,
    /// 0 out, 1 fully there: its size and its light's reach.
    pub(crate) level: f32,
}

/// The holocron's state: on or off, and where it floats.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Holocron {
    on: bool,
    level: f32,
    /// Its centre without the bob, following the player.
    position: Option<Vec3>,
    last: Option<Instant>,
    /// An eye and yaw to float by without a session, for the world shots.
    #[cfg(test)]
    pub(crate) shot_anchor: Option<(Vec3, f32)>,
}

impl Holocron {
    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// Where it floats from `eye` facing `yaw` (radians), without its bob.
    pub(crate) fn place(eye: Vec3, yaw: f32) -> Vec3 {
        eye + Quat::from_rotation_z(yaw) * OFFSET
    }

    /// Step to `now`. `anchor` is the eye and the view's yaw in radians while the
    /// holocron may show, `None` otherwise (it then goes out where it is);
    /// `seconds` is the presentation time, for its bob and turn.
    pub(crate) fn advance(
        &mut self,
        anchor: Option<(Vec3, f32)>,
        seconds: f32,
        now: Instant,
    ) -> Option<Pose> {
        let dt = self
            .last
            .map_or(0.0, |last| {
                now.saturating_duration_since(last).as_secs_f32()
            })
            .min(0.1);
        self.last = Some(now);
        let target = anchor
            .filter(|_| self.on)
            .map(|(eye, yaw)| Self::place(eye, yaw));
        let shown = if target.is_some() { 1.0 } else { 0.0 };
        // Out, it comes back at the player rather than gliding from where it went out.
        let was_out = self.level <= 0.0;
        self.level = if self.level < shown {
            (self.level + dt / FADE).min(shown)
        } else {
            (self.level - dt / FADE).max(shown)
        };
        if let Some(target) = target {
            let position = match self.position {
                Some(position) if !was_out && position.distance(target) < SNAP => {
                    let followed = position + (target - position) * (1.0 - (-dt * FOLLOW).exp());
                    target + (followed - target).clamp_length_max(MAX_LAG)
                }
                _ => target,
            };
            self.position = Some(position);
        }
        if self.level <= 0.0 {
            return None;
        }
        let bob = (seconds * std::f32::consts::TAU / BOB_PERIOD).sin() * BOB;
        Some(Pose {
            position: self.position? + Vec3::Z * bob,
            rotation: Quat::from_rotation_z(seconds * SPIN)
                * Quat::from_rotation_x(TILT[0])
                * Quat::from_rotation_y(TILT[1]),
            level: smooth(self.level),
        })
    }
}

/// Ease in and out.
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

impl GpuState {
    /// `cg_illuminate`: Illuminate is on the Force wheel.
    pub(crate) fn illuminate_enabled(&self) -> bool {
        self.console
            .as_ref()
            .and_then(|console| console.integer_cvar(CVAR))
            .unwrap_or(1)
            != 0
    }

    /// `force_illuminate`, or `+useforce` on the wheel's Illuminate.
    pub(crate) fn toggle_illuminate(&mut self) {
        if self.illuminate_enabled() {
            self.illuminate.toggle();
        }
    }

    /// Add this frame's holocron and its light. Called right after the frame's
    /// lights are cleared, so a full list never drops the player's own light.
    pub(crate) fn submit_illuminate(&mut self, presentation_time: i64, now: Instant) {
        let enabled = self.illuminate_enabled();
        if !enabled {
            self.illuminate.on = false;
            self.illuminate.level = 0.0;
            return;
        }
        let anchor = self
            .live_session
            .as_ref()
            .filter(|_| !self.detached_camera && !self.free_camera_active())
            .filter(|session| playing(&session.latest_snapshot().player))
            .map(|_| (self.camera_position, self.camera_yaw));
        #[cfg(test)]
        let anchor = anchor.or(self.illuminate.shot_anchor);
        let Some(pose) = self
            .illuminate
            .advance(anchor, presentation_time as f32 * 0.001, now)
        else {
            return;
        };
        self.dynamic_lights.push_radiant(PointLight {
            origin: pose.position.to_array(),
            radius: RADIUS * pose.level,
            color: COLOR,
        });
        let Some(mesh) = self
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance.model == MODEL)
        else {
            return;
        };
        let mut instance = ActorInstance::new(
            pose.position.to_array(),
            pose.rotation.to_array(),
            [pose.level; 3],
        );
        if !self.third_person {
            instance.view_flags |= 1;
        }
        self.object_groups[mesh].push(instance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(start: Instant, seconds: f32) -> Instant {
        start + Duration::from_secs_f32(seconds)
    }

    #[test]
    fn the_bundled_files_mount_and_the_model_is_a_cube() {
        let mut vfs = VirtualFileSystem::new();
        mount(&mut vfs).unwrap();
        for (path, _) in FILES {
            assert!(vfs.read(path).unwrap().is_some(), "{path}");
        }
        let model = sjk_model::Md3::parse(&vfs.read(MODEL).unwrap().unwrap().bytes).unwrap();
        let surface = &model.surfaces[0];
        assert_eq!(surface.shaders, ["models/sjk/holocron"]);
        assert_eq!((surface.frames[0].len(), surface.triangles.len()), (24, 12));
        for vertex in &surface.frames[0] {
            // Every corner 3 units out on each axis, normals of unit length
            // pointing out of their face.
            assert!(vertex.position.iter().all(|c| (c.abs() - 3.0).abs() < 1e-3));
            let normal = Vec3::from_array(vertex.normal);
            assert!((normal.length() - 1.0).abs() < 0.02);
            assert!(normal.dot(Vec3::from_array(vertex.position)) > 2.9);
        }
        // Clockwise seen from outside, stock's front side.
        for [a, b, c] in &surface.triangles {
            let [a, b, c] =
                [a, b, c].map(|&i| Vec3::from_array(surface.frames[0][i as usize].position));
            let facing = (b - a).cross(c - a);
            assert!(facing.dot(a + b + c) < 0.0);
        }
    }

    /// Step at 60 frames a second from `from` to `to` seconds; the last pose.
    fn run(
        holocron: &mut Holocron,
        anchor: Option<(Vec3, f32)>,
        start: Instant,
        from: f32,
        to: f32,
    ) -> Option<Pose> {
        let mut pose = None;
        let mut t = from;
        while t <= to + 1e-4 {
            pose = holocron.advance(anchor, 0.0, at(start, t));
            t += 1.0 / 60.0;
        }
        pose
    }

    #[test]
    fn it_appears_by_the_left_shoulder_and_goes_out() {
        let start = Instant::now();
        let mut holocron = Holocron::default();
        let eye = Vec3::new(100.0, 0.0, 50.0);
        // Off: nothing, even with somewhere to float.
        assert_eq!(run(&mut holocron, Some((eye, 0.0)), start, 0.0, 0.5), None);
        holocron.toggle();
        let pose = run(&mut holocron, Some((eye, 0.0)), start, 0.5, 0.6).unwrap();
        assert!(pose.level > 0.0 && pose.level < 1.0);
        let pose = run(&mut holocron, Some((eye, 0.0)), start, 0.6, 1.0).unwrap();
        assert_eq!(pose.level, 1.0);
        assert_eq!(pose.position, eye + OFFSET);
        // Facing +y (yaw 90 degrees), its left is -x.
        let mut turned = Holocron::default();
        turned.toggle();
        let pose = run(
            &mut turned,
            Some((eye, std::f32::consts::FRAC_PI_2)),
            start,
            0.0,
            1.0,
        )
        .unwrap();
        assert!((pose.position - eye - Vec3::new(-18.0, -6.0, -4.0)).length() < 1e-3);
        // Dead or spectating: it goes out where it was.
        let pose = run(&mut holocron, None, start, 1.0, 1.1).unwrap();
        assert_eq!(pose.position, eye + OFFSET);
        assert!(pose.level < 1.0);
        assert_eq!(run(&mut holocron, None, start, 1.1, 1.5), None);
        // Back: it fades in where the player now is, even close by.
        let elsewhere = eye + Vec3::X * 30.0;
        let pose = run(&mut holocron, Some((elsewhere, 0.0)), start, 1.5, 1.55).unwrap();
        assert_eq!(pose.position, elsewhere + OFFSET);
    }

    #[test]
    fn it_trails_a_little_and_jumps_with_a_teleport() {
        let start = Instant::now();
        let mut holocron = Holocron::default();
        holocron.toggle();
        let eye = Vec3::new(0.0, 0.0, 50.0);
        run(&mut holocron, Some((eye, 0.0)), start, 0.0, 0.5);
        // Running at 300 units a second for a frame: it lags, never past MAX_LAG.
        let moved = eye + Vec3::X * 40.0;
        let pose = holocron
            .advance(Some((moved, 0.0)), 0.0, at(start, 0.51))
            .unwrap();
        let lag = pose.position.distance(moved + OFFSET);
        assert!(lag > 1.0 && lag <= MAX_LAG + 1e-3, "{lag}");
        let far = eye + Vec3::X * 1000.0;
        let pose = holocron
            .advance(Some((far, 0.0)), 0.0, at(start, 0.52))
            .unwrap();
        assert_eq!(pose.position, far + OFFSET);
    }
}
