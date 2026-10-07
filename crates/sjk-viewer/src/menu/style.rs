//! The `ui_menuStyle` setting: which layout the main menu uses.
//!
//! `modern` is the native hero layout ([`super::main_view`]); `classic`
//! follows the original Jedi Academy multiplayer menus ([`super::classic`]);
//! `sjk` is the SJK UI ([`super::sjk`]), SJK's own menus drawn over the live
//! map, which is being built screen by screen and becomes the default once
//! done. SJK starts on `classic`; JKR's default is `modern`.
//! The in-game menu follows the same setting ([`crate::ingame_menu`]).
//! Screens without a classic version yet use their modern layout in both;
//! screens without an SJK UI version yet use their classic one
//! ([`MenuStyle::classic_screens`]).

use super::ClientMenu;
use super::art::{self, ArtSet};
use crate::console::ViewerConsole;

/// Archived cvar naming the menu style.
pub(crate) const CVAR: &str = "ui_menuStyle";

/// Layout family of the main menu.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum MenuStyle {
    /// The native layout: one column of entries over the live map.
    Modern,
    /// Close to the retail menus in layout and flow, for players who know
    /// where things were in the original game. SJK's default.
    #[default]
    Classic,
    /// The SJK UI: SJK's own menus over the live map, in its own type.
    Sjk,
}

impl MenuStyle {
    /// Values the settings screen offers, in [`MenuStyle`] order.
    pub(crate) const NAMES: [&'static str; 3] = ["modern", "classic", "sjk"];
    /// The `ui_menuStyle` value of the default style.
    pub(crate) const DEFAULT_NAME: &'static str = Self::NAMES[1];

    /// Read the cvar value: `modern` (any case) or `0` selects the modern
    /// style, `sjk` (any case) the SJK UI; anything else, including a missing
    /// or mistyped value, the default classic one, so a typo never leaves the
    /// player without a menu.
    pub(crate) fn from_cvar(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(text) if text.eq_ignore_ascii_case("modern") || text == "0" => Self::Modern,
            Some(text) if text.eq_ignore_ascii_case("sjk") => Self::Sjk,
            _ => Self::Classic,
        }
    }

    /// Whether screens drawn in a classic and a modern version use the
    /// classic one: in the classic style, and in the SJK UI until the screen
    /// has its own version.
    pub(crate) fn classic_screens(self) -> bool {
        matches!(self, Self::Classic | Self::Sjk)
    }

    /// The map the menus stand on from the start: the SJK UI's main page is
    /// drawn over mp/duel6's temple courtyard; the other styles keep mp/ffa3,
    /// whose camera routes, gate and player stage the menu backdrop has
    /// (`menu_backdrop`).
    pub(crate) fn boot_map(self) -> &'static str {
        match self {
            Self::Sjk => "maps/mp/duel6.bsp",
            Self::Modern | Self::Classic => "maps/mp/ffa3.bsp",
        }
    }
}

impl ClientMenu {
    /// Apply the player's `ui_menuStyle`. A change puts the main menu back on
    /// its first page and entry, since selections do not carry across
    /// layouts.
    pub(crate) fn set_menu_style(&mut self, style: MenuStyle, console: &ViewerConsole) {
        if style != self.menu_style {
            let classic_screens = self.menu_style.classic_screens();
            let sjk_settings = self.sjk_settings_on_show();
            self.menu_style = style;
            self.main_selection = 0;
            self.classic.reset();
            self.home.reset();
            // An open option panel carries on as the modern screen (the
            // Menu style row itself sits on the Interface panel), on the tab
            // holding the row; between the classic style and the SJK UI,
            // which share the panels, it stays as it is, but for the SJK UI's
            // own Settings, which hands over to the classic panel.
            if classic_screens != style.classic_screens() {
                self.leave_sjk_settings();
                self.leave_classic_panel();
                self.settings.continue_modern(console);
            } else if sjk_settings {
                self.sjk_settings_to_classic(console);
            }
        }
    }

    /// The retail artwork the classic pages can draw this frame; the player
    /// screen follows the style and gets the same pieces.
    pub(crate) fn set_menu_art(&mut self, art: ArtSet) {
        self.art = art;
        self.player
            .set_style(self.menu_style.classic_screens(), art);
    }
}

