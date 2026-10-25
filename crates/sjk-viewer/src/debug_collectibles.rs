//! `debug_collectibles`: the SJK team see every collectible as held, on this PC only, to
//! try them out without granting and taking them back at the hub (which would leave
//! those grants in its records and logs).
//!
//! It dresses a copy of the player's own profile (`Snapshot::me`) with every saber
//! shader, medal, achievement and card the client knows, for the screens that show what
//! the player holds: the Collection, the Profile, the Saber tab, the `saberskin` command
//! and the own blade drawn in the world. Nothing else reads it: the hub keeps the real
//! profile, the look sent to it stays what the player really holds (it refuses a skin
//! they lack), so other players see that; the new medal, achievement and holocron
//! pop-ups and the Staff page read the real profile too, so the seen lists and staff
//! actions are untouched. It only works while the hub says the player's key is staff.
//!
//! A growing blade (`blade_growth.rs`) shows fully grown in it, held or added, or held
//! the days [`GROWTH_DAYS_CVAR`] gives, so staff can see each stage: in the copy and in
//! the own blade drawn, never in the look sent.

use sjk_identity::{Snapshot, Status};

/// The switch: 1 shows every collectible as held (staff only), 0 the real profile.
pub(crate) const CVAR: &str = "debug_collectibles";

/// Not archived: the days held every growing blade shows while the mode is on, to see
/// each stage; -1 (the default), fully grown.
pub(crate) const GROWTH_DAYS_CVAR: &str = "debug_growthDays";

/// The note on every shader and medal the mode adds, which their cards show.
pub(crate) const NOTE: &str = "Debug collectibles: shown as yours on this PC only.";

/// Whether `grant` is one the mode added.
pub(crate) fn added(grant: &sjk_identity::Unlock) -> bool {
    grant.note == NOTE
}

/// What the mode is asked for on this PC ([`wanted`]).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Wanted {
    /// `debug_collectibles` is on.
    pub(crate) on: bool,
    /// The days held a growing blade shows ([`GROWTH_DAYS_CVAR`]); `None`, fully grown.
    pub(crate) growth_days: Option<u32>,
}

impl Wanted {
    /// The days held growing blade `growth` shows while the mode is on.
    pub(crate) fn days_of(self, growth: &crate::blade_growth::Growth) -> u32 {
        self.growth_days.unwrap_or_else(|| growth.full())
    }
}

/// What `debug_collectibles` and [`GROWTH_DAYS_CVAR`] ask for on `console`.
pub(crate) fn wanted(console: &crate::console::ViewerConsole) -> Wanted {
    Wanted {
        on: console.bool_cvar(CVAR) == Some(true),
        growth_days: console
            .integer_cvar(GROWTH_DAYS_CVAR)
            .and_then(|days| u32::try_from(days).ok()),
    }
}

/// Whether the mode applies to `snapshot`: `wanted`, the hub answered and says the
/// player's key is staff.
pub(crate) fn active(wanted: Wanted, snapshot: &Snapshot) -> bool {
    wanted.on
        && !matches!(snapshot.status, Status::Disabled | Status::NoHub)
        && snapshot.me.as_ref().is_some_and(|me| me.staff)
}

/// `snapshot` with every collectible held when the mode applies to it ([`active`]);
/// what the profile really holds keeps its dates, notes and counts, but a growing blade's
/// growth starts so that it shows the days `wanted` asks for.
pub(crate) fn dressed(wanted: Wanted, mut snapshot: Snapshot) -> Snapshot {
    if !active(wanted, &snapshot) {
        return snapshot;
    }
    let now = crate::blade_growth::now();
    let faces = crate::cards::faces::current();
    if let Some(me) = snapshot.me.as_mut() {
        dress(me, &faces.list, now, wanted);
    }
    snapshot
}

