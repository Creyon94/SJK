//! Pointer-only console scrolling; command behavior remains keyboard driven. The
//! command and cvar browser takes every pointer event while it is open.

use super::ViewerConsole;
use sjk_ui::InputEvent;

impl ViewerConsole {
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) {
        if self.credits_pointer(event)
            || self.changelog_pointer(event)
            || self.update_panel_pointer(event)
            || self.debug_panel_pointer(event)
        {
            return;
        }
        if self.browser.is_open() {
            let action = self.browser.handle_pointer(event);
            self.browser_action(action);
            return;
        }
        if self.console_style() == super::console_options::ConsoleStyle::Classic {
            // `Console_Key`: the wheel pages like Page Up/Down (Ctrl five times
            // as far), and with Shift walks the history.
            if let InputEvent::PointerWheel { delta, .. } = event {
                match (self.shift, delta.y > 0.0, delta.y < 0.0) {
                    (true, true, _) => self.navigate_history(-1),
                    (true, _, true) => self.navigate_history(1),
                    (false, true, _) => {
                        self.scroll_rows(super::classic::page_rows(self.control) as isize);
                    }
                    (false, _, true) => {
                        self.scroll_rows(-(super::classic::page_rows(self.control) as isize));
                    }
                    _ => {}
                }
            } else {
                self.selection.pointer(event, self.shift);
            }
            return;
        }
        let Some(delta) = self.presentation.pointer(event) else {
            self.selection.pointer(event, self.shift);
            return;
        };
        let line_count = self
            .shell
            .lines()
            .map(|line| line.text.split('\n').count())
            .sum();
        if delta > 0.0 {
            self.scroll_offset = self.scroll_offset.saturating_add(3).min(line_count);
        } else if delta < 0.0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(3);
        }
    }
}
