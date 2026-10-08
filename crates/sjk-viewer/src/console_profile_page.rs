//! Console side of the Profile page (see `profile_panel.rs`): opening it on a tab,
//! saving the bio it asks to save, and routing keys and pointer events to it while it
//! is open, as for the Identity page. It is drawn in the SJK UI's look whatever the
//! menu style (`console_sjk_pages.rs`).

use super::profile_panel::{Inputs, PanelAction, Tab};
use super::*;
use sjk_ui::InputEvent;

/// The `profile` command.
pub(crate) const PROFILE_COMMAND: &str = "profile";
pub(crate) const PROFILE_HELP: &str = "Open your SJK profile: medals, bio and record";
/// The `achievements` command.
pub(crate) const ACHIEVEMENTS_COMMAND: &str = "achievements";
pub(crate) const ACHIEVEMENTS_HELP: &str = "Open the achievements board";

impl ViewerConsole {
    /// `profile` or `achievements`: show the page on `tab` in front of the console,
    /// opening it if needed, or close it when it already shows that tab.
    pub(crate) fn toggle_profile_panel(&mut self, tab: Tab) {
        if self.open && self.profile_panel.is_open() && self.profile_panel.tab() == tab {
            self.close_profile_panel();
            return;
        }
        self.open_profile_panel(tab);
    }

    /// Show the page on `tab` (the SJK menus' Profile and Achievements entries).
    pub(crate) fn open_profile_panel(&mut self, tab: Tab) {
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
        self.staff_panel.close();
        self.dead_key.settle();
        self.profile_panel.open(tab, owns_console);
    }

    fn close_profile_panel(&mut self) {
        if self.profile_panel.close() {
            self.set_open(false);
        }
    }

    fn profile_panel_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_profile_panel(),
            PanelAction::Staff => {
                let owns_console = self.profile_panel.close();
                self.open_staff_panel();
                if owns_console {
                    self.staff_panel_owns_console();
                }
            }
            PanelAction::Identity => {
                // The Identity page takes the console over; Escape there closes both
                // when the Profile page had opened the console.
                let owns_console = self.profile_panel.close();
                self.open_identity_panel_owned(owns_console);
            }
            PanelAction::Save { bio } => {
                // The page shows the hub's answer, or why nothing could be sent.
                if !crate::player_identity::set_bio(bio) {
                    self.push_log("^3profile: the identity is not running yet");
                }
            }
        }
    }

    /// The identity service's last notice, to tell a save's answer.
    fn profile_notice() -> Option<String> {
        crate::player_identity::snapshot().and_then(|snapshot| snapshot.notice)
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn profile_panel_key(&mut self, event: &KeyEvent) -> bool {
        if !self.profile_panel.is_open() {
            return false;
        }
        let action = self
            .profile_panel
            .handle_key(event, self.shift, Self::profile_notice());
        self.profile_panel_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn profile_panel_pointer(&mut self, event: InputEvent) -> bool {
        if !self.profile_panel.is_open() {
            return false;
        }
        let action = self
            .profile_panel
            .handle_pointer(event, Self::profile_notice());
        self.profile_panel_action(action);
        true
    }

    /// Draw the page with its text to `target`.
    pub(super) fn append_profile_panel(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        #[cfg(test)]
        if let Some(preview) = self.profile_panel.preview.take() {
            let inputs = Inputs {
                enabled: true,
                snapshot: Some(&preview.snapshot),
                standings: &preview.standings,
                record: &preview.record,
            };
            self.profile_panel.append_sjk(&inputs, target, viewport);
            self.profile_panel.preview = Some(preview);
            return;
        }
        // Copied out so the page can borrow itself mutably while it draws.
        let snapshot = crate::player_identity::snapshot();
        let held = snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.me.as_ref())
            .map(|me| me.achievements.clone())
            .unwrap_or_default();
        let standings = crate::achievements::standings(&held);
        let record = record();
        let inputs = Inputs {
            enabled: self.bool_cvar("cl_identity") == Some(true),
            snapshot: snapshot.as_ref(),
            standings: &standings,
            record: &record,
        };
        self.profile_panel.append_sjk(&inputs, target, viewport);
    }

    /// Show `preview` in place of the live identity and counts, for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_profile(&mut self, preview: super::profile_panel::Preview) {
        self.profile_panel.preview = Some(preview);
    }

    /// Type `text` into the open page's bio, for a world shot.
    #[cfg(test)]
    pub(crate) fn profile_type_bio(&mut self, text: &str) {
        self.profile_panel.type_for_shot(text);
    }

    /// The page's draw list while it is shown.
    pub(super) fn profile_panel_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.profile_panel.is_open()).then(|| self.profile_panel.draw_list())
    }
}

/// The player's record as the Profile tab lists it.
fn record() -> Vec<(&'static str, String)> {
    use crate::achievements::{Counter, count};
    let minutes = count(Counter::Minutes);
    vec![
        ("Players defeated", count(Counter::Kills).to_string()),
        ("Saber kills", count(Counter::SaberKills).to_string()),
        ("Best streak", count(Counter::BestStreak).to_string()),
        ("Duels won", count(Counter::DuelWins).to_string()),
        ("Flags captured", count(Counter::Captures).to_string()),
        ("Maps played", count(Counter::Maps).to_string()),
        ("Servers played", count(Counter::Servers).to_string()),
        (
            "Time played",
            format!("{} h {:02} min", minutes / 60, minutes % 60),
        ),
    ]
}
