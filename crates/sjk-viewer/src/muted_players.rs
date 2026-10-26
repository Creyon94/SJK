//! Muted players on the server being played (`docs/hub-chat.md`, "Muting a player"):
//! which game slots the mute list names ([`crate::chat_mutes::slots`]), worked out only
//! when the list, the hub's claims, the server's players or the player's own slot
//! change, and what that does here. A muted player is drawn as Kyle with the default
//! saber in the default colour and without their hat or cape ([`appearance`],
//! [`sabers`], [`equipment`]), their chat is hidden and nothing they cause is heard
//! (`audio_mute.rs`). It is all local: no command goes to the server, and the game goes
//! on unchanged.

pub(crate) use crate::chat_mutes::SLOTS;
use crate::chat_mutes::Slots;
use sjk_client::ChatTarget;
use sjk_runtime::{Appearance, HeldEquipment};

/// The model a muted player is drawn as (`DEFAULT_MODEL`, the game's own stand-in).
pub(crate) const MODEL: &str = "kyle/default";
/// Kyle's model folder, as an appearance names it.
const MODEL_PATH: &str = "models/players/kyle";
/// The saber a muted player is drawn with: the `saber1` default.
pub(crate) const SABER: &str = "single_1";
/// The saber a muted player holding a staff is drawn with: the stock staff.
pub(crate) const STAFF: &str = "dual_1";
/// What the game's names of a muted player read here (scoreboard, kill feed,
/// nameplates, the crosshair).
pub(crate) const LABEL: &str = "BLOCKED";

