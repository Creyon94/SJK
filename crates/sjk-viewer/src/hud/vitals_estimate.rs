//! Estimated health and shield (armour) of other players.
//!
//! The server sends a player's health and armour only to that player (and to
//! teammates through `tinfo`), but what it sends about everyone leaks most of
//! it. Each player is kept as a [`Range`] built from the server's own rules:
//!
//! - **Spawn** (`ClientSpawn`): a quarter over the maximum health and a quarter
//!   of it as armour, 125 and 25 in stock games, 100 and none in Duel. Learnt from
//!   the local player's own spawns when it has been seen, for mods that change it.
//! - **Decay** (`ClientTimerActions`): health and armour over the maximum lose a
//!   point a second.
//! - **Pain** (`P_DamageFeedback`): a hit of ten or more, at most every 700 ms,
//!   raises `EV_PAIN` on the player with the health left: exact.
//! - **Shield hits** (`EV_SHIELD_HIT`): how much armour a hit took, exactly.
//! - **Saber hits** (`EV_SABER_HIT`): the damage, in three sizes (under 5, under
//!   20, more). A missile that strikes a player (`EV_MISSILE_HIT`) is a hit of
//!   unknown size. With no pain after it, outside the 700 ms, a hit was under ten.
//! - **The local player's hits** (`PERS_ATTACKEE_ARMOR`): the health and armour
//!   of whoever it last hit, before the hit, matched to its victim.
//! - **Falls** (`EV_FALL`, `EV_ROLL`): the landing's damage, exactly.
//! - **Pickups** (`EV_ITEM_PICKUP`): medpacks and shields add their amount.
//! - **Force heal** (its sound at the player): 5, 10 or 25 by level.
//! - **Drain** heals the drainer by what it takes, an unknown amount.
//! - **Deaths** (`EF_DEAD`, `EV_OBITUARY`) and the respawn after one. A spawn
//!   with no death seen (a round or map restart) toggles `EF_TELEPORT_BIT`, as a
//!   teleport does, so that only raises the high bound to the spawn values.
//!
//! What is missed (lightning, hurt triggers, splash under ten, anything unseen
//! while a player is out of view) widens the range rather than moving the guess.
//!
//! Servers can hide the pain values: JAPro's `g_stopHealthESP` sends a fixed 50 or
//! no pain event at all. The local player's own pain events are checked against
//! its real health ([`PainReport`]); until one has been checked, pain values are
//! trusted but their absence proves nothing.
use super::estimate::Range;
use sjk_game_jka::items::{ITEMS, Kind};
use sjk_protocol::{EntityState, GameState, Snapshot};

const CLIENTS: usize = 32;
const ENTITIES: usize = 1_024;
const ET_PLAYER: u8 = 1;
const ET_ITEM: u8 = 2;
const ET_EVENTS: u8 = 18;
const EF_DEAD: u32 = 2;
const EF_TELEPORT_BIT: u32 = 1 << 3;
const EVENT_MASK: u16 = 0xff;
const EV_FALL: u16 = 11;
const EV_ROLL: u16 = 17;
const EV_ITEM_PICKUP: u16 = 22;
const EV_SABER_HIT: u16 = 30;
const EV_GENERAL_SOUND: u16 = 76;
const EV_MISSILE_HIT: u16 = 85;
const EV_PAIN: u16 = 89;
const EV_OBITUARY: u16 = 93;
const EV_FORCE_DRAINED: u16 = 96;
const EV_SHIELD_HIT: u16 = 110;
/// `CS_SOUNDS`.
const CS_SOUNDS: usize = 811;
/// `PERS_HITS`, `PERS_ATTACKEE_ARMOR`.
const PERS_HITS: usize = 1;
const PERS_ATTACKEE_ARMOR: usize = 7;
/// `FP_PROTECT`, `FP_DRAIN`.
const FP_PROTECT: u32 = 9;
const FP_DRAIN: u32 = 13;
/// `GT_DUEL`, `GT_POWERDUEL`: no armour and no extra health at spawn.
const GT_DUEL: i32 = 3;
const GT_POWERDUEL: i32 = 4;
/// `P_DamageFeedback`'s pain debounce.
const PAIN_DEBOUNCE: i32 = 700;
/// Under ten a frame raises no pain; a snapshot may hold two server frames.
const UNFELT: f32 = 18.0;
/// A player unseen this long (server milliseconds) may have changed out of view.
const GAP_MILLIS: i32 = 1_500;
/// How much health a player out of view may lose or regain a second, for the bounds.
const UNSEEN_LOSS: f32 = 10.0;
const UNSEEN_GAIN: f32 = 5.0;
/// A player respawns at least this long after dying.
const RESPAWN_MILLIS: i32 = 1_000;
/// After this long dead and unseen, a respawn's values say little.
const RESPAWN_TRUSTED: i32 = 10_000;
/// Damage of a saber hit by its `EV_SABER_HIT` size (`WP_SaberDoHit`): 3 under 5,
/// 2 under 20, 1 for more (whose top is a guess at the heaviest blows).
const SABER_SMALL: (f32, f32, f32) = (1.0, 3.0, 4.0);
const SABER_MEDIUM: (f32, f32, f32) = (5.0, 12.0, 19.0);
const SABER_LARGE: (f32, f32, f32) = (20.0, 40.0, 150.0);
/// A missile's damage: any weapon's.
const MISSILE: (f32, f32, f32) = (1.0, 20.0, 100.0);
/// Force heal by level, 1 to 3, the guess level 3's.
const HEAL: (f32, f32, f32) = (5.0, 25.0, 25.0);
/// Health a drainer may gain per millisecond (up to 4 a 100 ms tick), and the guess.
const DRAIN_GAIN: (f32, f32) = (0.01, 0.04);
/// Remote pains all at exactly 50 in a row before the value is taken as masked.
const MASKED_FIFTIES: u8 = 4;
/// Heal sound attributed to the player within this distance of it.
const HEAL_REACH: f32 = 64.0;