/// Give `me` every unlockable, medal and achievement the client knows and one copy of
/// each card of `faces` in every finish, as held at `now`; each growing blade held the
/// days `wanted` asks for.
fn dress(
    me: &mut sjk_identity::Profile,
    faces: &[crate::cards::faces::Face],
    now: i64,
    wanted: Wanted,
) {
    for unlockable in &crate::unlockables::ALL {
        if !me.unlocks.iter().any(|unlock| unlock.id == unlockable.id) {
            me.unlocks.push(sjk_identity::Unlock {
                id: unlockable.id.to_owned(),
                granted: now,
                since: 0,
                note: NOTE.to_owned(),
                count: 1,
                medal: None,
                card: None,
            });
        }
    }
    for unlock in &mut me.unlocks {
        if let Some(growth) = crate::blade_growth::of(&unlock.id) {
            unlock.since = crate::blade_growth::start_for(wanted.days_of(growth), now);
        }
    }
    for medal in crate::medals::Medal::ALL {
        if !me.medals.iter().any(|held| held.id == medal.id()) {
            me.medals.push(sjk_identity::Medal {
                id: medal.id().to_owned(),
                count: 1,
                awarded: now,
                note: NOTE.to_owned(),
            });
        }
    }
    for kind in &crate::achievements::ALL {
        match me.achievements.iter_mut().find(|held| held.id == kind.id) {
            Some(held) if held.unlocked > 0 => {}
            Some(held) => {
                held.progress = kind.goal;
                held.goal = kind.goal;
                held.unlocked = now;
            }
            None => me.achievements.push(sjk_identity::Achievement {
                id: kind.id.to_owned(),
                progress: kind.goal,
                goal: kind.goal,
                unlocked: now,
                reward: None,
            }),
        }
    }
    for face in faces {
        for finish in &crate::cards::FINISHES {
            let held = me.cards.iter().any(|stack| {
                stack.card.as_deref() == Some(face.id.as_str()) && stack.finish == finish.id
            });
            if !held {
                me.cards.push(sjk_identity::CardStack {
                    card: Some(face.id.clone()),
                    rarity: face.rarity.id.to_owned(),
                    finish: finish.id.to_owned(),
                    edition: crate::editions::ALPHA.to_owned(),
                    count: 1,
                    first: now,
                    last: now,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mode on, growing blades fully grown.
    const ON: Wanted = Wanted {
        on: true,
        growth_days: None,
    };

    fn snapshot(staff: bool) -> Snapshot {
        let me = sjk_identity::Profile {
            staff,
            unlocks: vec![sjk_identity::Unlock {
                id: "saber_sun".to_owned(),
                granted: 7,
                since: 0,
                note: "kept".to_owned(),
                count: 3,
                medal: None,
                card: None,
            }],
            ..crate::console::profile_panel::tests::me("")
        };
        crate::console::profile_panel::tests::snapshot(Some(me), None)
    }

    #[test]
    fn staff_who_want_it_hold_everything_and_keep_what_they_had() {
        let dressed = dressed(ON, snapshot(true));
        let me = dressed.me.as_ref().expect("a profile");
        assert_eq!(me.unlocks.len(), crate::unlockables::ALL.len());
        let sun = me.unlocks.iter().find(|unlock| unlock.id == "saber_sun");
        assert_eq!(sun.map(|sun| (sun.granted, sun.count)), Some((7, 3)));
        assert_eq!(me.medals.len(), crate::medals::Medal::COUNT);
        assert_eq!(me.achievements.len(), crate::achievements::ALL.len());
        assert!(me.achievements.iter().all(|held| held.unlocked > 0));
        let holdings = crate::unlockables::Holdings::of(true, Some(&dressed));
        assert!(crate::unlockables::blade_skins().all(|skin| holdings.can_wear(skin)));
        // The Unique ones too, which only their holders see otherwise.
        assert_eq!(
            holdings.listed_count(),
            crate::unlockables::blade_skins().count()
        );
    }

    #[test]
    fn cards_come_in_every_finish() {
        let mut me = crate::console::profile_panel::tests::me("");
        let face = crate::cards::faces::Face {
            id: "sol_001".to_owned(),
            name: "Sol".to_owned(),
            set: "sjk_alpha".to_owned(),
            set_name: "SJK Alpha".to_owned(),
            number: 1,
            kind: crate::cards::faces::Kind::Game,
            rarity: &crate::cards::RARITIES[2],
            lore: String::new(),
            artist: "Sol".to_owned(),
            art: String::new(),
            grants: None,
            blade: None,
        };
        dress(&mut me, std::slice::from_ref(&face), 5, ON);
        let finishes: Vec<_> = me.cards.iter().map(|stack| stack.finish.as_str()).collect();
        assert_eq!(finishes, ["normal", "holo", "solar"]);
        assert!(me.cards.iter().all(|stack| stack.rarity == "rare"));
    }

    #[test]
    fn only_staff_who_want_it_and_a_hub_that_answered() {
        for (on, staff) in [(false, true), (true, false)] {
            let wanted = Wanted {
                on,
                growth_days: None,
            };
            let dressed = dressed(wanted, snapshot(staff));
            assert_eq!(dressed.me.as_ref().map(|me| me.unlocks.len()), Some(1));
        }
        let mut no_hub = snapshot(true);
        no_hub.status = Status::NoHub;
        assert_eq!(dressed(ON, no_hub).me.map(|me| me.unlocks.len()), Some(1));
    }

    /// A growing blade shows fully grown in the mode, the one really held too (its grant's
    /// date kept), or held the days `debug_growthDays` gives.
    #[test]
    fn growing_blades_show_fully_grown_or_the_days_asked() {
        let _growing = crate::blade_growth::tests::fake("saber_sun", r#"{"glint":30,"haze":91}"#);
        let stage = |wanted: Wanted| {
            let dressed = dressed(wanted, snapshot(true));
            let holdings = crate::unlockables::Holdings::of(true, Some(&dressed));
            let sun = holdings.unlock("saber_sun").map(|sun| sun.granted);
            let stage =
                crate::blade_growth::Stage::of("saber_sun", &holdings, crate::blade_growth::now())
                    .map(|stage| (stage.held, stage.grown()));
            (sun, stage)
        };
        assert_eq!(stage(ON), (Some(7), Some((91, true))));
        let asked = Wanted {
            on: true,
            growth_days: Some(40),
        };
        assert_eq!(stage(asked), (Some(7), Some((40, false))));
        assert_eq!(asked.days_of(&crate::blade_growth::Growth::default()), 40);
    }
}
