//! A value the nameplates estimate rather than read: the lowest and the highest it
//! can be, and the best guess between them. The plate draws the guess as the bar
//! and the space between the bounds as a grey haze, so a wide range reads as a
//! blurred edge and an exact one as a sharp edge.

/// An estimate with its bounds; `low <= best <= high` after every operation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Range {
    pub(super) low: f32,
    pub(super) best: f32,
    pub(super) high: f32,
}

impl Range {
    /// A value known exactly.
    pub(super) const fn exact(value: f32) -> Self {
        Self {
            low: value,
            best: value,
            high: value,
        }
    }

    /// A range from its parts, put back in order.
    pub(super) fn new(low: f32, best: f32, high: f32) -> Self {
        Self { low, best, high }.ordered()
    }

    /// Add a change known only within `low..=high` (best guess `best`): the low
    /// bound takes the smallest change and the high bound the largest.
    pub(super) fn add(self, low: f32, best: f32, high: f32) -> Self {
        Self {
            low: self.low + low,
            best: self.best + best,
            high: self.high + high,
        }
        .ordered()
    }

    /// Apply `f`, which must never decrease (a cap, a decay, a pickup), to each part.
    pub(super) fn map(self, f: impl Fn(f32) -> f32) -> Self {
        Self {
            low: f(self.low),
            best: f(self.best),
            high: f(self.high),
        }
        .ordered()
    }

    /// Keep every part within `min..=max`.
    pub(super) fn clamp(self, min: f32, max: f32) -> Self {
        self.map(|value| value.clamp(min, max))
    }

    /// Raise every part to at least `floor`: something seen proves the value was
    /// at least that much.
    pub(super) fn at_least(self, floor: f32) -> Self {
        self.map(|value| value.max(floor))
    }

    /// Lower every part to at most `ceiling`.
    pub(super) fn at_most(self, ceiling: f32) -> Self {
        self.map(|value| value.min(ceiling))
    }

    /// The range as shares of `full`, each within `0..=1`.
    pub(super) fn share(self, full: f32) -> Self {
        if full <= 0.0 {
            return Self::default();
        }
        self.map(|value| (value / full).clamp(0.0, 1.0))
    }

    /// The range as shares of `full`, each within `0..=2`: health and armour go to
    /// twice the maximum (overheal, a large shield on a full one), drawn as a second
    /// layer over the bar.
    pub(super) fn share_over(self, full: f32) -> Self {
        if full <= 0.0 {
            return Self::default();
        }
        self.map(|value| (value / full).clamp(0.0, 2.0))
    }

    /// How far apart the bounds are.
    pub(super) fn width(self) -> f32 {
        self.high - self.low
    }

    fn ordered(self) -> Self {
        let low = self.low.min(self.high);
        let high = self.low.max(self.high);
        Self {
            low,
            best: self.best.clamp(low, high),
            high,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_widen_the_bounds_and_keep_the_guess_inside() {
        let range = Range::exact(100.0).add(-50.0, -20.0, -5.0);
        assert_eq!(range, Range::new(50.0, 80.0, 95.0));
        assert_eq!(range.width(), 45.0);
        // A guess pushed past a bound is held at it.
        let held = Range {
            low: 10.0,
            best: 20.0,
            high: 20.0,
        };
        assert_eq!(Range::new(10.0, 40.0, 20.0), held);
        assert_eq!(Range::new(30.0, 5.0, 20.0).best, 20.0);
    }

    #[test]
    fn floors_ceilings_and_shares() {
        let range = Range::new(0.0, 30.0, 90.0);
        assert_eq!(range.at_least(40.0), Range::new(40.0, 40.0, 90.0));
        assert_eq!(range.at_most(20.0), Range::new(0.0, 20.0, 20.0));
        assert_eq!(range.clamp(10.0, 50.0), Range::new(10.0, 30.0, 50.0));
        let share = Range::new(25.0, 50.0, 150.0).share(100.0);
        assert_eq!(share, Range::new(0.25, 0.5, 1.0));
        assert_eq!(range.share(0.0), Range::default());
        let over = Range::new(99.0, 150.0, 250.0).share_over(100.0);
        assert_eq!(over, Range::new(0.99, 1.5, 2.0));
    }
}