/// Whether the server's pain events carry real health.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum PainReport {
    /// Not checked yet: values trusted, silence proves nothing.
    #[default]
    Unchecked,
    /// The local player's pains matched its health.
    Honest,
    /// They did not (or every pain says 50): values ignored.
    Masked,
}

/// One player's estimate.
#[derive(Clone, Copy, Debug, Default)]
struct Track {
    health: Range,
    armor: Range,
    /// Seen alive since the last death or reset.
    alive: bool,
    /// When it was last known to die, until seen alive again.
    died: Option<i32>,
    last_seen: i32,
    /// Until when no new pain event can come (`pain_debounce_time`).
    pain_until: i32,
    /// `EF_TELEPORT_BIT` when last seen.
    teleport: bool,
}

/// What one snapshot shows about one player.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Facts {
    pain: Option<u8>,
    /// Armour taken by shield hits.
    absorbed: f32,
    shield_hit: bool,
    /// Damage dealt before armour, when struck.
    hit: Option<Range>,
    /// The hit was a missile's (which some weapons split with the armour).
    missile: bool,
    fall: f32,
    heals: u8,
    pickups: [u8; 2],
    pickup_count: u8,
    died: bool,
    drained: bool,
    /// Health and armour just before the local player's hit.
    attackee: Option<(f32, f32)>,
}

/// What the server's rules give a player at spawn.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Profile {
    max_health: f32,
    /// Learnt from the local player's own spawn: health, armour.
    learnt: Option<(f32, f32)>,
}

impl Profile {
    fn spawn(&self, mode: i32) -> (f32, f32) {
        self.learnt
            .unwrap_or(if matches!(mode, GT_DUEL | GT_POWERDUEL) {
                (100.0, 0.0)
            } else {
                (
                    (self.max_health * 1.25).floor(),
                    (self.max_health * 0.25).floor(),
                )
            })
    }
}

/// Context of one player's step.
#[derive(Clone, Copy, Debug)]
struct Step {
    time: i32,
    max_health: f32,
    spawn: (f32, f32),
    pain: PainReport,
    protected: bool,
    draining: bool,
    /// `EF_TELEPORT_BIT`, which toggles at every spawn and teleport.
    teleport: bool,
}

/// Fixed-capacity estimator for every client slot, observed once per snapshot.
pub(super) struct Estimator {
    tracks: [Track; CLIENTS],
    profile: Profile,
    honest: u16,
    masked: u16,
    fifties: u8,
    /// Previous event signatures and presence, as cgame's `previousEvent`.
    signatures: Box<[u16; ENTITIES]>,
    present: Box<[bool; ENTITIES]>,
    /// The item (`bg_itemlist` index) each item entity was last seen as.
    items: Box<[u8; ENTITIES]>,
    own_event: u16,
    own_hits: u32,
    own_alive: bool,
    own_died: bool,
    last_time: i32,
    /// Drained this snapshot, for the Force estimate.
    drained: u32,
}

impl Default for Estimator {
    fn default() -> Self {
        Self {
            tracks: [Track::default(); CLIENTS],
            profile: Profile {
                max_health: 100.0,
                learnt: None,
            },
            honest: 0,
            masked: 0,
            fifties: 0,
            signatures: Box::new([0; ENTITIES]),
            present: Box::new([false; ENTITIES]),
            items: Box::new([0; ENTITIES]),
            own_event: 0,
            own_hits: 0,
            own_alive: false,
            own_died: false,
            last_time: i32::MIN,
            drained: 0,
        }
    }
}

impl Estimator {
    /// Health of `slot` (points), if tracked alive.
    pub(super) fn health(&self, slot: u16) -> Option<Range> {
        let track = self.tracks.get(usize::from(slot))?;
        track.alive.then_some(track.health)
    }

    /// Armour of `slot` (points), if tracked alive.
    pub(super) fn armor(&self, slot: u16) -> Option<Range> {
        let track = self.tracks.get(usize::from(slot))?;
        track.alive.then_some(track.armor)
    }

    /// A full bar's worth of health or armour.
    pub(super) fn full(&self) -> f32 {
        self.profile.max_health
    }

    /// Whether the server's pain events can be trusted.
    pub(super) fn pain_report(&self) -> PainReport {
        if self.masked >= 2 && self.masked > self.honest {
            PainReport::Masked
        } else if self.honest > 0 {
            PainReport::Honest
        } else if self.fifties >= MASKED_FIFTIES {
            PainReport::Masked
        } else {
            PainReport::Unchecked
        }
    }

