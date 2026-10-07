//! The SJK UI's Settings screen (`docs/sjk-ui.md`, Settings): every category
//! of settings down one rail, opened from the main page's Settings (and at
//! start for First setup). The rows are the classic+ panels' groups
//! ([`crate::settings::SettingsMenu`] draws them in [`crate::settings::Rail`]'s
//! frame); Key bindings opens the classic+ key bindings until they get an SJK
//! UI screen of their own, and comes back here.

use super::super::classic::layout::{Entry, Page, Panel, Span};
use super::super::classic::panel::Frame as PanelFrame;
use super::super::{ClientMenu, ClientPhase, MenuAction};
use super::TextTarget;
use crate::console::ViewerConsole;
use crate::player_menu::ReturnTarget;
use crate::settings::{Group, SettingsMenu, SettingsResult};

/// What a category of the rail shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Shows {
    /// A panel of settings rows.
    Rows(Panel),
    /// The key bindings (the classic+ screen, for now).
    Bindings,
}

/// One category of the rail: its name, its settings icon and what it shows.
#[derive(Clone, Copy, Debug)]
struct Category {
    label: &'static str,
    icon: &'static str,
    shows: Shows,
}

const fn tab(caption: &'static str) -> Shows {
    Shows::Rows(Panel::Settings {
        caption,
        span: Span::ALL,
    })
}

/// The rail, top to bottom.
const CATEGORIES: [Category; 11] = [
    Category {
        label: "First setup",
        icon: "first_setup",
        shows: Shows::Rows(Panel::Group(Group::Quick)),
    },
    Category {
        label: "Display",
        icon: "video",
        shows: tab("VIDEO"),
    },
    Category {
        label: "Graphics",
        icon: "graphics",
        shows: Shows::Rows(Panel::Group(Group::Graphics)),
    },
    Category {
        label: "Sound",
        icon: "sound",
        shows: tab("AUDIO"),
    },
    Category {
        label: "Mouse",
        icon: "mouse_joystick",
        shows: tab("CONTROLS"),
    },
    Category {
        label: "Key bindings",
        icon: "key_bindings",
        shows: Shows::Bindings,
    },
    Category {
        label: "Gameplay",
        icon: "game_options",
        shows: Shows::Rows(Panel::Group(Group::GameOptions)),
    },
    Category {
        label: "Interface",
        icon: "interface",
        shows: Shows::Rows(Panel::Group(Group::Interface)),
    },
    Category {
        label: "HUD",
        icon: "hud",
        shows: Shows::Rows(Panel::Group(Group::Hud)),
    },
    Category {
        label: "Scoreboard",
        icon: "scoreboard",
        shows: Shows::Rows(Panel::Group(Group::Scoreboard)),
    },
    Category {
        label: "Network",
        icon: "network",
        shows: tab("NETWORK"),
    },
];

/// The rail's names and icons, as the view takes them.
const RAIL: [(&str, &str); CATEGORIES.len()] = {
    let mut rail = [("", ""); CATEGORIES.len()];
    let mut index = 0;
    while index < CATEGORIES.len() {
        rail[index] = (CATEGORIES[index].label, CATEGORIES[index].icon);
        index += 1;
    }
    rail
};

/// The category First setup is.
pub(crate) const FIRST_SETUP: usize = 0;
/// The category the screen opens on first in a run.
const OPENING: usize = 1;

/// Whether the screen is open and on which category.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SettingsPage {
    /// The settings phase shows this screen (not a classic+ panel).
    open: bool,
    /// The category on show, or last shown: the screen opens on it again.
    category: usize,
    /// The key bindings were opened from the rail; closing them comes back.
    bindings: bool,
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self {
            open: false,
            category: OPENING,
            bindings: false,
        }
    }
}

impl SettingsPage {
    /// The category on show, or the one the screen opens on next.
    pub(crate) fn category(&self) -> usize {
        self.category
    }
}

