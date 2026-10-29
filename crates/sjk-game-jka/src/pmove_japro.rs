//! jaPRO command policy from TaystJK `2d60c0fa5`, multiplayer `codemp/game/bg_pmove.c`.
//! These rules interpret existing snapshot slots only for the advertised jaPRO dialect.
use sjk_protocol::{GameState, InfoString, PlayerState, ServerDialect, ServerProfile};

use crate::pmove::MovementState;
use crate::pmove_roll::{BOTH_ROLL_F, BOTH_ROLL_R};

/// TaystJK `codemp/game/bg_public.h` stat and movement-style ordinals.
const STAT_RACEMODE: usize = 11;
const STAT_MOVEMENTSTYLE: usize = 13;
const MV_OCPM: u32 = 16;
const TEAM_SPECTATOR: u8 = 3;
/// `JAPRO_CINFO_HIGHFPSFIX` in that header's advertised capability table.
const HIGH_FPS_FIX: u32 = 1 << 21;

/// Snapshot metadata used by jaPRO's command wrapper. Other dialects may reuse
/// these stat slots; callers must select [`JaProRules`] before interpreting them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JaProState {
    /// `STAT_RACEMODE`, stat slot 11.
    pub race: bool,
    /// `STAT_MOVEMENTSTYLE`, stat slot 13; `MV_OCPM` is 16.
    pub style: u32,
}

impl JaProState {
    /// Retain the existing decoded snapshot slots without changing the wire format.
    pub fn from_player_state(player: &PlayerState) -> Self {
        Self {
            race: player.stats[STAT_RACEMODE] != 0,
            style: player.stats[STAT_MOVEMENTSTYLE],
        }
    }
}

/// Advertised jaPRO policy. The default leaves the stock predictor unchanged.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JaProRules {
    /// Interpret jaPRO movement metadata and run its command policy.
    pub enabled: bool,
    /// `JAPRO_CINFO_HIGHFPSFIX`, `jcinfo` bit 21.
    pub high_fps_fix: bool,
    /// `JAPRO_CINFO_HEADSLIDE`, `jcinfo` bit 5: standing on a player has no friction.
    pub head_slide: bool,
}

impl JaProRules {
    /// Resolve the existing server dialect once when serverinfo changes.
    pub fn from_game_state(game: &GameState) -> Self {
        let Some(info) = game
            .config_string(0)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|text| InfoString::parse(text).ok())
        else {
            return Self::default();
        };
        match ServerProfile::from_server_info(&info).dialect {
            ServerDialect::TaystJk {
                ja_pro_capabilities,
                ..
            } => Self {
                enabled: true,
                high_fps_fix: ja_pro_capabilities.contains(HIGH_FPS_FIX),
                head_slide: ja_pro_capabilities.contains(1 << 5),
            },
            _ => Self::default(),
        }
    }

    /// `Pmove`: race commands bypass fixed/66 ms chopping, except active rolls
    /// and OCPM, which use at most 8 ms. Re-evaluate after every slice.
    pub(crate) fn slice_millis(
        self,
        state: &MovementState,
        remaining: i32,
        fixed: Option<i32>,
    ) -> i32 {
        let maximum = if self.enabled && state.ja_pro.race {
            if state.ja_pro.style == MV_OCPM
                || state.legs_timer > 0 && matches!(state.legs_anim, BOTH_ROLL_F..=BOTH_ROLL_R)
            {
                8
            } else {
                remaining
            }
        } else {
            fixed.map_or(66, |value| value.clamp(8, 33))
        };
        remaining.min(maximum)
    }

    /// `PmoveSingle` final velocity policy. This runs only on paths that reach
    /// the end of the move; spectator/noclip early returns remain early returns.
    pub(crate) fn snap_velocity(
        self,
        state: &MovementState,
        millis: i32,
        stock_snap: bool,
    ) -> bool {
        if !self.enabled {
            return stock_snap;
        }
        if state.team == TEAM_SPECTATOR {
            return true;
        }
        if state.ja_pro.race && state.ja_pro.style != MV_OCPM {
            return false;
        }
        if self.high_fps_fix && !(4..=25).contains(&millis) {
            return false;
        }
        stock_snap
    }

    /// Race wallbug prevention happens before snapping: an unchanged origin
    /// cannot retain an accumulating velocity. Spectators are excluded.
    pub(crate) fn finish_velocity(
        self,
        state: &mut MovementState,
        previous_origin: [f32; 3],
        millis: i32,
        stock_snap: bool,
    ) {
        if self.enabled
            && state.ja_pro.race
            && state.team != TEAM_SPECTATOR
            && state.origin == previous_origin
        {
            state.velocity = [0.0; 3];
        }
        if self.snap_velocity(state, millis, stock_snap) {
            state.velocity = state.velocity.map(f32::round_ties_even);
        }
    }
}

#[cfg(test)]
#[path = "pmove_japro_tests.rs"]
mod tests;
