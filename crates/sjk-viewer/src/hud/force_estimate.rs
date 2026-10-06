//! Estimated Force points of other players.
//!
//! The server sends a client's Force pool only to that client, so another
//! player's pool is rebuilt from what their entity state shows, with the
//! server's own rules (`WP_ForcePowerStart`, `WP_ForcePowerRun`,
//! `WP_ForcePowersUpdate`, see `sjk-game-jka`'s `force_powers.rs`):
//!
//! - the pool starts full at spawn and refills a point every
//!   `g_forceRegenTime` (200 ms unless the server says otherwise), six times as
//!   fast with the boon, but not while any power but drain is active or a saber
//!   is thrown;
//! - a power that switches on in `forcePowersActive` costs its `forcePowerNeeded`
//!   entry; protect and absorb take another point every 300 and 600 ms, grip every
//!   100 ms and lightning every 50 ms while on; a force jump costs half its level's
//!   price (the real cost scales with the jump's charge, which is not sent);
//! - a push or pull (seen as its torso animation starting) and a saber throw cost
//!   their price.
//!
//! The players' power levels are not sent either, so every cost assumes level 3,
//! the cheapest. Other drains (being drained, saber blocks in some mods) and
//! pickups are not seen. The pool refills while a player idles, so any error
//! heals within about twenty seconds, and a respawn resets it to full. The result
//! is an estimate and is drawn as one.
use sjk_game_jka::force_powers::{
    FORCE_POWER_MAX, FORCE_POWER_NEEDED, FP_ABSORB, FP_DRAIN, FP_GRIP, FP_LEVITATION, FP_LIGHTNING,
    FP_PROTECT, FP_PULL, FP_PUSH, FP_SABER_THROW, NUM_FORCE_POWERS,
};

/// A full pool.
const MAX_POOL: f32 = FORCE_POWER_MAX as f32;
/// The level every power is assumed to be at (see the module notes).
const ASSUMED_LEVEL: usize = 3;
/// `g_forceRegenTime`'s default.
pub(super) const DEFAULT_REGEN_MILLIS: f32 = 200.0;
/// A player unseen this long (in server milliseconds) is taken to have idled.
const GAP_MILLIS: i32 = 1_500;
/// Share of a force jump's top price charged at its start.
const JUMP_SHARE: f32 = 0.5;
/// Clients tracked.
const CLIENTS: usize = 32;

/// What one snapshot shows of a player's Force use.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Observation {
    /// Server time of the snapshot, in milliseconds.
    pub(super) time: i32,
    /// `entityState_t::forcePowersActive`.
    pub(super) active: u32,
    /// Torso animation number.
    pub(super) torso_animation: u16,
    /// `entityState_t::saberInFlight`.
    pub(super) saber_in_flight: bool,
    /// A saber special move is under way (`BG_SaberInSpecial`), which also stops the refill.
    pub(super) saber_special: bool,
    /// How many times faster than normal the pool refills (boon, Jedi Master).
    pub(super) regen_multiplier: f32,
    /// The player is dead.
    pub(super) dead: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Track {
    pool: f32,
    previous: Observation,
    seen: bool,
}

/// Animation numbers of the instant powers.
#[derive(Clone, Copy, Debug)]
struct Animations {
    push: Option<u16>,
    pull: Option<u16>,
}

impl Animations {
    fn lookup() -> Self {
        let index = |name| sjk_game_jka::legacy_animation_index(name).map(|n| n as u16);
        Self {
            push: index("BOTH_FORCEPUSH"),
            pull: index("BOTH_FORCEPULL"),
        }
    }
}

/// Measures the server's real refill pace from the local player's own pool, which
/// the server does send. `g_forceRegenTime` is not part of the stock info string
/// and mods (JA+) change the pace, so the estimate trusts this over any cvar.
///
/// While the local player idles (no power but drain, no thrown saber, no boon,
/// not Jedi Master) every point gained over a snapshot interval is a point of
/// regeneration; the total milliseconds over the total points is the pace.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Calibration {
    seen: bool,
    last_time: i32,
    last_pool: i32,
    last_idle: bool,
    millis: f32,
    points: f32,
}

/// Points of idle regeneration needed before the measured pace is used: one
/// unseen point either end then costs at most 5%.
const CALIBRATION_POINTS: f32 = 20.0;
/// Past this many points the totals are halved, so the pace follows changes.
const CALIBRATION_WINDOW: f32 = 400.0;

