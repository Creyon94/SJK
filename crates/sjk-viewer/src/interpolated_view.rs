//! Stock `CG_InterpolatePlayerState`: smooth authoritative views between snapshots.
//! Follow views interpolate angles; synchronous local views keep immediate mouse-look.

use super::GpuState;
use glam::Vec3;
use sjk_game_jka::movement_styles;
use sjk_game_jka::prediction_policy::{self, Interpolation, NoPredict};
use sjk_protocol::{GameState, PlayerState, Snapshot};

/// Whether the local view is the server's rather than predicted, and how
/// (`CG_PredictPlayerState`): following, a synchronous server, `cg_noPredict`, a
/// JA+ kick victim, or a jaPRO movement style SJK's pmove does not model
/// (`movement_styles`); `profile` is the session's server.
pub(crate) fn mode(
    console: Option<&crate::console::ViewerConsole>,
    game: &GameState,
    profile: &sjk_client::CompatProfile,
    player: &PlayerState,
) -> Option<Interpolation> {
    let no_predict = console
        .and_then(|console| console.integer_cvar("cg_noPredict"))
        .unwrap_or(0);
    prediction_policy::interpolation(
        player,
        NoPredict(no_predict),
        !crate::local_prediction::predicts_local_view(player.movement_flags()),
        server_synchronous(game),
        || sjk_game_jka::pmove_japlus::JaPlusRules::from_game_state(game).enabled,
    )
    .or_else(|| {
        let predict_anyway = console
            .and_then(|console| console.bool_cvar(movement_styles::PREDICT_ANYWAY_CVAR))
            .unwrap_or(false);
        let japro = *profile == sjk_client::CompatProfile::TaystJk;
        let style = movement_styles::unpredicted(player, japro, predict_anyway);
        announce_style(style);
        style.map(|_| Interpolation::KeepAngles)
    })
}

/// Say in the log, once per change, that a movement style is shown unpredicted.
fn announce_style(style: Option<&'static str>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    // The last style's name by address (0 for none): the names are statics.
    static LAST: AtomicUsize = AtomicUsize::new(0);
    let address = style.map_or(0, |name| name.as_ptr() as usize);
    if LAST.swap(address, Ordering::Relaxed) == address {
        return;
    }
    if let Some(name) = style {
        crate::log::progress(format_args!(
            "movement style {name}: SJK does not predict it yet, so your movement shows as \
             the server has it ({} 1 predicts it as JKA)",
            movement_styles::PREDICT_ANYWAY_CVAR,
        ));
    }
}

pub(crate) fn server_synchronous(game: &GameState) -> bool {
    game.config_string(1)
        .and_then(|info| sjk_protocol::info_value(info, b"g_synchronousClients"))
        .is_some_and(|value| sjk_game_jka::userinfo::atoi(value) != 0)
}

/// Do not bridge a teleport, a followed-player change, or a change of view mode.
fn fraction(previous: &Snapshot, next: &Snapshot, time: i32) -> f32 {
    if next.server_time <= previous.server_time
        || (previous.player.entity_flags() ^ next.player.entity_flags()) & 8 != 0
        || previous.player.client_num() != next.player.client_num()
        || (previous.player.movement_flags() ^ next.player.movement_flags()) & 4096 != 0
    {
        return 0.0;
    }
    ((time - previous.server_time) as f32 / (next.server_time - previous.server_time) as f32)
        .clamp(0.0, 1.0)
}

fn lerp_angle(from: f32, mut to: f32, fraction: f32) -> f32 {
    if to - from > 180.0 {
        to -= 360.0;
    }
    if to - from < -180.0 {
        to += 360.0;
    }
    from + fraction * (to - from)
}

fn sample(previous: &Snapshot, next: &Snapshot, time: i32) -> ([f32; 3], [f32; 3]) {
    let f = fraction(previous, next, time);
    let from = previous.player.origin();
    let to = next.player.origin();
    let mut eye = std::array::from_fn(|axis| from[axis] + f * (to[axis] - from[axis]));
    eye[2] += previous.player.view_height() as f32;
    let from = previous.player.view_angles();
    let to = next.player.view_angles();
    (
        eye,
        std::array::from_fn(|axis| lerp_angle(from[axis], to[axis], f)),
    )
}

pub(crate) fn present(gpu: &mut GpuState, time: i32) {
    let Some(session) = gpu.live_session.as_ref() else {
        return;
    };
    let latest = session.latest_snapshot();
    // Intermission owns its camera. A missing predictor alone is not enough.
    if matches!(latest.player.movement_type(), 7 | 8) {
        return;
    }
    let Some(mode) = mode(
        gpu.console.as_ref(),
        session.game_state(),
        session.compat_profile(),
        &latest.player,
    ) else {
        return;
    };
    let previous = session.snapshot_at_or_before(time);
    let next = session
        .snapshot_after(previous.server_time)
        .unwrap_or(previous);
    let (eye, angles) = sample(previous, next, time);
    gpu.camera_position = Vec3::from_array(eye);
    if mode == Interpolation::ServerAngles {
        gpu.camera_pitch = -angles[0].to_radians();
        gpu.camera_yaw = angles[1].to_radians();
    }
}