    /// Clients drained (`EV_FORCE_DRAINED`) in the last observed snapshot, as bits.
    pub(super) fn drained(&self) -> u32 {
        self.drained
    }

    /// Take one accepted snapshot; every snapshot must be observed once, in order.
    pub(super) fn observe(&mut self, snapshot: &Snapshot, game: &GameState, mode: i32) {
        let time = snapshot.server_time;
        if time < self.last_time {
            // A new map or a restart.
            let profile = self.profile;
            *self = Self {
                profile,
                ..Self::default()
            };
        }
        if time == self.last_time {
            return;
        }
        self.last_time = time;
        let local = snapshot.player.client_num();
        let mut facts = [Facts::default(); CLIENTS];
        let mut local_victims = 0_u32;
        self.learn_local(snapshot);
        self.scan(snapshot, game, &mut facts, &mut local_victims);
        self.attribute_own_hit(snapshot, &mut facts, local_victims);
        self.drained = facts
            .iter()
            .enumerate()
            .filter(|(_, fact)| fact.drained)
            .fold(0_u32, |bits, (slot, _)| bits | 1 << slot);
        let (max_health, spawn, pain) = (
            self.profile.max_health,
            self.profile.spawn(mode),
            self.pain_report(),
        );
        let step = |entity: &EntityState| Step {
            time,
            max_health,
            spawn,
            pain,
            protected: entity.force_powers_active() & (1 << FP_PROTECT) != 0,
            draining: entity.force_powers_active() & (1 << FP_DRAIN) != 0,
            teleport: entity.e_flags() & EF_TELEPORT_BIT != 0,
        };
        for entity in &snapshot.entities {
            let number = entity.number();
            if entity.entity_type() != ET_PLAYER || number >= CLIENTS as u16 || number == local {
                continue;
            }
            let context = step(entity);
            let slot = usize::from(number);
            let dead = entity.e_flags() & EF_DEAD != 0;
            advance(&mut self.tracks[slot], &facts[slot], dead, context);
        }
        // Players out of view who died there.
        for (slot, fact) in facts.iter().enumerate() {
            if fact.died && self.tracks[slot].last_seen != time {
                let track = &mut self.tracks[slot];
                track.alive = false;
                track.died = Some(time);
            }
        }
    }

    /// The local player: its spawn values, and its pain events against its health.
    fn learn_local(&mut self, snapshot: &Snapshot) {
        let player = &snapshot.player;
        if player.max_health() > 0 {
            self.profile.max_health = player.max_health() as f32;
        }
        let alive = player.health() > 0 && !player.is_spectator();
        if alive && !self.own_alive && self.own_died {
            self.profile.learnt = Some((player.health() as f32, player.armor().max(0) as f32));
        }
        if player.health() <= 0 && !player.is_spectator() {
            self.own_died = true;
        }
        self.own_alive = alive;
        let event = player.external_event();
        if event != self.own_event && event & EVENT_MASK == EV_PAIN && alive {
            if i32::from(player.external_event_parameter()) == player.health() {
                self.honest = self.honest.saturating_add(1);
            } else {
                self.masked = self.masked.saturating_add(1);
            }
        }
        self.own_event = event;
    }

