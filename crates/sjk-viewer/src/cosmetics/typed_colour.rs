//! A blade colour typed on its own keeps the worn hat or cape.
//!
//! JoF EJK carries the hat after the digits of `color1` and the cape after those of
//! `color2` (`4santahat`), so `color1 2` typed in the console, a bind or a script
//! replaced the whole value and took the hat off (a player's bug report, 14/10/2026).
//! JoF's own colour commands keep the piece (`CG_SetSaberColorCvar`,
//! `cg_consolecmds.c:1136-1152`); a typed number now does too. A value with a name
//! after its digits (`2tophat`, `4none`) is set as it is, so a piece can still be
//! changed or taken off that way, and the menus and the `cosmetics` command set the
//! cvars directly, outside this path.

use sjk_client::CosmeticSlot;

/// The cvar and value to set instead when `tokens` (`color1 2`, `set color1 2`,
/// `seta color2 3`) set a colour with digits only while `worn` holds a piece for that
/// cvar, `worn` being each slot's current piece; `None` leaves the command alone.
pub(crate) fn keep_worn(
    tokens: &[String],
    worn: &[Option<String>; 2],
) -> Option<(&'static str, String)> {
    let (name, value) = match tokens {
        [name, value] => (name, value),
        [set, name, value]
            if ["set", "seta", "setu"]
                .iter()
                .any(|command| set.eq_ignore_ascii_case(command)) =>
        {
            (name, value)
        }
        _ => return None,
    };
    let slot = CosmeticSlot::ALL
        .into_iter()
        .find(|slot| name.eq_ignore_ascii_case(slot.cvar()))?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let piece = worn[slot.index()].as_deref()?;
    let colour = sjk_client::split_color_value(value).0;
    Some((
        slot.cvar(),
        sjk_client::join_color_value(colour, Some(piece)),
    ))
}

/// The piece each slot wears now, read from `console`'s `color1` and `color2`.
pub(crate) fn worn(console: &crate::console::ViewerConsole) -> [Option<String>; 2] {
    CosmeticSlot::ALL.map(|slot| {
        console
            .text_value(slot.cvar())
            .and_then(|value| sjk_client::split_color_value(value).1)
            .map(str::to_owned)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_typed_number_keeps_the_piece_and_a_named_value_does_not() {
        let worn = [Some("santahat".to_owned()), Some("royalcape".to_owned())];
        assert_eq!(
            keep_worn(&tokens("color1 2"), &worn),
            Some(("color1", "2santahat".to_owned()))
        );
        assert_eq!(
            keep_worn(&tokens("SETA Color2 6"), &worn),
            Some(("color2", "6royalcape".to_owned()))
        );
        // Named values, other cvars, a bare query and nothing worn: left alone.
        assert_eq!(keep_worn(&tokens("color1 2tophat"), &worn), None);
        assert_eq!(keep_worn(&tokens("color1 4none"), &worn), None);
        assert_eq!(keep_worn(&tokens("cg_fov 100"), &worn), None);
        assert_eq!(keep_worn(&tokens("color1"), &worn), None);
        assert_eq!(keep_worn(&tokens("color1 2"), &[None, None]), None);
    }

    #[test]
    fn typing_a_colour_in_the_console_keeps_the_hat() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar("color1", "4santahat");
        console.set_cvar("color2", "4royalcape");
        console.execute_console_line("color1 2", None);
        assert_eq!(console.text_value("color1"), Some("2santahat"));
        console.execute_console_line("seta color2 6", None);
        assert_eq!(console.text_value("color2"), Some("6royalcape"));
        // Naming another piece, or none, still changes it.
        console.execute_console_line("color1 3none", None);
        assert_eq!(console.text_value("color1"), Some("3none"));
    }
}
