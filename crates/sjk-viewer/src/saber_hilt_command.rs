//! JoF EJK's `saber` console command (`CG_Saber_f`, `cg_consolecmds.c:1051-1085`):
//! choose the sabers by name, as Profile > Saber does.
//!
//! `saber <saber1>` holds one saber (a single or a staff, `saber2` becomes `none`),
//! `saber <saber1> <saber2>` two; with no name it prints the sabers held. It writes the
//! `saber1`/`saber2` cvars, the one place the menu reads and the userinfo sends, so a
//! choice made from the console or a quick-wheel custom command shows in Profile > Saber
//! and stays on the next map. Sent to the server as is, the command swapped the saber
//! on JA+ servers but left the cvars on the menu's choice, which the next map's
//! userinfo brought back (Kushina's report #126). JA+ and jaPRO-lineage servers still
//! get the server's `saber` command, from [`sjk_client::SaberChangeNotifier`], which
//! sends it for any change of the two cvars.

use crate::console::ViewerConsole;

/// Completion and help entry.
pub(crate) const COMMANDS: &[(&str, &str)] = &[(
    "saber",
    "Choose your sabers: saber <saber1> (one saber or a staff), saber <saber1> <saber2> (two)",
)];

/// Lowercase name the client's command dispatch matches.
pub(crate) const COMMAND: &str = "saber";

/// Run `saber` with `args` (the words after the command).
pub(crate) fn run(console: &mut ViewerConsole, args: &[String]) -> Vec<String> {
    let (saber1, saber2) = match args {
        [saber1] => (saber1.as_str(), "none"),
        [saber1, saber2] => (saber1.as_str(), saber2.as_str()),
        _ => {
            let saber1 = console.text_value("saber1").unwrap_or_default();
            let saber2 = console.text_value("saber2").unwrap_or_default();
            let held = if saber2.is_empty() || saber2.eq_ignore_ascii_case("none") {
                saber1.to_owned()
            } else {
                format!("{saber1} - {saber2}")
            };
            return vec![format!("Sabers: {held}")];
        }
    };
    console.set_cvar("saber1", saber1);
    console.set_cvar("saber2", saber2);
    Vec::new()
}

impl crate::GpuState {
    /// `saber ...`.
    pub(crate) fn saber_hilt_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let console = self.console.as_mut().ok_or("Console unavailable")?;
        Ok(run(console, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_command_sets_the_cvars_the_menu_and_userinfo_read() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        // Two sabers, then a staff: the second hand empties.
        run(&mut console, &args("single_2 single_3"));
        assert_eq!(console.text_value("saber1"), Some("single_2"));
        assert_eq!(console.text_value("saber2"), Some("single_3"));
        run(&mut console, &args("dual_1"));
        assert_eq!(console.text_value("saber1"), Some("dual_1"));
        assert_eq!(console.text_value("saber2"), Some("none"));
        let userinfo = console.userinfo().unwrap();
        assert_eq!(userinfo.saber1, "dual_1");
        assert_eq!(userinfo.saber2, "none");
        // No name prints the sabers held and changes nothing.
        assert_eq!(run(&mut console, &[]), ["Sabers: dual_1"]);
        run(&mut console, &args("single_1 single_4"));
        assert_eq!(run(&mut console, &[]), ["Sabers: single_1 - single_4"]);
        assert_eq!(console.text_value("saber1"), Some("single_1"));
    }
}