    /// New events of the snapshot, sorted by the player they are about.
    fn scan(
        &mut self,
        snapshot: &Snapshot,
        game: &GameState,
        facts: &mut [Facts; CLIENTS],
        local_victims: &mut u32,
    ) {
        let local = snapshot.player.client_num();
        let dmflags = super::identification::info_number(game.config_string(0), "dmflags") as u32;
        let mut seen = [false; ENTITIES];
        for entity in &snapshot.entities {
            let number = usize::from(entity.number());
            if number >= ENTITIES {
                continue;
            }
            seen[number] = true;
            if entity.entity_type() == ET_ITEM {
                self.items[number] = entity.model_index() as u8;
            }
            let temporary = entity.entity_type() >= ET_EVENTS;
            let signature = if temporary {
                u16::from(entity.entity_type())
            } else {
                entity.event()
            };
            if signature == 0 || (self.present[number] && self.signatures[number] == signature) {
                self.present[number] = true;
                self.signatures[number] = signature;
                continue;
            }
            self.present[number] = true;
            self.signatures[number] = signature;
            let event = if temporary {
                u16::from(entity.entity_type() - ET_EVENTS)
            } else {
                signature & EVENT_MASK
            };
            let about = |slot: u16| (slot < CLIENTS as u16).then_some(usize::from(slot));
            let player = !temporary && entity.entity_type() == ET_PLAYER;
            match event {
                EV_SHIELD_HIT => {
                    if let Some(slot) = about(entity.other_entity_num()) {
                        let absorbed = entity.integer_field(61).unwrap_or(0).max(0) as f32;
                        facts[slot].absorbed += absorbed;
                        facts[slot].shield_hit = true;
                    }
                }
                EV_SABER_HIT => {
                    let size = match entity.event_parameter() {
                        3 => SABER_SMALL,
                        2 => SABER_MEDIUM,
                        1 => SABER_LARGE,
                        _ => continue,
                    };
                    if let Some(slot) = about(entity.other_entity_num()) {
                        struck(&mut facts[slot], size);
                        if entity.other_entity_num2() == local {
                            *local_victims |= 1 << slot;
                        }
                    }
                }
                EV_MISSILE_HIT => {
                    if let Some(slot) = about(entity.other_entity_num()) {
                        struck(&mut facts[slot], MISSILE);
                        facts[slot].missile = true;
                    }
                }
                EV_OBITUARY => {
                    if let Some(slot) = about(entity.other_entity_num()) {
                        facts[slot].died = true;
                    }
                }
                EV_FORCE_DRAINED => {
                    if let Some(slot) = about(entity.owner()) {
                        facts[slot].drained = true;
                    }
                }
                EV_GENERAL_SOUND if temporary => {
                    if is_heal_sound(game, entity.event_parameter())
                        && let Some(slot) = nearest_player(snapshot, entity.trajectory_base())
                    {
                        facts[slot].heals = facts[slot].heals.saturating_add(1);
                    }
                }
                EV_PAIN if player => {
                    if let Some(slot) = about(entity.number()) {
                        facts[slot].pain = Some(entity.event_parameter());
                    }
                }
                EV_ITEM_PICKUP if player => {
                    let item = self.items[usize::from(entity.event_parameter())];
                    if let Some(slot) = about(entity.number())
                        && item != 0
                    {
                        let fact = &mut facts[slot];
                        if usize::from(fact.pickup_count) < fact.pickups.len() {
                            fact.pickups[usize::from(fact.pickup_count)] = item;
                            fact.pickup_count += 1;
                        }
                    }
                }
                EV_FALL | EV_ROLL if player => {
                    let delta = i32::from(entity.event_parameter());
                    if let (Some(slot), Some(damage)) = (
                        about(entity.number()),
                        sjk_game_jka::damage::fall_damage(delta, dmflags, false),
                    ) {
                        facts[slot].fall += damage as f32;
                    }
                }
                _ => {}
            }
        }
        for (present, seen) in self.present.iter_mut().zip(seen) {
            *present &= seen;
        }
        // Remote pains all at 50 are a masked value (`g_stopHealthESP 1`).
        if self.pain_report() == PainReport::Unchecked {
            for fact in facts.iter() {
                match fact.pain {
                    Some(50) => self.fifties = self.fifties.saturating_add(1),
                    Some(_) => self.fifties = 0,
                    None => {}
                }
            }
        }
    }

    /// `PERS_ATTACKEE_ARMOR`: when the local player's hit count moved, the health
    /// and armour of whoever it hit, before the hit. Matched to a victim when only
    /// one fits: its saber's victim, its duel opponent, or the one player whose
    /// estimate allows the values.
    fn attribute_own_hit(
        &mut self,
        snapshot: &Snapshot,
        facts: &mut [Facts; CLIENTS],
        local_victims: u32,
    ) {
        let player = &snapshot.player;
        let hits = player.persistent[PERS_HITS];
        if hits == self.own_hits {
            return;
        }
        self.own_hits = hits;
        let value = player.persistent[PERS_ATTACKEE_ARMOR];
        let (health, armor) = ((value >> 8) as f32, (value & 0xff) as f32);
        // A masked value (`g_stopHealthESP`) has no health.
        if health < 1.0 {
            return;
        }
        let victim = if local_victims.count_ones() == 1 {
            Some(local_victims.trailing_zeros() as usize)
        } else if player.duel_in_progress() {
            Some(usize::from(player.duel_index())).filter(|slot| *slot < CLIENTS)
        } else {
            let time = self.last_time;
            let fits =
                |range: Range, value: f32| range.low - 0.5 <= value && value <= range.high + 0.5;
            let mut candidates = self.tracks.iter().enumerate().filter(|(_, track)| {
                track.alive
                    && time - track.last_seen <= GAP_MILLIS
                    && fits(track.health, health)
                    && fits(track.armor, armor)
            });
            match (candidates.next(), candidates.next()) {
                (Some((slot, _)), None) => Some(slot),
                _ => None,
            }
        };
        if let Some(slot) = victim
            && slot != usize::from(player.client_num())
        {
            facts[slot].attackee = Some((health, armor));
        }
    }
}

/// Add a hit of `size` damage.
fn struck(fact: &mut Facts, (low, best, high): (f32, f32, f32)) {
    let hit = fact.hit.unwrap_or(Range::exact(0.0));
    fact.hit = Some(hit.add(low, best, high));
}

/// Whether sound `index` is Force heal's.
fn is_heal_sound(game: &GameState, index: u8) -> bool {
    const HEAL_SOUND: &[u8] = b"weapons/force/heal.wav";
    game.config_string(CS_SOUNDS + usize::from(index))
        .and_then(|name| {
            name.len()
                .checked_sub(HEAL_SOUND.len())
                .map(|at| &name[at..])
        })
        .is_some_and(|tail| tail.eq_ignore_ascii_case(HEAL_SOUND))
}

