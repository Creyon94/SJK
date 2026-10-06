//! Viewer bridge from legacy muzzle requests to complete retail EFX graphs.

use super::*;

/// Spawn one graph at the verified game-facing `*flash` socket.
///
/// `CG_AddPlayerWeapon` calls `FX_PlayEffectID` on every rendered frame in
/// the flash window (`codemp/cgame/cg_weapons.c:733-762`). Each call schedules
/// fresh primitive copies; there is no cross-call collapse
/// (`codemp/client/FxScheduler.cpp:790-983`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn(
    particles: &mut Vec<Particle>,
    auxiliary: &mut crate::effect_aux::Runtime,
    effects: &mut EffectLibrary,
    vfs: &VirtualFileSystem,
    audio: &mut Option<GameAudio>,
    request: sjk_client::LegacyMuzzleEffectRequest,
    socket: muzzle_flash::Socket,
    now: Instant,
    presentation_time: i32,
) {
    // Re-played on the reference 8 ms cadence, not per rendered frame (`effect_cadence.rs`).
    if !auxiliary.continuous.due(now) {
        return;
    }
    let shakes = auxiliary.pending_shakes();
    effect_runtime::spawn_effect(
        particles,
        auxiliary,
        effects,
        vfs,
        request.effect_name,
        Vec3::from_array(socket.origin),
        now,
        u32::from(request.client_num) | (presentation_time as u32).rotate_left(11),
        0,
        audio,
        combat_effects::rotation_from_direction(socket.direction),
    );
    // Muzzle flashes carry a short `CameraShake` (JoF's HD weapon effects: radius
    // 60), but JoF EternalJK shows no shake when firing, in first or third person
    // (frame-to-frame motion measured on the contributor's recordings), so the
    // flash's shake is dropped. Explosions and other effects still shake.
    auxiliary.drop_shakes_since(shakes);
}
