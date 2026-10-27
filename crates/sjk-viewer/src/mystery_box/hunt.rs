//! The box hunt (Sol's idea, 27/10/2026; `docs/mystery-boxes.md`, "Box hunt"): players
//! with the Box hunt toy on (`crate::box_hunt`) take part. Every few minutes the hub
//! picks one of them on the server as the mark; another who kills the mark wins a free
//! mystery box roll. Only players with it on see the mark or can be it. The hub never
//! sees kills: this client tells it who killed its player when its player is the mark,
//! so no one can claim a kill they did not make.

use super::label::{Label, Tone};
use crate::GpuState;
use glam::Vec3;
use sjk_identity::mystery_box::{self as wire, HuntEvent, HuntState};
use std::time::{Duration, Instant};

/// How often the hunt is read while the player takes part (a new mark is also told at
/// once by the hub's toy event).
const READ_EVERY: Duration = Duration::from_secs(20);
/// Soonest a read follows another asked by an event.
const READ_GAP: Duration = Duration::from_secs(2);
/// How high over a player's origin the mark's label stands, and the point it is seen by.
const MARK_RISE: f32 = 70.0;
const MARK_SIGHT: f32 = 20.0;
/// `MAX_CLIENTS`: slots of players.
const SLOTS: u16 = 32;

/// The hunt as this client follows it.
#[derive(Default)]
pub(crate) struct Hunt {
    /// The hunt last read, and the server it was read on.
    state: Option<(String, HuntState)>,
    /// The last read taken from the identity snapshot.
    seen: u64,
    /// When the hunt was last asked for.
    asked: Option<Instant>,
    /// Asked for at once (an event changed it).
    stale: bool,
    /// The obituaries counted so far.
    obituaries: u64,
    /// The mark last told to the player, so each is told once.
    told: Option<u8>,
}

impl Hunt {
    fn mark(&self) -> Option<&wire::HuntMark> {
        self.state
            .as_ref()
            .filter(|(_, state)| state.hunting)
            .and_then(|(_, state)| state.mark.as_ref())
    }

    /// Whether the hunt should be read at `now`.
    fn due(&self, now: Instant) -> bool {
        self.asked.is_none_or(|asked| {
            let since = now.saturating_duration_since(asked);
            since >= READ_EVERY || (self.stale && since >= READ_GAP)
        })
    }
}

/// What the player is told when slot `slot` (named `name`) becomes the mark; `own` is
/// the player's slot.
fn mark_words(slot: u8, own: u8, name: &str) -> (bool, String) {
    if slot == own {
        (
            true,
            "You are the mark!\nBox hunters win a free mystery box roll by killing you".to_owned(),
        )
    } else {
        (
            false,
            format!("Box hunt: {name}^7 is the mark. Kill them for a free mystery box roll"),
        )
    }
}

