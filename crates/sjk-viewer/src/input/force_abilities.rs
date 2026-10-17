//! Table-driven server ability controls; no server gameplay is implemented here.
use sjk_client::force_wheel;

/// Player override for the table's explicitly flagged server extensions.
pub(crate) const CVAR: &str = "cg_forceDestruction";

pub(super) fn command_slot(command: &str) -> Option<u8> {
    // Repulse/Dash already fall through to the server command route. Take all
    // instant pseudo-abilities here so binds and the wheel use the same grants.
    force_wheel::ABILITIES
        .iter()
        .find(|a| a.command == Some(command))
        .map(|a| a.slot)
}

pub(crate) fn support(
    game: Option<&sjk_protocol::GameState>,
    console: Option<&crate::console::ViewerConsole>,
) -> u32 {
    force_wheel::support(
        game,
        console
            .and_then(|c| c.integer_cvar("cg_forcedestruction"))
            .unwrap_or(-1),
    )
}

impl crate::GpuState {
    pub(crate) fn refresh_force_support(&mut self) {
        let game = self
            .live_session
            .as_ref()
            .map(|s| s.game_state())
            .or_else(|| self.demo_session.as_ref().map(|s| s.game_state()));
        self.gameplay_input
            .selection
            .set_support(support(game, self.console.as_ref()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{GameplayInput, InputAction};
    #[test]
    fn all_instant_bindings_use_local_capability_dispatch() {
        let mut input = GameplayInput::default();
        for ability in force_wheel::ABILITIES {
            if let Some(command) = ability.command {
                assert!(GameplayInput::recognizes(command));
                assert_eq!(
                    input.apply(command),
                    Some(InputAction::ForceAbility(ability.slot))
                );
            }
        }
    }
}
