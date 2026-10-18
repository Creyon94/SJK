//! Team saber colours, three settings:
//!
//! - `cg_saberTeamColors` (archived, default 1), JoF EJK's: in team gametypes other than
//!   siege and jedi-vs-merc a red player's blades are red and a blue player's blue
//!   (`codemp/cgame/cg_players.c:8096-8111`); at 0 only a blade in the other team's
//!   colour or an extended one (RGB, black) is replaced
//!   ([`sjk_client::legacy_saber_team_colors`]). A blade skin stays worn and takes the
//!   team colour as its base, as it takes the player's own colour elsewhere.
//! - `cg_ctfTeamSabers` (archived, off by default; SJK pull request #113 by Slash, made an
//!   option at the merge, Sol's choice 17/10/2026): in capture the flag and capture the
//!   ysalamiri every red or blue player's blades are drawn in the team's stock colour with
//!   no blade shader over them (`sjk_client::legacy_team_blade`).
//! - `cg_siegeTeamSabers` (archived, on by default; Slash's request, 17/10/2026): in Siege,
//!   which EJK's rule leaves out, a red player's blades are red and a blue player's blue,
//!   the blade skin worn over the colour as under `cg_saberTeamColors`
//!   (`sjk_client::legacy_siege_team_blade`).

use crate::console::ViewerConsole;
use sjk_protocol::GameState;

/// The archived CTF setting.
pub(crate) const CVAR: &str = "cg_ctfTeamSabers";
/// Lower case, for the per-frame lookup without an allocation.
const LOOKUP: &str = "cg_ctfteamsabers";
/// The archived JoF EJK setting.
pub(crate) const TEAM_COLORS_CVAR: &str = "cg_saberTeamColors";
const TEAM_COLORS_LOOKUP: &str = "cg_saberteamcolors";
/// The archived Siege setting.
pub(crate) const SIEGE_CVAR: &str = "cg_siegeTeamSabers";
const SIEGE_LOOKUP: &str = "cg_siegeteamsabers";

/// The three settings, read once a frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Settings {
    /// `cg_ctfTeamSabers`.
    pub(crate) ctf: bool,
    /// `cg_saberTeamColors`: every blade, not only the other team's or an extended one.
    pub(crate) always: bool,
    /// `cg_siegeTeamSabers`.
    pub(crate) siege: bool,
}

impl Settings {
    /// The settings as the console holds them (their defaults without a console).
    pub(crate) fn of(console: Option<&ViewerConsole>) -> Self {
        Self {
            ctf: console
                .and_then(|console| console.bool_cvar(LOOKUP))
                .unwrap_or(false),
            always: console
                .and_then(|console| console.bool_cvar(TEAM_COLORS_LOOKUP))
                .unwrap_or(true),
            siege: console
                .and_then(|console| console.bool_cvar(SIEGE_LOOKUP))
                .unwrap_or(true),
        }
    }
}

/// The team colours one player's blades take.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TeamBlade {
    /// Primary and secondary saber; `None` keeps the player's own colour.
    pub(crate) colors: [Option<[u8; 3]>; 2],
    /// `cg_ctfTeamSabers`: drawn without the player's blade skin.
    pub(crate) plain: bool,
}

impl TeamBlade {
    /// Whether any blade takes a team colour.
    pub(crate) fn any(self) -> bool {
        self.colors.iter().any(Option::is_some)
    }

    /// The colour saber `hand` (0 primary, 1 secondary) is drawn in, `own` when the team
    /// leaves it alone.
    pub(crate) fn color(self, hand: usize, own: [u8; 3]) -> [u8; 3] {
        self.colors.get(hand).copied().flatten().unwrap_or(own)
    }
}

/// One gamestate's team rules under the settings, read once a frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rules {
    /// `cg_ctfTeamSabers` is on and the game is CTF or CTY.
    ctf: bool,
    /// `cg_siegeTeamSabers` is on and the game is Siege.
    siege: bool,
    always: bool,
    policy: sjk_client::TeamColorPolicy,
}

impl Rules {
    /// The rules of `game` under `settings`.
    pub(crate) fn new(settings: Settings, game: &GameState) -> Self {
        Self {
            ctf: settings.ctf && sjk_client::legacy_team_sabers(game),
            siege: settings.siege && sjk_client::legacy_siege(game),
            always: settings.always,
            policy: sjk_client::TeamColorPolicy::from_game_state(game),
        }
    }