/// The player standing at `origin`, if one is close enough.
fn nearest_player(snapshot: &Snapshot, origin: [f32; 3]) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    let mut consider = |slot: usize, at: [f32; 3]| {
        let distance = (0..3)
            .map(|i| (at[i] - origin[i]).powi(2))
            .sum::<f32>()
            .sqrt();
        if distance <= HEAL_REACH && best.is_none_or(|(_, d)| distance < d) {
            best = Some((slot, distance));
        }
    };
    for entity in &snapshot.entities {
        if entity.entity_type() == ET_PLAYER && entity.number() < CLIENTS as u16 {
            consider(usize::from(entity.number()), entity.trajectory_base());
        }
    }
    best.map(|(slot, _)| slot)
}

/// Raise each part of `range` below `cap` by its amount, no further than `cap`.
fn gain(range: Range, (low, best, high): (f32, f32, f32), cap: f32) -> Range {
    let raise = |value: f32, amount: f32| {
        if value < cap {
            (value + amount).min(cap)
        } else {
            value
        }
    };
    Range::new(
        raise(range.low, low),
        raise(range.best, best),
        raise(range.high, high),
    )
}

/// One player's estimate through one snapshot.
fn advance(track: &mut Track, facts: &Facts, dead: bool, step: Step) {
    let Step {
        time, max_health, ..
    } = step;
    if dead {
        if track.alive || track.died.is_none() {
            track.died = Some(time);
        }
        track.alive = false;
        track.last_seen = time;
        return;
    }
    if !track.alive {
        appear(track, step);
    } else {
        let elapsed = time - track.last_seen;
        if elapsed > GAP_MILLIS {
            let seconds = elapsed as f32 / 1_000.0;
            track.health = gain(
                track.health.add(-UNSEEN_LOSS * seconds, 0.0, 0.0),
                (0.0, 0.0, UNSEEN_GAIN * seconds),
                max_health,
            );
            track.armor = track.armor.add(-UNSEEN_LOSS * seconds, 0.0, 0.0);
        }
        // `ClientTimerActions`: over the maximum, a point a second.
        let decay = |value: f32| {
            if value > max_health {
                (value - elapsed.max(0) as f32 / 1_000.0).max(max_health)
            } else {
                value
            }
        };
        track.health = track.health.map(decay);
        track.armor = track.armor.map(decay);
        if step.draining {
            let elapsed = elapsed.clamp(0, GAP_MILLIS) as f32;
            let (best, high) = DRAIN_GAIN;
            track.health = gain(
                track.health,
                (0.0, best * elapsed, high * elapsed),
                max_health,
            );
        }
        if step.teleport != track.teleport {
            // A teleport, or a spawn with no death seen (a round or map restart).
            let (health, armor) = step.spawn;
            track.health = Range::new(
                track.health.low,
                track.health.best,
                track.health.high.max(health),
            );
            track.armor = Range::new(
                track.armor.low,
                track.armor.best,
                track.armor.high.max(armor),
            );
        }
    }
    track.teleport = step.teleport;
    hurt(track, facts, step);
    track.health = track.health.clamp(1.0, 255.0);
    track.armor = track.armor.clamp(0.0, 255.0);
    track.last_seen = time;
}

/// A player seen alive again: freshly spawned after a death seen not long ago,
/// otherwise anything a living player can be.
fn appear(track: &mut Track, step: Step) {
    let Step {
        time,
        max_health,
        spawn: (health, armor),
        ..
    } = step;
    *track = match track.died {
        Some(died) if time - died <= RESPAWN_TRUSTED => {
            // Spawned at least a second after dying; decaying since then.
            let decayed = ((time - died - RESPAWN_MILLIS).max(0) as f32 / 1_000.0)
                .min((health - max_health).max(0.0));
            Track {
                health: Range::new(health - decayed, health - decayed * 0.5, health),
                armor: Range::exact(armor),
                ..Track::default()
            }
        }
        _ => Track {
            health: Range::new(1.0, max_health, health.max(max_health)),
            armor: Range::new(0.0, armor, max_health),
            ..Track::default()
        },
    };
    track.alive = true;
}

