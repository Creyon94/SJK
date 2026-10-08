//! Force drained from players, for the nameplates' Force estimate.
//!
//! A player holding drain (`FP_DRAIN` in `forcePowersActive`) shoots every 50 ms
//! (`WP_ForcePowerRun`'s `FORCE_DEBOUNCE_TIME`). Each shot (`ForceShootDrain`,
//! `ForceDrainDamage`, ported in `sjk-game-jka`'s `force_dark.rs`):
//!
//! - at level 3 reaches every enemy whose box is within 512 units of the drainer's
//!   origin, in front of it (the middle of the box at most 60 degrees off the view)
//!   and in clear sight; at levels 1 and 2 only the first player on a 2048-unit line
//!   along the view;
//! - takes 2, 3 or 4 points from each victim by the drainer's level; against an
//!   absorb that is up, what the two levels' difference leaves (0 to 2), and the
//!   absorb gives a point back;
//! - holds back the victim's refill for 800 ms;
//! - costs the drainer 5 points (at levels 1 and 2 only when the line found a player)
//!   and holds back its own refill for 500 ms.
//!
//! Only `EV_FORCE_DRAINED`, at most every 400 ms a victim, names a victim, so the
//! shots are rebuilt from where everybody stands and looks, with the world's walls in
//! the way (not movers, nor other players in the arc's line of sight). Other players'
//! levels are not sent: the guess is level 3's arc, the low bound of a pool takes the
//! dearest level that reaches it and the high bound the cheapest. A victim the event
//! names lost at least a shot whatever the geometry said. The local player's own level
//! is its Force profile's, so its shots are exact.
use glam::Vec3;
use sjk_game_jka::force_powers::{FP_ABSORB, FP_DRAIN};

const CLIENTS: usize = 32;
/// `FORCE_DEBOUNCE_TIME`: a shot every 50 ms.
const SHOT_MILLIS: i32 = 50;
/// A drainer unseen this long starts its shots afresh.
const GAP_MILLIS: i32 = 1_500;
/// `MAX_DRAIN_DISTANCE` (the arc) and the line's reach.
const ARC_RADIUS: f32 = 512.0;
const LINE_REACH: f32 = 2_048.0;
/// The arc's cone: the cosine of 60 degrees.
const ARC_COSINE: f32 = 0.5;
/// Points a shot takes at levels 1 to 3 (index 0 unused).
const TAKEN: [f32; 4] = [0.0, 2.0, 3.0, 4.0];
/// What a shot costs the drainer.
const SHOT_COST: f32 = 5.0;
/// How long a shot holds back the victim's and the drainer's refill.
const VICTIM_HOLD: i32 = 800;
const DRAINER_HOLD: i32 = 500;
/// `CONTENTS_SOLID | CONTENTS_TERRAIN`: the world part of `MASK_SHOT`.
const WORLD_SHOT: u32 = 0x1 | 0x1000;
/// A player with no packed box (`SV_LinkEntity`) stands.
const STANDING: ([f32; 3], [f32; 3]) = ([-15.0, -15.0, -24.0], [15.0, 15.0, 40.0]);

/// Bounds of a pool, in [`super::estimate::Range`] order.
const LOW: usize = 0;
const BEST: usize = 1;
const HIGH: usize = 2;

/// What one snapshot's drain shots did to a player's pool, per bound of its range
/// (the low bound, the guess, the high bound).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Effect {
    /// Points each bound loses; negative when an absorb gave more back.
    pub(super) loss: [f32; 3],
    /// Until when (server milliseconds) each bound's refill is held back.
    pub(super) hold: [i32; 3],
}

impl Effect {
    fn lose(&mut self, bound: usize, points: f32, hold_until: i32) {
        self.loss[bound] += points;
        self.hold[bound] = self.hold[bound].max(hold_until);
    }
}

/// One player as a drain shot sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Body {
    pub(super) number: u16,
    pub(super) origin: [f32; 3],
    /// View angles in degrees (pitch, yaw, roll).
    pub(super) view: [f32; 3],
    /// Box corners about the origin.
    pub(super) mins: [f32; 3],
    pub(super) maxs: [f32; 3],
    /// `forcePowersActive`.
    pub(super) active: u32,
    pub(super) team: i32,
    pub(super) duelling: bool,
    /// The drain level when known (the local player's), else `None`.
    pub(super) level: Option<u8>,
    /// Its pool is estimated: an alive player other than the local one.
    pub(super) estimated: bool,
}

