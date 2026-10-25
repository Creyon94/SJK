//! Growing blades (`docs/unlockables.md`, "Growing blades"; 25/10/2026, Sol's Hearth): a
//! signature shader whose parts come one after another the longer its holder has held it.
//! Its header says when, `// @grows {"<option id>": <days>, ...}` ([`Growth`]): an option
//! it names is available once the blade has been held that many whole days, counted from
//! the unlock's `since` when the hub set one, else from when it was granted ([`days_held`]);
//! the options it does not name are there from the start.
//!
//! A part not grown yet is off for its holder, on top of what they switched off themselves
//! ([`locked_words`], [`gated`]): in how their own blade is drawn and in the look sent to the
//! hub (`saber_off`, `looks_frame.rs`), so every other SJK player sees the same stage with
//! nothing new on the wire. The looks work it out again twice a second from the clock and
//! the profile, so a stage reached while playing (or a `since` the hub moved) goes out with
//! the next look. The Collection shows such a part locked, with when it grows ([`Stage`],
//! [`grows_in`]), and may preview the blade fully grown on its stage alone.

/// A day, in seconds.
const DAY: i64 = 86_400;

/// Past this many days, when a part grows is said in months (of 30 days).
const DAYS_SAID: u32 = 60;

/// A growing blade's stages (its signature's `@grows` line): the options it names, each with
/// the whole days its holder must have held the blade before it is available.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Growth {
    /// `(option id, days)`, the soonest first.
    stages: Vec<(String, u32)>,
}

impl Growth {
    /// Read an `@grows` value, a JSON object of option ids and days; `None` when it does not
    /// parse or names no option.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let named: std::collections::BTreeMap<String, u32> = serde_json::from_str(value).ok()?;
        let mut stages: Vec<(String, u32)> = named.into_iter().collect();
        // A stable sort keeps the ids in order within a day.
        stages.sort_by_key(|(_, days)| *days);
        (!stages.is_empty()).then_some(Self { stages })
    }

    /// The options it names with their days, the soonest first.
    pub(crate) fn stages(&self) -> impl Iterator<Item = (&str, u32)> {
        self.stages.iter().map(|(id, days)| (id.as_str(), *days))
    }

    /// The days held after which it is fully grown.
    pub(crate) fn full(&self) -> u32 {
        self.stages.last().map_or(0, |(_, days)| *days)
    }

    /// The options not grown yet after `held` days.
    pub(crate) fn locked_ids(&self, held: u32) -> impl Iterator<Item = &str> {
        self.stages()
            .filter(move |(_, days)| *days > held)
            .map(|(id, _)| id)
    }

    /// The days left after `held` before option `id` grows; `None` once it is available (or
    /// for an option there from the start).
    pub(crate) fn days_left(&self, id: &str, held: u32) -> Option<u32> {
        self.stages()
            .find(|(named, _)| *named == id)
            .and_then(|(_, days)| days.checked_sub(held))
            .filter(|left| *left > 0)
    }
}

/// Where a player's growing blade stands: how it grows and the whole days they have held it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stage<'a> {
    pub(crate) growth: &'a Growth,
    pub(crate) held: u32,
}

impl Stage<'static> {
    /// Blade skin `skin` as the player with `holdings` holds it at `now` (unix seconds):
    /// `None` unless it grows and their profile lists it.
    pub(crate) fn of(
        skin: &str,
        holdings: &crate::unlockables::Holdings<'_>,
        now: i64,
    ) -> Option<Self> {
        let growth = of(skin)?;
        let unlock = holdings.unlock(skin)?;
        Some(Self {
            growth,
            held: days_held(unlock, now),
        })
    }
}

impl Stage<'_> {
    /// The days left before option `id` grows; `None` once it is available.
    pub(crate) fn days_left(&self, id: &str) -> Option<u32> {
        self.growth.days_left(id, self.held)
    }

    /// Whether every part has grown.
    pub(crate) fn grown(&self) -> bool {
        self.held >= self.growth.full()
    }

    /// How long it has been held, as the Collection says it: "Held 12 days", and ", fully
    /// grown" once it is.
    pub(crate) fn held_words(&self) -> String {
        let days = if self.held == 1 { "day" } else { "days" };
        if self.grown() {
            format!("Held {} {days}, fully grown", self.held)
        } else {
            format!("Held {} {days}", self.held)
        }
    }
}

/// When a part grows, `left` days from now: "Grows in 23 days", and in months past
/// [`DAYS_SAID`] days ("Grows in 4 months").
pub(crate) fn grows_in(left: u32) -> String {
    match left {
        1 => "Grows in 1 day".to_owned(),
        left if left > DAYS_SAID => format!("Grows in {} months", (left + 15) / 30),
        left => format!("Grows in {left} days"),
    }
}

