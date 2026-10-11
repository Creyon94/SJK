//! Projection from persisted shell cvars into the multiplayer profile.

use super::ViewerConsole;
use sjk_client::PlayerProfile;
use sjk_network::LegacyUserInfo;
use std::error::Error;

/// The lowest rate a server takes (`sv_minRate`'s floor in `SV_UserinfoChanged`).
const MIN_RATE: i64 = 1000;
/// The most snapshots a second asked for; servers send at most `sv_fps`.
const MAX_SNAPS: i64 = 1000;

impl ViewerConsole {
    /// Return the legacy connection settings backed by typed cvars.
    pub(crate) fn userinfo(&self) -> Result<LegacyUserInfo, Box<dyn Error>> {
        let profile = self.player_profile()?;
        let (rate, snaps) = self.rate_and_snaps();
        let password = self
            .text_cvar("password")
            .ok()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        let mut userinfo = profile.legacy_userinfo(
            rate,
            snaps,
            self.bool_cvar("cg_predictItems").unwrap_or(true),
            password,
        );
        userinfo.plugin_disable = self
            .integer_cvar("cp_pluginDisable")
            .and_then(|bits| u32::try_from(bits).ok());
        self.force_profile.apply_to_userinfo(&mut userinfo);
        Ok(userinfo)
    }

    /// `rate` and `snaps` as sent: clamped to what a server accepts (OpenJK's
    /// `SV_UserinfoChanged` raises a rate under 1000 and snaps under 1) instead of
    /// refused, since a refused value (`rate 0`, `snaps 0` typed in the console)
    /// stopped every join and every later userinfo update, name changes included.
    fn rate_and_snaps(&self) -> (u32, u16) {
        let rate = self
            .integer_cvar("rate")
            .unwrap_or(25_000)
            .clamp(MIN_RATE, i64::from(u32::MAX)) as u32;
        let snaps = self.integer_cvar("snaps").unwrap_or(40).clamp(1, MAX_SNAPS) as u16;
        (rate, snaps)
    }

    fn player_profile(&self) -> Result<PlayerProfile, Box<dyn Error>> {
        PlayerProfile::from_cvar_values(
            self.text_cvar("name")?,
            self.text_cvar("model")?,
            self.text_cvar("sex")?,
            self.text_cvar("saber1")?,
            self.text_cvar("saber2")?,
            self.text_value("color1").unwrap_or("4"),
            self.text_value("color2").unwrap_or("4"),
            [
                self.integer_cvar("cp_sbRGB1").unwrap_or(0),
                self.integer_cvar("cp_sbRGB2").unwrap_or(0),
            ],
            [
                self.integer_cvar("char_color_red").unwrap_or(255),
                self.integer_cvar("char_color_green").unwrap_or(255),
                self.integer_cvar("char_color_blue").unwrap_or(255),
            ],
            self.text_cvar("forcepowers")?,
            self.integer_cvar("handicap").unwrap_or(100),
        )
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_or_huge_rate_and_snaps_are_clamped_not_refused() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(console.rate_and_snaps(), (25_000, 120));
        console.set_cvar("rate", "0");
        console.set_cvar("snaps", "0");
        assert_eq!(console.rate_and_snaps(), (1_000, 1));
        assert!(console.userinfo().is_ok(), "joining still works");
        console.set_cvar("snaps", "70000");
        console.set_cvar("rate", "-5");
        assert_eq!(console.rate_and_snaps(), (1_000, 1_000));
    }
}
