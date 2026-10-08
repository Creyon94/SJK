//! The SJK UI's Settings screen (`docs/sjk-ui.md`, Settings): every category
//! of settings down one rail, opened from the main page's Settings; First
//! setup at start (and the `firstsetup` command) shows its category as a
//! pop-up instead ([`crate::settings::SettingsMenu::append_sjk_popup`]). The rows are the classic+ panels' groups
//! ([`crate::settings::SettingsMenu`] draws them in [`crate::settings::Rail`]'s
//! frame), and Key bindings the classic+ list of every binding
//! ([`crate::keybind_editor::KeybindEditor`] draws it in the same frame).

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
    /// The key bindings: every action with its keys.
    Keys,
    /// The quick wheel's pages and their choices, edited
    /// ([`crate::settings::WheelMode::Category`]).
    Wheel,
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
const CATEGORIES: [Category; 12] = [
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
        shows: Shows::Keys,
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
    // A spare icon of the settings board until the wheel has its own.
    Category {
        label: "Quick wheel",
        icon: "options",
        shows: Shows::Wheel,
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
/// The category the quick wheel's pages are.
pub(crate) const QUICK_WHEEL: usize = 9;
const _: () = assert!(matches!(CATEGORIES[QUICK_WHEEL].shows, Shows::Wheel));
/// The category the screen opens on first in a run.
const OPENING: usize = 1;

/// Whether the screen is open and on which category.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SettingsPage {
    /// The settings phase shows this screen (not a classic+ panel).
    open: bool,
    /// The category on show, or last shown: the screen opens on it again.
    category: usize,
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self {
            open: false,
            category: OPENING,
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
            // The editor shows over Interface's rows, which hold its row.
            Shows::Wheel => Panel::Group(Group::Interface),
            Shows::Keys => {
                self.settings.leave_classic();
                self.keybinds.open_classic(console, 0, Span::ALL);
                self.keybinds.set_elsewhere(0);
                // Escape leaves the screen, as it does from Settings' rows.
                self.keybinds_direct = true;
                self.settings_return = target;
                self.renderer_panel = None;
                self.classic_panel = None;
                self.sjk_settings = SettingsPage {
                    open: true,
                    category: index,
                };
                self.state.open_keybinds();
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
        if CATEGORIES[index].shows == Shows::Wheel {
            self.settings.select_wheel_row();
            self.settings
                .open_wheel_editor(console, crate::settings::WheelMode::Category);
        }
        self.keybinds_direct = false;
        self.settings_return = target;
        self.renderer_panel = None;
        self.classic_panel = None;
        self.sjk_settings = SettingsPage {
            open: true,
            category: index,
        };
        self.state.open_settings();
    }

    /// Open First setup as the SJK UI's pop-up over the map, returning to
    /// `target` when it closes; its All settings opens the whole screen on
    /// First setup.
    pub(crate) fn open_sjk_first_setup(&mut self, console: &ViewerConsole, target: ReturnTarget) {
        self.open_sjk_settings(console, FIRST_SETUP, target);
        self.settings.open_first_setup_popup(console);
    }

    /// Open the SJK UI's Settings over a match, from its in-game menu, on the
    /// category last shown; it returns to the in-game menu.
    pub(crate) fn open_sjk_settings_from_game(&mut self, console: &ViewerConsole) {
        let category = self.sjk_settings.category();
        self.open_sjk_settings(console, category, ReturnTarget::InGame);
    }

    /// The way back the screen's top bar names: the game menu over a match,
    /// else the main menu.
    fn sjk_settings_back(&self) -> &'static str {
        if self.settings_return == ReturnTarget::InGame {
            "Game menu"
        } else {
            "Main menu"
        }
    }

    /// Whether the SJK UI's Settings is the screen on show: a category of
    /// settings rows, or Key bindings.
    pub(crate) fn sjk_settings_on_show(&self) -> bool {
        self.sjk_settings.open
            && self.classic_panel.is_none()
            && match self.state.phase() {
                ClientPhase::Settings => self.settings.has_panel_rows(),
                ClientPhase::Keybinds => self.keybinds.has_list(),
                _ => false,
            }
    }

    /// The screen is left (to the main menu or the game): it is no longer open.
    pub(in crate::menu) fn close_sjk_settings(&mut self) {
        self.sjk_settings.open = false;
    }

    /// Another settings screen opened: this one is not the one on show.
    pub(in crate::menu) fn leave_sjk_settings(&mut self) {
        self.sjk_settings.open = false;
    }

    /// What the key bindings asked of the screen around them, as
    /// [`Self::sjk_settings_result`] reads the settings rows'.
    pub(in crate::menu) fn sjk_keys_result(
        &mut self,
        result: &crate::keybind_editor::EditorResult,
        console: &ViewerConsole,
    ) -> Option<MenuAction> {
        use crate::keybind_editor::EditorResult;
        if !self.sjk_settings_on_show() {
            return None;
        }
        let target = self.settings_return;
        match *result {
            EditorResult::Classic(index) if index < CATEGORIES.len() => {
                if index != self.sjk_settings.category
                    || !self.keybinds.search_text().trim().is_empty()
                {
                    self.open_sjk_settings(console, index, target);
                }
                Some(MenuAction::None)
            }
            EditorResult::Classic(_) => Some(MenuAction::None),
            EditorResult::ClassicCycle(direction) => {
                let next = next_category(self.sjk_settings.category, direction);
                self.open_sjk_settings(console, next, target);
                Some(MenuAction::None)
            }
            _ => None,
        }
    }

    /// Draw the SJK UI's Key bindings.
    pub(crate) fn append_sjk_keys(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let reveal = self.screen_reveal();
        let searched = !self.keybinds.search_text().trim().is_empty();
        let rail = crate::settings::Rail {
            categories: &RAIL,
            current: (!searched).then_some(self.sjk_settings.category),
            back: self.sjk_settings_back(),
        };
        self.keybinds.append_sjk(target, viewport, reveal, &rail);
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
                if index != self.sjk_settings.category || self.settings.searching_results() {
                    self.open_sjk_settings(console, index, target);
                }
                Some(MenuAction::None)
            }
            SettingsResult::Classic(_) => Some(MenuAction::None),
            SettingsResult::ClassicCycle(direction) => {
                let next = next_category(self.sjk_settings.category, direction);
                self.open_sjk_settings(console, next, target);
                Some(MenuAction::None)
            }
            SettingsResult::AllSettings => {
                self.open_sjk_settings(console, FIRST_SETUP, target);
                Some(MenuAction::None)
            }
            // Interface's "Quick wheel pages" row: the Quick wheel category.
            SettingsResult::OpenWheelPages => {
                self.open_sjk_settings(console, QUICK_WHEEL, target);
                Some(MenuAction::None)
            }
            _ => None,
        }
    }

    /// Draw the SJK UI's Settings, or First setup's pop-up.
    pub(crate) fn append_sjk_settings(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let reveal = self.screen_reveal();
        if self.settings.popup() {
            self.settings.append_sjk_popup(target, viewport, reveal);
            return;
        }
        let rail = crate::settings::Rail {
            categories: &RAIL,
            current: (!self.settings.searching_results()).then_some(self.sjk_settings.category),
            back: self.sjk_settings_back(),
        };
        self.settings.append_sjk(target, viewport, reveal, &rail);
    }
}

