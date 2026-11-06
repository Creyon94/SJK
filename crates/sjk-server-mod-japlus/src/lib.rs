//! JA+ for SJK's dedicated server: `g_mod japlus` runs JA+ 2.4 Build 7's rules in place
//! of the base game's, so that the server plays as a JA+ server and every JA+ client
//! (SJK's, EternalJK, the JA+ plugin) predicts it.
//!
//! JA+ is closed source. The reference is VisionJK (github.com/Blackwolf1337/visionjk,
//! GPL-2.0), Blackwolf's rebuild of the JA+ 2.4 B7 server module from its Linux binary,
//! compared with the original in random games; its behaviour is ported here, not its
//! code. What runs so far, and what does not, is in `docs/server.md` (JA+ mode).
//!
//! - [`cvars`]: JA+'s settings, at JA+'s defaults.
//! - [`cinfo`]: `jp_cinfo`, the rules the server announces, only those it plays.
//! - [`JaPlus::movement`]: the JA+ movement of the shared `Pmove`
//!   (`sjk_game_jka::pmove_japlus`), selected from the same bits the clients read.
//! - [`JaPlus::duels`]: JA+'s private duels (`sjk_game_jka::duel::DuelRules`), and its
//!   `engage_FullForceDuel` ([`JaPlus::command`]).

pub mod cinfo;
pub mod cvars;

use sjk_game_jka::RollRules;
use sjk_game_jka::duel::{DuelRules, WinnerReport};
use sjk_game_jka::pmove_japlus::JaPlusRules;
use sjk_protocol::{JaPlusCapabilities, ServerDialect};
use sjk_server_mod::{ModCommand, ModMovement, ServerMod, Settings};

/// `gamename` of JA+ 2.4 B7 (VisionJK `g_japlus.h` `JP_GAMENAME`): clients and server
/// browsers tell a JA+ server by it (`sjk_protocol::is_ja_plus_game_name`).
pub const GAMENAME: &str = "JA+ Mod v2.4 B7";
/// Its server info `V` (`JP_SERVER_VERSION`), `<version>B<build>`, which the JA+ plugin
/// reads.
pub const VERSION: &str = "2.4B7";

/// The `dmflags` bit with which a JA+ server keeps the base game's collision boxes
/// (`JP_DF_OLD_COLLISION`), which clients read as `JaPlusRules::old_collision`.
const DF_OLD_COLLISION: i32 = 256;

/// The JA+ server mod.
#[derive(Clone, Copy, Debug, Default)]
pub struct JaPlus;

