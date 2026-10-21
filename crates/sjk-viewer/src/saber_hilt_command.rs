//! The `saber` console command (`docs/client.md`, "Useful console commands"): the hilts the
//! player holds, as JoF EJK players type them. `saber kyle kyle` holds two sabers (`saber1`
//! and `saber2`), `saber dual_4` one (a staff's hilt makes it a staff; `saber2` goes to
//! `none`), and `saber` alone says what is held. The Profile's Saber tab and the
//! Collection's stage follow the cvars, so the preview shows the new sabers at once.

use crate::console::ViewerConsole;

/// Console command name.
pub(crate) const COMMAND: &str = "saber";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Hold one saber, a staff or two sabers: saber <hilt> [<second hilt>] (saber kyle kyle, saber dual_4)";

const USAGE: &str = "saber <hilt> [<second hilt> | none]";

/// A hilt's name as the cvars keep it: its own spelling, or `None` when it is not a name.
fn hilt(word: &str) -> Option<&str> {
    let word = word.trim();
    (!word.is_empty()
        && word
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')))
    .then_some(word)
}

/// Run `saber` with `args` on `console`.
pub(crate) fn run(console: &mut ViewerConsole, args: &[String]) -> Result<Vec<String>, String> {
    let (first, second) = match args {
        [] => {
            let first = console
                .text_value("saber1")
                .unwrap_or("single_1")
                .to_owned();
            let second = console.text_value("saber2").unwrap_or("none").to_owned();
            return Ok(vec![if second.eq_ignore_ascii_case("none") {
                format!("You hold {first}. ({USAGE})")
            } else {
                format!("You hold {first} and {second}. ({USAGE})")
            }]);
        }
        [first] => (first.as_str(), "none"),
        [first, second] => (first.as_str(), second.as_str()),
        _ => return Err(format!("usage: {USAGE}")),
    };
    let first = hilt(first).ok_or_else(|| format!("usage: {USAGE}"))?;
    if first.eq_ignore_ascii_case("none") {
        return Err("The first saber cannot be none.".to_owned());
    }
    let second = hilt(second).ok_or_else(|| format!("usage: {USAGE}"))?;
    let second = if second.eq_ignore_ascii_case("none") {
        "none"
    } else {
        second
    };
    if !console.set_cvar("saber1", first) || !console.set_cvar("saber2", second) {
        return Err("saber1 and saber2 could not be set".to_owned());
    }
    Ok(vec![if second == "none" {
        format!("You hold {first}.")
    } else {
        format!("You hold {first} and {second}.")
    }])
}

impl crate::GpuState {
    /// `saber [<hilt> [<second hilt>]]`.
    pub(crate) fn saber_hilt_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let Some(console) = self.console.as_mut() else {
            return Err("The console is not ready".to_owned());
        };
        run(console, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_command_holds_two_sabers_a_staff_or_one() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            ViewerConsole::new(directory.path().join("config.cfg")).expect("a console");
        let lines = run(&mut console, &words("kyle kyle")).unwrap();
        assert_eq!(lines, ["You hold kyle and kyle."]);
        assert_eq!(console.text_value("saber1"), Some("kyle"));
        assert_eq!(console.text_value("saber2"), Some("kyle"));
        run(&mut console, &words("dual_4")).unwrap();
        assert_eq!(console.text_value("saber1"), Some("dual_4"));
        assert_eq!(console.text_value("saber2"), Some("none"));
        run(&mut console, &words("single_1 NONE")).unwrap();
        assert_eq!(console.text_value("saber2"), Some("none"));
        assert!(run(&mut console, &[]).unwrap()[0].contains("You hold single_1."));
        assert!(run(&mut console, &words("none")).is_err());
        assert!(run(&mut console, &words("a b c")).is_err());
        assert!(run(&mut console, &words("kyle;quit")).is_err());
        assert_eq!(console.text_value("saber1"), Some("single_1"));
    }
}