#[cfg(test)]
impl ClientMenu {
    /// The SJK UI's Key bindings, the action of `command` focused and awaiting
    /// its second key when `capture` (world shots).
    pub(crate) fn sjk_keys_for_shot(
        &mut self,
        console: &ViewerConsole,
        command: &str,
        capture: bool,
    ) {
        let keys = CATEGORIES
            .iter()
            .position(|category| category.shows == Shows::Keys)
            .unwrap_or(0);
        self.open_sjk_settings(console, keys, ReturnTarget::MainMenu);
        self.keybinds.focus_for_shot(command, capture);
    }

    /// Settings > Quick wheel (opened if it is not), after `keys`, each typing
    /// its text if it has one (world shots).
    pub(crate) fn sjk_wheel_for_shot(
        &mut self,
        console: &mut ViewerConsole,
        keys: &[(winit::keyboard::KeyCode, Option<&str>)],
    ) {
        if !self.settings.wheel_editor_open() {
            self.open_sjk_settings(console, QUICK_WHEEL, ReturnTarget::MainMenu);
        }
        self.settings.wheel_keys_for_shot(keys, console);
    }

    /// The quick wheel's editor opened from Interface's row of the classic+
    /// (`classic`) or the modern settings (world shots).
    pub(crate) fn wheel_overlay_for_shot(&mut self, console: &ViewerConsole, classic: bool) {
        if classic {
            self.open_classic_panel(
                console,
                Page::Gameplay,
                Entry::Interface,
                PanelFrame::Main,
                ReturnTarget::MainMenu,
            );
        } else {
            self.open_settings_from(console, ReturnTarget::MainMenu, 0);
        }
        self.settings
            .open_wheel_editor(console, crate::settings::WheelMode::Overlay);
    }

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

/// The category `direction` steps from `index` along the rail, wrapping.
fn next_category(index: usize, direction: i32) -> usize {
    (index as i32 + direction.signum()).rem_euclid(CATEGORIES.len() as i32) as usize
}

/// The classic Setup page and entry whose panel is category `index`'s, if one
/// is (the classic pages have no Graphics of all four renderer tabs); Key
/// bindings is the Controls page's.
fn classic_place(index: usize) -> Option<(Page, Entry)> {
    let panel = match CATEGORIES.get(index)?.shows {
        Shows::Rows(panel) => panel,
        // Interface holds the row that opens the quick wheel's pages.
        Shows::Wheel => Panel::Group(Group::Interface),
        Shows::Keys => return Some((Page::Controls, Entry::Movement)),
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
    fn tab_steps_along_every_category_and_wraps() {
        let keys = CATEGORIES
            .iter()
            .position(|category| category.shows == Shows::Keys)
            .unwrap();
        assert_eq!(next_category(keys - 1, 1), keys);
        assert_eq!(next_category(keys, 1), keys + 1);
        assert_eq!(next_category(CATEGORIES.len() - 1, 1), 0);
        assert_eq!(next_category(0, -1), CATEGORIES.len() - 1);
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
    fn opened_from_a_game_it_names_and_returns_to_the_game_menu() {
        let (_directory, mut console) = console();
        let mut menu = menu();
        menu.open_sjk_settings(&console, 3, ReturnTarget::MainMenu);
        assert_eq!(menu.sjk_settings_back(), "Main menu");
        menu.settings_result(SettingsResult::Back, &mut console);
        // From the game, on the category last shown.
        menu.open_sjk_settings_from_game(&console);
        assert!(menu.sjk_settings_on_show() && menu.sjk_screen());
        assert_eq!(menu.sjk_settings.category, 3);
        assert_eq!(menu.sjk_settings_back(), "Game menu");
        let action = menu.settings_result(SettingsResult::Back, &mut console);
        assert_eq!(action, MenuAction::ReturnToGameMenu);
        assert!(!menu.sjk_settings_on_show());
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
    fn key_bindings_are_a_category_of_the_screen_and_tab_leaves_them() {
        use crate::keybind_editor::EditorResult;
        let (_directory, mut console) = console();
        let mut menu = menu();
        menu.open_sjk_settings(&console, 7, ReturnTarget::MainMenu);
        let keys = CATEGORIES
            .iter()
            .position(|category| category.shows == Shows::Keys)
            .unwrap();
        menu.settings_result(SettingsResult::Classic(keys), &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::Keybinds);
        assert!(
            menu.sjk_settings_on_show(),
            "the same screen, on Key bindings"
        );
        assert!(menu.classic_panel.is_none());
        // Tab to the next category, the rail back to Key bindings, then Escape.
        menu.keybinds_result(EditorResult::ClassicCycle(1), &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        assert_eq!(menu.sjk_settings.category, keys + 1);
        menu.settings_result(SettingsResult::Classic(keys), &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::Keybinds);
        menu.keybinds_result(EditorResult::Back, &mut console);
        assert_eq!(*menu.state.phase(), ClientPhase::MainMenu);
        assert!(!menu.sjk_settings_on_show());
        // A style change there hands over to the classic Controls panel.
        menu.open_sjk_settings(&console, keys, ReturnTarget::MainMenu);
        menu.set_menu_style(super::super::super::MenuStyle::Classic, &console);
        let panel = menu.classic_panel.expect("a classic panel");
        assert_eq!(panel.page, Page::Controls);
    }
}
