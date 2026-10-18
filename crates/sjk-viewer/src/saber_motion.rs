//! A blade's motion, for the Blade Lab designs that answer it (`saber.wgsl` `motion`,
//! [`crate::blade_lab_designs`]): how hard it swings, and the seconds since it was lit,
//! since it last clashed and since its last swing began. Kept per blade with its trail
//! ([`crate::saber_trail::BladeState`]); presentation only.

use crate::saber::Blade;
use std::sync::Mutex;

/// Seconds reported for something that has not happened, or long enough ago to be
/// settled (a blade lit before it was first seen, a swing long over).
pub(crate) const SETTLED: f32 = 1_000.0;
/// The tip's speed about the hilt (units a second) of a full swing: the lab's 5.5 m/s.
/// About the hilt, so running with the blade held still is no swing.
const FULL_SWING: f32 = 5.5 / 0.027;
/// The lab's swing rises half the way to the tip's speed each frame and falls 8% of the
/// way, at 60 frames a second.
const RISE: f32 = 0.5;
const FALL: f32 = 0.08;
/// A swing begins when the swing rises past this and ends when it falls under the other,
/// above an idle stance's sway; a chain of swings without a pause between them begins a
/// new one every `SWING_EVERY` milliseconds (each swing of a combo is one).
const SWING_BEGINS: f32 = 0.4;
const SWING_ENDS: f32 = 0.25;
const SWING_EVERY: i64 = 1_000;
/// A blade unseen this long (milliseconds) was put away; seen again, it is lit anew.
const UNSEEN: i64 = 250;

/// How near (units) a clash must be to a blade for it to be that blade's.
const CLASH_REACH: f32 = 12.0;

/// The latest clashes the snapshots brought (where, numbered in order), for the blades
/// to find theirs ([`note_clash`]); the newest eight.
static CLASHES: Mutex<Clashes> = Mutex::new(NO_CLASHES);

/// The newest eight clashes (where, and their numbers) and the newest's number.
type Clashes = ([([f32; 3], u64); 8], u64);
const NO_CLASHES: Clashes = ([([0.0; 3], 0); 8], 0);

/// A saber clash at `origin` (`impact_spawn.rs`, as its flare latches).
pub(crate) fn note_clash(origin: [f32; 3]) {
    if let Ok(mut clashes) = CLASHES.lock() {
        clashes.1 += 1;
        let number = clashes.1;
        clashes.0[(number % 8) as usize] = (origin, number);
    }
}

/// The motion `saber.wgsl` reads: swing (0 to 1), seconds since the last clash, since
/// the blade was lit and since the last swing began.
pub(crate) const STILL: [f32; 4] = [0.0, SETTLED, SETTLED, SETTLED];

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BladeMotion {
    tip: Option<[f32; 3]>,
    seen: i64,
    swing: f32,
    swinging: bool,
    lit: Option<i64>,
    swung: Option<i64>,
    clashed: Option<i64>,
    /// The newest clash this blade has looked at.
    clash_seen: u64,
}

impl BladeMotion {
    /// The blade drawn at `now` (presentation milliseconds), `growing` while it is still
    /// coming out of the hilt; its motion for the shader.
    pub(crate) fn record(&mut self, blade: Blade, now: i64, growing: bool) -> [f32; 4] {
        // The tip from the hilt: how the blade turns, not how its wearer moves.
        let tip = [
            blade.direction[0] * blade.length,
            blade.direction[1] * blade.length,
            blade.direction[2] * blade.length,
        ];
        let gap = now - self.seen;
        let fresh = self.tip.is_none() || !(0..=UNSEEN).contains(&gap);
        let clashes = CLASHES.lock().map(|clashes| *clashes).unwrap_or(NO_CLASHES);
        if fresh {
            *self = Self {
                lit: growing.then_some(now),
                clash_seen: clashes.1,
                ..Self::default()
            };
        } else if gap > 0 {
            let seconds = gap as f32 * 0.001;
            let last = self.tip.unwrap_or(tip);
            let moved = ((tip[0] - last[0]).powi(2)
                + (tip[1] - last[1]).powi(2)
                + (tip[2] - last[2]).powi(2))
            .sqrt();
            let target = (moved / seconds / FULL_SWING).clamp(0.0, 1.0);
            let frames = seconds * 60.0;
            let rate = if target > self.swing { RISE } else { FALL };
            self.swing += (target - self.swing) * (1.0 - (1.0 - rate).powf(frames));
            if !self.swinging && self.swing > SWING_BEGINS {
                self.swinging = true;
                self.swung = Some(now);
            } else if self.swinging && self.swing < SWING_ENDS {
                self.swinging = false;
            } else if self.swinging && self.swung.is_some_and(|at| now - at >= SWING_EVERY) {
                self.swung = Some(now);
            }
        }
        if clashes.1 > self.clash_seen {
            let near = clashes.0.iter().any(|&(origin, number)| {
                number > self.clash_seen && distance_to(origin, blade) < CLASH_REACH + blade.radius
            });
            if near {
                self.clash(now);
            }
            self.clash_seen = clashes.1;
        }
        self.tip = Some(tip);
        self.seen = now;
        self.motion(now)
    }

