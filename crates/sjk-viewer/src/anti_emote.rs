//! Optional JA+ push response when another player puts the local player in the
//! paired kiss animation.

use sjk_client::{CompatProfile, pmove::MovementState};
use sjk_protocol::PlayerState;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use std::sync::OnceLock;

/// JoF EternalJK's archived anti-kiss switch.
pub(crate) const KISS_CVAR: &str = "cg_antiAmKiss";

const PMF_FOLLOW: u16 = 0x1000;
const ANIM_TOGGLEBIT: u16 = 0x800;

#[derive(Default)]
pub(crate) struct State {
    kiss_active: bool,
}

impl State {
    fn observe(&mut self, kiss_active: bool) -> bool {
        let kiss_push = kiss_active && !self.kiss_active;
        self.kiss_active = kiss_active;
        kiss_push
    }

    fn clear(&mut self) {
        self.kiss_active = false;
    }
}

struct ServerCase {
    accepts: fn(&CompatProfile) -> bool,
    _name: &'static str,
    _evidence: &'static str,
}

/// The victim animation and the anti-kiss response are JA+ compatibility
/// behavior. Keep that decision in a table rather than teaching the frame loop a
/// server-name special case.
const SERVER_CASES: &[ServerCase] = &[ServerCase {
    accepts: |profile| matches!(profile, CompatProfile::JaPlus { .. }),
    _name: "JA+",
    _evidence: "JoF EJK cg_view.c:2635-2640; live JoF JA+ check 19/10/2026",
}];

pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        KISS_CVAR,
        false,
        CvarFlags::ARCHIVE,
        "Push when a JA+ player puts you in the amkiss victim animation",
    ))
}

fn server_case(profile: &CompatProfile) -> Option<&'static ServerCase> {
    SERVER_CASES.iter().find(|case| (case.accepts)(profile))
}

fn animation_index(name: &str) -> u16 {
    sjk_game_jka::legacy_animation_index(name).map_or(u16::MAX, |value| value as u16)
}

fn kiss_victim_animation() -> u16 {
    static ANIMATION: OnceLock<u16> = OnceLock::new();
    *ANIMATION.get_or_init(|| animation_index("BOTH_KISSEE"))
}

fn predicted_is_animation(state: &MovementState, expected: u16) -> bool {
    state.legs_anim & !ANIM_TOGGLEBIT == expected || state.torso_anim & !ANIM_TOGGLEBIT == expected
}

fn authoritative_is_animation(state: &PlayerState, expected: u16) -> bool {
    state.leg_animation() & !ANIM_TOGGLEBIT == expected
        || state.torso_animation() & !ANIM_TOGGLEBIT == expected
}

fn kiss_is_active(
    profile: &CompatProfile,
    predicted: Option<&MovementState>,
    authoritative: &PlayerState,
    enabled: bool,
) -> bool {
    if server_case(profile).is_none()
        || authoritative.movement_flags() & PMF_FOLLOW != 0
        || authoritative.movement_type() == sjk_game_jka::PM_SPECTATOR
    {
        return false;
    }
    let expected = kiss_victim_animation();
    enabled
        && (predicted.is_some_and(|state| predicted_is_animation(state, expected))
            || authoritative_is_animation(authoritative, expected))
}

impl crate::GpuState {
    /// Queue JoF EJK's `force_throw` response before the next usercmd is assembled.
    pub(crate) fn apply_anti_emote(&mut self) {
        let enabled = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(KISS_CVAR))
            .unwrap_or(false);
        if !enabled {
            self.anti_emote.clear();
            return;
        }
        let Some(session) = self.live_session.as_ref() else {
            self.anti_emote.clear();
            return;
        };
        let profile = session.compat_profile();
        if server_case(profile).is_none() {
            self.anti_emote.clear();
            return;
        }
        let authoritative = &session.latest_snapshot().player;
        let predicted = self.local_prediction.predicted_state();
        let active = kiss_is_active(profile, predicted, authoritative, enabled);
        if self.anti_emote.observe(active) {
            self.pending_generic_command = crate::input::generic_commands::GENCMD_FORCE_THROW;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn predicted_state(name: &str) -> MovementState {
        MovementState {
            legs_anim: animation_index(name),
            torso_anim: animation_index(name),
            ..MovementState::default()
        }
    }

    fn authoritative_state(name: &str) -> PlayerState {
        let animation = u32::from(animation_index(name));
        let mut state = PlayerState::default();
        assert!(state.set_raw_field(13, animation));
        assert!(state.set_raw_field(15, animation));
        state
    }

    fn ja_plus() -> CompatProfile {
        CompatProfile::JaPlus { version: None }
    }

    #[test]
    fn switch_is_archived_and_off_by_default() {
        let mut cvars = CvarRegistry::new();
        register(&mut cvars).expect("registers");
        let cvar = cvars.get(KISS_CVAR).expect("registered");
        assert_eq!(cvar.value.as_text(), "0");
        assert!(cvar.flags.contains(CvarFlags::ARCHIVE));
    }

    #[test]
    fn enabled_victim_animation_pushes_on_ja_plus() {
        assert!(kiss_is_active(
            &ja_plus(),
            Some(&predicted_state("BOTH_KISSEE")),
            &PlayerState::default(),
            true,
        ));
    }

    #[test]
    fn other_servers_following_and_spectating_never_push() {
        let kissed = predicted_state("BOTH_KISSEE");
        let authoritative = authoritative_state("BOTH_KISSEE");
        assert!(!kiss_is_active(
            &CompatProfile::BaseJka,
            Some(&kissed),
            &authoritative,
            true,
        ));
        assert!(!kiss_is_active(
            &CompatProfile::TaystJk,
            Some(&kissed),
            &authoritative,
            true,
        ));

        let mut following = authoritative.clone();
        assert!(following.set_raw_field(38, u32::from(PMF_FOLLOW)));
        assert!(!kiss_is_active(&ja_plus(), Some(&kissed), &following, true));

        let mut spectator = authoritative;
        assert!(spectator.set_raw_field(63, u32::from(sjk_game_jka::PM_SPECTATOR)));
        assert!(!kiss_is_active(&ja_plus(), Some(&kissed), &spectator, true));
    }

    #[test]
    fn animation_toggle_bit_does_not_hide_an_interaction() {
        let mut kissed = predicted_state("BOTH_KISSEE");
        kissed.legs_anim |= ANIM_TOGGLEBIT;
        kissed.torso_anim |= ANIM_TOGGLEBIT;
        assert!(kiss_is_active(
            &ja_plus(),
            Some(&kissed),
            &PlayerState::default(),
            true,
        ));
    }

    #[test]
    fn kiss_pushes_once_until_the_animation_ends() {
        let mut latch = State::default();
        assert!(latch.observe(true));
        assert!(!latch.observe(true));
        assert!(!latch.observe(false));
        assert!(latch.observe(true));
    }
}