/// Apply one snapshot's damage, healing and pain to a living player.
fn hurt(track: &mut Track, facts: &Facts, step: Step) {
    let Step {
        time,
        max_health,
        pain,
        protected,
        ..
    } = step;
    if let Some((health, armor)) = facts.attackee {
        track.health = Range::exact(health);
        track.armor = Range::exact(armor);
    }
    if let Some(hit) = facts.hit {
        let absorbed = facts.absorbed;
        if facts.shield_hit {
            // The armour held at least what it took.
            track.armor = track
                .armor
                .at_least(absorbed)
                .add(-absorbed, -absorbed, -absorbed);
        } else {
            // A hit with no shield flash met no armour.
            track.armor = Range::new(0.0, 0.0, track.armor.high);
        }
        // No pain from a server known to send it, outside the debounce: the
        // frame's damage was under ten.
        let mut hit = hit;
        if pain == PainReport::Honest && facts.pain.is_none() && time >= track.pain_until {
            hit = hit.at_most(UNFELT);
        }
        // What got past the armour: nothing while some is surely left (a missile
        // may split it), else the rest of the hit.
        let through = if facts.shield_hit && track.armor.low > 0.0 {
            if facts.missile {
                Range::new(0.0, 0.0, absorbed)
            } else {
                Range::exact(0.0)
            }
        } else {
            hit.map(|damage| (damage - absorbed).max(0.0))
        };
        // Protect may take any of it.
        let least = if protected { 0.0 } else { through.low };
        track.health = track.health.add(-through.high, -through.best, -least);
    } else if facts.shield_hit {
        track.armor = track.armor.at_least(facts.absorbed).add(
            -facts.absorbed,
            -facts.absorbed,
            -facts.absorbed,
        );
    }
    if facts.fall > 0.0 {
        track.health = track.health.add(-facts.fall, -facts.fall, -facts.fall);
    }
    for item in &facts.pickups[..usize::from(facts.pickup_count)] {
        let Some(row) = ITEMS.get(usize::from(*item)) else {
            continue;
        };
        let quantity = row.quantity as f32;
        match row.kind {
            Kind::Armor => {
                let cap = max_health * row.tag as f32;
                track.armor = track
                    .armor
                    .at_most(cap - 1.0)
                    .map(|value| (value + quantity).min(cap));
            }
            Kind::Health => {
                let cap = if matches!(row.quantity, 5 | 100) {
                    max_health * 2.0
                } else {
                    max_health
                };
                track.health = track
                    .health
                    .at_most(cap - 1.0)
                    .map(|value| (value + quantity).min(cap));
            }
            _ => {}
        }
    }
    for _ in 0..facts.heals {
        // Heal only works below the maximum.
        track.health = gain(track.health.at_most(max_health - 1.0), HEAL, max_health);
    }
    if let Some(value) = facts.pain {
        track.pain_until = time + PAIN_DEBOUNCE;
        if pain == PainReport::Masked {
            // Ten or more landed, some of it perhaps on the armour.
            let least = if track.armor.high < 1.0 { 10.0 } else { 0.0 };
            track.health = track.health.add(-60.0, -15.0, -least);
        } else {
            track.health = Range::exact(f32::from(value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(time: i32) -> Step {
        Step {
            time,
            max_health: 100.0,
            spawn: (125.0, 25.0),
            pain: PainReport::Honest,
            protected: false,
            draining: false,
            teleport: false,
        }
    }

    /// `range` within a tenth of a point of `(low, best, high)`: a few
    /// milliseconds of decay over the maximum is not the point of a test.
    fn close(range: Range, (low, best, high): (f32, f32, f32)) {
        let near = |a: f32, b: f32| (a - b).abs() < 0.1;
        assert!(
            near(range.low, low) && near(range.best, best) && near(range.high, high),
            "{range:?} is not {:?}",
            (low, best, high)
        );
    }

    fn spawned(time: i32) -> Track {
        let mut track = Track::default();
        advance(&mut track, &Facts::default(), true, step(time - 2_000));
        advance(&mut track, &Facts::default(), false, step(time));
        track
    }

    #[test]
    fn a_respawn_seen_soon_after_the_death_is_exact() {
        let track = spawned(10_000);
        assert!(track.alive);
        assert_eq!(track.health, Range::new(124.0, 124.5, 125.0));
        assert_eq!(track.armor, Range::exact(25.0));
    }

    #[test]
    fn a_player_first_seen_could_have_any_health() {
        let mut track = Track::default();
        advance(&mut track, &Facts::default(), false, step(5_000));
        assert_eq!(track.health, Range::new(1.0, 100.0, 125.0));
        assert_eq!(track.armor, Range::new(0.0, 25.0, 100.0));
    }

    #[test]
    fn health_over_the_maximum_decays_a_point_a_second() {
        let mut track = Track {
            health: Range::exact(125.0),
            armor: Range::exact(25.0),
            alive: true,
            last_seen: 0,
            ..Track::default()
        };
        for second in 1..=10 {
            advance(&mut track, &Facts::default(), false, step(second * 1_000));
        }
        assert_eq!(track.health, Range::exact(115.0));
        assert_eq!(track.armor, Range::exact(25.0), "under the maximum: kept");
        for second in 11..=40 {
            advance(&mut track, &Facts::default(), false, step(second * 1_000));
        }
        assert_eq!(track.health, Range::exact(100.0));
    }

    #[test]
    fn a_pain_event_gives_the_health_exactly() {
        let mut track = spawned(10_000);
        let facts = Facts {
            pain: Some(61),
            hit: Some(Range::new(20.0, 40.0, 150.0)),
            ..Facts::default()
        };
        advance(&mut track, &facts, false, step(10_050));
        assert_eq!(track.health, Range::exact(61.0));
        assert_eq!(track.pain_until, 10_750);
    }

    #[test]
    fn a_masked_pain_only_says_ten_or_more_landed() {
        let mut track = spawned(10_000);
        let mut masked = step(10_050);
        masked.pain = PainReport::Masked;
        track.armor = Range::exact(0.0);
        let facts = Facts {
            pain: Some(50),
            ..Facts::default()
        };
        advance(&mut track, &facts, false, masked);
        assert!(track.health.high <= 115.0);
        assert!(track.health.low < 70.0);
        assert!(track.health.width() > 40.0);
    }

    #[test]
    fn the_armour_takes_a_hit_first() {
        let mut track = spawned(10_000);
        let facts = Facts {
            hit: Some(Range::new(5.0, 12.0, 19.0)),
            absorbed: 12.0,
            shield_hit: true,
            ..Facts::default()
        };
        advance(&mut track, &facts, false, step(10_050));
        assert_eq!(track.armor, Range::exact(13.0));
        // Armour is left, so the saber hit went no further.
        close(track.health, (124.0, 124.5, 125.0));
    }

    #[test]
    fn a_hit_with_no_shield_flash_found_no_armour_and_an_unfelt_one_was_small() {
        let mut track = spawned(10_000);
        let facts = Facts {
            hit: Some(Range::new(20.0, 40.0, 150.0)),
            ..Facts::default()
        };
        advance(&mut track, &facts, false, step(10_050));
        assert_eq!(track.armor.best, 0.0);
        // No pain outside the debounce: it was under ten a frame.
        assert!(
            track.health.low >= 124.0 - UNFELT - 0.1,
            "{:?}",
            track.health
        );
        // Inside the debounce, a big hit could have done anything.
        track.pain_until = 20_000;
        advance(&mut track, &facts, false, step(10_100));
        assert!(track.health.low < 10.0);
    }

    #[test]
    fn pickups_and_heals_add_their_amount_up_to_the_cap() {
        let mut track = Track {
            health: Range::exact(60.0),
            armor: Range::exact(10.0),
            alive: true,
            last_seen: 0,
            ..Track::default()
        };
        // item_medpak_instant, item_shield_sm_instant.
        let facts = Facts {
            pickups: [3, 1],
            pickup_count: 2,
            ..Facts::default()
        };
        advance(&mut track, &facts, false, step(50));
        assert_eq!(track.health, Range::exact(85.0));
        assert_eq!(track.armor, Range::exact(35.0));
        let facts = Facts {
            heals: 1,
            ..Facts::default()
        };
        advance(&mut track, &facts, false, step(100));
        assert_eq!(track.health, Range::new(90.0, 100.0, 100.0));
    }

    #[test]
    fn falls_cost_their_damage_and_the_local_players_hit_resets_the_estimate() {
        let mut track = spawned(10_000);
        let facts = Facts {
            fall: 9.0,
            ..Facts::default()
        };
        advance(&mut track, &facts, false, step(10_050));
        close(track.health, (115.0, 115.5, 116.0));
        let facts = Facts {
            attackee: Some((70.0, 0.0)),
            hit: Some(Range::new(1.0, 3.0, 4.0)),
            ..Facts::default()
        };
        advance(&mut track, &facts, false, step(10_100));
        assert_eq!(track.health, Range::new(66.0, 67.0, 69.0));
        assert_eq!(track.armor, Range::exact(0.0));
    }

    #[test]
    fn out_of_view_the_range_widens_with_time() {
        let mut track = Track {
            health: Range::exact(80.0),
            armor: Range::exact(20.0),
            alive: true,
            last_seen: 0,
            ..Track::default()
        };
        advance(&mut track, &Facts::default(), false, step(4_000));
        assert_eq!(track.health, Range::new(40.0, 80.0, 100.0));
        assert_eq!(track.armor, Range::new(0.0, 20.0, 20.0));
    }

    /// Snapshots as the server sends them, through [`Estimator::observe`].
    mod snapshots {
        use super::super::*;
        use sjk_protocol::{LEGACY_ENTITY_FIELDS, PlayerState};

        const ORIGIN: [f32; 3] = [100.0, 200.0, 24.0];

        fn game() -> GameState {
            let mut game = GameState::empty_local(0);
            game.replace_config_string(CS_SOUNDS + 5, b"sound/weapons/force/heal.wav".to_vec())
                .unwrap();
            game
        }

        fn local(health: i32) -> PlayerState {
            let mut player = PlayerState::zero();
            player.set_client_num(0);
            player.stats[0] = health as u32;
            player.stats[8] = 100;
            player
        }

        fn entity(number: u16, kind: u8) -> EntityState {
            let mut state = EntityState::zero(number, &LEGACY_ENTITY_FIELDS);
            state.set_raw_field(8, u32::from(kind));
            for (field, value) in [2, 1, 4].into_iter().zip(ORIGIN) {
                state.set_raw_field(field, value.to_bits());
            }
            state
        }

        /// Player 3, with `event` (toggle bits included) and its parameter.
        fn player(event: u32, parameter: u32) -> EntityState {
            let mut state = entity(3, ET_PLAYER);
            state.set_raw_field(28, event);
            state.set_raw_field(42, parameter);
            state
        }

        /// A temporary event entity about player 3.
        fn temporary(number: u16, event: u16, parameter: u32) -> EntityState {
            let mut state = entity(number, ET_EVENTS + event as u8);
            state.set_raw_field(42, parameter);
            state.set_raw_field(59, 3);
            state
        }

        fn snapshot(time: i32, player: PlayerState, entities: Vec<EntityState>) -> Snapshot {
            Snapshot {
                message_sequence: 0,
                reliable_acknowledge: 0,
                server_commands: Vec::new(),
                server_time: time,
                delta_from: None,
                flags: 0,
                area_mask: Vec::new(),
                player,
                vehicle_player: None,
                entities,
                consumed_bits: 0,
            }
        }

        #[test]
        fn pains_shield_hits_pickups_and_heals_are_read_from_the_snapshots() {
            let game = game();
            let mut estimator = Estimator::default();
            let mut observe = |time, entities| {
                estimator.observe(&snapshot(time, local(100), entities), &game, 0);
                estimator
                    .health(3)
                    .map(|h| (h, estimator.armor(3).unwrap()))
            };
            let (health, armor) = observe(1_000, vec![player(0, 0)]).unwrap();
            assert_eq!(health, Range::new(1.0, 100.0, 125.0));
            assert_eq!(armor, Range::new(0.0, 25.0, 100.0));
            let (health, _) = observe(1_050, vec![player(0x100 | 89, 70)]).unwrap();
            assert_eq!(health, Range::exact(70.0));
            let mut shield = temporary(100, EV_SHIELD_HIT, 0);
            shield.set_raw_field(61, 10);
            let (_, armor) = observe(1_100, vec![player(0x100 | 89, 70), shield]).unwrap();
            assert_eq!(armor, Range::new(0.0, 15.0, 90.0));
            // A medpack: the item entity is seen, then the player takes it.
            let mut medpack = entity(200, ET_ITEM);
            medpack.set_raw_field(46, 3);
            observe(1_150, vec![player(0x100 | 89, 70), medpack]);
            let (health, _) = observe(1_200, vec![player(0x200 | 22, 200)]).unwrap();
            assert_eq!(health, Range::exact(95.0));
            let heal = temporary(101, EV_GENERAL_SOUND, 5);
            let (health, _) = observe(1_250, vec![player(0x200 | 22, 200), heal]).unwrap();
            assert_eq!(health, Range::new(100.0, 100.0, 100.0));
            assert_eq!(estimator.pain_report(), PainReport::Unchecked);
        }

        #[test]
        fn the_local_players_pains_tell_whether_pain_values_are_real() {
            let game = game();
            let mut estimator = Estimator::default();
            let mut feel = |time, event: u32, parameter: u32, health: i32| {
                let mut player = local(health);
                player.set_raw_field(56, event);
                player.set_raw_field(64, parameter);
                estimator.observe(&snapshot(time, player, Vec::new()), &game, 0);
                estimator.pain_report()
            };
            assert_eq!(feel(1_000, 0, 0, 100), PainReport::Unchecked);
            assert_eq!(feel(1_050, 0x100 | 89, 50, 80), PainReport::Unchecked);
            assert_eq!(feel(1_900, 0x200 | 89, 50, 61), PainReport::Masked);
            let mut estimator = Estimator::default();
            let mut player = local(77);
            player.set_raw_field(56, 0x100 | 89);
            player.set_raw_field(64, 77);
            estimator.observe(&snapshot(1_000, local(100), Vec::new()), &game, 0);
            estimator.observe(&snapshot(1_050, player, Vec::new()), &game, 0);
            assert_eq!(estimator.pain_report(), PainReport::Honest);
        }

        #[test]
        fn the_local_players_saber_hit_names_its_victim() {
            let game = game();
            let mut estimator = Estimator::default();
            estimator.observe(&snapshot(1_000, local(100), vec![player(0, 0)]), &game, 0);
            let mut hitter = local(100);
            hitter.persistent[PERS_HITS] = 1;
            hitter.persistent[PERS_ATTACKEE_ARMOR] = (60 << 8) | 5;
            let mut hit = temporary(100, EV_SABER_HIT, 3);
            hit.set_raw_field(39, 0);
            let mut shield = temporary(101, EV_SHIELD_HIT, 0);
            shield.set_raw_field(61, 3);
            estimator.observe(
                &snapshot(1_050, hitter, vec![player(0, 0), hit, shield]),
                &game,
                0,
            );
            // 60 and 5 before; the small hit went into the armour, which held.
            assert_eq!(estimator.health(3), Some(Range::exact(60.0)));
            assert_eq!(estimator.armor(3), Some(Range::exact(2.0)));
        }
    }

    #[test]
    fn a_teleport_may_have_been_a_respawn() {
        let mut track = Track {
            health: Range::exact(40.0),
            armor: Range::exact(0.0),
            alive: true,
            last_seen: 0,
            ..Track::default()
        };
        let mut moved = step(50);
        moved.teleport = true;
        advance(&mut track, &Facts::default(), false, moved);
        assert_eq!(track.health, Range::new(40.0, 40.0, 125.0));
        assert_eq!(track.armor, Range::new(0.0, 0.0, 25.0));
        // Only a change of the bit counts.
        moved.time = 100;
        advance(&mut track, &Facts::default(), false, moved);
        assert!((track.health.high - 125.0).abs() < 0.1);
    }

    #[test]
    fn spawn_values_follow_the_game_type_until_learnt() {
        let mut profile = Profile {
            max_health: 100.0,
            learnt: None,
        };
        assert_eq!(profile.spawn(0), (125.0, 25.0));
        assert_eq!(profile.spawn(GT_DUEL), (100.0, 0.0));
        profile.learnt = Some((150.0, 50.0));
        assert_eq!(profile.spawn(GT_DUEL), (150.0, 50.0));
    }
}