    /// A clash at `now`.
    pub(crate) fn clash(&mut self, now: i64) {
        self.clashed = Some(now);
    }

    fn motion(&self, now: i64) -> [f32; 4] {
        let since = |at: Option<i64>| {
            at.map_or(SETTLED, |at| {
                ((now - at).max(0) as f32 * 0.001).min(SETTLED)
            })
        };
        [
            self.swing,
            since(self.clashed),
            since(self.lit),
            since(self.swung),
        ]
    }
}

/// How far `point` is from `blade` (the segment from its base to its tip).
fn distance_to(point: [f32; 3], blade: Blade) -> f32 {
    let v = [
        point[0] - blade.base[0],
        point[1] - blade.base[1],
        point[2] - blade.base[2],
    ];
    let d = blade.direction;
    let along = (v[0] * d[0] + v[1] * d[1] + v[2] * d[2]).clamp(0.0, blade.length);
    ((v[0] - d[0] * along).powi(2) + (v[1] - d[1] * along).powi(2) + (v[2] - d[2] * along).powi(2))
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blade(angle: f32) -> Blade {
        Blade {
            base: [0.0; 3],
            direction: [angle.cos(), angle.sin(), 0.0],
            length: 40.0,
            radius: 3.0,
        }
    }

    #[test]
    fn a_blade_held_still_is_settled_and_one_seen_lighting_is_young() {
        let mut motion = BladeMotion::default();
        assert_eq!(motion.record(blade(0.0), 1_000, false), STILL);
        let mut lighting = BladeMotion::default();
        assert_eq!(lighting.record(blade(0.0), 1_000, true)[2], 0.0);
        // Half a second later, by 100 ms steps (never unseen).
        let mut lighting = BladeMotion::default();
        lighting.record(blade(0.0), 1_000, true);
        let mut last = [0.0; 4];
        for step in 1..=5 {
            last = lighting.record(blade(0.0), 1_000 + step * 100, false);
        }
        assert!((last[2] - 0.5).abs() < 1e-4);
        assert_eq!(last[0], 0.0);
    }

    #[test]
    fn a_fast_turn_swings_and_marks_its_start_then_settles() {
        let mut motion = BladeMotion::default();
        motion.record(blade(0.0), 0, false);
        // A quarter turn in 100 ms, 16 ms a frame: the tip moves about 630 units a second.
        let mut at = 0;
        let mut began = None;
        for frame in 1..=6 {
            at = frame * 16;
            let m = motion.record(blade(frame as f32 * 0.25), at, false);
            if m[3] < SETTLED && began.is_none() {
                began = Some(at);
            }
        }
        let swung = motion.record(blade(1.5), at + 16, false);
        assert!(swung[0] > 0.8, "{swung:?}");
        assert!(began.is_some());
        // Held still for two seconds, it settles and the swing's start keeps counting.
        let mut still = swung;
        for frame in 1..=120 {
            still = motion.record(blade(1.5), at + 16 + frame * 16, false);
        }
        assert!(still[0] < 0.01, "{still:?}");
        assert!(still[3] > 1.9 && still[3] < 2.1, "{still:?}");
    }

    #[test]
    fn running_with_the_blade_held_still_is_no_swing() {
        let mut motion = BladeMotion::default();
        for frame in 0..60 {
            // 400 units a second along x, the blade upright.
            let moving = Blade {
                base: [frame as f32 * 6.4, 0.0, 0.0],
                ..blade(1.5)
            };
            let m = motion.record(moving, frame * 16, false);
            assert!(m[0] < 0.01 && m[3] == SETTLED, "{m:?}");
        }
    }

    #[test]
    fn a_long_chain_of_swings_begins_one_every_so_often() {
        let mut motion = BladeMotion::default();
        motion.record(blade(0.0), 0, false);
        let mut starts = 0;
        let mut last = SETTLED;
        // Two seconds of turning without a pause.
        for frame in 1..=125 {
            let m = motion.record(blade(frame as f32 * 0.25), frame * 16, false);
            if m[3] < last {
                starts += 1;
            }
            last = m[3];
        }
        assert_eq!(starts, 2);
    }

    #[test]
    fn a_clash_counts_from_when_it_happened() {
        let mut motion = BladeMotion::default();
        motion.record(blade(0.0), 0, false);
        motion.clash(100);
        assert!((motion.record(blade(0.0), 200, false)[1] - 0.1).abs() < 1e-4);
    }

    #[test]
    fn a_clash_on_the_blade_is_its_own_and_one_far_off_is_not() {
        // Far along another axis, so other tests' clashes (the list is shared) are not ours.
        let place = |angle: f32| Blade {
            base: [5_000.0, 0.0, 0.0],
            ..blade(angle)
        };
        let mut near = BladeMotion::default();
        near.record(place(0.0), 0, false);
        note_clash([5_020.0, 4.0, 0.0]);
        assert_eq!(near.record(place(0.0), 16, false)[1], 0.0);
        let far_blade = Blade {
            base: [9_000.0, 0.0, 0.0],
            ..blade(0.0)
        };
        let mut other = BladeMotion::default();
        other.record(far_blade, 0, false);
        note_clash([5_020.0, 4.0, 0.0]);
        assert_eq!(other.record(far_blade, 16, false)[1], SETTLED);
    }
}
