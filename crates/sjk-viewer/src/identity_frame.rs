//! The per-frame hook of player identity (`player_identity.rs`): twice a second,
//! hand the service the player's settings and where they are playing, the new
//! medal pop-up the medals of the player's own profile (`medal_popup.rs`), and the
//! achievements their counts (`achievements_frame.rs`).

use super::*;
use sjk_identity::Settings;

impl GpuState {
    /// Tell the identity service about the settings and the live session. Does
    /// nothing between its twice-a-second turns.
    pub(crate) fn update_identity(&mut self) {
        if !player_identity::due() {
            return;
        }
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let settings = Settings {
            enabled: console.bool_cvar("cl_identity") == Some(true),
            hub_url: console
                .text_cvar("cl_hubUrl")
                .unwrap_or_default()
                .trim()
                .to_owned(),
        };
        let name = console
            .text_cvar("name")
            .unwrap_or_default()
            .trim()
            .to_owned();
        let location = self.live_session.as_ref().and_then(|session| {
            player_identity::location(session.server(), session.is_local(), session.game_state())
        });
        player_identity::apply(console.config_directory(), settings, name, location);
        self.offer_medals();
        self.update_achievements();
    }
}
