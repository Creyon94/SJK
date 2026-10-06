//! Console side of the changelog page (see `changelog.rs`): opening it, and routing
//! keys and pointer events to it while it is open, as for the `debug_panel` list.

use super::changelog::PanelAction;
use super::*;
use sjk_ui::InputEvent;

impl ViewerConsole {
    /// `changelog`: show the page in front of the console, opening the console if
    /// needed, or close it (and the console, if the page opened it).
    pub(crate) fn toggle_changelog(&mut self) {
        if self.open && self.changelog.is_open() {
            self.close_changelog();
            return;
        }
        self.open_changelog();
    }

    /// Show the page (the main menu's Changelog entry).
    pub(crate) fn open_changelog(&mut self) {
        let owns_console = !self.open;
        if !self.open {
            self.set_open(true);
        }
        self.browser.close();
        self.debug_panel.close();
        self.update_panel.close();
        self.credits.close();
        self.identity_panel.close();
        self.dead_key.settle();
        self.changelog.open(owns_console);
    }

    /// The page, for snapshots.
    #[cfg(test)]
    pub(crate) fn changelog_mut(&mut self) -> &mut super::changelog::Panel {
        &mut self.changelog
    }

    fn close_changelog(&mut self) {
        if self.changelog.close() {
            self.set_open(false);
        }
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn changelog_key(&mut self, event: &KeyEvent) -> bool {
        if !self.changelog.is_open() {
            return false;
        }
        if self.changelog.handle_key(event) == PanelAction::Close {
            self.close_changelog();
        }
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn changelog_pointer(&mut self, event: InputEvent) -> bool {
        if !self.changelog.is_open() {
            return false;
        }
        if self.changelog.handle_pointer(event) == PanelAction::Close {
            self.close_changelog();
        }
        true
    }

    /// Draw the page in place of the console; false when it is not shown.
    pub(super) fn append_changelog(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) -> bool {
        if !(self.open && self.changelog.is_open()) {
            return false;
        }
        self.changelog.append(vertices, font, viewport);
        true
    }

    /// The page's draw list while it is shown.
    pub(super) fn changelog_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.changelog.is_open()).then(|| self.changelog.draw_list())
    }
}
