//! Typing, pasting and copying in a one-line field of the menus: the server address and
//! password prompts, the browser's search. The caret is at the end of the text.
//!
//! Every text field takes Ctrl+V (and Shift+Insert) to paste, Ctrl+C (and Ctrl+Insert) to
//! copy and a held Backspace to delete on, as the console and the SJK chat do (Sol's rule,
//! 12/10/2026, `AGENTS.md`). The menu lets key repeats through for a field that is open
//! ([`crate::menu::ClientMenu::handle_key`]); this module is what such a field calls.

use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, SmolStr};

/// Ctrl+C on Windows types this character.
const CONTROL_C: &str = "\u{3}";

/// Whether a pressed `key` that typed `text` copies: Ctrl+C, or Ctrl+Insert. Like
/// [`crate::sjk_chat_field::is_paste`], Windows may hand the letter without Ctrl, or the
/// Ctrl character itself.
pub(crate) fn is_copy(key: KeyCode, text: Option<&str>, control: bool) -> bool {
    text == Some(CONTROL_C)
        || (control && text.is_some_and(|text| text.eq_ignore_ascii_case("c")))
        || (control && key == KeyCode::KeyC && text.is_none())
        || (control && key == KeyCode::Insert)
}

/// The event a paste types in place of `event` (the Ctrl+V press): the clipboard's first
/// line, trimmed and without control characters, as the text of the key. Handlers that read
/// `event.text` take it like typing. `None` when the clipboard has no text to type.
pub(crate) fn pasted_event(event: &KeyEvent, clipboard: &str) -> Option<KeyEvent> {
    let line = clipboard.lines().next().unwrap_or_default().trim();
    let text: String = line.chars().filter(|c| !c.is_control()).collect();
    if text.is_empty() {
        return None;
    }
    let mut typed = event.clone();
    typed.repeat = false;
    typed.logical_key = Key::Character(SmolStr::new(&text));
    typed.text = Some(SmolStr::new(&text));
    Some(typed)
}

/// A pressed `key` that typed `text` into `field`, up to `max` characters: pasted from the
/// clipboard, copied to it (when `copyable`: a password is not), Backspace taking the last
/// character back, or typed (control characters left out). Whether `field` changed.
pub(crate) fn edit(
    field: &mut String,
    key: KeyCode,
    text: Option<&str>,
    shift: bool,
    control: bool,
    max: usize,
    copyable: bool,
) -> bool {
    if key == KeyCode::Backspace {
        return field.pop().is_some();
    }
    if crate::sjk_chat_field::is_paste(key, text, shift, control) {
        return crate::console::clipboard::paste()
            .is_some_and(|clipboard| paste_into(field, &clipboard, max));
    }
    if is_copy(key, text, control) {
        if copyable && !field.is_empty() {
            let _ = crate::console::clipboard::copy(field);
        }
        return false;
    }
    text.is_some_and(|text| type_into(field, text, max))
}

/// Typed `text` added to `field`, control characters left out, up to `max` characters.
fn type_into(field: &mut String, text: &str, max: usize) -> bool {
    let before = field.len();
    let room = max.saturating_sub(field.chars().count());
    field.extend(text.chars().filter(|c| !c.is_control()).take(room));
    field.len() != before
}

/// Pasted `text` added to `field`: its first line, trimmed (an address copied with a line
/// break after it pastes clean), control characters left out, up to `max` characters.
fn paste_into(field: &mut String, text: &str, max: usize) -> bool {
    let line = text.lines().next().unwrap_or_default().trim();
    type_into(field, line, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(field: &str, text: &str, max: usize) -> String {
        let mut field = field.to_owned();
        type_into(&mut field, text, max);
        field
    }

    #[test]
    fn ctrl_c_and_ctrl_insert_copy() {
        assert!(is_copy(KeyCode::KeyC, Some("\u{3}"), true));
        assert!(is_copy(KeyCode::KeyC, Some("c"), true));
        assert!(is_copy(KeyCode::KeyC, None, true));
        assert!(is_copy(KeyCode::Insert, None, true));
        assert!(!is_copy(KeyCode::KeyC, Some("c"), false));
        assert!(!is_copy(KeyCode::KeyC, Some("k"), true));
    }

    #[test]
    fn typing_stops_at_the_limit_and_drops_control_characters() {
        assert_eq!(typed("ab", "c\td", 4), "abcd");
        assert_eq!(typed("abcd", "e", 4), "abcd");
        assert_eq!(typed("", "\u{16}", 4), "");
    }

    #[test]
    fn a_paste_is_its_first_line_trimmed() {
        let mut field = "192.0.2.1".to_owned();
        assert!(paste_into(&mut field, "  :29070 \r\nsecond line", 255));
        assert_eq!(field, "192.0.2.1:29070");
        assert!(!paste_into(&mut field, "\n", 255));
    }

    #[test]
    fn backspace_takes_the_last_character_back() {
        let mut field = "ab".to_owned();
        assert!(edit(
            &mut field,
            KeyCode::Backspace,
            None,
            false,
            false,
            9,
            true
        ));
        assert_eq!(field, "a");
        assert!(edit(
            &mut field,
            KeyCode::Backspace,
            None,
            false,
            false,
            9,
            true
        ));
        assert!(!edit(
            &mut field,
            KeyCode::Backspace,
            None,
            false,
            false,
            9,
            true
        ));
    }
}
