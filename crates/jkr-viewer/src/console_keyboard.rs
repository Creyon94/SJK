//! Console keyboard capture and editing.
use super::line_edit::LineEdit;
use super::*;
use crate::input::dead_key::TypingField;
use std::ops::Range;

/// Does `text` (the characters this key press produced) appear in a
/// `cl_consoleKeys` list? Entries are literal characters or `0x` hex
/// codepoints (`cl_main.cpp:2826`).
pub(crate) fn is_console_key(list: &str, text: &str) -> bool {
    let mut characters = text.chars();
    let (Some(typed), None) = (characters.next(), characters.next()) else {
        return false;
    };
    list.split_whitespace().any(|entry| {
        match entry
            .strip_prefix("0x")
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .and_then(char::from_u32)
        {
            Some(code) => code == typed,
            None => {
                let mut entry = entry.chars();
                (entry.next(), entry.next()) == (Some(typed), None)
            }
        }
    })
}

impl ViewerConsole {
    /// Stock console keys: a `cl_consoleKeys` character, Shift+Escape
    /// (`cl_keys.cpp:1318`), or a key the user explicitly bound to
    /// `toggleconsole`. The physical key is not bound by default, so layouts
    /// where it types `^` keep that character for colour codes.
    pub(super) fn configured_console_key(&self, key: KeyCode, text: Option<&str>) -> bool {
        if key == KeyCode::Escape {
            return self.shift;
        }
        let native = self.integer_cvar("cl_consoleusescancode").unwrap_or(0) != 0;
        if native
            && key == KeyCode::Backquote
            && super::client_options::native_console(
                self.integer_cvar("cl_consoleshiftrequirement").unwrap_or(0),
                self.shift,
                self.open,
            )
        {
            return true;
        }
        let list = self
            .shell
            .cvars
            .get("cl_consoleKeys")
            .and_then(|cvar| match &cvar.value {
                CvarValue::Text(value) => Some(value.as_str()),
                _ => None,
            })
            .unwrap_or("");
        !native && text.is_some_and(|text| is_console_key(list, text))
    }

    fn toggles_console(&self, event: &KeyEvent, key: KeyCode) -> bool {
        if self.configured_console_key(key, event.text.as_deref()) {
            return true;
        }
        if key == KeyCode::Escape {
            return false;
        }
        let Some(key_name) = crate::input::keys::key_name(event) else {
            return false;
        };
        self.shell
            .binds
            .commands_for_event(key_name.as_str(), true)
            .is_ok_and(|commands| {
                commands
                    .iter()
                    .any(|command| command.eq_ignore_ascii_case("toggleconsole"))
            })
    }
    /// Consume console input before gameplay input gets a chance to observe it.
    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        session: Option<&mut ClientSession>,
    ) -> bool {
        let PhysicalKey::Code(key) = event.physical_key else {
            return self.open;
        };
        if !self.open {
            if event.state == ElementState::Pressed && self.toggles_console(event, key) {
                self.set_open(true);
                return true;
            }
            let Some(key_name) = crate::input::keys::key_name(event) else {
                return false;
            };
            let commands = match self
                .shell
                .binds
                .commands_for_event(key_name.as_str(), event.state == ElementState::Pressed)
            {
                Ok(commands) => commands,
                Err(error) => {
                    self.shell.push_log(format!("^1Bind error: {error}"));
                    return true;
                }
            };
            let mut consumed = false;
            for command in commands {
                if command.eq_ignore_ascii_case("toggleconsole") {
                    self.execute_bound_command(&command);
                    consumed = true;
                }
            }
            return consumed;
        }

        if event.state != ElementState::Pressed {
            return true;
        }
        if self.toggles_console(event, key) {
            self.set_open(false);
            return true;
        }
        if self.browser.is_open() {
            let action = self.browser.handle_key(event, self.shift);
            self.browser_action(action);
            return true;
        }
        // These keys end or replace the line, so a dead key shown at the caret stays
        // typed (see `input::dead_key`).
        if matches!(
            key,
            KeyCode::Escape
                | KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::Backspace
                | KeyCode::Tab
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown
        ) || event.text.as_deref() == Some("\u{16}")
        {
            self.dead_key.settle();
        }
        match key {
            KeyCode::F3 if !event.repeat => {
                // The browser takes the keys from here, so a shown dead key stays typed.
                self.dead_key.settle();
                self.browser.open(&self.shell);
            }
            KeyCode::Escape => self.set_open(false),
            KeyCode::Enter | KeyCode::NumpadEnter => self.submit(session),
            KeyCode::Tab => self.complete_command(CompletionKey::Tab),
            KeyCode::ArrowUp if !event.repeat => self.navigate_history(-1),
            KeyCode::ArrowDown if !event.repeat => self.navigate_history(1),
            // Caret, deletion and clipboard keys: see `console_editing.rs`. One that
            // brings text ends a pending dead key, which then stays typed; caret
            // motion leaves it pending, as the platform does.
            _ if self.edit_key(event, key) => self.dead_key.other_key(event.text.as_deref()),
            _ if !event.repeat => self.dead_key.type_key(
                &mut PromptLine {
                    text: &mut self.input,
                    edit: &mut self.edit,
                },
                &event.logical_key,
                event.text.as_deref(),
            ),
            _ => {}
        }
        true
    }
}