impl Body {
    /// Box corners from the packed `entityState_t::solid` (`SV_LinkEntity`: bits 0-7
    /// the half width, 8-15 the depth below the origin, 16-23 the top plus 32).
    pub(super) fn unpack_box(solid: u32) -> ([f32; 3], [f32; 3]) {
        let (side, below, above) = (solid & 255, (solid >> 8) & 255, (solid >> 16) & 255);
        if solid == 0 || side == 0 || above == 0 {
            return STANDING;
        }
        let (side, below, above) = (side as f32, below as f32, above as f32 - 32.0);
        ([-side, -side, -below], [side, side, above])
    }

    /// The standing box, for a player whose box is not sent (the local player).
    pub(super) fn standing_box() -> ([f32; 3], [f32; 3]) {
        STANDING
    }

    fn draining(&self) -> bool {
        self.active & (1 << FP_DRAIN) != 0
    }

    fn absorbing(&self) -> bool {
        self.active & (1 << FP_ABSORB) != 0
    }

    /// `r.absmin`, `r.absmax`: the box in the world, an epsilon wider.
    fn bounds(&self) -> (Vec3, Vec3) {
        let origin = Vec3::from_array(self.origin);
        (
            origin + Vec3::from_array(self.mins) - Vec3::ONE,
            origin + Vec3::from_array(self.maxs) + Vec3::ONE,
        )
    }

    fn forward(&self) -> Vec3 {
        let (pitch, yaw) = (self.view[0].to_radians(), self.view[1].to_radians());
        Vec3::new(
            pitch.cos() * yaw.cos(),
            pitch.cos() * yaw.sin(),
            -pitch.sin(),
        )
    }
}

/// The rules of the match that decide who can drain whom.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Rules {
    /// A team game: teammates are not drained (`g_friendlyFire` off).
    pub(super) teams: bool,
}

/// Follows every drainer's shots and gives each player its [`Effect`] per snapshot.
pub(super) struct Tracker {
    /// When each drainer's next shot is due, while it drains.
    next_shot: [Option<i32>; CLIENTS],
    effects: [Effect; CLIENTS],
    last_time: i32,
}

impl Default for Tracker {
    fn default() -> Self {
        Self {
            next_shot: [None; CLIENTS],
            effects: [Effect::default(); CLIENTS],
            last_time: i32::MIN,
        }
    }
}

impl Tracker {
    /// What the last observed snapshot's shots did to `slot`.
    pub(super) fn effect(&self, slot: u16) -> Effect {
        self.effects
            .get(usize::from(slot))
            .copied()
            .unwrap_or_default()
    }

