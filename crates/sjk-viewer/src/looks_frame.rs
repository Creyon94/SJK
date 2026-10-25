//! The per-frame hook of looks (`looks.rs`): other players' looks from the SJK hub's
//! roster and feed, the local player's own from its settings, and the own look handed
//! to the identity service when it changes. A frame costs the feed's queue and the
//! roster's revision (two uncontended locks each) and the own Illuminate; the names
//! are compared and the table rebuilt only when a roster or an event comes, the own
//! look changes, or twice a second. Twice a second the own look also takes the parts of
//! the player's growing blades not grown yet off (`blade_growth.rs`), from the clock and
//! the profile as they are then.

use super::*;
use crate::looks::Worn;

impl GpuState {
    /// Bring the looks in line with the hub and the settings; `due` is the identity's
    /// twice-a-second turn, when the own blade skin is read again and given to the
    /// service.
    pub(crate) fn update_looks(&mut self, due: bool) {
        let session = self.live_session.as_ref();
        let server = session
            .filter(|session| !session.is_local())
            .map(|session| session.server());
        // Only the looks read for this server under the feed's current reading.
        let received = player_identity::take_looks(server);
        let mut changed = due;
        if self.looks.server != server {
            self.looks.clear(server);
            self.illuminate_others.clear();
            changed = true;
        }
        changed |= self.looks.follow_feed(received.generation);
        if server.is_some() {
            let revision = player_identity::revision();
            if self.looks.roster_revision != Some(revision) {
                self.looks.roster_revision = Some(revision);
                let looks = &mut self.looks;
                player_identity::with_roster(|players| {
                    looks.replace_roster(players.iter().map(|player| {
                        (
                            player.slot,
                            player.claimed_name.as_str(),
                            player.look.as_ref(),
                        )
                    }));
                });
                changed = true;
            }
            for event in &received.events {
                self.looks
                    .apply_event(event.slot, &event.claimed_name, &event.look());
                changed = true;
            }
        }
        let own_slot =
            session.and_then(|session| u8::try_from(session.game_state().client_num).ok());
        let lit = self.illuminate.lit();
        // The Hax's trap holds while away: hackers' clients may then hack the player away.
        let away_hackable = self.hax.on() && crate::hax::away_hackable(self.console.as_ref());
        let text = |name: &str| {
            self.console
                .as_ref()
                .and_then(|console| console.text_cvar(name).ok())
                .unwrap_or_default()
                .to_owned()
        };
        let options = if due {
            text(crate::saber_skin_options::CVAR)
        } else {
            String::new()
        };
        // The look the hub relays is what the player holds; `debug_collectibles` changes
        // only the blade drawn here (the hub refuses a skin the player lacks).
        let mut sent = None;
        let (saber_skin, options_off, sounds) = if due {
            let setting = text(crate::unlockables::SABER_SKIN_CVAR);
            let sounds = text(crate::saber_sound_choice::CVAR);
            #[cfg(test)]
            let shot = self.looks.shot_owns_unlocks;
            #[cfg(not(test))]
            let shot = false;
            let owned = |id: &str| shot || player_identity::owns_unlock(id);
            // A growing blade's parts not grown yet are off on top of the player's own
            // choice, worked out again from the clock and the profile at each turn, so a
            // stage reached while playing (or a growth start the hub moved) goes out with
            // the next look (a world shot's blades are fully grown).
            let now = crate::blade_growth::now();
            let locked = crate::blade_growth::locked_words(|skin| {
                if shot {
                    return Some(u32::MAX);
                }
                player_identity::with_own_unlock(skin, |unlock| {
                    crate::blade_growth::days_held(unlock, now)
                })
            });
            let held_options = crate::blade_growth::gated(&options, &locked);
            let held = Worn::own(&setting, &held_options, owned, lit).with_sounds(&sounds, owned);
            let wanted = self
                .console
                .as_ref()
                .map(crate::debug_collectibles::wanted)
                .unwrap_or_default();
            let debug = wanted.on && player_identity::is_staff();
            // `debug_collectibles` draws every growing blade fully grown, or held the days
            // `debug_growthDays` gives, as its Collection shows them.
            let (own, drawn_locked) = if debug {
                let locked = crate::blade_growth::locked_words(|skin| {
                    crate::blade_growth::of(skin).map(|growth| wanted.days_of(growth))
                });
                let drawn = crate::blade_growth::gated(&options, &locked);
                (
                    Worn::own(&setting, &drawn, |_| true, lit).with_sounds(&sounds, |_| true),
                    locked,
                )
            } else {
                (held, locked)
            };
            sent = Some((
                held.with_toy(crate::cardboard_box::TOY, self.cardboard_box.on())
                    .with_toy(crate::order66::TOY, self.order66.on())
                    .with_toy(crate::hax::TOY, self.hax.on())
                    .with_toy(crate::hax::AWAY_TOY, away_hackable)
                    .with_toy(crate::drone::TOY, self.drone.on()),
                held_options,
                drawn_locked,
            ));
            (own.saber_skin, own.options_off, own.sounds)
        } else {
            let own = self.looks.own();
            (own.saber_skin, own.options_off, own.sounds)
        };
        let own = Worn {
            saber_skin,
            illuminate: lit,
            options_off,
            toys: 0,
            sounds,
        }
        .with_toy(crate::cardboard_box::TOY, self.cardboard_box.on())
        .with_toy(crate::order66::TOY, self.order66.on())
        .with_toy(crate::hax::TOY, self.hax.on())
        .with_toy(crate::hax::AWAY_TOY, away_hackable)
        .with_toy(crate::drone::TOY, self.drone.on());
        if let Some((sent, held_options, drawn_locked)) = sent {
            player_identity::set_look(&sent.to_look(&held_options));
            self.looks.set_own_options(options, drawn_locked);
            // Who sits where, and the own name, for the saber shaders drawn from them.
            let slots = session.map_or_else(
                || [crate::saber_persona::Persona::default(); crate::saber_skins::MAX_CLIENTS],
                |session| crate::saber_persona::Persona::of_slots(session.game_state()),
            );
            let name = self
                .console
                .as_ref()
                .and_then(|console| console.text_cvar("name").ok())
                .unwrap_or_default();
            self.saber_skins
                .set_personas(slots, crate::saber_persona::Persona::of(name.as_bytes(), 0));
        }
        if own != self.looks.own() || own_slot != self.looks.own_slot() {
            self.looks.set_own(own_slot, own);
        }
        if changed {
            let game_state = session.map(|session| session.game_state());
            self.looks.rebuild(|slot| {
                game_state.and_then(|state| player_identity::shown_name(state, usize::from(slot)))
            });
        }
    }
}
