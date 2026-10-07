//! The First setup screen: Settings' FIRST SETUP tab, offered at every start until
//! its "Don't show at start" row is ticked, and opened by the `firstsetup` command
//! (`quicksetup`, its old name); `firstsetup import` opens the Import page
//! (`config_import.rs`).

use super::classic::layout::{Entry, Page};
use super::classic::panel::Frame;
use super::*;
use crate::settings::quick::HIDE_CVAR;

/// Console command that opens the screen.
pub(crate) const COMMAND: &str = "firstsetup";
/// The command's name before the screen was renamed (Quick setup), still accepted.
pub(crate) const OLD_COMMAND: &str = "quicksetup";
pub(crate) const OLD_HELP: &str = "Old name of firstsetup";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Open First setup; firstsetup import <file.cfg> brings name, model, FOV and keys from another client";
/// The command's argument that opens the Import page instead.
pub(crate) const IMPORT: &str = "import";

impl ClientMenu {
    /// Open the First setup screen, returning to `target` when it closes: the
    /// classic Setup panel's FIRST SETUP group under the classic style, the FIRST SETUP
    /// tab of the modern screen otherwise.
    pub(crate) fn open_quick_setup(&mut self, console: &ViewerConsole, target: ReturnTarget) {
        if self.menu_style == MenuStyle::Classic {
            let frame = match target {
                ReturnTarget::MainMenu => Frame::Main,
                ReturnTarget::InGame => Frame::InGame,
            };
            if self.open_classic_panel(console, Page::Setup, Entry::FirstSetup, frame, target) {
                return;
            }
        }
        self.open_settings_from(console, target, SettingsMenu::quick_tab());
    }

    /// The first time the main menu is up in a run, once the menu style is known,
    /// open the screen over it, unless the player ticked "Don't show at start"
    /// (`ui_hideFirstSetup`). Escape leaves it for this run.
    pub(crate) fn offer_quick_setup(&mut self, console: &mut ViewerConsole) -> bool {
        if self.first_setup_offered
            || console.bool_cvar(HIDE_CVAR) == Some(true)
            || *self.state.phase() != ClientPhase::MainMenu
        {
            return false;
        }
        self.first_setup_offered = true;
        self.open_quick_setup(console, ReturnTarget::MainMenu);
        true
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

    #[test]
    fn every_start_offers_the_screen_once_until_it_is_hidden() {
        let (_directory, mut console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        assert_eq!(console.bool_cvar(HIDE_CVAR), Some(false));
        assert!(menu.offer_quick_setup(&mut console));
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        // Not again in the same run.
        menu.state.main_menu();
        assert!(!menu.offer_quick_setup(&mut console));
        // The next start offers it again, until the player hides it.
        let mut next = ClientMenu::new(true, String::new());
        assert!(next.offer_quick_setup(&mut console));
        console.set_cvar(HIDE_CVAR, "1");
        let mut hidden = ClientMenu::new(true, String::new());
        assert!(!hidden.offer_quick_setup(&mut console));
        assert_eq!(*hidden.state.phase(), ClientPhase::MainMenu);
    }

    #[test]
    fn the_classic_style_opens_the_setup_panel_group() {
        let (_directory, console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        assert_eq!(menu.menu_style, MenuStyle::Classic);
        menu.open_quick_setup(&console, ReturnTarget::MainMenu);
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        let panel = menu.classic_panel.expect("a classic panel");
        assert_eq!((panel.page, panel.entry), (Page::Setup, Entry::FirstSetup));
        assert_eq!(panel.frame, Frame::Main);
        // In a game it is the in-game frame.
        menu.open_quick_setup(&console, ReturnTarget::InGame);
        assert_eq!(
            menu.classic_panel.map(|panel| panel.frame),
            Some(Frame::InGame)
        );
    }

    #[test]
    fn the_modern_style_opens_the_first_setup_tab() {
        let (_directory, console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        menu.menu_style = MenuStyle::Modern;
        menu.open_quick_setup(&console, ReturnTarget::MainMenu);
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        assert!(menu.classic_panel.is_none());
    }
}