impl ClientMenu {
    /// Open the SJK UI's Settings on category `index`, returning to `target`
    /// when it closes. Key bindings open the classic+ key bindings.
    pub(crate) fn open_sjk_settings(
        &mut self,
        console: &ViewerConsole,
        index: usize,
        target: ReturnTarget,
    ) {
        let index = index.min(CATEGORIES.len() - 1);
        let panel = match CATEGORIES[index].shows {
            Shows::Rows(panel) => panel,
            Shows::Bindings => {
                if let Some(entry) = Page::Controls.opening_panel() {
                    self.open_classic_panel(
                        console,
                        Page::Controls,
                        entry,
                        PanelFrame::Main,
                        target,
                    );
                    self.sjk_settings.bindings = true;
                }
                return;
            }
        };
        self.keybinds.leave_classic();
        match panel {
            Panel::Settings { caption, span } => {
                let tab = SettingsMenu::tab_index(caption).unwrap_or(0);
                self.settings
                    .open_classic(console, tab, span, PanelFrame::Main);
            }
            Panel::Group(group) => {
                self.settings
                    .open_classic_group(console, group, PanelFrame::Main)
            }
            Panel::Renderer { tab } => {
                self.settings
                    .open_classic_renderer(console, tab, PanelFrame::Main);
            }
            Panel::Keybinds { .. } => return,
        }
        self.settings.set_elsewhere(0);
        self.keybinds_direct = false;
        self.settings_return = target;
        self.renderer_panel = None;
        self.classic_panel = None;
        self.sjk_settings = SettingsPage {
            open: true,
            category: index,
            bindings: false,
        };
        self.state.open_settings();
    }

    /// Whether the SJK UI's Settings is the screen on show.
    pub(crate) fn sjk_settings_on_show(&self) -> bool {
        self.sjk_settings.open
            && self.classic_panel.is_none()
            && matches!(self.state.phase(), ClientPhase::Settings)
            && self.settings.has_panel_rows()
    }

    /// The screen is left (to the main menu or the game): it is no longer open,
    /// and key bindings opened from it no longer come back to it.
    pub(in crate::menu) fn close_sjk_settings(&mut self) {
        self.sjk_settings.open = false;
        self.sjk_settings.bindings = false;
    }

    /// Another settings screen opened: this one is not the one on show.
    pub(in crate::menu) fn leave_sjk_settings(&mut self) {
        self.sjk_settings.open = false;
    }

    /// Closing the key bindings opened from the rail: back to the category
    /// they were opened from. False when they were not opened from it.
    pub(in crate::menu) fn return_from_bindings(&mut self, console: &ViewerConsole) -> bool {
        if !std::mem::take(&mut self.sjk_settings.bindings)
            || self.menu_style != super::super::MenuStyle::Sjk
        {
            return false;
        }
        let target = self.settings_return;
        self.leave_classic_panel();
        self.open_sjk_settings(console, self.sjk_settings.category, target);
        true
    }

    /// What the settings rows asked of the screen around them: a category of
    /// the rail (`Classic(index)`), or the next or previous one (Tab). `None`
    /// for results the screen leaves to the settings phase.
    pub(in crate::menu) fn sjk_settings_result(
        &mut self,
        result: &SettingsResult,
        console: &ViewerConsole,
    ) -> Option<MenuAction> {
        if !self.sjk_settings_on_show() {
            return None;
        }
        let target = self.settings_return;
        match *result {
            SettingsResult::Classic(index) if index < CATEGORIES.len() => {
                if index != self.sjk_settings.category
                    || !matches!(CATEGORIES[index].shows, Shows::Rows(_))
                    || self.settings.searching_results()
                {
                    self.open_sjk_settings(console, index, target);
                }
                Some(MenuAction::None)
            }
            SettingsResult::Classic(_) => Some(MenuAction::None),
            SettingsResult::ClassicCycle(direction) => {
                let next = next_rows(self.sjk_settings.category, direction);
                self.open_sjk_settings(console, next, target);
                Some(MenuAction::None)
            }
            _ => None,
        }
    }