    /// The team blade of client slot `client`.
    pub(crate) fn of_client(self, game: &GameState, client: u16) -> TeamBlade {
        if self.ctf
            && let Some(rgb) = sjk_client::legacy_team_blade(game, client)
        {
            return TeamBlade {
                colors: [Some(rgb); 2],
                plain: true,
            };
        }
        if self.siege {
            return TeamBlade {
                colors: [sjk_client::legacy_siege_team_blade(game, client); 2],
                plain: false,
            };
        }
        TeamBlade {
            colors: sjk_client::legacy_saber_team_colors(game, self.policy, client, self.always),
            plain: false,
        }
    }
}

/// Each client slot's team blade this frame; all default outside team games.
pub(crate) type Blades = [TeamBlade; 32];

/// The team blades of every client slot, read once a frame.
pub(crate) fn blades(console: Option<&ViewerConsole>, game: Option<&GameState>) -> Blades {
    let mut blades = [TeamBlade::default(); 32];
    let Some(game) = game else {
        return blades;
    };
    let rules = Rules::new(Settings::of(console), game);
    for (client, blade) in (0u16..).zip(blades.iter_mut()) {
        *blade = rules.of_client(game, client);
    }
    blades
}

/// The team blade of the actor or corpse whose 1-based entity number is `entity`.
pub(crate) fn of(blades: &Blades, entity: u64) -> TeamBlade {
    entity
        .checked_sub(1)
        .and_then(|slot| usize::try_from(slot).ok())
        .and_then(|slot| blades.get(slot).copied())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 3] = [255, 51, 51];
    const BLUE: [u8; 3] = [51, 102, 255];

    fn game(gametype: &[u8]) -> GameState {
        let mut game = GameState::empty_local(0);
        game.replace_config_string(0, gametype.to_vec()).unwrap();
        game.replace_config_string(1131 + 3, br"\n\Red\t\1\c1\2\c2\4".to_vec())
            .unwrap();
        game
    }

    #[test]
    fn team_colours_are_on_by_default_in_team_games() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let tffa = game(br"\g_gametype\6");
        let on = blades(Some(&console), Some(&tffa));
        assert_eq!(on[3].colors, [Some(RED); 2]);
        assert!(!on[3].plain, "the blade skin stays worn");
        assert_eq!(of(&on, 4), on[3]);
        assert_eq!(of(&on, 0), TeamBlade::default());
        // At 0 only the other team's blue goes.
        console.set_cvar(TEAM_COLORS_CVAR, "0");
        let partial = blades(Some(&console), Some(&tffa));
        assert_eq!(partial[3].colors, [None, Some(RED)]);
        assert_eq!(partial[3].color(0, [1, 2, 3]), [1, 2, 3]);
        assert_eq!(partial[3].color(1, BLUE), RED);
        // Free for all keeps every choice.
        assert_eq!(
            blades(Some(&console), Some(&game(br"\g_gametype\0"))),
            [TeamBlade::default(); 32]
        );
    }

    #[test]
    fn ctf_team_sabers_go_plain_in_ctf_only() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let ctf = game(br"\g_gametype\8");
        assert!(!blades(Some(&console), Some(&ctf))[3].plain);
        console.set_cvar(CVAR, "1");
        let on = blades(Some(&console), Some(&ctf));
        assert_eq!(
            on[3],
            TeamBlade {
                colors: [Some(RED); 2],
                plain: true
            }
        );
        // Team FFA is not CTF: the skin stays.
        assert!(!blades(Some(&console), Some(&game(br"\g_gametype\6")))[3].plain);
    }

    #[test]
    fn siege_sides_take_their_colour_unless_turned_off() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut siege = game(br"\g_gametype\7");
        siege
            .replace_config_string(1131 + 5, br"\n\Blue\t\2\c1\0\c2\3".to_vec())
            .unwrap();
        siege
            .replace_config_string(1131 + 6, br"\n\Watcher\t\3\c1\2".to_vec())
            .unwrap();
        let on = blades(Some(&console), Some(&siege));
        assert_eq!(
            on[3],
            TeamBlade {
                colors: [Some(RED); 2],
                plain: false
            }
        );
        assert_eq!(on[5].colors, [Some(BLUE); 2]);
        assert_eq!(on[6], TeamBlade::default(), "spectators keep their own");
        // Off, Siege keeps every choice, as in EJK.
        console.set_cvar(SIEGE_CVAR, "0");
        assert_eq!(
            blades(Some(&console), Some(&siege)),
            [TeamBlade::default(); 32]
        );
    }
}
