//! The beam of an armed trip mine (`CG_General`, `cg_ents.c:1789-1814`): a stuck mine
//! (`ET_GENERAL`, `WP_TRIP_MINE`, `time == -1`) with `EF_FIRING` plays
//! `tripMine/laserMP` (or `tripMine/glowbit` in proximity mode, `bolt2 == 1`) every
//! cgame frame, 6.6 units out along its facing, pointed along `pos.trDelta`, the
//! surface normal the game stores there.

use crate::effect_runtime::EffectLibrary;
use crate::{GameAudio, Particle, combat_effects, effect_runtime};
use glam::Vec3;
use sjk_protocol::Snapshot;
use sjk_vfs::VirtualFileSystem;
use std::time::Instant;

const ET_GENERAL: u8 = 0;
const WP_TRIP_MINE: u8 = 13;
const EF_FIRING: u32 = 1 << 9;
const LASER: &str = "tripMine/laserMP";
const GLOW: &str = "tripMine/glowbit";

/// One armed mine's beam: where it starts, which way it points and which effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Beam {
    pub(crate) origin: [f32; 3],
    pub(crate) direction: [f32; 3],
    pub(crate) effect: &'static str,
}

/// The beam of `state`, when it is an armed, stuck trip mine.
pub(crate) fn beam(state: &sjk_protocol::EntityState) -> Option<Beam> {
    if state.entity_type() != ET_GENERAL
        || state.weapon() != WP_TRIP_MINE
        || state.time() != -1
        || state.e_flags() & EF_FIRING == 0
    {
        return None;
    }
    // A stuck mine is stationary: `lerpOrigin` and `lerpAngles` are the bases.
    let [pitch, yaw, _] = state.angular_trajectory_base().map(f32::to_radians);
    // `AnglesToAxis`: axis[0], the facing.
    let forward = Vec3::new(
        pitch.cos() * yaw.cos(),
        pitch.cos() * yaw.sin(),
        -pitch.sin(),
    );
    let origin = Vec3::from_array(state.trajectory_base()) + forward * 6.6;
    Some(Beam {
        origin: origin.to_array(),
        direction: state.trajectory_delta(),
        effect: if state.bolt2() == 1 { GLOW } else { LASER },
    })
}

/// `FX_MAX_TRACE_DIST` (`FxScheduler.h`): how far an `org2fromTrace` line reaches.
const TRACE_DISTANCE: f32 = 16_384.0;

/// The beam's `Line` is an `org2fromTrace` primitive: it runs from the mine to
/// whatever solid its facing hits (`CFxScheduler::CreateEffect`). SJK's lines take
/// only their authored `origin2`, which for this beam is a zero-length offset, so
/// the beam showed as a dot; the lines just spawned are stretched to the trace.
fn stretch_to_trace(
    particles: &mut [Particle],
    origin: Vec3,
    direction: Vec3,
    bsp: &sjk_bsp::Bsp,
    scratch: &mut sjk_bsp::TraceScratch,
) {
    let direction = direction.normalize_or(Vec3::Z);
    let end = Vec3::from_array(
        bsp.trace_box_with(
            scratch,
            origin.to_array(),
            (origin + direction * TRACE_DISTANCE).to_array(),
            sjk_bsp::Aabb::POINT,
            // MASK_SOLID.
            0x0000_0001,
        )
        .end_position,
    );
    for particle in particles {
        if let Some(streak) = &mut particle.streak {
            *streak = end - particle.motion.sample().origin;
        }
    }
}

/// Play every armed mine's beam on the reference cgame cadence (`effect_cadence.rs`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn(
    snapshot: &Snapshot,
    bsp: &sjk_bsp::Bsp,
    scratch: &mut sjk_bsp::TraceScratch,
    particles: &mut Vec<Particle>,
    auxiliary: &mut crate::effect_aux::Runtime,
    effects: &mut EffectLibrary,
    vfs: &VirtualFileSystem,
    audio: &mut Option<GameAudio>,
    now: Instant,
    presentation_time: i32,
) {
    if !auxiliary.continuous.due(now) {
        return;
    }
    for state in &snapshot.entities {
        let Some(beam) = beam(state) else {
            continue;
        };
        let first = particles.len();
        effect_runtime::spawn_effect(
            particles,
            auxiliary,
            effects,
            vfs,
            beam.effect,
            Vec3::from_array(beam.origin),
            now,
            u32::from(state.number()) ^ (presentation_time as u32).rotate_left(11),
            0,
            audio,
            combat_effects::rotation_from_direction(beam.direction),
        );
        if let Some(spawned) = particles.get_mut(first..) {
            stretch_to_trace(
                spawned,
                Vec3::from_array(beam.origin),
                Vec3::from_array(beam.direction),
                bsp,
                scratch,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_protocol::{EntityState, LEGACY_ENTITY_FIELDS};

    fn mine(flags: u32, time: i32, bolt2: u32) -> EntityState {
        let mut state = EntityState::zero(1, &LEGACY_ENTITY_FIELDS);
        state.set_raw_field(14, u32::from(WP_TRIP_MINE));
        state.set_raw_field(19, flags);
        state.set_raw_field(65, time as u32);
        state.set_raw_field(63, bolt2);
        state
    }

    #[test]
    fn only_armed_stuck_mines_have_a_beam() {
        assert!(beam(&mine(0, -1, 0)).is_none());
        assert!(beam(&mine(EF_FIRING, 0, 0)).is_none());
        assert_eq!(beam(&mine(EF_FIRING, -1, 0)).unwrap().effect, LASER);
        assert_eq!(beam(&mine(EF_FIRING, -1, 1)).unwrap().effect, GLOW);
    }
}
