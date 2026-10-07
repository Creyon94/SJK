//! The FIRST SETUP tab of Settings: the few settings worth choosing once on a first
//! start (display, aim, sound, the HUD, nameplates, the Force shells and the
//! network opt-ins). The rows are the catalogue's own, looked up by cvar, so a
//! change here is the same change the other tabs make; the last row, whether the
//! screen opens at start, is the tab's own.

use super::catalog::*;
use std::sync::OnceLock;

/// Set to 1 by the "Don't show at start" row: the screen no longer opens at start.
pub(crate) const HIDE_CVAR: &str = "ui_hideFirstSetup";

/// The tab's own last row.
const HIDE_ROW: Setting = Setting {
    label: "Don't show at start",
    cvar: HIDE_CVAR,
    kind: ValueKind::Bool,
};

/// The tab's cvars, in order.
const CVARS: &[&str] = &[
    "r_resolution",
    "r_fullscreen",
    "r_vsync",
    "cg_fov",
    "sensitivity",
    "m_invert",
    "cl_run",
    "s_volume",
    "s_musicVolume",
    crate::menu_hud::STYLE_CVAR,
    "cg_hudScale",
    "cg_crosshair",
    "cg_nameplate",
    "cg_nameplateRange",
    "cg_nameplateBars",
    "cg_nameplateForce",
    "cg_nameplateIcons",
    "cg_auraShell",
    "cg_spProtAbsColor",
    "cl_autoUpdate",
    "cl_identity",
];

/// The tab's settings, looked up in the other tabs once.
pub(super) fn rows() -> &'static [Setting] {
    static ROWS: OnceLock<Vec<Setting>> = OnceLock::new();
    ROWS.get_or_init(|| {
        CVARS
            .iter()
            .filter_map(|cvar| {
                // Not the QUICK tab itself, which is what is being built.
                (0..QUICK_TAB)
                    .flat_map(super::settings)
                    .find(|setting| setting.cvar.eq_ignore_ascii_case(cvar))
                    .copied()
            })
            .chain(std::iter::once(HIDE_ROW))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_cvar_is_a_setting() {
        assert_eq!(rows().len(), CVARS.len() + 1);
        assert_eq!(rows().last().map(|row| row.cvar), Some(HIDE_CVAR));
    }

    #[test]
    fn the_tab_is_last_so_the_other_tabs_keep_their_numbers() {
        assert_eq!(TABS[QUICK_TAB], FIRST_SETUP_CAPTION);
        assert_eq!(QUICK_TAB, TABS.len() - 1);
    }
}