/// The console input line with its caret and selection, typed at the caret.
struct PromptLine<'a> {
    text: &'a mut String,
    edit: &'a mut LineEdit,
}

impl TypingField for PromptLine<'_> {
    fn line(&self) -> &str {
        self.text
    }

    fn caret(&self) -> usize {
        self.edit.cursor(self.text)
    }

    fn insert(&mut self, text: &str) {
        self.edit.insert(self.text, text, INPUT_LIMIT);
    }

    fn remove(&mut self, range: Range<usize>) {
        self.edit.remove(self.text, range);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::dead_key::DeadKey;
    use winit::keyboard::{Key, NamedKey, SmolStr};

    /// `input` with the caret at its end, as after typing it.
    fn at_end(input: &str) -> (String, LineEdit) {
        let mut edit = LineEdit::default();
        edit.to_end(input);
        (input.to_owned(), edit)
    }

    #[test]
    fn prompt_line_types_a_dead_key_colour_code_exactly() {
        let (mut input, mut edit) = at_end("say ");
        let mut dead = DeadKey::default();
        let mut line = PromptLine {
            text: &mut input,
            edit: &mut edit,
        };
        dead.type_key(&mut line, &Key::Dead(Some('^')), None);
        assert_eq!(line.line(), "say ^");
        dead.type_key(&mut line, &Key::Named(NamedKey::Shift), None);
        dead.type_key(&mut line, &Key::Character(SmolStr::new("1")), Some("^1"));
        assert_eq!(input, "say ^1");
        assert_eq!(edit.cursor(&input), input.len());
    }

    #[test]
    fn prompt_line_composes_at_a_caret_inside_the_line() {
        let (mut input, mut edit) = at_end("say hi");
        edit.place(&input, 4, false);
        let mut dead = DeadKey::default();
        let mut line = PromptLine {
            text: &mut input,
            edit: &mut edit,
        };
        dead.type_key(&mut line, &Key::Dead(Some('^')), None);
        assert_eq!(line.line(), "say ^hi");
        dead.type_key(&mut line, &Key::Character(SmolStr::new("1")), Some("^1"));
        assert_eq!(input, "say ^1hi");
        assert_eq!(edit.cursor(&input), 6);
    }

    #[test]
    fn prompt_line_keeps_its_limit_for_a_dead_key() {
        let (mut input, mut edit) = at_end(&"x".repeat(INPUT_LIMIT));
        let mut dead = DeadKey::default();
        let mut line = PromptLine {
            text: &mut input,
            edit: &mut edit,
        };
        dead.type_key(&mut line, &Key::Dead(Some('^')), None);
        assert_eq!(input.len(), INPUT_LIMIT);
        assert_eq!(dead, DeadKey::default());
    }
}