impl Calibration {
    /// Take one snapshot of the local player: its pool (`fd.forcePower`), and
    /// whether it was regenerating normally (see the type's notes).
    pub(super) fn observe(&mut self, time: i32, pool: i32, idle: bool) {
        if !self.seen {
            *self = Self {
                seen: true,
                last_time: time,
                last_pool: pool,
                last_idle: idle,
                ..*self
            };
            return;
        }
        if time <= self.last_time {
            return;
        }
        let elapsed = time - self.last_time;
        let gained = pool - self.last_pool;
        // Only a whole idle interval below a full pool says anything.
        if self.last_idle && idle && elapsed <= GAP_MILLIS && gained >= 0 && pool < 100 {
            self.millis += elapsed as f32;
            self.points += gained as f32;
            if self.points > CALIBRATION_WINDOW {
                self.millis *= 0.5;
                self.points *= 0.5;
            }
        }
        self.last_time = time;
        self.last_pool = pool;
        self.last_idle = idle;
    }

    /// Measured milliseconds per point, once enough has been seen.
    pub(super) fn millis_per_point(&self) -> Option<f32> {
        (self.points >= CALIBRATION_POINTS).then(|| self.millis / self.points)
    }
}

/// Fixed-capacity estimator for every client slot.
pub(super) struct Estimator {
    tracks: [Track; CLIENTS],
    regen_millis: f32,
    animations: Animations,
}

impl Default for Estimator {
    fn default() -> Self {
        Self {
            tracks: [Track::default(); CLIENTS],
            regen_millis: DEFAULT_REGEN_MILLIS,
            animations: Animations::lookup(),
        }
    }
}

impl Estimator {
    /// Use the server's `g_forceRegenTime` (milliseconds per point) when it
    /// sends one; anything unusable keeps the default.
    pub(super) fn set_regen_millis(&mut self, millis: Option<f32>) {
        self.regen_millis = millis
            .filter(|m| m.is_finite() && *m >= 1.0)
            .unwrap_or(DEFAULT_REGEN_MILLIS);
    }

    /// Milliseconds per point in use.
    pub(super) fn regen_millis(&self) -> f32 {
        self.regen_millis
    }

    /// Estimated share of a full pool for `slot`, if it has been seen alive.
    pub(super) fn ratio(&self, slot: u16) -> Option<f32> {
        let track = self.tracks.get(usize::from(slot))?;
        (track.seen && !track.previous.dead).then(|| (track.pool / MAX_POOL).clamp(0.0, 1.0))
    }

    /// Take one snapshot's view of `slot`. Observations must come in server-time order;
    /// repeating a time changes nothing.
    pub(super) fn observe(&mut self, slot: u16, now: Observation) {
        let regen_millis = self.regen_millis;
        let animations = self.animations;
        let Some(track) = self.tracks.get_mut(usize::from(slot)) else {
            return;
        };
        if now.dead {
            track.previous = now;
            track.seen = false;
            return;
        }
        if !track.seen {
            *track = Track {
                pool: MAX_POOL,
                previous: now,
                seen: true,
            };
            return;
        }
        let before = track.previous;
        let elapsed = now.time.wrapping_sub(before.time);
        if elapsed <= 0 {
            return;
        }
        let elapsed_f = elapsed as f32;
        if elapsed > GAP_MILLIS {
            track.pool += elapsed_f / regen_millis * now.regen_multiplier.max(1.0);
        } else {
            track.pool -= running_cost(before.active, elapsed_f);
            if regenerates(&before) {
                track.pool += elapsed_f / regen_millis * before.regen_multiplier.max(1.0);
            }
            // The pool tops out before what began in this interval is paid.
            track.pool = track.pool.min(MAX_POOL);
            track.pool -= starting_cost(&before, &now, &animations);
        }
        track.pool = track.pool.clamp(0.0, MAX_POOL);
        track.previous = now;
    }
}

/// Whether the server lets the pool refill while `state` holds
/// (`WP_ForcePowersUpdate`): no power but drain on, no saber thrown.
fn regenerates(state: &Observation) -> bool {
    state.active & !(1 << FP_DRAIN) == 0 && !state.saber_in_flight && !state.saber_special
}

