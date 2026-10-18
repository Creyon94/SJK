//! Local world visibility during a private duel.

/// Archived player preference for hiding `ET_BODY` corpses while the local
/// player is in a private duel.
pub(crate) const CORPSES_CVAR: &str = "cg_hideCorpsesInDuel";

/// Archived player preference for hiding `ET_ITEM` and `ET_HOLOCRON` pickups
/// while the local player is in a private duel.
pub(crate) const PICKUPS_CVAR: &str = "cg_hidePickupsInDuel";