    /// Take one snapshot at server `time`: every player in it (`bodies`), the victims
    /// `EV_FORCE_DRAINED` named in it (bits by client), and `sight`, the fraction of a
    /// straight line between two points the world leaves clear.
    pub(super) fn observe(
        &mut self,
        time: i32,
        bodies: &[Body],
        rules: Rules,
        named: u32,
        mut sight: impl FnMut([f32; 3], [f32; 3]) -> f32,
    ) {
        if time < self.last_time {
            // A new map or a restart.
            *self = Self::default();
        }
        if time == self.last_time {
            return;
        }
        let fresh = self.last_time == i32::MIN || time - self.last_time > GAP_MILLIS;
        self.last_time = time;
        self.effects = [Effect::default(); CLIENTS];
        // A drainer out of view starts afresh when it comes back.
        let present = bodies
            .iter()
            .filter(|body| usize::from(body.number) < CLIENTS)
            .fold(0_u32, |bits, body| bits | 1 << body.number);
        for (slot, next) in self.next_shot.iter_mut().enumerate() {
            if present & (1 << slot) == 0 || fresh {
                *next = None;
            }
        }
        // The cheapest and dearest shot any drainer in sight can fire, for the
        // victims the event names.
        let mut floor: Option<[f32; 3]> = None;
        for drainer in bodies {
            let slot = usize::from(drainer.number);
            if slot >= CLIENTS {
                continue;
            }
            if !drainer.draining() {
                self.next_shot[slot] = None;
                continue;
            }
            let shots = match self.next_shot[slot] {
                Some(mut due) => {
                    let mut shots = 0;
                    while due <= time {
                        shots += 1;
                        due += SHOT_MILLIS;
                    }
                    self.next_shot[slot] = Some(due);
                    shots
                }
                None => {
                    self.next_shot[slot] = Some(time + SHOT_MILLIS);
                    1
                }
            };
            let levels = match drainer.level {
                Some(level) => level.clamp(1, 3)..=level.clamp(1, 3),
                None => 1..=3,
            };
            let shot = [
                levels
                    .clone()
                    .map(|l| TAKEN[usize::from(l)])
                    .fold(0.0, f32::max),
                TAKEN[usize::from(drainer.level.unwrap_or(3).clamp(1, 3))],
                levels
                    .clone()
                    .map(|l| TAKEN[usize::from(l)])
                    .fold(5.0, f32::min),
            ];
            floor = Some(match floor {
                Some(f) => [
                    f[LOW].max(shot[LOW]),
                    f[BEST].max(shot[BEST]),
                    f[HIGH].min(shot[HIGH]),
                ],
                None => shot,
            });
            if shots == 0 {
                continue;
            }
            self.shoot(
                drainer,
                bodies,
                rules,
                levels,
                shots as f32,
                time,
                &mut sight,
            );
        }
        let floor = floor.unwrap_or([TAKEN[3], TAKEN[3], TAKEN[1]]);
        for slot in 0..CLIENTS {
            if named & (1 << slot) == 0 {
                continue;
            }
            let effect = &mut self.effects[slot];
            for bound in [LOW, BEST, HIGH] {
                effect.loss[bound] = effect.loss[bound].max(floor[bound]);
                effect.hold[bound] = effect.hold[bound].max(time + VICTIM_HOLD);
            }
        }
    }

    /// `shots` shots of `drainer` at the `levels` it may have drain at.
    #[allow(clippy::too_many_arguments)]
    fn shoot(
        &mut self,
        drainer: &Body,
        bodies: &[Body],
        rules: Rules,
        levels: std::ops::RangeInclusive<u8>,
        shots: f32,
        time: i32,
        sight: &mut impl FnMut([f32; 3], [f32; 3]) -> f32,
    ) {
        let centre = Vec3::from_array(drainer.origin);
        let forward = drainer.forward();
        let line = first_on_line(drainer, bodies, centre, forward, sight);
        let guess_level = drainer.level.unwrap_or(3).clamp(1, 3);
        // What the drainer pays at each bound: the dearest level, the guess, the cheapest.
        let pays = |level: u8| level == 3 || line.is_some();
        let paid = [
            levels.clone().any(pays),
            pays(guess_level),
            levels.clone().all(pays),
        ];
        let slot = usize::from(drainer.number);
        for (bound, paid) in paid.into_iter().enumerate() {
            if paid {
                self.effects[slot].lose(bound, SHOT_COST * shots, time + DRAINER_HOLD);
            }
        }
        for victim in bodies {
            if !victim.estimated
                || victim.number == drainer.number
                || usize::from(victim.number) >= CLIENTS
                || !drainable(drainer, victim, rules)
            {
                continue;
            }
            let in_arc = in_arc(victim, centre, forward, sight);
            let on_line = line == Some(victim.number);
            let reaches = |level: u8| if level == 3 { in_arc } else { on_line };
            // Per shot, what each level takes: the most and the least by absorb level.
            let take = |level: u8| -> Option<(f32, f32, f32)> {
                if !reaches(level) {
                    return None;
                }
                Some(if victim.absorbing() {
                    // `WP_AbsorbConversion`: drain level less absorb level (absorb 1 to
                    // 3, the guess 3) is taken, and a point given back.
                    let most = f32::from(level.saturating_sub(1).min(2)) - 1.0;
                    let guess = f32::from(level.saturating_sub(3)) - 1.0;
                    (most, guess, -1.0)
                } else {
                    let taken = TAKEN[usize::from(level)];
                    (taken, taken, taken)
                })
            };
            let effect = &mut self.effects[usize::from(victim.number)];
            let until = time + VICTIM_HOLD;
            // The low bound: the dearest level that reaches, held if any does.
            let dearest = levels
                .clone()
                .filter_map(take)
                .map(|(most, _, _)| most)
                .reduce(f32::max);
            if let Some(most) = dearest {
                effect.lose(LOW, most * shots, until);
            }
            if let Some((_, guess, _)) = take(guess_level) {
                effect.lose(BEST, guess * shots, until);
            }
            // The high bound: the cheapest level, nothing (and no hold) if one misses.
            let cheapest = levels
                .clone()
                .map(take)
                .try_fold(f32::INFINITY, |least, take| {
                    take.map(|(_, _, least_here)| least.min(least_here))
                });
            if let Some(least) = cheapest {
                effect.lose(HIGH, least * shots, until);
            }
        }
    }
}