impl GpuState {
    /// The frame's box hunt: read the hunt, take the hub's events of it (`events`, this
    /// server's toy events), tell the hub who killed the player when they are the mark.
    pub(super) fn update_box_hunt(
        &mut self,
        events: &[sjk_identity::ToyEvent],
        server: &str,
        own: u8,
        now: Instant,
    ) {
        let on = self.box_hunt.on();
        let hunt = &mut self.mystery_boxes.hunt;
        // A read for this server.
        let read = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .box_hunt
                .as_ref()
                .filter(|(serial, read_on, _)| *serial != hunt.seen && read_on == server)
                .cloned()
        })
        .flatten();
        if let Some((serial, read_on, state)) = read {
            hunt.seen = serial;
            self.mystery_boxes.free = state.free;
            hunt.state = Some((read_on, state));
        }
        if hunt
            .state
            .as_ref()
            .is_some_and(|(read_on, _)| read_on != server)
        {
            hunt.state = None;
            hunt.told = None;
        }
        let mut lines = Vec::new();
        for event in events.iter().filter(|event| event.toy == wire::HUNT_TOY) {
            let Some(said) = wire::parse_hunt_event(&event.event) else {
                continue;
            };
            hunt.stale = true;
            if !on {
                continue;
            }
            match said {
                HuntEvent::Mark(slot) if hunt.told != Some(slot) => {
                    hunt.told = Some(slot);
                    lines.push(mark_words(slot, own, &event.claimed_name));
                }
                HuntEvent::Hunted { slot, won } => {
                    hunt.told = None;
                    lines.push(if slot == own && won {
                        (
                            true,
                            "You hunted the mark!\nFree mystery box roll".to_owned(),
                        )
                    } else if slot == own {
                        (
                            false,
                            "Box hunt: you hunted the mark (no free roll: you hold the most \
                             or won enough today)"
                                .to_owned(),
                        )
                    } else {
                        (
                            false,
                            format!("Box hunt: {}^7 hunted the mark", event.claimed_name),
                        )
                    });
                }
                HuntEvent::Mark(_) => {}
            }
        }
        // The mark read without its event (joining mid-hunt): told once too.
        let read_mark = hunt.mark().map(|mark| (mark.slot, mark.name.clone()));
        if on
            && let Some((slot, name)) = read_mark
            && hunt.told != Some(slot)
        {
            hunt.told = Some(slot);
            lines.push(mark_words(slot, own, &name));
        }
        // The player, the mark, killed by another player: the hub is told by whom.
        let decoded = self.obituaries.decoded();
        let new = decoded.saturating_sub(hunt.obituaries).min(8) as usize;
        hunt.obituaries = decoded;
        let marked = on && hunt.mark().is_some_and(|mark| mark.slot == own);
        let killer = (0..new)
            .filter_map(|offset| self.obituaries.feed().newest(offset))
            .find(|kill| {
                kill.target == u16::from(own)
                    && kill.attacker < SLOTS
                    && kill.attacker != kill.target
            })
            .map(|kill| kill.attacker as u8);
        // Read when due: while taking part, while a box stands here (free rolls), or
        // when the hunt is not known yet.
        let wanted = on || !self.mystery_boxes.here.is_empty();
        let read_now = wanted && hunt.due(now);
        if read_now {
            hunt.asked = Some(now);
            hunt.stale = false;
        }
        if let (true, Some(by)) = (marked, killer) {
            hunt.asked = Some(now);
            crate::player_identity::with_service(|service| {
                service.box_hunt_killed(server.to_owned(), by);
            });
        } else if read_now {
            crate::player_identity::with_service(|service| {
                service.read_box_hunt(server.to_owned());
            });
        }
        if !on {
            self.mystery_boxes.hunt.told = None;
        }
        for (centre, text) in lines {
            if centre {
                self.mystery_box_centre(text);
            } else {
                self.mystery_box_print(text);
            }
        }
    }

    /// The mark's label over their head, while the player takes part and someone else
    /// is the mark: the point it stands at, the point it is seen by, and the label.
    pub(super) fn box_hunt_mark_label(&mut self) -> Option<(Vec3, Vec3, Label)> {
        if !self.box_hunt.on() {
            return None;
        }
        let slot = self.mystery_boxes.hunt.mark()?.slot;
        let session = self.live_session.as_ref()?;
        if i32::from(slot) == session.game_state().client_num {
            return None;
        }
        let time = self.presentation_clock.sample(Instant::now());
        let presented = self
            .live_world
            .entity(sjk_runtime::EntityId::new(u64::from(slot) + 1))?;
        let origin = Vec3::from_array(presented.sample(time).translation);
        Some((
            origin + Vec3::Z * MARK_RISE,
            origin + Vec3::Z * MARK_SIGHT,
            Label {
                headline: "MARK".to_owned(),
                line: "Kill for a free box roll".to_owned(),
                tone: Tone::Mark,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_is_told_to_themselves_and_to_hunters() {
        let (centre, words) = mark_words(3, 3, "Kyle");
        assert!(centre);
        assert!(words.starts_with("You are the mark!"));
        let (centre, words) = mark_words(3, 5, "Kyle");
        assert!(!centre);
        assert!(words.contains("Kyle^7 is the mark"));
    }

    #[test]
    fn the_hunt_is_read_every_while_and_soon_after_an_event() {
        let now = Instant::now();
        let mut hunt = Hunt::default();
        assert!(hunt.due(now));
        hunt.asked = Some(now);
        assert!(!hunt.due(now + Duration::from_secs(5)));
        assert!(hunt.due(now + READ_EVERY));
        hunt.stale = true;
        assert!(!hunt.due(now + Duration::from_secs(1)));
        assert!(hunt.due(now + READ_GAP));
    }
}
