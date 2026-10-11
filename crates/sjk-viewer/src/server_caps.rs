//! Servers known to take user commands no faster than a step of their own, though
//! their serverinfo does not say so (they are not `pmove_fixed`). The gameplay FPS
//! (`gameplay_fps.rs`, `sjk_client::command_rate::command_step`) never makes
//! commands faster than such a server takes, unless the player lifts the caps with
//! [`OVERRIDE_CVAR`]. Servers that drop fast inputs and are not listed open the
//! gameplay FPS card by themselves (`input_health.rs`); one found that way belongs
//! in [`KNOWN`].

use sjk_client::CompatProfile;
use sjk_protocol::InfoString;

/// 1 lifts every cap in [`KNOWN`]: the gameplay FPS applies as set.
pub(crate) const OVERRIDE_CVAR: &str = "gameplay_fps_nocap";

/// A server family and the shortest command step it takes.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ServerCap {
    /// Who runs it, as the log and the card name it.
    pub(crate) name: &'static str,
    /// Only JA+ servers (`CompatProfile::JaPlus`) are this family.
    pub(crate) ja_plus_only: bool,
    /// The serverinfo key and the value (case and spaces ignored) that tell it.
    pub(crate) key: &'static str,
    pub(crate) value: &'static str,
    /// The shortest step it takes, in milliseconds.
    pub(crate) step: i32,
}

impl ServerCap {
    /// Commands a second at most.
    pub(crate) fn per_second(&self) -> i32 {
        1000 / self.step.max(1)
    }

    fn matches(&self, profile: &CompatProfile, info: &InfoString) -> bool {
        (!self.ja_plus_only || matches!(profile, CompatProfile::JaPlus { .. }))
            && info
                .get(self.key)
                .is_some_and(|value| value.trim().eq_ignore_ascii_case(self.value))
    }
}

/// The servers known to cap commands.
pub(crate) const KNOWN: &[ServerCap] = &[
    // JoF's JA+ servers (serverinfo `V` 2.5B0, as JoF EJK's `CL_JoFTrustedServer`
    // and the `jof` mod tell them) report `pmove_fixed` 0, yet 1000 commands a
    // second made the game unplayable there (Sol, 11/10/2026); JoF EJK sends them
    // 125 a second (`cl_cmdratecap`).
    ServerCap {
        name: "JoF",
        ja_plus_only: true,
        key: "V",
        value: "2.5B0",
        step: sjk_client::command_rate::COMMAND_MILLIS,
    },
];

/// The known cap of the server whose serverinfo is `server_info`, if any.
pub(crate) fn find(profile: &CompatProfile, server_info: &str) -> Option<&'static ServerCap> {
    let info = InfoString::parse(server_info).ok()?;
    KNOWN.iter().find(|cap| cap.matches(profile, &info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jof_servers_are_known_and_others_are_not() {
        let ja_plus = CompatProfile::JaPlus { version: None };
        let jof = find(&ja_plus, r"\gamename\JA+ Mod v2.5\V\2.5B0").unwrap();
        assert_eq!((jof.name, jof.step, jof.per_second()), ("JoF", 8, 125));
        assert!(find(&ja_plus, r"\gamename\JA+ Mod v2.4\V\2.4B7").is_none());
        assert!(find(&CompatProfile::BaseJka, r"\V\2.5B0").is_none());
        assert!(find(&ja_plus, "not an info string").is_none());
    }

    #[test]
    fn every_known_cap_is_a_real_step() {
        for cap in KNOWN {
            assert!((1..=50).contains(&cap.step), "{}", cap.name);
            assert!(!cap.key.is_empty() && !cap.value.is_empty(), "{}", cap.name);
        }
    }
}
