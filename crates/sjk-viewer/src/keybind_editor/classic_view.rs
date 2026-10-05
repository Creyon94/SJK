//! The key bindings as a classic option panel (`controls.menu` bind items):
//! the action's label, then its keys as retail's `BindingFromName` writes
//! them ("A or B", one key, or "???"); the action being rebound turns red
//! and the description line asks for the new key.
//!
//! The panel is classic+ (`docs/classic-plus.md`): the detail box under the
//! rows names the focused action's keys, its console command and its default
//! key, an action bound differently from its default carries a mark, and
//! the description line names the keys of the panel.

use super::*;
use crate::menu::classic::layout::Span;
use crate::menu::classic::panel::{BINDING, Detail, OPTION, PanelFrame, focus_text};
use crate::menu::classic::view::Caps;
use std::fmt::Write as _;

/// The rows' wheel target, under them, while the list scrolls; clear of the
/// action rows, the secondary slots (600), the chrome (800) and the footer
/// and scrollbar tokens (902-910).
const ROWS_SCROLL_TOKEN: u16 = 911;

/// Retail `WAITING_FOR_NEW_KEY` (`mp_ingame.str`).
const WAITING: &str = "Enter new key, or ESC to cancel, BACKSPACE to clear.";
/// The description line's keys otherwise.
const KEYS: &str = "ENTER to bind a key   \u{b7}   BACKSPACE clears the action's keys";

/// Whether action `action`'s keys (`keys`, as the panel shows them) differ
/// from its default key.
fn rebound(action: usize, keys: &[String; 2]) -> bool {
    let default = ACTIONS[action].default_key;
    let default = (!default.is_empty()).then(|| sjk_shell::key_names::display_key(default));
    let bound = keys
        .iter()
        .filter(|key| *key != "UNBOUND" && *key != "-")
        .collect::<Vec<_>>();
    match default {
        None => !bound.is_empty(),
        Some(default) => bound.len() != 1 || !bound[0].eq_ignore_ascii_case(&default),
    }
}

impl KeybindEditor {
    /// Show rows `span` of key-binding category `category` as a classic
    /// option panel.
    pub(crate) fn open_classic(&mut self, console: &ViewerConsole, category: usize, span: Span) {
        let tab = category.min(CATEGORIES.len() - 1);
        self.set_tab(tab);
        let all = category_range(tab);
        let rows = span.within(all.start, all.len());
        self.selected = rows.start;
        self.classic = Some(rows);
        self.binding_slot = 0;
        self.refresh(console);
    }

    /// Back to the full tabbed editor on the same category.
    pub(crate) fn leave_classic(&mut self) {
        if self.classic.take().is_some() {
            self.set_tab(self.tab);
        }
    }

    /// Draw the classic panel screen around the open span's bindings.
    pub(crate) fn append_classic(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
        reveal: f32,
        frame: &PanelFrame,
    ) {
        let rows = self.rows();
        let place = frame.begin(&mut self.ui, viewport, reveal);
        self.visible = place.capacity();
        self.first = self.first.min(rows.len().saturating_sub(self.visible));
        let shown = rows.start + self.first..rows.end.min(rows.start + self.first + self.visible);
        // A list longer than the panel scrolls with the wheel anywhere over
        // its rows, not only over the thin bar.
        if rows.len() > self.visible {
            let top = place.row(0);
            self.ui.scroll_region(
                ROWS_SCROLL_TOKEN,
                Rect::new(top.x, top.y, top.width, self.visible as f32 * top.height),
            );
        }
        if shown.contains(&self.selected) {
            place.highlight(&mut self.ui, self.selected - shown.start);
        }
        for (slot, action) in shown.clone().enumerate() {
            let focused = action == self.selected;
            let color = if focused { focus_text() } else { OPTION };
            place.label(&mut self.ui, slot, ACTIONS[action].label, color);
            if self
                .keys
                .get(action)
                .is_some_and(|keys| rebound(action, keys))
            {
                place.changed_mark(&mut self.ui, slot);
            }
            self.ui.hit_region(action as u16, place.row(slot));
            let value_color = if focused && self.capture {
                BINDING
            } else {
                color
            };
            match self.keys.get(action) {
                Some([first, _]) if first == "UNBOUND" => {
                    place.value(&mut self.ui, slot, "???", value_color);
                }
                Some([first, second]) if second == "-" => {
                    place.value(&mut self.ui, slot, first, value_color);
                }
                // Retail writes "A or B" and raises the whole line to capitals.
                Some([first, second]) => place.value_fmt(
                    &mut self.ui,
                    slot,
                    format_args!("{} OR {}", Caps(first), Caps(second)),
                    value_color,
                ),
                None => place.value(&mut self.ui, slot, "???", value_color),
            }
        }
        if rows.len() > self.visible {
            self.ui.scrollbar(
                SCROLLBAR_TOKEN,
                place.scrollbar_track(self.visible),
                self.first,
                self.visible,
                rows.len(),
            );
        }
        let focused = rows.contains(&self.selected).then_some(self.selected);
        self.write_detail(focused);
        let detail = match focused {
            Some(action) => Detail {
                title: ACTIONS[action].label,
                value: &self.detail[0],
                lines: [&self.detail[1], &self.detail[2]],
                facts: &self.detail[3],
                name: "",
            },
            None => Detail::default(),
        };
        place.detail(&mut self.ui, &detail);
        place.finish(
            &mut self.ui,
            Some(if self.capture { WAITING } else { KEYS }),
        );
        self.ui.finish(self.selected as u16);
        self.ui.append_text(vertices, font, viewport);
    }

