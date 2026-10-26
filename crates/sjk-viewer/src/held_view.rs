//! A view the game server holds: a melee grapple or throw, a saber lock and, on a JA+
//! server, its kisses, kicks, knockdowns, get-ups and ledges
//! ([`sjk_game_jka::prediction_policy::server_holds_view`], the table of cases with their
//! evidence). The server rewrites `delta_angles` every frame then, so anything else that
//! turns the view fights it:
//!
//! - the mouse: the view shook as the player looked around while kicked down (Rollembones'
//!   report #146). The mouse does not turn a held view ([`LOOK_CVAR`] lets it);
//! - a toy whose commands carry their own angles (the Ragdoll's walk, the Hax's steering):
//!   each server rewrite turned the camera on by the gap between the two views, so it spun
//!   (#139, #140). The toys stop while the view is held ([`crate::ragdoll`],
//!   [`crate::hax`]).

use crate::GpuState;

/// `1` lets the mouse turn a view the server holds, as before (it shakes).
pub(crate) const LOOK_CVAR: &str = "cg_heldViewLook";
/// [`LOOK_CVAR`]'s default: the mouse waits.
pub(crate) const DEFAULT_LOOK: bool = false;

impl GpuState {
    /// Whether the game server holds the player's own view in the latest snapshot.
    pub(crate) fn server_holds_view(&self) -> bool {
        let Some(session) = self.live_session.as_ref() else {
            return false;
        };
        let player = &session.latest_snapshot().player;
        if player.is_spectator()
            || !crate::local_prediction::predicts_local_view(player.movement_flags())
        {
            return false;
        }
        sjk_game_jka::prediction_policy::server_holds_view(player, || {
            sjk_game_jka::pmove_japlus::JaPlusRules::from_game_state(session.game_state()).enabled
        })
    }

    /// Whether the mouse leaves the view alone this frame: the server holds it, the
    /// player has not asked to look anyway, and no free or fake-noclip camera flies.
    pub(crate) fn mouse_look_held(&self) -> bool {
        if self.local_prediction.fake_noclip() || self.local_prediction.free_camera() {
            return false;
        }
        let look = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(LOOK_CVAR))
            .unwrap_or(DEFAULT_LOOK);
        !look && self.server_holds_view()
    }
}