/// The muted slots, for the places that draw names and have no frame state at hand
/// ([`label`]); set by the game frame with the muted slots.
static SHOWN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Remember the muted slots for [`label`].
pub(crate) fn set_shown(slots: u32) {
    SHOWN.store(slots, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the player in `slot` is muted, for the places that draw names.
pub(crate) fn shown_muted(slot: impl Into<u32>) -> bool {
    let slot = slot.into();
    slot < SLOTS as u32 && SHOWN.load(std::sync::atomic::Ordering::Relaxed) & (1 << slot) != 0
}

/// What to draw for the name `name` of the player in `slot`: [`LABEL`] for a muted
/// player, else the name.
pub(crate) fn label(slot: impl Into<u32>, name: &str) -> &str {
    if shown_muted(slot) { LABEL } else { name }
}

/// Their blades' colour: `saber_colors_t` 4, blue, the `color1` default
/// (`cg_players.c:6371-6394` gives it this tint).
pub(crate) const BLADE: [u8; 3] = [51, 102, 255];

/// What the slots were worked out from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Inputs {
    mutes: u64,
    claims: u64,
    map: u64,
    roster: [Option<ChatTarget>; SLOTS],
    own: i32,
}

/// The muted slots of the server being played, and the ones drawn as the stand-in.
#[derive(Debug, Default)]
pub(crate) struct MutedPlayers {
    slots: Slots,
    inputs: Option<Inputs>,
    /// Slots whose actor and sabers were last rebuilt muted.
    pub(crate) applied: u32,
}

impl MutedPlayers {
    /// The muted slots.
    pub(crate) fn slots(&self) -> Slots {
        self.slots
    }

    /// Whether `client` is muted.
    pub(crate) fn contains(&self, client: u16) -> bool {
        self.slots.contains(usize::from(client))
    }

    /// Work the slots out again when what they come from changed: the mute list's, the
    /// hub's claims' and the map blocks' revisions (`map`, `map_blocks.rs`), the server's
    /// players (`roster`, one roster observation per slot) and the player's own slot;
    /// `resolve` works them out. Returns whether they changed.
    pub(crate) fn refresh(
        &mut self,
        mutes: u64,
        claims: u64,
        map: u64,
        roster: [Option<ChatTarget>; SLOTS],
        own: i32,
        resolve: impl FnOnce() -> Slots,
    ) -> bool {
        let inputs = Inputs {
            mutes,
            claims,
            map,
            roster,
            own,
        };
        if self.inputs == Some(inputs) {
            return false;
        }
        self.inputs = Some(inputs);
        let slots = resolve();
        let changed = slots != self.slots;
        self.slots = slots;
        changed
    }

    /// Forget the slots (no server); returns whether there was anything to forget.
    pub(crate) fn clear(&mut self) -> bool {
        let had = self.inputs.is_some();
        self.slots = Slots::default();
        self.inputs = None;
        // The next server's players are all drawn as themselves until worked out
        // again, so a mute that comes back to the same slots must rebuild them.
        self.applied = 0;
        had
    }
}

/// What a muted player is drawn as, from `appearance`, what the game would draw with
/// Kyle forced: Kyle in his default skin, or in the red or blue one a team game's
/// colours give him, so the teams still show.
pub(crate) fn appearance(appearance: &Appearance) -> Appearance {
    let skin = appearance.variant.to_ascii_lowercase();
    let variant = if skin == "red" || skin.ends_with("_red") {
        "red"
    } else if skin == "blue" || skin.ends_with("_blue") {
        "blue"
    } else {
        "default"
    };
    Appearance {
        model: MODEL_PATH.to_owned(),
        variant: variant.to_owned(),
    }
}

/// The sabers a muted player is drawn with: the default saber in each hand that holds
/// one (two for a dual wielder), or the stock staff for a lone staff (`is_staff` says
/// which sabers are staffs).
pub(crate) fn sabers(names: &mut [Option<String>; 2], is_staff: impl Fn(&str) -> bool) {
    let staff = names[1].is_none() && names[0].as_deref().is_some_and(&is_staff);
    for name in names.iter_mut().flatten() {
        let stand_in = if staff { STAFF } else { SABER };
        if name != stand_in {
            *name = stand_in.to_owned();
        }
    }
}

/// A muted player's held weapon: their blades in the default colour.
pub(crate) fn equipment(mut held: HeldEquipment) -> HeldEquipment {
    held.color = BLADE;
    held.secondary_color = BLADE;
    held
}

/// Whether the scene entity `entity_id` (a client's is its slot plus one) is a player
/// in the muted `slots` (bits).
pub(crate) fn entity_muted(slots: u32, entity_id: u64) -> bool {
    entity_id
        .checked_sub(1)
        .filter(|client| *client < SLOTS as u64)
        .is_some_and(|client| slots & (1 << client) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots(muted: u32) -> Slots {
        Slots { muted, by_name: 0 }
    }

    fn worn(model: &str, variant: &str) -> Appearance {
        Appearance {
            model: format!("models/players/{model}"),
            variant: variant.to_owned(),
        }
    }

    #[test]
    fn a_muted_player_is_kyle_with_the_default_saber() {
        assert_eq!(
            appearance(&worn("kyle", "default")),
            worn("kyle", "default")
        );
        // A team game's colours stay, so the teams still show; any other skin goes.
        assert_eq!(appearance(&worn("kyle", "red")), worn("kyle", "red"));
        assert_eq!(appearance(&worn("kyle", "jedi_blue")), worn("kyle", "blue"));
        assert_eq!(
            appearance(&worn("kyle", "mp_custom")),
            worn("kyle", "default")
        );
        let is_staff = |name: &str| name.starts_with("dual_") || name == "staff_9";
        let mut dual = [Some("dual_3".to_owned()), Some("single_8".to_owned())];
        sabers(&mut dual, is_staff);
        assert_eq!(dual, [Some(SABER.to_owned()), Some(SABER.to_owned())]);
        let mut staff = [Some("staff_9".to_owned()), None];
        sabers(&mut staff, is_staff);
        assert_eq!(staff, [Some(STAFF.to_owned()), None]);
        let mut single = [Some("single_8".to_owned()), None];
        sabers(&mut single, is_staff);
        assert_eq!(single, [Some(SABER.to_owned()), None]);
        // `saber_colors_t` 4 as the game tints it, the colour the `color1` default gives.
        let held = equipment(HeldEquipment {
            primary_in_flight: false,
            kind: sjk_runtime::HeldItemKind::EnergyBlade,
            weapon: 3,
            active: true,
            color: [255, 51, 51],
            secondary_active: true,
            secondary_color: [230, 51, 255],
            trail_duration_millis: 40,
        });
        assert_eq!((held.color, held.secondary_color), (BLADE, BLADE));
        assert_eq!(BLADE, [51, 102, 255]);
        assert!(
            held.active && held.secondary_active,
            "only the colour changes"
        );
    }

    #[test]
    fn only_the_muted_slots_entities_are_muted() {
        assert!(
            entity_muted(1 << 4, 5),
            "a client's entity is its slot plus one"
        );
        assert!(!entity_muted(1 << 4, 4) && !entity_muted(1 << 4, 0));
        assert!(!entity_muted(u32::MAX, 40), "not a client");
        let players = MutedPlayers {
            slots: slots(1 << 4),
            ..MutedPlayers::default()
        };
        assert!(players.contains(4) && !players.contains(5));
    }

    #[test]
    fn the_slots_are_worked_out_only_when_their_inputs_change() {
        let mut players = MutedPlayers::default();
        let roster = [None; SLOTS];
        let asked = std::cell::Cell::new(0);
        let resolve = |muted| {
            let asked = &asked;
            move || {
                asked.set(asked.get() + 1);
                slots(muted)
            }
        };
        assert!(players.refresh(1, 1, 0, roster, 0, resolve(1 << 3)));
        assert!(players.contains(3));
        assert!(
            !players.refresh(1, 1, 0, roster, 0, resolve(0)),
            "nothing changed"
        );
        assert!(players.contains(3));
        assert!(
            players.refresh(2, 1, 0, roster, 0, resolve(0)),
            "the list changed"
        );
        assert!(!players.contains(3));
        assert!(
            !players.refresh(2, 1, 0, roster, 3, resolve(0)),
            "own slot moved"
        );
        assert_eq!(asked.get(), 3, "worked out only when an input changed");
        players.applied = 0b1000;
        assert!(players.clear());
        assert_eq!(
            players.applied, 0,
            "a new server starts with nothing applied"
        );
        assert_eq!(players.slots(), Slots::default());
        assert!(!players.clear(), "nothing left to forget");
    }
}