    /// The detail box's lines for action `action`: its keys, its command, the
    /// other actions its keys also do, and its default key.
    fn write_detail(&mut self, action: Option<usize>) {
        for line in &mut self.detail {
            line.clear();
        }
        let Some(action) = action else {
            return;
        };
        let [value, command, shared, facts] = &mut self.detail;
        match self.keys.get(action) {
            Some([first, second]) if first != "UNBOUND" && second != "-" => {
                let _ = write!(value, "{first} or {second}");
            }
            Some([first, _]) if first != "UNBOUND" => value.push_str(first),
            _ => value.push_str("not bound"),
        }
        let _ = write!(command, "Console command: {}", ACTIONS[action].command);
        let bound = self.keys.get(action).into_iter().flatten();
        for key in bound.filter(|key| *key != "UNBOUND" && *key != "-") {
            let others = self.keys.iter().enumerate().filter(|(other, keys)| {
                *other != action
                    && keys
                        .iter()
                        .any(|other_key| other_key.eq_ignore_ascii_case(key))
            });
            for (count, (other, _)) in others.enumerate() {
                let label = ACTIONS[other].label;
                let _ = match (count, shared.is_empty()) {
                    (0, true) => write!(shared, "{key} also does: {label}"),
                    (0, false) => write!(shared, "; {key} also does: {label}"),
                    _ => write!(shared, ", {label}"),
                };
            }
        }
        let default = ACTIONS[action].default_key;
        if default.is_empty() {
            facts.push_str("No default key");
        } else {
            let _ = write!(
                facts,
                "Default {}",
                Caps(&sjk_shell::key_names::display_key(default))
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(first: &str, second: &str) -> [String; 2] {
        [first.to_owned(), second.to_owned()]
    }

    fn action(command: &str) -> usize {
        ACTIONS
            .iter()
            .position(|action| action.command == command)
            .unwrap()
    }

    #[test]
    fn actions_on_their_default_key_are_not_marked() {
        let forward = action("+forward");
        let shown = sjk_shell::key_names::display_key(ACTIONS[forward].default_key).into_owned();
        assert!(!rebound(forward, &keys(&shown, "-")));
        assert!(rebound(forward, &keys(&shown, "UPARROW")));
        assert!(rebound(forward, &keys("UNBOUND", "-")));
        let unbound_by_default = action("invnext");
        assert!(!rebound(unbound_by_default, &keys("UNBOUND", "-")));
        assert!(rebound(unbound_by_default, &keys("]", "-")));
    }

    #[test]
    fn the_detail_names_keys_command_and_default() {
        let (_directory, console) = {
            let directory = tempfile::tempdir().unwrap();
            let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
            (directory, console)
        };
        let mut editor = KeybindEditor::new();
        editor.open_classic(&console, 0, Span::ALL);
        let forward = action("+forward");
        editor.write_detail(Some(forward));
        assert_eq!(editor.detail[1], "Console command: +forward");
        assert!(
            editor.detail[3].starts_with("Default "),
            "{}",
            editor.detail[3]
        );
        assert!(!editor.detail[0].is_empty());
        // A key two actions share is named on both.
        let jump = action("+moveup");
        let shown = editor.keys[forward][0].clone();
        editor.keys[jump] = [shown.clone(), "-".to_owned()];
        editor.write_detail(Some(forward));
        assert_eq!(editor.detail[2], format!("{shown} also does: Jump"));
        editor.write_detail(None);
        assert!(editor.detail.iter().all(String::is_empty));
    }
}