/// When `unlock`'s growth counts from (unix seconds): its `since` when the hub set one,
/// else when it was granted.
fn start(unlock: &sjk_identity::Unlock) -> i64 {
    if unlock.since > 0 {
        unlock.since
    } else {
        unlock.granted
    }
}

/// The whole days `unlock` has been held at `now` (unix seconds), from [`start`]; 0 before.
pub(crate) fn days_held(unlock: &sjk_identity::Unlock, now: i64) -> u32 {
    u32::try_from((now - start(unlock)).max(0) / DAY).unwrap_or(u32::MAX)
}

/// The growth start that makes a blade show `days` held at `now`: what
/// `debug_collectibles` gives a growing blade (never before 1970, which reads as unset).
pub(crate) fn start_for(days: u32, now: i64) -> i64 {
    (now - i64::from(days) * DAY).max(1)
}

/// Now, unix seconds.
pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

/// The growing blade skins, each with how it grows, read once from the signatures' headers.
fn growing() -> &'static [(&'static str, Growth)] {
    static GROWING: std::sync::OnceLock<Vec<(&'static str, Growth)>> = std::sync::OnceLock::new();
    GROWING.get_or_init(|| {
        crate::blade_signatures::SIGNATURES
            .iter()
            .filter_map(|signature| Some((signature.id, signature.grows()?)))
            .collect()
    })
}

/// Every growing blade skin with how it grows.
fn each() -> impl Iterator<Item = (&'static str, &'static Growth)> {
    #[cfg(test)]
    let fake = tests::faked();
    #[cfg(not(test))]
    let fake = None;
    fake.into_iter()
        .chain(growing().iter().map(|(skin, growth)| (*skin, growth)))
}

/// How blade skin `skin` grows, if it does.
pub(crate) fn of(skin: &str) -> Option<&'static Growth> {
    each().find(|(id, _)| *id == skin).map(|(_, growth)| growth)
}

/// The parts not grown yet of every growing blade `held` gives the whole days held for
/// (`None`: not held), as `cg_saberSkinOptions` words (`saber_hearth.blaze`): what [`gated`]
/// adds to the player's own choice.
pub(crate) fn locked_words(held: impl Fn(&str) -> Option<u32>) -> String {
    let mut words = String::new();
    for (skin, growth) in each() {
        let Some(days) = held(skin) else {
            continue;
        };
        for id in growth.locked_ids(days) {
            if !words.is_empty() {
                words.push(' ');
            }
            words.push_str(skin);
            words.push('.');
            words.push_str(id);
        }
    }
    words
}

