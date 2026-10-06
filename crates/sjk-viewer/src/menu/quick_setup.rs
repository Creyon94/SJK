//! The Quick setup screen: Settings' QUICK tab, offered once on a first start and
//! opened again by the `quicksetup` command.

use super::*;
use crate::settings::quick::SEEN_CVAR;

/// Console command that opens the screen.
pub(crate) const COMMAND: &str = "quicksetup";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Open Quick setup: the settings worth choosing on a first start";

impl ClientMenu {
    /// Open the Quick setup screen, returning to `target` when it closes.
    pub(crate) fn open_quick_setup(&mut self, console: &ViewerConsole, target: ReturnTarget) {
        self.open_settings_from(console, target, SettingsMenu::quick_tab());
    }

    /// On the first start (`ui_quickSetup` still 0) open the screen over the main
    /// menu and mark it seen, so leaving it with Escape dismisses it for good.
    pub(crate) fn offer_quick_setup(&mut self, console: &mut ViewerConsole) -> bool {
        if console.integer_cvar(SEEN_CVAR) != Some(0) {
            return false;
        }
        console.set_cvar(SEEN_CVAR, "1");
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
    fn the_first_start_offers_the_screen_once() {
        let (_directory, mut console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        assert_eq!(console.integer_cvar(SEEN_CVAR), Some(0));
        assert!(menu.offer_quick_setup(&mut console));
        assert_eq!(console.integer_cvar(SEEN_CVAR), Some(1));
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
        let mut again = ClientMenu::new(true, String::new());
        assert!(!again.offer_quick_setup(&mut console));
        assert_eq!(*again.state.phase(), ClientPhase::MainMenu);
    }

    #[test]
    fn the_command_opens_the_quick_tab_on_demand() {
        let (_directory, console) = console();
        let mut menu = ClientMenu::new(true, String::new());
        menu.open_quick_setup(&console, ReturnTarget::MainMenu);
        assert_eq!(*menu.state.phase(), ClientPhase::Settings);
    }
}
