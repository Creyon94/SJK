//! The small mark after a player's name on the scoreboard when the SJK hub
//! knows them (`player_identity.rs`): `SJK` for a registered player, `VERIFIED`
//! in gold for one the hub's operator vouches for.

use crate::player_identity::Tag;
use sjk_ui::Color;

const VERIFIED: Color = Color::new(1.0, 0.82, 0.25, 1.0);
const REGISTERED: Color = Color::new(0.70, 0.74, 0.80, 0.85);

/// What to write and in which colour.
pub(super) fn mark(tag: Tag) -> (&'static str, Color) {
    if tag.verified {
        ("VERIFIED", VERIFIED)
    } else {
        ("SJK", REGISTERED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_players_are_marked_apart_from_registered_ones() {
        assert_eq!(mark(Tag { verified: true }).0, "VERIFIED");
        assert_eq!(mark(Tag { verified: false }).0, "SJK");
        assert_ne!(
            mark(Tag { verified: true }).1,
            mark(Tag { verified: false }).1
        );
    }
}