/// `options` (`cg_saberSkinOptions`) with the `locked` words of [`locked_words`] too: the
/// parts the own blade is drawn and sent without.
pub(crate) fn gated(options: &str, locked: &str) -> String {
    if locked.is_empty() {
        options.to_owned()
    } else {
        format!("{options} {locked}")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        /// A blade skin made to grow by the test running on this thread.
        static FAKE: Cell<Option<(&'static str, &'static Growth)>> = const { Cell::new(None) };
    }

    /// The blade skin the test on this thread made grow, if any.
    pub(super) fn faked() -> Option<(&'static str, &'static Growth)> {
        FAKE.with(Cell::get)
    }

    /// While the guard lives, blade skin `skin` grows as `value` (an `@grows` value) says,
    /// on this test's thread only: no signature grows before Sol's Hearth lands.
    pub(crate) fn fake(skin: &'static str, value: &str) -> Fake {
        let growth: &'static Growth = Box::leak(Box::new(Growth::parse(value).expect("@grows")));
        FAKE.with(|fake| fake.set(Some((skin, growth))));
        Fake
    }

    /// Ends a [`fake`] when dropped.
    pub(crate) struct Fake;

    impl Drop for Fake {
        fn drop(&mut self) {
            FAKE.with(|fake| fake.set(None));
        }
    }

    /// A grant of `id` whose growth started `days` whole days (and a little) before now.
    pub(crate) fn held_for(id: &str, days: u32) -> sjk_identity::Unlock {
        sjk_identity::Unlock {
            id: id.to_owned(),
            granted: now() - 900 * DAY,
            since: start_for(days, now()) - 60,
            note: String::new(),
            count: 1,
            medal: None,
            card: None,
        }
    }

    /// Hearth's line (25/10/2026).
    const HEARTH: &str = r#"{"hearth_flames":30,"heat_shimmer":91,"blaze":182,"hearthstones":365}"#;

    #[test]
    fn a_grows_line_reads_its_stages_soonest_first() {
        let growth = Growth::parse(HEARTH).unwrap();
        assert_eq!(
            growth.stages().collect::<Vec<_>>(),
            [
                ("hearth_flames", 30),
                ("heat_shimmer", 91),
                ("blaze", 182),
                ("hearthstones", 365)
            ]
        );
        assert_eq!(growth.full(), 365);
        for broken in ["", "{}", "[30]", r#"{"blaze":-1}"#, r#"{blaze:30}"#] {
            assert_eq!(Growth::parse(broken), None, "{broken}");
        }
    }

    /// Each stage comes on its day: before it the part is locked, with the days left.
    #[test]
    fn parts_grow_on_their_day() {
        let growth = Growth::parse(HEARTH).unwrap();
        let locked = |held| growth.locked_ids(held).collect::<Vec<_>>();
        assert_eq!(locked(0).len(), 4, "nothing grown on the first day");
        assert_eq!(locked(29).len(), 4);
        assert_eq!(locked(30), ["heat_shimmer", "blaze", "hearthstones"]);
        assert_eq!(locked(91), ["blaze", "hearthstones"]);
        assert_eq!(locked(364), ["hearthstones"]);
        assert!(locked(365).is_empty(), "fully grown at a year");
        assert_eq!(growth.days_left("hearth_flames", 7), Some(23));
        assert_eq!(growth.days_left("hearth_flames", 30), None);
        assert_eq!(growth.days_left("embers", 0), None, "there from the start");
        let stage = Stage {
            growth: &growth,
            held: 100,
        };
        assert!(!stage.grown());
        assert_eq!(stage.days_left("blaze"), Some(82));
        assert_eq!(stage.held_words(), "Held 100 days");
        let year = Stage {
            growth: &growth,
            held: 400,
        };
        assert!(year.grown());
        assert_eq!(year.days_left("hearthstones"), None);
        assert_eq!(year.held_words(), "Held 400 days, fully grown");
        assert_eq!(Stage { held: 1, ..year }.held_words(), "Held 1 day");
    }

    /// Days held count from `since` when the hub set one, else from the grant, in whole days.
    #[test]
    fn days_held_count_from_since_or_the_grant() {
        let now = 1_793_000_000;
        let mut unlock = held_for("saber_sun", 0);
        unlock.granted = now - 10 * DAY - 5;
        unlock.since = 0;
        assert_eq!(days_held(&unlock, now), 10, "from the grant");
        unlock.since = now - 3 * DAY + 5;
        assert_eq!(days_held(&unlock, now), 2, "from since, whole days");
        unlock.since = now + DAY;
        assert_eq!(days_held(&unlock, now), 0, "a start to come");
        assert_eq!(days_held(&held_for("saber_sun", 7), super::now()), 7);
        assert_eq!(start_for(5, now), now - 5 * DAY);
        assert_eq!(start_for(u32::MAX, now), 1, "never unset");
    }

    #[test]
    fn when_a_part_grows_is_said_in_days_then_months() {
        assert_eq!(grows_in(1), "Grows in 1 day");
        assert_eq!(grows_in(23), "Grows in 23 days");
        assert_eq!(grows_in(60), "Grows in 60 days");
        assert_eq!(grows_in(61), "Grows in 2 months");
        assert_eq!(grows_in(120), "Grows in 4 months");
        assert_eq!(grows_in(365), "Grows in 12 months");
    }

    /// The words added to the own choice: the parts not grown of each growing blade held.
    #[test]
    fn the_locked_words_name_each_held_blade_s_parts_not_grown() {
        assert_eq!(locked_words(|_| Some(0)), "", "no signature grows yet");
        let _fake = fake("saber_sun", r#"{"glint":30,"haze":91}"#);
        assert_eq!(of("saber_sun").map(Growth::full), Some(91));
        assert_eq!(of("saber_void"), None);
        assert_eq!(locked_words(|_| None), "", "not held");
        assert_eq!(locked_words(|_| Some(0)), "saber_sun.glint saber_sun.haze");
        assert_eq!(locked_words(|_| Some(40)), "saber_sun.haze");
        assert_eq!(locked_words(|_| Some(u32::MAX)), "");
        let options = gated("saber_sun.sparks", &locked_words(|_| Some(40)));
        assert_eq!(
            crate::saber_skin_options::off_list(&options, "saber_sun"),
            ["haze", "sparks"]
        );
        assert_eq!(gated("saber_sun.sparks", ""), "saber_sun.sparks");
    }
}
