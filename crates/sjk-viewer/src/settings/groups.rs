//! The classic Setup page's own groups. Retail's `setup.menu` and JKR's
//! settings tabs cut the settings differently, so classic+ regroups them by
//! what they are about (`docs/classic-plus.md`): gameplay options, the
//! menus and console, the HUD, the scoreboard. Each group lists its rows'
//! cvars; the settings come from the catalogue, so a group shows the same
//! rows the modern tabs do.

use super::catalog::*;
use std::sync::OnceLock;

/// A classic Setup group gathering rows of several settings tabs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Group {
    /// Gameplay: pickups, models, trails, remaps, camera and prediction.
    GameOptions,
    /// The menus' style, colours and fonts, and the console's look.
    Interface,
    /// The status HUD, crosshair and the readouts over the game.
    Hud,
    /// The scoreboard's style and columns.
    Scoreboard,
    /// The first-start settings ([`super::quick`]); gathers rows the other groups
    /// and tabs already hold, so it is not one of [`Group::ALL`].
    Quick,
}

impl Group {
    pub(crate) const ALL: [Self; 4] = [
        Self::GameOptions,
        Self::Interface,
        Self::Hud,
        Self::Scoreboard,
    ];

    /// The group's name, as the one tab of the modern screen showing it.
    const CAPTIONS: [&'static str; 5] = [
        "GAME OPTIONS",
        "INTERFACE",
        "HUD",
        "SCOREBOARD",
        "QUICK SETUP",
    ];

    fn index(self) -> usize {
        match self {
            Self::Quick => Self::ALL.len(),
            _ => Self::ALL
                .iter()
                .position(|group| *group == self)
                .unwrap_or(0),
        }
    }

    /// The tab list of a screen showing the group: its name alone.
    pub(super) fn tabs(self) -> &'static [&'static str] {
        std::slice::from_ref(&Self::CAPTIONS[self.index()])
    }

    /// The cvars of the group's rows, in order.
    fn cvars(self) -> &'static [&'static str] {
        match self {
            Self::GameOptions => &[
                "cg_simpleItems",
                "cg_forceModel",
                "cg_saberTrail",
                "cg_speedTrail",
                "cg_auraShell",
                "cg_spProtAbsColor",
                "cg_shieldSphere",
                "cg_shieldBrightness",
                "cg_remaps",
                "cg_thirdPersonCameraDamp",
                "cg_thirdPersonTargetDamp",
                "cg_errorDecay",
            ],
            Self::Interface => &[
                crate::menu::style::CVAR,
                "ui_accent",
                "ui_menuContrast",
                crate::game_font::CVAR,
                crate::text::style::SCALE_CVAR,
                crate::text::style::TRACKING_CVAR,
                crate::console::console_options::STYLE_CVAR,
                "con_scale",
                "con_lineSpacing",
            ],
            Self::Hud => &[
                "cg_drawHud",
                crate::menu_hud::STYLE_CVAR,
                crate::menu_hud::FILES_CVAR,
                "cg_hudScale",
                "cg_classicHudFont",
                "cg_drawStatus",
                "cg_drawWeapon",
                "cg_crosshair",
                "cg_crosshairSize",
                "cg_drawCrosshairNames",
                "cg_drawPlayerNames",
                "cg_drawPlayerNamesScale",
                "cg_drawFriend",
                "cg_playerCard",
                "cg_playerCardDelay",
                "cg_nameplate",
                "cg_nameplateRange",
                "cg_nameplateNear",
                "cg_nameplateScale",
                "cg_nameplateBars",
                "cg_nameplateForce",
                "cg_nameplatePredict",
                "cg_nameplateWeapon",
                "cg_nameplateSelf",
                "cg_nameplateIcons",
                "cg_nameplateWalls",
                "cg_nameplateNpcs",
                "cg_drawTimer",
                crate::version_overlay::CVAR,
                "cg_speedometer",
                "cg_drawTeamOverlay",
                "cg_lagometer",
                "cg_drawChat",
                crate::ground_hud::CVAR,
            ],
            Self::Scoreboard => &[
                crate::scoreboard::style::CVAR,
                "cg_showClientIDs",
                "cg_drawScoreboardIcons",
                "cg_smallScoreboard",
            ],
            Self::Quick => &[],
        }
    }

    /// The group's settings, looked up in the catalogue once.
    pub(super) fn rows(self) -> &'static [Setting] {
        if self == Self::Quick {
            return super::quick::rows();
        }
        static ROWS: [OnceLock<Vec<Setting>>; 4] = [
            OnceLock::new(),
            OnceLock::new(),
            OnceLock::new(),
            OnceLock::new(),
        ];
        ROWS[self.index()].get_or_init(|| {
            self.cvars()
                .iter()
                .filter_map(|cvar| {
                    (0..TABS.len())
                        .flat_map(super::settings)
                        .find(|setting| setting.cvar.eq_ignore_ascii_case(cvar))
                        .copied()
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_cvar_is_a_setting() {
        for group in Group::ALL {
            assert_eq!(group.rows().len(), group.cvars().len(), "{group:?}");
        }
    }

    #[test]
    fn the_groups_hold_the_game_hud_and_text_tabs_once_each() {
        let grouped: Vec<&str> = Group::ALL
            .iter()
            .flat_map(|group| group.rows())
            .map(|setting| setting.cvar)
            .collect();
        for caption in ["GAME", "HUD", "HUD+", "TEXT"] {
            let tab = TABS.iter().position(|tab| *tab == caption).unwrap();
            for setting in super::super::settings(tab) {
                let count = grouped.iter().filter(|cvar| **cvar == setting.cvar).count();
                assert_eq!(count, 1, "{} ({caption})", setting.cvar);
            }
        }
    }
}
