//! The `ui_menuStyle` setting: which layout the main menu uses.
//!
//! `modern` is the native hero layout ([`super::main_view`]); `classic`
//! follows the original Jedi Academy multiplayer menus ([`super::classic`]).
//! SJK starts on `classic`; JKR's default is `modern`.
//! The in-game menu follows the same setting ([`crate::ingame_menu`]).
//! Screens without a classic version yet use their modern layout in both.

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
}

impl MenuStyle {
    /// Values the settings screen offers, in [`MenuStyle`] order.
    pub(crate) const NAMES: [&'static str; 2] = ["modern", "classic"];
    /// The `ui_menuStyle` value of the default style.
    pub(crate) const DEFAULT_NAME: &'static str = Self::NAMES[1];

    /// Read the cvar value: `modern` (any case) or `0` selects the modern
    /// style; anything else, including a missing or mistyped value, the
    /// default classic one, so a typo never leaves the player without a menu.
    pub(crate) fn from_cvar(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(text) if text.eq_ignore_ascii_case("modern") || text == "0" => Self::Modern,
            _ => Self::Classic,
        }
    }
}

impl ClientMenu {
    /// Apply the player's `ui_menuStyle`. A change puts the main menu back on
    /// its first page and entry, since selections do not carry across
    /// layouts.
    pub(crate) fn set_menu_style(&mut self, style: MenuStyle, console: &ViewerConsole) {
        if style != self.menu_style {
            self.menu_style = style;
            self.main_selection = 0;
            self.classic.reset();
            // An open option panel carries on as the modern screen (the
            // Menu style row itself sits on the Interface panel), on the tab
            // holding the row.
            self.leave_classic_panel();
            self.settings.continue_modern(console);
        }
    }

    /// The retail artwork the classic pages can draw this frame; the player
    /// screen follows the style and gets the same pieces.
    pub(crate) fn set_menu_art(&mut self, art: ArtSet) {
        self.art = art;
        self.player
            .set_style(self.menu_style == MenuStyle::Classic, art);
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
        // SJK's emblem is on both styles' main page.
        crate::menu::emblem::request();
        self.ui_shapes.install_emblem(&self.device, &self.queue);
        if style == MenuStyle::Classic || classic_console {
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
        if let Some(console) = &mut self.console {
            console.set_browser_art(art);
            console.set_changelog_look(style == MenuStyle::Classic, art);
            console.set_credits_look(style == MenuStyle::Classic);
            console.set_identity_look(style == MenuStyle::Classic, art);
        }
        // The first start's Quick setup waits for the style, so it opens in the right one.
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
        assert_eq!(parsed, [MenuStyle::Modern, MenuStyle::Classic]);
        assert_eq!(
            MenuStyle::from_cvar(Some(MenuStyle::DEFAULT_NAME)),
            MenuStyle::default()
        );
    }
}