impl crate::GpuState {
    /// Apply `ui_menuStyle` to the main and in-game menus. While the
    /// classic style is on, or the classic console (whose command browser is
    /// classic+), the player's retail menu artwork is decoded (once, on a
    /// worker) and uploaded when ready; the menus and the console's browser
    /// are told which pieces they can draw.
    pub(crate) fn sync_menu_style(&mut self) {
        let Some(console) = &self.console else {
            return;
        };
        let style = MenuStyle::from_cvar(console.text_value(CVAR));
        let classic_console = crate::game_font::classic_console(Some(console));
        // SJK's emblem is on every style's main page.
        crate::menu::emblem::request();
        self.ui_shapes.install_emblem(&self.device, &self.queue);
        if style.classic_screens() || classic_console {
            if let Some(vfs) = &self.vfs {
                art::request(vfs);
            }
            self.ui_shapes.install_menu_art(&self.device, &self.queue);
        }
        let art = self.ui_shapes.menu_art();
        if let Some(menu) = &mut self.client_menu {
            menu.set_menu_style(style, console);
            menu.set_menu_art(art);
        }
        self.in_game_menu.set_style(style, art);
        let classic = style.classic_screens();
        self.text_dialog.set_look(classic, art);
        if let Some(console) = &mut self.console {
            console.set_browser_art(art);
            console.set_changelog_look(classic, art);
            console.set_credits_look(classic);
            console.set_identity_look(classic, art);
        }
        // The first start's First setup waits for the style, so it opens in the right one.
        if let (Some(menu), Some(console)) = (&mut self.client_menu, &mut self.console) {
            menu.offer_quick_setup(console);
        }
        self.sync_classic_loading();
    }

    /// Feed the classic loading screen this frame's progress: the join's
    /// destination world (the portal) from the menu, or the map change's
    /// load on a server world.
    fn sync_classic_loading(&mut self) {
        use super::classic::loading::{WorldStage, progress};
        let Some(menu) = &self.client_menu else {
            return;
        };
        if !menu.is_classic() || !menu.is_loading_screen() {
            return;
        }
        let stage = if self.is_menu_world {
            self.portal.stage()
        } else if self.world_install_task.is_some() {
            Some(WorldStage::Building)
        } else if self.world_load_task.is_some() {
            Some(WorldStage::Parsing)
        } else {
            None
        };
        let joined = self.resident.session.is_some() || self.live_session.is_some();
        let map = menu.loading_map().to_owned();
        let (subject, ticks) = progress(&map, stage, joined);
        if let Some(menu) = &mut self.client_menu {
            menu.loading_mut().set_progress(&subject, ticks);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_needs_an_explicit_value() {
        assert_eq!(MenuStyle::from_cvar(None), MenuStyle::Classic);
        assert_eq!(MenuStyle::from_cvar(Some("")), MenuStyle::Classic);
        assert_eq!(MenuStyle::from_cvar(Some("modern")), MenuStyle::Modern);
        assert_eq!(MenuStyle::from_cvar(Some(" Modern ")), MenuStyle::Modern);
        assert_eq!(MenuStyle::from_cvar(Some("modrn")), MenuStyle::Classic);
        assert_eq!(MenuStyle::from_cvar(Some("0")), MenuStyle::Modern);
        assert_eq!(MenuStyle::from_cvar(Some("classic")), MenuStyle::Classic);
        assert_eq!(MenuStyle::from_cvar(Some(" Classic ")), MenuStyle::Classic);
        assert_eq!(MenuStyle::from_cvar(Some("1")), MenuStyle::Classic);
    }

    #[test]
    fn offered_names_parse_in_order() {
        let parsed = MenuStyle::NAMES.map(|name| MenuStyle::from_cvar(Some(name)));
        assert_eq!(
            parsed,
            [MenuStyle::Modern, MenuStyle::Classic, MenuStyle::Sjk]
        );
        assert_eq!(MenuStyle::from_cvar(Some(" SJK ")), MenuStyle::Sjk);
        // The SJK UI borrows the classic screens it has no version of yet.
        assert!(MenuStyle::Sjk.classic_screens() && MenuStyle::Classic.classic_screens());
        assert!(!MenuStyle::Modern.classic_screens());
        assert_eq!(
            MenuStyle::from_cvar(Some(MenuStyle::DEFAULT_NAME)),
            MenuStyle::default()
        );
    }
}
