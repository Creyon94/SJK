//! Console side of the SJK chat page (see `sjk_chat_panel.rs`): opening it, sending
//! what it asks (messages to the hub, mutes on this PC through the mute list in
//! `player_mutes.rs`, staff requests), and routing keys and pointer events to it, as
//! for the Staff page.

use super::sjk_chat_panel::{Inputs, PanelAction, StaffShown};
use super::*;
use sjk_ui::InputEvent;

/// The `sjkchat` command.
pub(crate) const SJK_CHAT_COMMAND: &str = "sjkchat";
pub(crate) const SJK_CHAT_HELP: &str =
    "Open the SJK chat, which every SJK player shares through the SJK hub";

impl ViewerConsole {
    /// `sjkchat`: show the page, or close it.
    pub(crate) fn toggle_sjk_chat_panel(&mut self) {
        if self.open && self.sjk_chat_panel.is_open() {
            self.close_sjk_chat_panel();
            return;
        }
        self.open_sjk_chat_panel();
    }

    /// Show the page (the main page's docked chat, the in-game SJK menu).
    pub(crate) fn open_sjk_chat_panel(&mut self) {
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
        self.collection_panel.close();
        self.holocrons_panel.close();
        self.staff_panel.close();
        self.dead_key.settle();
        self.sjk_chat_panel.open(owns_console);
    }

    fn close_sjk_chat_panel(&mut self) {
        if self.sjk_chat_panel.close() {
            self.set_open(false);
        }
    }

    fn sjk_chat_panel_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_sjk_chat_panel(),
            PanelAction::Send(text) => {
                if self.bool_cvar("cl_sjkChat") == Some(false) {
                    self.push_log("^3sjkchat: SJK chat is off (cl_sjkChat 1 turns it on)");
                } else if !crate::player_identity::chat(text) {
                    self.push_log("^3sjkchat: the SJK identity is off (cl_identity 1)");
                }
            }
            PanelAction::Mute {
                key_id,
                name,
                muted,
            } => crate::player_mutes::set_muted(Some(&key_id), &name, muted),
            PanelAction::GiphyKey(key) => {
                // Saved with the profile; never logged.
                self.set_cvars([(crate::chat_gifs::search::KEY_CVAR, key.as_str())]);
            }
            PanelAction::Staff(request) => {
                let serial = crate::player_identity::staff_state().map_or(0, |state| state.serial);
                if crate::player_identity::staff(request) {
                    self.sjk_chat_panel.staff_sent(serial);
                } else {
                    self.push_log("^3sjkchat: the identity is not running");
                }
            }
        }
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn sjk_chat_panel_key(&mut self, event: &KeyEvent) -> bool {
        if !self.sjk_chat_panel.is_open() {
            return false;
        }
        let action = self
            .sjk_chat_panel
            .handle_key(event, self.shift, self.control);
        self.sjk_chat_panel_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn sjk_chat_panel_pointer(&mut self, event: InputEvent) -> bool {
        if !self.sjk_chat_panel.is_open() {
            return false;
        }
        let action = self.sjk_chat_panel.handle_pointer(event);
        self.sjk_chat_panel_action(action);
        true
    }

    /// Draw the page with its text to `target`.
    pub(super) fn append_sjk_chat_panel(
        &mut self,
        target: crate::menu::sjk::TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        self.sjk_chat_panel.place_card(
            crate::player_mutes::place,
            crate::player_identity::avatar_version,
        );
        self.sjk_chat_panel
            .find_pictures(crate::player_identity::avatar_version);
        let muted = crate::player_mutes::muted_keys();
        let staff = crate::player_identity::is_staff();
        let staff_state = staff.then(crate::player_identity::staff_state).flatten();
        let staff_shown = staff_state.as_ref().map(|state| StaffShown {
            serial: state.serial,
            message: &state.message,
            failed: state.failed,
            busy: state.busy,
        });
        let enabled = self.bool_cvar("cl_sjkChat") != Some(false);
        let now = crate::menu::sjk::recent::now();
        // A copy, drawn once the lock is let go: drawing asks the identity for the
        // player's own key (to keep it off the chosen message and the sender card),
        // which takes that lock again and would wait on itself for ever.
        // `None`: the identity is not running, and the page says so.
        let chat = crate::player_identity::with_chat(Clone::clone);
        #[cfg(test)]
        let chat = self.sjk_chat_panel.preview.clone().or(chat);
        let giphy_key = self
            .text_cvar(crate::chat_gifs::search::KEY_CVAR)
            .map(str::to_owned)
            .unwrap_or_default();
        let inputs = Inputs {
            chat: chat.as_ref(),
            muted: &muted,
            staff,
            enabled,
            now,
            staff_state: staff_shown,
            giphy_key: &giphy_key,
        };
        self.sjk_chat_panel.append_sjk(&inputs, target, viewport);
    }

    /// Open the page on `chat` in place of the live one, `chosen` the message chosen,
    /// for a world shot.
    #[cfg(test)]
    pub(crate) fn preview_sjk_chat(&mut self, chat: sjk_identity::ChatState, chosen: Option<u64>) {
        self.open_sjk_chat_panel();
        self.sjk_chat_panel.preview = Some(chat);
        if let Some(id) = chosen {
            self.sjk_chat_panel.choose_for_shot(id);
        }
    }

    /// Open the page on `chat` with the GIF picker on tab `tab`, `typed` searched, for a
    /// world shot.
    #[cfg(test)]
    pub(crate) fn preview_sjk_chat_gifs(
        &mut self,
        chat: sjk_identity::ChatState,
        tab: usize,
        typed: &str,
    ) {
        self.preview_sjk_chat(chat, None);
        self.sjk_chat_panel.open_gifs_for_shot(tab, typed);
    }

    /// Open the page on `chat` with the GIF picker's key card, `typed` in its field, for
    /// a world shot.
    #[cfg(test)]
    pub(crate) fn preview_sjk_chat_key_card(&mut self, chat: sjk_identity::ChatState, typed: &str) {
        self.preview_sjk_chat(chat, None);
        self.sjk_chat_panel.open_key_card_for_shot(typed);
    }

    /// Answer the open picker's search with `ids` of `total`, for a world shot.
    #[cfg(test)]
    pub(crate) fn answer_sjk_chat_gif_search(
        &mut self,
        ids: Vec<crate::chat_gifs::GifId>,
        total: usize,
    ) {
        self.sjk_chat_panel.answer_gif_search_for_shot(ids, total);
    }

    /// Rest the pointer on a player of the open page's who-is-online window, for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn sjk_chat_hover_person(&mut self, index: Option<u16>) {
        self.sjk_chat_panel.hover_person_for_shot(index);
    }

    /// The page's draw list while it is shown.
    pub(super) fn sjk_chat_panel_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.sjk_chat_panel.is_open()).then(|| self.sjk_chat_panel.draw_list())
    }
}