    /// Draw the SJK UI's Settings.
    pub(crate) fn append_sjk_settings(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let reveal = self.screen_reveal();
        let rail = crate::settings::Rail {
            categories: &RAIL,
            current: (!self.settings.searching_results()).then_some(self.sjk_settings.category),
        };
        self.settings.append_sjk(target, viewport, reveal, &rail);
    }
}

#[cfg(test)]
impl ClientMenu {
    /// The SJK UI's Settings on category `category`, the row of `cvar` focused,
    /// its list open (`list`) or a `search` typed (menu snapshots).
    pub(crate) fn sjk_settings_for_snapshot(
        &mut self,
        console: &ViewerConsole,
        category: usize,
        cvar: Option<&str>,
        list: bool,
        search: Option<&str>,
    ) {
        self.menu_style = super::super::MenuStyle::Sjk;
        self.open_sjk_settings(console, category, ReturnTarget::MainMenu);
        if let Some(cvar) = cvar {
            self.settings.select_cvar(cvar);
        }
        if list {
            self.settings.dropdown_for_snapshot(console);
        }
        if let Some(text) = search {
            self.settings.search_for_snapshot(console, text);
        }
    }
}

/// The category of rows `direction` steps from `index` along the rail,
/// wrapping and passing over Key bindings, which leaves the screen.
fn next_rows(index: usize, direction: i32) -> usize {
    let count = CATEGORIES.len() as i32;
    let mut next = index as i32;
    for _ in 0..CATEGORIES.len() {
        next = (next + direction.signum()).rem_euclid(count);
        if matches!(CATEGORIES[next as usize].shows, Shows::Rows(_)) {
            return next as usize;
        }
    }
    index
}

/// The classic Setup page and entry whose panel is category `index`'s, if one
/// is (the classic pages have no Graphics of all four renderer tabs).
fn classic_place(index: usize) -> Option<(Page, Entry)> {
    let Shows::Rows(panel) = CATEGORIES.get(index)?.shows else {
        return None;
    };
    [Page::Setup, Page::Graphics, Page::Gameplay]
        .into_iter()
        .find_map(|page| {
            let slot = page
                .slots()
                .iter()
                .find(|slot| slot.entry.panel() == Some(panel))?;
            Some((page, slot.entry))
        })
}

