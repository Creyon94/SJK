//! Console ownership and settings for the JoF content card.
use super::*;
use crate::jof_content::{self, card::Action};

impl ViewerConsole {
    /// Open the card over whatever is on screen; a first look starts a refresh.
    pub(crate) fn open_jof_content(&mut self, game_data: &std::path::Path) {
        let owns_console = !self.open;
        let on = self.bool_cvar(jof_content::SWITCH).unwrap_or(false);
        let selection = sjk_jof_content::Selection::parse(
            self.text_value(jof_content::GROUPS).unwrap_or("default"),
        );
        let folder = jof_content::folder(self);
        let mut card = std::mem::take(&mut self.jof_content);
        let refresh = card.open(game_data, on, selection, &folder);
        if refresh {
            card.refresh(jof_content::job_settings(self, game_data));
        }
        self.set_open(false);
        self.set_open(true);
        card.owns_console = owns_console;
        card.open = true;
        self.jof_content = card;
    }

    fn jof_content_action(&mut self, action: Action) {
        match action {
            Action::None => {}
            Action::Close => {
                self.jof_content.open = false;
                if self.jof_content.owns_console {
                    self.set_open(false);
                }
            }
            Action::Switch(on) => {
                self.set_cvar(jof_content::SWITCH, if on { "1" } else { "0" });
                jof_content::follow(self);
            }
            Action::Groups(value) => {
                self.set_cvar(jof_content::GROUPS, &value);
            }
            Action::Check => {
                let settings = jof_content::job_settings(self, self.jof_content.game_data());
                self.jof_content.refresh(settings);
            }
            Action::Download => {
                let settings = jof_content::job_settings(self, self.jof_content.game_data());
                self.jof_content.download(settings);
            }
        }
    }

    pub(super) fn jof_content_key(&mut self, event: &KeyEvent) -> bool {
        if !self.jof_content.open {
            return false;
        }
        let action = self.jof_content.key(event);
        self.jof_content_action(action);
        true
    }

    pub(super) fn jof_content_pointer(&mut self, event: sjk_ui::InputEvent) -> bool {
        if !self.jof_content.open {
            return false;
        }
        let action = self.jof_content.pointer(event);
        self.jof_content_action(action);
        true
    }
}
