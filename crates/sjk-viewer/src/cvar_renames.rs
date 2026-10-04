//! SJK's names for JKR's `jkr_*` client cvars and commands.
//!
//! SJK gives its settings neutral engine names (`r_*`, `cg_*`, `cl_*`, `com_*`)
//! instead of JKR's `jkr_*`. Each old name stays a registry alias of the new one,
//! so a JKR `config.cfg`, a typed `jkr_*` command or upstream code that still
//! says `jkr_*` reaches the same variable; the config is saved under the new
//! names. Names that rend2 or EternalJK already use with another meaning
//! (`r_hdr`, `r_toneMap`, `r_bloom`, `r_renderScale`, `r_sunShadows`) are avoided,
//! so an exec'd EternalJK config cannot change these.

use sjk_shell::{CvarError, CvarRegistry};

/// `(JKR name, SJK name)` for every renamed client cvar.
pub(crate) const RENAMED: &[(&str, &str)] = &[
    ("jkr_hdr", "r_sceneHdr"),
    ("jkr_hdrExposure", "r_hdrExposure"),
    ("jkr_tonemap", "r_toneCurve"),
    ("jkr_bloom", "r_sceneBloom"),
    ("jkr_fxaa", "r_fxaa"),
    ("jkr_renderScale", "r_superSample"),
    ("jkr_softParticles", "r_softParticles"),
    ("jkr_modelDiffusePixels", "r_modelPixelLight"),
    ("jkr_sunShadows", "r_actorSunShadows"),
    ("jkr_worldSunShadows", "r_worldSunShadows"),
    ("jkr_shadowDistance", "r_sunShadowDistance"),
    ("jkr_shadowNear", "r_sunShadowNear"),
    ("jkr_shadowResolution", "r_sunShadowResolution"),
    ("jkr_shadowTaps", "r_sunShadowTaps"),
    ("jkr_shadowGapClose", "r_sunShadowGapClose"),
    ("jkr_contactShadows", "r_contactShadows"),
    ("jkr_volumetrics", "r_volumetrics"),
    ("jkr_volumetricClarity", "r_volumetricClarity"),
    ("jkr_dayNight", "r_dayNight"),
    ("jkr_dayHour", "r_dayHour"),
    ("jkr_dayMinutes", "r_dayMinutes"),
    ("jkr_dayBrightness", "r_dayBrightness"),
    ("jkr_dayDebug", "r_dayDebug"),
    ("jkr_ambientFill", "r_ambientFill"),
    ("jkr_ambientFillOcclusion", "r_ambientFillOcclusion"),
    ("jkr_indirectBoost", "r_indirectBoost"),
    ("jkr_realtime", "r_liveLighting"),
    ("jkr_dust", "r_dustMotes"),
    ("jkr_exclusiveFullscreen", "r_exclusiveFullscreen"),
    ("jkr_groundHud", "cg_groundHud"),
    ("jkr_bindDefaultsVersion", "cl_bindDefaultsVersion"),
    ("jkr_sensitivityScaleVersion", "cl_sensitivityScaleVersion"),
    ("jkr_maxfpsDefaultVersion", "com_maxfpsDefaultVersion"),
];

/// `(JKR name, SJK name)` for the renamed demo-director commands.
pub(crate) const RENAMED_COMMANDS: &[(&str, &str)] =
    &[("jkr_camera", "demo_camera"), ("jkr_sun", "demo_sun")];

/// Make every old name an alias of its new one. Call once every cvar is
/// registered and before the configuration is loaded; a new name that is not
/// registered (a setting this build lacks) is skipped.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), CvarError> {
    for &(old, new) in RENAMED {
        if cvars.get(new).is_some() {
            cvars.register_alias(old, new)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::ViewerConsole;

    #[test]
    fn every_jkr_name_reaches_its_sjk_cvar() {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        for &(old, new) in RENAMED {
            let value = console.cvar(new);
            assert!(value.is_some(), "{new} is not registered");
            assert_eq!(console.cvar(old), value, "{old} does not reach {new}");
        }
    }

    #[test]
    fn no_registered_cvar_keeps_a_jkr_name() {
        // A new upstream `jkr_*` cvar fails here: give it a neutral name in
        // `RENAMED` and keep the old one as an alias.
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let left: Vec<_> = console
            .cvar_names()
            .filter(|name| name.to_ascii_lowercase().starts_with("jkr_"))
            .collect();
        assert!(
            left.is_empty(),
            "jkr_ cvars without a neutral name: {left:?}"
        );
    }

    #[test]
    fn a_jkr_config_is_read_and_saved_under_sjk_names() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.cfg");
        std::fs::write(&path, "seta jkr_hdr \"0\"\nseta jkr_dayHour \"15.5\"\n").unwrap();
        {
            let console = ViewerConsole::new(path.clone()).unwrap();
            assert_eq!(console.integer_cvar("r_sceneHdr"), Some(0));
            assert_eq!(console.float_cvar("r_dayHour"), Some(15.5));
        }
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("seta r_sceneHdr \"0\""), "{saved}");
        assert!(saved.contains("r_dayHour"), "{saved}");
        assert!(!saved.contains("jkr_"), "{saved}");
    }

    #[test]
    fn renames_are_distinct_and_leave_jkr() {
        let mut seen = std::collections::HashSet::new();
        for &(old, new) in RENAMED.iter().chain(RENAMED_COMMANDS) {
            assert!(old.starts_with("jkr_"), "{old}");
            assert!(!new.to_ascii_lowercase().starts_with("jkr_"), "{new}");
            assert!(seen.insert(old.to_ascii_lowercase()), "{old} twice");
            assert!(seen.insert(new.to_ascii_lowercase()), "{new} twice");
        }
    }
}
