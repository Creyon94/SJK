//! Console ownership and settings for the content cards: JoF content and the
//! Sith cloud, one card each with its own settings.
use super::*;
use crate::jof_content::card::{Action, Card};
use crate::{jof_content, sith_cloud};

/// Which content card.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cloud {
    Jof,
    Sith,
}

const CLOUDS: [Cloud; 2] = [Cloud::Jof, Cloud::Sith];

impl Cloud {
    /// The switch, groups and files settings.
    fn cvars(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Jof => (jof_content::SWITCH, jof_content::GROUPS, jof_content::FILES),
            Self::Sith => (sith_cloud::SWITCH, sith_cloud::GROUPS, sith_cloud::FILES),
        }
    }
}

impl ViewerConsole {
    pub(super) fn content_card(&mut self, cloud: Cloud) -> &mut Card {
        match cloud {
            Cloud::Jof => &mut self.jof_content,
            Cloud::Sith => &mut self.sith_cloud,
        }
    }

    /// The open content card, if any.
    pub(super) fn open_cloud(&self) -> Option<Cloud> {
        CLOUDS.into_iter().find(|&cloud| match cloud {
            Cloud::Jof => self.jof_content.open,
            Cloud::Sith => self.sith_cloud.open,
        })
    }

    fn content_settings(
        &self,
        cloud: Cloud,
        game_data: &std::path::Path,
    ) -> sjk_jof_content::job::Settings {
        match cloud {
            Cloud::Jof => jof_content::job_settings(self, game_data),
            Cloud::Sith => sith_cloud::job_settings(self, game_data),
        }
    }

    /// Open the card over whatever is on screen; a first look starts a refresh.
    pub(crate) fn open_jof_content(&mut self, game_data: &std::path::Path) {
        self.open_content(Cloud::Jof, game_data);
    }

    pub(crate) fn open_sith_cloud(&mut self, game_data: &std::path::Path) {
        self.open_content(Cloud::Sith, game_data);
    }

    fn open_content(&mut self, cloud: Cloud, game_data: &std::path::Path) {
        let owns_console = !self.open;
        let (switch, groups, files) = cloud.cvars();
        let on = self.bool_cvar(switch).unwrap_or(false);
        let selection =
            sjk_jof_content::Selection::parse(self.text_value(groups).unwrap_or("default"))
                .with_files(self.text_value(files).unwrap_or(""));
        let folder = match cloud {
            Cloud::Jof => jof_content::folder(self),
            Cloud::Sith => sith_cloud::folder(game_data),
        };
        let mut card =
            std::mem::replace(self.content_card(cloud), Card::new(&jof_content::card::JOF));
        let refresh = card.open(game_data, on, selection, &folder);
        if refresh {
            card.refresh(self.content_settings(cloud, game_data));
        }
        self.set_open(false);
        self.set_open(true);
        card.owns_console = owns_console;
        card.open = true;
        *self.content_card(cloud) = card;
    }

    fn content_action(&mut self, cloud: Cloud, action: Action) {
        let (switch, groups_cvar, files_cvar) = cloud.cvars();
        match action {
            Action::None => {}
            Action::Close => {
                let card = self.content_card(cloud);
                card.open = false;
                if card.owns_console {
                    self.set_open(false);
                }
            }
            Action::Switch(on) => {
                self.set_cvar(switch, if on { "1" } else { "0" });
                if cloud == Cloud::Jof {
                    jof_content::follow(self);
                }
            }
            Action::Choice { groups, files } => {
                self.set_cvar(groups_cvar, &groups);
                self.set_cvar(files_cvar, &files);
            }
            Action::Check => {
                let game_data = self.content_card(cloud).game_data().to_owned();
                let settings = self.content_settings(cloud, &game_data);
                self.content_card(cloud).refresh(settings);
            }
            Action::Download => {
                let game_data = self.content_card(cloud).game_data().to_owned();
                let settings = self.content_settings(cloud, &game_data);
                self.content_card(cloud).download(settings);
            }
        }
    }

    pub(super) fn jof_content_key(&mut self, event: &KeyEvent) -> bool {
        let Some(cloud) = self.open_cloud() else {
            return false;
        };
        let action = self.content_card(cloud).key(event);
        self.content_action(cloud, action);
        true
    }

    pub(super) fn jof_content_pointer(&mut self, event: sjk_ui::InputEvent) -> bool {
        let Some(cloud) = self.open_cloud() else {
            return false;
        };
        let action = self.content_card(cloud).pointer(event);
        self.content_action(cloud, action);
        true
    }

    /// The open content card's drawing, if one is open.
    pub(crate) fn content_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        match self.open_cloud()? {
            Cloud::Jof => Some(self.jof_content.draw_list()),
            Cloud::Sith => Some(self.sith_cloud.draw_list()),
        }
    }

    /// Close both content cards.
    pub(crate) fn close_content(&mut self) {
        self.jof_content.open = false;
        self.sith_cloud.open = false;
    }
}