impl ServerMod for JaPlus {
    fn id(&self) -> &'static str {
        "japlus"
    }

    fn name(&self) -> &'static str {
        "JA+"
    }

    fn cvars(&self) -> &'static [sjk_server_mod::ModCvar] {
        cvars::CVARS
    }

    fn identity(&self) -> &'static [(&'static str, &'static str)] {
        &[("gamename", GAMENAME), ("V", VERSION)]
    }

    fn game_directory(&self) -> &'static str {
        // JA+ servers run in `japlus` (its `server.cfg`, `+set fs_game japlus`).
        "japlus"
    }

    fn derived_info(&self, settings: &dyn Settings) -> Vec<(&'static str, String)> {
        vec![("jp_cinfo", cinfo::cinfo(settings).to_string())]
    }

    fn movement(&self, settings: &dyn Settings) -> ModMovement {
        // What a client makes of this server's info (`JaPlusRules::from_game_state`,
        // `RollRules::from_dialect`), from the same bits.
        let dialect = ServerDialect::JaPlus {
            capabilities: JaPlusCapabilities(cinfo::cinfo(settings)),
        };
        ModMovement {
            ja_plus: JaPlusRules {
                old_collision: settings.integer("dmflags") & DF_OLD_COLLISION != 0,
                ..JaPlusRules::from_dialect(&dialect)
            },
            fix_roll: RollRules::from_dialect(&dialect).fix_roll,
            // JA+ is built on the 1.01 game code, which has none of OpenJK's animation
            // fixes (VisionJK `docs/japlus/differences.md`: `g_fixRunWalkAnims`,
            // `g_fixWeaponAttackAnim` and `g_fixSaberMoveData` are 0 there).
            legacy_fixes: 0,
        }
    }

    fn duels(&self, settings: &dyn Settings) -> DuelRules {
        // VisionJK `g_cmds.c:3262-3452`, `g_active.c:2439-2572`, `japlus/g_japlus_dim.c`,
        // `w_force.c:6256-6257`; the settings' meanings are its web panel's
        // (`webadmin/wa_settings_table.h:130,241-246`).
        DuelRules {
            ja_plus: true,
            // `jp_DuelAlpha` -1 switches the isolation off, as its clients read it
            // (`cgs.jpDuelAlpha >= 0`, VisionJK `cgame/cg_predict.c:478`).
            isolation: settings.integer("jp_DuelAlpha") >= 0,
            team_duels: settings.integer("jp_allowTeamDuel") != 0,
            distance_limit: settings.integer("jp_removeDistanceDuelLimit") == 0,
            bow: settings.integer("jp_autoDuelBOW") == 1,
            start_armor: settings.integer("jp_duelStartArmor"),
            winner_report: WinnerReport::from_setting(settings.integer("jp_endDuelBroadcastMSG")),
            force_regen_time: Some(settings.integer("jp_privateDuelForceRegen")),
        }
    }

    fn command(&self, name: &[u8]) -> Option<ModCommand> {
        // VisionJK `g_cmds.c:3455-3462,3756`.
        name.eq_ignore_ascii_case(b"engage_FullForceDuel")
            .then_some(ModCommand::FullForceDuel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_protocol::{InfoString, ServerProfile};
    use std::collections::HashMap;

    struct Table(HashMap<&'static str, i32>);

    impl Settings for Table {
        fn integer(&self, name: &str) -> i32 {
            self.0.get(name).copied().unwrap_or_else(|| {
                cvars::CVARS
                    .iter()
                    .find(|cvar| cvar.name == name)
                    .map_or(0, |cvar| cvar.default.parse().unwrap_or(0))
            })
        }
        fn value(&self, name: &str) -> f32 {
            self.integer(name) as f32
        }
    }

    #[test]
    fn clients_read_the_identity_as_ja_plus() {
        let info = format!(r"\gamename\{GAMENAME}\V\{VERSION}\jp_cinfo\19");
        let profile = ServerProfile::from_server_info(&InfoString::parse(&info).unwrap());
        assert_eq!(
            profile.dialect,
            ServerDialect::JaPlus {
                capabilities: JaPlusCapabilities(19)
            }
        );
    }

    #[test]
    fn the_server_moves_as_its_clients_predict() {
        let settings = Table(HashMap::new());
        let derived = JaPlus.derived_info(&settings);
        let info = format!(
            r"\gamename\{GAMENAME}\V\{VERSION}\jp_cinfo\{}",
            derived[0].1
        );
        let profile = ServerProfile::from_server_info(&InfoString::parse(&info).unwrap());
        let movement = JaPlus.movement(&settings);
        assert_eq!(
            movement.ja_plus,
            JaPlusRules::from_dialect(&profile.dialect)
        );
        assert_eq!(
            movement.fix_roll,
            RollRules::from_dialect(&profile.dialect).fix_roll
        );
        assert!(movement.ja_plus.enabled && movement.ja_plus.flip_kick);
        assert!(movement.ja_plus.yellow_dfa && !movement.ja_plus.head_slide);
        assert_eq!(movement.fix_roll, 1);
        assert_eq!(movement.legacy_fixes, 0);
    }

    #[test]
    fn dmflags_256_keeps_the_old_collision() {
        let mut settings = Table(HashMap::new());
        assert!(!JaPlus.movement(&settings).ja_plus.old_collision);
        settings.0.insert("dmflags", 256 | 8);
        assert!(JaPlus.movement(&settings).ja_plus.old_collision);
    }

    #[test]
    fn the_duels_follow_the_settings() {
        let mut settings = Table(HashMap::new());
        let rules = JaPlus.duels(&settings);
        // JA+'s defaults: isolated, team duels, no distance limit, the bow, 100 armour,
        // everyone told, 200 ms a Force point.
        assert!(rules.ja_plus && rules.isolation && rules.team_duels && rules.bow);
        assert!(!rules.distance_limit);
        assert_eq!(rules.start_armor, 100);
        assert_eq!(rules.winner_report, WinnerReport::Everyone);
        assert_eq!(rules.force_regen_time, Some(200));
        settings.0.insert("jp_DuelAlpha", -1);
        settings.0.insert("jp_allowTeamDuel", 0);
        settings.0.insert("jp_removeDistanceDuelLimit", 0);
        settings.0.insert("jp_autoDuelBOW", 0);
        settings.0.insert("jp_endDuelBroadcastMSG", 2);
        let rules = JaPlus.duels(&settings);
        assert!(!rules.isolation && !rules.team_duels && !rules.bow);
        assert!(rules.distance_limit);
        assert_eq!(rules.winner_report, WinnerReport::Duellists);
        settings.0.insert("jp_DuelAlpha", 0);
        settings.0.insert("jp_endDuelBroadcastMSG", 0);
        let rules = JaPlus.duels(&settings);
        assert!(rules.isolation);
        assert_eq!(rules.winner_report, WinnerReport::Silent);
    }

    #[test]
    fn the_full_force_duel_is_a_command() {
        assert_eq!(
            JaPlus.command(b"engage_fullforceduel"),
            Some(ModCommand::FullForceDuel)
        );
        assert_eq!(JaPlus.command(b"engage_duel"), None);
    }

    #[test]
    fn a_rule_turned_off_is_off_for_the_server_too() {
        let mut settings = Table(HashMap::new());
        settings.0.insert("jp_allowflipkick", 0);
        settings.0.insert("jp_fixRoll", 0);
        let movement = JaPlus.movement(&settings);
        assert!(movement.ja_plus.enabled && !movement.ja_plus.flip_kick);
        assert_eq!(movement.fix_roll, 0);
    }
}