/// `ForcePowerUsableOn` and `OnSameTeam` for drain: no teammates in team games, and
/// a duellist only drains its opponent (both must be duelling).
fn drainable(drainer: &Body, victim: &Body, rules: Rules) -> bool {
    let teammates = rules.teams && matches!(drainer.team, 1 | 2) && drainer.team == victim.team;
    !teammates && drainer.duelling == victim.duelling
}

/// Level 3's arc: the box within reach, its middle in the cone, in clear sight.
fn in_arc(
    victim: &Body,
    centre: Vec3,
    forward: Vec3,
    sight: &mut impl FnMut([f32; 3], [f32; 3]) -> f32,
) -> bool {
    let (absmin, absmax) = victim.bounds();
    let nearest = (absmin - centre).max(Vec3::ZERO) + (centre - absmax).max(Vec3::ZERO);
    if nearest.length() >= ARC_RADIUS {
        return false;
    }
    let middle = (absmin + absmax) * 0.5;
    if (middle - centre).normalize_or_zero().dot(forward) < ARC_COSINE {
        return false;
    }
    sight(centre.to_array(), middle.to_array()) >= 1.0
}

/// Levels 1 and 2's line: the first player it meets before the world stops it.
fn first_on_line(
    drainer: &Body,
    bodies: &[Body],
    centre: Vec3,
    forward: Vec3,
    sight: &mut impl FnMut([f32; 3], [f32; 3]) -> f32,
) -> Option<u16> {
    let (number, entry) = bodies
        .iter()
        .filter(|body| body.number != drainer.number)
        .filter_map(|body| {
            let origin = Vec3::from_array(body.origin);
            let (low, high) = (
                origin + Vec3::from_array(body.mins),
                origin + Vec3::from_array(body.maxs),
            );
            ray_box(centre, forward, low, high).map(|entry| (body.number, entry))
        })
        .filter(|(_, entry)| *entry <= LINE_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    let reach = centre + forward * entry;
    (sight(centre.to_array(), reach.to_array()) >= 1.0).then_some(number)
}

/// Where a ray from `start` along `direction` enters the box, as a distance; `None`
/// when it misses or starts inside (a trace that starts solid finds nothing).
fn ray_box(start: Vec3, direction: Vec3, low: Vec3, high: Vec3) -> Option<f32> {
    let mut near = f32::NEG_INFINITY;
    let mut far = f32::INFINITY;
    for axis in 0..3 {
        let (s, d) = (start[axis], direction[axis]);
        if d.abs() < 1e-6 {
            if s < low[axis] || s > high[axis] {
                return None;
            }
            continue;
        }
        let (a, b) = ((low[axis] - s) / d, (high[axis] - s) / d);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near > 0.0 && near <= far).then_some(near)
}

/// `sight` against `bsp`'s world: the clear fraction of the line.
pub(super) fn world_sight(
    bsp: &sjk_bsp::Bsp,
    scratch: &mut sjk_bsp::TraceScratch,
) -> impl FnMut([f32; 3], [f32; 3]) -> f32 {
    let point = sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).expect("an empty box is valid");
    move |start, end| {
        let trace = bsp.trace_box_with(scratch, start, end, point, WORLD_SHOT);
        if trace.start_solid || trace.all_solid {
            0.0
        } else {
            trace.fraction
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL: u16 = 0;
    const DRAINER: u16 = 1;
    const VICTIM: u16 = 2;

    fn body(number: u16, origin: [f32; 3], yaw: f32) -> Body {
        let (mins, maxs) = STANDING;
        Body {
            number,
            origin,
            view: [0.0, yaw, 0.0],
            mins,
            maxs,
            estimated: number != LOCAL,
            ..Body::default()
        }
    }

    fn draining(mut body: Body, level: Option<u8>) -> Body {
        body.active |= 1 << FP_DRAIN;
        body.level = level;
        body
    }

    fn clear(_: [f32; 3], _: [f32; 3]) -> f32 {
        1.0
    }

    fn observe(tracker: &mut Tracker, time: i32, bodies: &[Body]) {
        tracker.observe(time, bodies, Rules::default(), 0, clear);
    }

    #[test]
    fn a_level_3_drain_takes_four_a_shot_from_everyone_in_the_arc() {
        let mut tracker = Tracker::default();
        let drainer = draining(body(DRAINER, [0.0; 3], 0.0), None);
        // In front, 200 units off; another behind; a third far beyond the arc.
        let front = body(VICTIM, [200.0, 0.0, 0.0], 0.0);
        let behind = body(3, [-200.0, 0.0, 0.0], 0.0);
        let far = body(4, [900.0, 0.0, 0.0], 0.0);
        observe(&mut tracker, 1_000, &[drainer, front, behind, far]);
        let hit = tracker.effect(VICTIM);
        // The guess and the low bound take level 3's 4; the high bound may be level 2's
        // line, which this victim stands on, or level 1's 2.
        assert_eq!(hit.loss, [4.0, 4.0, 2.0]);
        assert_eq!(hit.hold, [1_800; 3]);
        assert_eq!(tracker.effect(3), Effect::default());
        assert_eq!(
            tracker.effect(4),
            Effect::default(),
            "the line stops at the first"
        );
        // The drainer pays 5 a shot whatever its level: its line found somebody.
        assert_eq!(tracker.effect(DRAINER).loss, [5.0; 3]);
        assert_eq!(tracker.effect(DRAINER).hold, [1_500; 3]);
        // Alone on the line beyond the arc, only levels 1 and 2 reach: the guess (3)
        // takes nothing, the low bound level 2's 3 and the high bound nothing.
        let mut tracker = Tracker::default();
        observe(&mut tracker, 1_000, &[drainer, far]);
        assert_eq!(tracker.effect(4).loss, [3.0, 0.0, 0.0]);
        assert_eq!(tracker.effect(4).hold, [1_800, 0, 0]);
    }

    #[test]
    fn a_shot_every_50_ms_while_the_drain_lasts() {
        let mut tracker = Tracker::default();
        let drainer = draining(body(DRAINER, [0.0; 3], 0.0), Some(3));
        let victim = body(VICTIM, [100.0, 50.0, 0.0], 0.0);
        let mut taken = 0.0;
        // Snapshots every 25 ms for half a second: a shot at the start, then every 50.
        for step in 0..=20 {
            observe(&mut tracker, 1_000 + step * 25, &[drainer, victim]);
            taken += tracker.effect(VICTIM).loss[BEST];
        }
        assert_eq!(taken, 11.0 * 4.0);
        // It stops when the drain does.
        observe(&mut tracker, 1_525, &[body(DRAINER, [0.0; 3], 0.0), victim]);
        assert_eq!(tracker.effect(VICTIM), Effect::default());
    }

    #[test]
    fn the_local_players_known_level_is_exact() {
        let mut tracker = Tracker::default();
        let local = draining(body(LOCAL, [0.0; 3], 90.0), Some(2));
        // Level 2 drains the line only: the victim straight ahead, not the one aside.
        let ahead = body(VICTIM, [0.0, 300.0, 0.0], 0.0);
        let aside = body(3, [100.0, 200.0, 0.0], 0.0);
        observe(&mut tracker, 2_000, &[local, ahead, aside]);
        assert_eq!(tracker.effect(VICTIM).loss, [3.0; 3]);
        assert_eq!(tracker.effect(3), Effect::default());
    }

    #[test]
    fn walls_teammates_and_duels_stop_the_drain() {
        let drainer = draining(body(DRAINER, [0.0; 3], 0.0), Some(3));
        let victim = body(VICTIM, [200.0, 0.0, 0.0], 0.0);
        let mut tracker = Tracker::default();
        tracker.observe(1_000, &[drainer, victim], Rules::default(), 0, |_, _| 0.5);
        assert_eq!(tracker.effect(VICTIM), Effect::default(), "a wall between");
        let teams = Rules { teams: true };
        let (mut red, mut also_red) = (drainer, victim);
        red.team = 1;
        also_red.team = 1;
        let mut tracker = Tracker::default();
        tracker.observe(1_000, &[red, also_red], teams, 0, clear);
        assert_eq!(tracker.effect(VICTIM), Effect::default(), "a teammate");
        let mut duellist = victim;
        duellist.duelling = true;
        let mut tracker = Tracker::default();
        observe(&mut tracker, 1_000, &[drainer, duellist]);
        assert_eq!(tracker.effect(VICTIM), Effect::default(), "in a duel");
    }

    #[test]
    fn an_absorb_gives_a_point_back() {
        let mut tracker = Tracker::default();
        let drainer = draining(body(DRAINER, [0.0; 3], 0.0), Some(3));
        let mut victim = body(VICTIM, [200.0, 0.0, 0.0], 0.0);
        victim.active |= 1 << FP_ABSORB;
        observe(&mut tracker, 1_000, &[drainer, victim]);
        // Absorb 1 leaves 2 of level 3's drain (net 1); absorb 3 leaves none (net -1).
        assert_eq!(tracker.effect(VICTIM).loss, [1.0, -1.0, -1.0]);
    }

    #[test]
    fn a_named_victim_lost_at_least_a_shot() {
        let mut tracker = Tracker::default();
        // The drainer looks away (or is out of view): the event alone tells.
        let drainer = draining(body(DRAINER, [0.0; 3], 180.0), None);
        let victim = body(VICTIM, [200.0, 0.0, 0.0], 0.0);
        tracker.observe(
            1_000,
            &[drainer, victim],
            Rules::default(),
            1 << VICTIM,
            clear,
        );
        assert_eq!(tracker.effect(VICTIM).loss, [4.0, 4.0, 2.0]);
        assert_eq!(tracker.effect(VICTIM).hold, [1_800; 3]);
        let mut tracker = Tracker::default();
        tracker.observe(1_000, &[victim], Rules::default(), 1 << VICTIM, clear);
        assert_eq!(tracker.effect(VICTIM).loss, [4.0, 4.0, 2.0]);
    }

    #[test]
    fn the_packed_box_unpacks() {
        let standing = (72 << 16) | (24 << 8) | 15;
        assert_eq!(
            Body::unpack_box(standing),
            ([-15.0, -15.0, -24.0], [15.0, 15.0, 40.0])
        );
        let crouching = (48 << 16) | (24 << 8) | 15;
        assert_eq!(Body::unpack_box(crouching).1[2], 16.0);
        assert_eq!(Body::unpack_box(0), STANDING);
    }

    #[test]
    fn the_line_meets_the_nearest_box() {
        let start = Vec3::ZERO;
        let x = Vec3::X;
        let near = ray_box(
            start,
            x,
            Vec3::new(100.0, -15.0, -24.0),
            Vec3::new(130.0, 15.0, 40.0),
        );
        assert_eq!(near, Some(100.0));
        assert_eq!(
            ray_box(
                start,
                x,
                Vec3::new(100.0, 20.0, -24.0),
                Vec3::new(130.0, 50.0, 40.0)
            ),
            None
        );
        assert_eq!(
            ray_box(
                start,
                x,
                Vec3::new(-10.0, -10.0, -10.0),
                Vec3::new(10.0, 10.0, 10.0)
            ),
            None,
            "starts inside"
        );
    }
}
