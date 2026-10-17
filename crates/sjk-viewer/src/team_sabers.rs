//! CTF team saber colours (`cg_ctfTeamSabers`, off by default; SJK pull request #113 by
//! Slash, made an option at the merge, Sol's choice 17/10/2026): in capture the flag and
//! capture the ysalamiri every red or blue player's blades are drawn in the team's stock
//! colour, whatever `c1`..`c4` and their blade shader say (`sjk_client::legacy_team_blade`).

use crate::console::ViewerConsole;
use sjk_protocol::GameState;

/// The archived player setting.
pub(crate) const CVAR: &str = "cg_ctfTeamSabers";
/// Lower case, for the per-frame lookup without an allocation.
const LOOKUP: &str = "cg_ctfteamsabers";

/// Each client slot's team blade this frame; all `None` while the setting is off or the
/// game is not CTF or CTY.
pub(crate) type Blades = [Option<[u8; 3]>; 32];

/// Whether the setting is on.
pub(crate) fn enabled(console: Option<&ViewerConsole>) -> bool {
    console
        .and_then(|console| console.bool_cvar(LOOKUP))
        .unwrap_or(false)
}

/// The team blades of every client slot, read once a frame.
pub(crate) fn blades(console: Option<&ViewerConsole>, game: Option<&GameState>) -> Blades {
    let mut blades = [None; 32];
    if let Some(game) = game.filter(|game| enabled(console) && sjk_client::legacy_team_sabers(game))
    {
        for (client, blade) in (0u16..).zip(blades.iter_mut()) {
            *blade = sjk_client::legacy_team_blade(game, client);
        }
    }
    blades
}

/// The team blade of the actor or corpse whose 1-based entity number is `entity`.
pub(crate) fn of(blades: &Blades, entity: u64) -> Option<[u8; 3]> {
    let slot = usize::try_from(entity.checked_sub(1)?).ok()?;
    *blades.get(slot)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_by_default_and_outside_ctf() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut game = GameState::empty_local(0);
        game.replace_config_string(0, br"\g_gametype\8".to_vec())
            .unwrap();
        game.replace_config_string(1131 + 3, br"\n\Red\t\1\c1\2".to_vec())
            .unwrap();
        assert_eq!(blades(Some(&console), Some(&game)), [None; 32]);
        console.set_cvar(CVAR, "1");
        let on = blades(Some(&console), Some(&game));
        assert!(on[3].is_some());
        assert_eq!(of(&on, 4), on[3]);
        assert_eq!(of(&on, 0), None);
        game.replace_config_string(0, br"\g_gametype\0".to_vec())
            .unwrap();
        assert_eq!(blades(Some(&console), Some(&game)), [None; 32]);
    }
}
