//! Server mods: rule sets SJK's dedicated server can run in place of the base game's,
//! as a JKA server runs a mod's game module (JA+ in `japlus`, for example).
//!
//! A mod is one [`ServerMod`]. The server (`sjk-dedicated`) keeps the table of the mods
//! it offers and runs the one its `g_mod` setting names; with none, it is the base game.
//! What a mod may change goes through the hooks of this crate, so the server's own code
//! never asks which mod runs: it asks the running mod.
//!
//! The rule a mod follows for movement: the server runs what its server info tells the
//! clients to predict. A JA+ server announces its rules in `jp_cinfo`, every JA+ client
//! (SJK's, EternalJK, the JA+ plugin) predicts from it, and the server's own moves follow
//! the same bits ([`ServerMod::movement`]), so no client is pulled back for a rule it was
//! told about.

use sjk_game_jka::pmove::MovementConfig;
use sjk_game_jka::pmove_japlus::JaPlusRules;

/// The cvar flags a mod's settings use, the bits of the engine's `cvar.cpp`
/// (`q_shared.h`), which the server's cvar table keeps as they are.
pub mod flags {
    /// `CVAR_ARCHIVE`: saved to the server's configuration file.
    pub const ARCHIVE: u32 = 0x1;
    /// `CVAR_SERVERINFO`: part of the server info string clients and browsers read.
    pub const SERVERINFO: u32 = 0x4;
    /// `CVAR_SYSTEMINFO`: copied to every client.
    pub const SYSTEMINFO: u32 = 0x8;
    /// `CVAR_LATCH`: a change waits for the next map.
    pub const LATCH: u32 = 0x20;
    /// `CVAR_ROM`: set by the server only.
    pub const ROM: u32 = 0x40;
    /// `CVAR_INTERNAL`: never shown in an info string (passwords).
    pub const INTERNAL: u32 = 0x800;
}

/// One setting a mod registers when it runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModCvar {
    pub name: &'static str,
    /// The value it starts with, the mod's own default.
    pub default: &'static str,
    /// [`flags`] bits.
    pub flags: u32,
    /// Every change is announced to the players (`Server: <name> changed to <value>`).
    pub announced: bool,
}

/// The server's settings as a mod reads them.
pub trait Settings {
    /// `Cvar_VariableIntegerValue`: 0 for a setting that does not exist.
    fn integer(&self, name: &str) -> i32;
    /// `Cvar_VariableValue`: 0 for a setting that does not exist.
    fn value(&self, name: &str) -> f32;
}

/// The movement rules a mod runs, which the server puts into every player's moves.
/// [`Default`] is the base game's, as SJK's server runs it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModMovement {
    /// JA+'s rules (`MovementConfig::ja_plus`); off on the base game.
    pub ja_plus: JaPlusRules,
    /// The roll fix in force (`RollRules::fix_roll`, 0 to 3); 0 on the base game.
    pub fix_roll: u8,
    /// Which of OpenJK's animation fixes run (`MovementConfig::legacy_fixes`, published
    /// in `CS_LEGACY_FIXES`, which clients predict from).
    pub legacy_fixes: u32,
}

impl Default for ModMovement {
    fn default() -> Self {
        let base = MovementConfig::default();
        Self {
            ja_plus: base.ja_plus,
            fix_roll: base.roll_rules.fix_roll,
            legacy_fixes: base.legacy_fixes,
        }
    }
}

impl ModMovement {
    /// `config` with these rules, the rest of it (a player's sabers, its replay state's
    /// policy) left alone.
    pub fn apply(self, config: &mut MovementConfig) {
        config.ja_plus = self.ja_plus;
        config.roll_rules.fix_roll = self.fix_roll;
        config.legacy_fixes = self.legacy_fixes;
    }
}

/// A rule set the server can run in place of the base game.
pub trait ServerMod: Sync {
    /// Its `g_mod` value (`japlus`): letters, digits and `_` only.
    fn id(&self) -> &'static str;
    /// The name operators and players know it by (`JA+`).
    fn name(&self) -> &'static str;
    /// The settings it registers when it runs, at its own defaults.
    fn cvars(&self) -> &'static [ModCvar];
    /// The read-only server info it announces itself with (`gamename`, a version key),
    /// which clients and server browsers tell the mod by.
    fn identity(&self) -> &'static [(&'static str, &'static str)];
    /// The mod folder its servers report to server browsers (`getinfo`'s `game`, which
    /// is a JKA server's `fs_game`): browsers list and filter mods by it. Only the
    /// report; clients load no files of that folder for it.
    fn game_directory(&self) -> &'static str;
    /// Server info worked out from its settings (JA+'s `jp_cinfo`), as `(setting,
    /// value)`. The server asks again whenever a setting changes and writes the values
    /// into those settings, which the mod registers read-only ([`Self::cvars`]).
    fn derived_info(&self, settings: &dyn Settings) -> Vec<(&'static str, String)>;
    /// The movement rules its settings select: those its server info tells clients to
    /// predict, never more.
    fn movement(&self, settings: &dyn Settings) -> ModMovement;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_movement_is_the_base_servers() {
        let mut config = MovementConfig::default();
        let before = config;
        ModMovement::default().apply(&mut config);
        assert_eq!(config, before);
        assert!(!config.ja_plus.enabled);
    }

    #[test]
    fn applying_rules_keeps_the_rest_of_a_players_config() {
        let mut config = MovementConfig {
            saber_speed_scales: [0.5, 1.0],
            ..MovementConfig::default()
        };
        config.roll_rules.saber_forbids_rolls = true;
        ModMovement {
            ja_plus: JaPlusRules {
                enabled: true,
                ..JaPlusRules::default()
            },
            fix_roll: 2,
            legacy_fixes: 0,
        }
        .apply(&mut config);
        assert!(config.ja_plus.enabled);
        assert_eq!(config.roll_rules.fix_roll, 2);
        assert!(config.roll_rules.saber_forbids_rolls);
        assert_eq!(config.legacy_fixes, 0);
        assert_eq!(config.saber_speed_scales, [0.5, 1.0]);
    }
}