impl ClientMenu {
    /// The menu style changed to classic under the screen (its Menu style row
    /// is on Interface): the classic+ panel of the category on show takes over,
    /// Graphics' first renderer tab for Graphics.
    pub(in crate::menu) fn sjk_settings_to_classic(&mut self, console: &ViewerConsole) {
        let target = self.settings_return;
        let (page, entry) = classic_place(self.sjk_settings.category)
            .unwrap_or((Page::Graphics, Entry::RenderImage));
        self.open_classic_panel(console, page, entry, PanelFrame::Main, target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    fn menu() -> ClientMenu {
        let mut menu = ClientMenu::new(true, String::new());
        menu.menu_style = super::super::super::MenuStyle::Sjk;
        menu
    }

    #[test]
    fn the_rail_shows_every_classic_setup_group_and_the_renderer_as_one() {
        // Every settings panel of the classic Setup pages has its category,
        // but the renderer's four, which Graphics gathers.
        for entry in [
            Entry::FirstSetup,
            Entry::Video,
            Entry::Sound,
            Entry::MouseJoystick,
            Entry::GameOptions,
            Entry::Interface,
            Entry::Hud,
            Entry::Scoreboard,
            Entry::Network,
        ] {
            assert!(
                (0..CATEGORIES.len())
                    .any(|index| classic_place(index).map(|(_, found)| found) == Some(entry)),
                "{entry:?}"
            );
        }
        assert_eq!(CATEGORIES[FIRST_SETUP].label, "First setup");
        for (label, icon) in RAIL {
            assert!(
                crate::settings_icons::ICONS
                    .iter()
                    .any(|(name, _)| *name == icon),
                "{label}: {icon}"
            );
        }
    }

    #[test]
    fn tab_steps_along_the_categories_of_rows() {
        let bindings = CATEGORIES
            .iter()
            .position(|category| category.shows == Shows::Bindings)
            .unwrap();
        assert_eq!(next_rows(bindings - 1, 1), bindings + 1);
        assert_eq!(next_rows(bindings + 1, -1), bindings - 1);
        assert_eq!(next_rows(CATEGORIES.len() - 1, 1), 0);
        assert_eq!(next_rows(0, -1), CATEGORIES.len() - 1);
    }

    #[test]
    fn the_screen_opens_switches_category_and_closes_to_the_main_menu() {
        let (_directory, mut console) = console();
        let mut menu = menu();
        menu.open_sjk_settings(&console, 2, ReturnTarget::MainMenu);
        assert!(menu.sjk_settings_on_show());
        assert!(
            menu.settings.renderer_open(),
            "Graphics holds the renderer's rows"
        );
        // A category of the rail, then Tab to the next.
        let action = menu.settings_result(SettingsResult::Classic(3), &mut console);
        assert_eq!(action, MenuAction::None);
        assert_eq!(menu.sjk_settings.category, 3);
        menu.settings_result(SettingsResult::ClassicCycle(1), &mut console);
        assert_eq!(menu.sjk_settings.category, 4);
        // Back leaves for the main menu; the screen opens again where it was.
        menu.settings_result(SettingsResult::Back, &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
        assert!(!menu.sjk_settings_on_show());
        menu.open_sjk_settings(&console, menu.sjk_settings.category, ReturnTarget::MainMenu);
        assert_eq!(menu.sjk_settings.category, 4);
    }

    #[test]
    fn a_style_change_on_the_screen_hands_over_to_the_same_group() {
        let (_directory, console) = console();
        let interface = CATEGORIES
            .iter()
            .position(|category| category.label == "Interface")
            .unwrap();
        // To classic: the classic+ panel of the same group.
        let mut classic = menu();
        classic.open_sjk_settings(&console, interface, ReturnTarget::MainMenu);
        classic.set_menu_style(super::super::super::MenuStyle::Classic, &console);
        assert!(!classic.sjk_settings_on_show());
        let panel = classic.classic_panel.expect("a classic panel");
        assert_eq!(
            (panel.page, panel.entry),
            (Page::Gameplay, Entry::Interface)
        );
        // To modern: the modern screen.
        let mut modern = menu();
        modern.open_sjk_settings(&console, interface, ReturnTarget::MainMenu);
        modern.set_menu_style(super::super::super::MenuStyle::Modern, &console);
        assert!(!modern.sjk_settings_on_show() && modern.classic_panel.is_none());
        assert_eq!(*modern.state.phase(), ClientPhase::Settings);
    }

    #[test]
    fn key_bindings_come_back_to_the_category_they_were_opened_from() {
        let (_directory, mut console) = console();
        let mut menu = menu();
        menu.open_sjk_settings(&console, 7, ReturnTarget::MainMenu);
        let bindings = CATEGORIES
            .iter()
            .position(|category| category.shows == Shows::Bindings)
            .unwrap();
        menu.settings_result(SettingsResult::Classic(bindings), &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::Keybinds);
        assert!(!menu.sjk_settings_on_show());
        let action = menu.close_keybinds(&console);
        assert_eq!(action, MenuAction::None);
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        assert!(menu.sjk_settings_on_show());
        assert_eq!(menu.sjk_settings.category, 7);
        // Opened another way, they close as before.
        menu.settings_result(SettingsResult::Back, &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
    }
}