/// Points taken over `elapsed` milliseconds by powers that stay on.
fn running_cost(active: u32, elapsed: f32) -> f32 {
    let on = |power: usize| active & (1 << power) != 0;
    let mut cost = 0.0;
    if on(FP_PROTECT) {
        cost += elapsed / 300.0;
    }
    if on(FP_ABSORB) {
        cost += elapsed / 600.0;
    }
    if on(FP_GRIP) {
        cost += elapsed / 100.0;
    }
    if on(FP_LIGHTNING) {
        cost += elapsed / 50.0;
    }
    cost
}

/// Points taken by what began between two observations.
fn starting_cost(before: &Observation, now: &Observation, animations: &Animations) -> f32 {
    let price = |power: usize| FORCE_POWER_NEEDED[ASSUMED_LEVEL][power] as f32;
    let mut cost = 0.0;
    let started = now.active & !before.active;
    for power in 0..NUM_FORCE_POWERS {
        if started & (1 << power) == 0 {
            continue;
        }
        cost += match power {
            FP_LEVITATION => price(power) * JUMP_SHARE,
            // Streams and grip are paid by the running cost; push and pull by
            // their animation below.
            FP_GRIP | FP_DRAIN | FP_PUSH | FP_PULL => 0.0,
            _ => price(power),
        };
    }
    if now.torso_animation != before.torso_animation {
        if Some(now.torso_animation) == animations.push {
            cost += price(FP_PUSH);
        } else if Some(now.torso_animation) == animations.pull {
            cost += price(FP_PULL);
        }
    }
    if now.saber_in_flight && !before.saber_in_flight {
        cost += price(FP_SABER_THROW);
    }
    cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_game_jka::force_powers::{FP_RAGE, FP_SPEED};

    fn at(time: i32) -> Observation {
        Observation {
            time,
            regen_multiplier: 1.0,
            ..Observation::default()
        }
    }

    fn with(mut observation: Observation, power: usize) -> Observation {
        observation.active |= 1 << power;
        observation
    }

    fn percent(estimator: &Estimator) -> f32 {
        estimator.ratio(3).unwrap() * 100.0
    }

    #[test]
    fn a_player_is_full_when_first_seen() {
        let mut estimator = Estimator::default();
        assert_eq!(estimator.ratio(3), None);
        estimator.observe(3, at(1_000));
        assert_eq!(percent(&estimator), 100.0);
    }

    #[test]
    fn a_power_costs_its_price_when_it_switches_on_and_refills_afterwards() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(1_000));
        estimator.observe(3, with(at(1_050), FP_RAGE));
        assert!((percent(&estimator) - 50.0).abs() < 0.01, "rage costs 50");
        estimator.observe(3, at(1_100));
        // 10 s idle at one point per 200 ms is 50 points: full again.
        for step in 1..=10 {
            estimator.observe(3, at(1_100 + step * 1_000));
        }
        assert!((percent(&estimator) - 100.0).abs() < 0.01);
    }

    #[test]
    fn no_refill_while_a_power_is_on() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(0));
        estimator.observe(3, with(at(50), FP_SPEED));
        let after_start = percent(&estimator);
        estimator.observe(3, with(at(1_050), FP_SPEED));
        assert_eq!(percent(&estimator), after_start);
    }

    #[test]
    fn protect_takes_a_point_every_300_ms_while_on() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(0));
        estimator.observe(3, with(at(50), FP_PROTECT));
        let start = percent(&estimator);
        assert!((start - 90.0).abs() < 0.01, "protect costs 10 at level 3");
        for step in 1..=30 {
            estimator.observe(3, with(at(50 + step * 100), FP_PROTECT));
        }
        assert!((percent(&estimator) - (start - 10.0)).abs() < 0.01);
    }

    #[test]
    fn a_force_jump_costs_half_its_price_once() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(0));
        estimator.observe(3, with(at(50), FP_LEVITATION));
        let jump = percent(&estimator);
        assert!((jump - 95.0).abs() < 0.01, "level 3 jump price 10, half");
        estimator.observe(3, with(at(100), FP_LEVITATION));
        assert_eq!(percent(&estimator), jump);
    }

    #[test]
    fn a_push_costs_its_price_when_its_animation_starts() {
        let mut estimator = Estimator::default();
        let push = estimator.animations.push.expect("BOTH_FORCEPUSH exists");
        estimator.observe(3, at(0));
        let mut pushing = at(50);
        pushing.torso_animation = push;
        estimator.observe(3, pushing);
        assert!((percent(&estimator) - 80.0).abs() < 0.01, "push costs 20");
        let mut still = at(100);
        still.torso_animation = push;
        estimator.observe(3, still);
        assert!((percent(&estimator) - 80.0).abs() < 0.5, "once per start");
    }

    #[test]
    fn a_thrown_saber_costs_and_blocks_refill() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(0));
        let mut thrown = at(50);
        thrown.saber_in_flight = true;
        estimator.observe(3, thrown);
        let after = percent(&estimator);
        assert!((after - 80.0).abs() < 0.01, "throw costs 20");
        thrown.time = 1_050;
        estimator.observe(3, thrown);
        assert_eq!(percent(&estimator), after, "no refill with a saber out");
    }

    #[test]
    fn a_saber_special_move_blocks_the_refill() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(0));
        estimator.observe(3, with(at(50), FP_RAGE));
        let mut special = at(100);
        special.saber_special = true;
        estimator.observe(3, special);
        let held = percent(&estimator);
        for step in 1..=20 {
            special.time = 100 + step * 100;
            estimator.observe(3, special);
        }
        assert_eq!(percent(&estimator), held);
    }

    #[test]
    fn dying_and_respawning_resets_the_pool() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(0));
        estimator.observe(3, with(at(50), FP_RAGE));
        let mut dead = at(100);
        dead.dead = true;
        estimator.observe(3, dead);
        assert_eq!(estimator.ratio(3), None);
        estimator.observe(3, at(5_000));
        assert_eq!(percent(&estimator), 100.0);
    }

    #[test]
    fn a_long_gap_counts_as_idle_time() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(0));
        estimator.observe(3, with(at(50), FP_RAGE));
        estimator.observe(3, at(60_000));
        assert_eq!(percent(&estimator), 100.0);
    }

    #[test]
    fn the_boon_and_the_servers_regen_time_speed_the_refill() {
        let mut estimator = Estimator::default();
        estimator.set_regen_millis(Some(100.0));
        estimator.observe(3, at(0));
        estimator.observe(3, with(at(50), FP_RAGE));
        estimator.observe(3, at(100));
        estimator.observe(3, at(1_100));
        // 1 s at one point per 100 ms: 10 points back.
        assert!((percent(&estimator) - 60.0).abs() < 0.01);
        estimator.set_regen_millis(Some(f32::NAN));
        assert_eq!(estimator.regen_millis, DEFAULT_REGEN_MILLIS);
    }

    #[test]
    fn the_pace_is_measured_from_the_local_pool() {
        let mut calibration = Calibration::default();
        assert_eq!(calibration.millis_per_point(), None);
        // A point every 150 ms, snapshots every 50 ms, from a pool of 10.
        let mut pool = 10;
        for step in 0..=120 {
            let time = step * 50;
            pool = 10 + time / 150;
            calibration.observe(time, pool, true);
        }
        assert!(pool < 100);
        let pace = calibration.millis_per_point().expect("enough points");
        assert!((pace - 150.0).abs() < 8.0, "{pace}");
    }

    #[test]
    fn spending_and_other_regeneration_are_not_counted() {
        let mut calibration = Calibration::default();
        // Not idle (a power on): the gain is not regeneration.
        for step in 0..200 {
            calibration.observe(step * 50, 10 + step / 2, false);
        }
        assert_eq!(calibration.millis_per_point(), None);
        // A pool already full cannot show a pace.
        let mut full = Calibration::default();
        for step in 0..200 {
            full.observe(step * 50, 100, true);
        }
        assert_eq!(full.millis_per_point(), None);
        // A long gap in snapshots is skipped.
        let mut gap = Calibration::default();
        gap.observe(0, 10, true);
        gap.observe(60_000, 90, true);
        assert_eq!(gap.millis_per_point(), None);
    }

    #[test]
    fn the_pool_never_leaves_its_range() {
        let mut estimator = Estimator::default();
        estimator.observe(3, at(0));
        for step in 1..20 {
            estimator.observe(3, with(at(step * 100), FP_LIGHTNING));
        }
        assert!(percent(&estimator) >= 0.0);
        assert_eq!(Estimator::default().ratio(40), None);
    }
}
