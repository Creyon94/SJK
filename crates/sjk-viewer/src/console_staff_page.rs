//! Console side of the Staff page (see `staff_panel.rs`): opening it for a staff
//! key, sending what it asks to the hub, and routing keys and pointer events to it,
//! as for the Profile page. Clearing the player's own achievements also resets this
//! client's counts for them (`achievements::forget`), so they can be unlocked again.

use super::staff_panel::{Inputs, PanelAction};
use super::*;
use sjk_identity::StaffRequest;
use sjk_ui::InputEvent;

/// The `staff` command.
pub(crate) const STAFF_COMMAND: &str = "staff";
pub(crate) const STAFF_HELP: &str = "Open the SJK staff tools (staff keys only)";

impl ViewerConsole {
    /// `staff`: show the page, or close it; a key that is not staff is told so.
    pub(crate) fn toggle_staff_panel(&mut self) {
        if self.open && self.staff_panel.is_open() {
            self.close_staff_panel();
            return;
        }
        self.open_staff_panel();
    }

    /// Show the page (the Profile page's Staff tools), and list the players seen lately.
    pub(crate) fn open_staff_panel(&mut self) {
        if !crate::player_identity::is_staff() {
            self.push_log("^3staff: only SJK staff keys can open the staff tools");
            return;
        }
        let owns_console = !self.open;
        if !self.open {
            self.set_open(true);
        }
        self.browser.close();
        self.debug_panel.close();
        self.changelog.close();
        self.update_panel.close();
        self.config_import.close();
        self.credits.close();
        self.identity_panel.close();
        self.profile_panel.close();
        self.dead_key.settle();
        self.staff_panel.open(owns_console);
        crate::player_identity::staff(StaffRequest::Search(String::new()));
    }

    /// Closing the page closes the console too (it was opened from the Profile page,
    /// which had opened it).
    pub(super) fn staff_panel_owns_console(&mut self) {
        self.staff_panel.own_console();
    }

    fn close_staff_panel(&mut self) {
        if self.staff_panel.close() {
            self.set_open(false);
        }
    }

    fn staff_panel_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_staff_panel(),
            PanelAction::Request(StaffRequest::ClearAchievements { key_id, id })
                if Some(&key_id) == crate::player_identity::own_key_id().as_ref() =>
            {
                // The player's own: the counts here go too, and every achievement on the
                // same counter with a higher goal, which the lower count no longer
                // reaches; else this client would send them straight back.
                if id.is_empty() {
                    crate::achievements::forget(None);
                    crate::player_identity::staff(StaffRequest::ClearAchievements { key_id, id });
                    return;
                }
                crate::achievements::forget(Some(&id));
                for cleared in crate::achievements::cleared_with(&id) {
                    crate::player_identity::staff(StaffRequest::ClearAchievements {
                        key_id: key_id.clone(),
                        id: cleared.to_owned(),
                    });
                }
            }
            PanelAction::Request(request) => {
                if !crate::player_identity::staff(request) {
                    self.push_log("^3staff: the identity is not running");
                }
            }
        }
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn staff_panel_key(&mut self, event: &KeyEvent) -> bool {
        if !self.staff_panel.is_open() {
            return false;
        }
        let action = self.staff_panel.handle_key(event, self.shift);
        self.staff_panel_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn staff_panel_pointer(&mut self, event: InputEvent) -> bool {
        if !self.staff_panel.is_open() {
            return false;
        }
        let action = self.staff_panel.handle_pointer(event);
        self.staff_panel_action(action);
        true
    }

    /// Draw the page with its text to `target`.
    pub(super) fn append_staff_panel(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        #[cfg(test)]
        if let Some((me, staff)) = self.staff_panel.preview.take() {
            let inputs = Inputs {
                me: Some(&me),
                staff: &staff,
            };
            self.staff_panel.append_sjk(&inputs, target, viewport);
            self.staff_panel.preview = Some((me, staff));
            return;
        }
        let snapshot = crate::player_identity::snapshot();
        let staff = crate::player_identity::staff_state().unwrap_or_default();
        let inputs = Inputs {
            me: snapshot.as_ref().and_then(|snapshot| snapshot.me.as_ref()),
            staff: &staff,
        };
        self.staff_panel.append_sjk(&inputs, target, viewport);
    }

    /// Open the page on `me` and `staff` in place of the live ones, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_staff(
        &mut self,
        me: sjk_identity::Profile,
        staff: sjk_identity::StaffState,
    ) {
        self.staff_panel.open(true);
        self.staff_panel.preview = Some((me, staff));
        self.set_open(true);
    }

    /// The page's draw list while it is shown.
    pub(super) fn staff_panel_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.staff_panel.is_open()).then(|| self.staff_panel.draw_list())
    }
}
