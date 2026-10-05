//! What each setting does, in the player's words, for the classic+ option
//! panels' detail box (see `docs/classic-plus.md`), as retail's `descText`
//! said what each `setup.menu` item did. Keyed by cvar; every row of the
//! catalogue has one. When a setting takes effect comes from its label's
//! "(restart)" or "(next map)", which the classic+ rows show as a mark.

/// Lines the detail box gives a description, and characters per line.
pub(super) const HELP_LINES: usize = 2;
pub(super) const HELP_LINE_CHARS: usize = 68;

/// When a changed setting takes effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Timing {
    Now,
    Restart,
    NextMap,
}

impl Timing {
    /// The detail box's words for it.
    pub(super) fn note(self) -> Option<&'static str> {
        match self {
            Self::Now => None,
            Self::Restart => Some("applies after a restart"),
            Self::NextMap => Some("applies on the next map"),
        }
    }
}

/// `label` without its timing note, and the timing it gave.
pub(super) fn timing(label: &str) -> (&str, Timing) {
    for (suffix, timing) in [
        (" (restart)", Timing::Restart),
        (" (next map)", Timing::NextMap),
    ] {
        if let Some(stem) = label.strip_suffix(suffix) {
            return (stem, timing);
        }
    }
    (label, Timing::Now)
}

/// What a classic+ row calls a setting: its label without the timing note
/// and without a trailing note in brackets ("Dynamic glow (0 off, 2 sabers)"
/// is "Dynamic glow"), which the retail label column has no room for; the
/// detail box shows the whole label and explains the values.
pub(super) fn row_label(label: &str) -> (&str, Timing) {
    let (label, timing) = timing(label);
    let short = match label.find(" (") {
        Some(at) if label.ends_with(')') && at > 0 => &label[..at],
        _ => label,
    };
    (short, timing)
}

/// Longest row label, in characters, that fits the in-game pop-up's label
/// column.
const ROW_LABEL_CHARS: usize = 23;

/// Classic row names of settings whose label is longer than the label
/// column; the detail box still shows the whole label.
const SHORT_LABELS: &[(&str, &str)] = &[
    ("cg_drawFps", "FPS readout"),
    ("cg_forceModel", "Everyone as my model"),
    ("cg_thirdPersonCameraDamp", "Camera damping"),
    ("cg_thirdPersonTargetDamp", "Target damping"),
    ("cg_errorDecay", "Error smoothing"),
    (crate::frame_target::aa::exposure::MIN_EV, "Max darken, EV"),
    (
        crate::frame_target::aa::exposure::MAX_EV,
        "Max brighten, EV",
    ),
    ("r_modelPixelLight", "Pixel model lighting"),
    ("r_ambientFillOcclusion", "Ambient fill shading"),
];

/// What a classic+ row calls setting `cvar` labelled `label`: its short name
/// if it has one, else [`row_label`].
pub(super) fn classic_label<'a>(cvar: &str, label: &'a str) -> (&'a str, Timing) {
    let (row, timing) = row_label(label);
    let short = SHORT_LABELS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(cvar))
        .map_or(row, |(_, short)| *short);
    (short, timing)
}

/// The description of setting `cvar`.
pub(super) fn help(cvar: &str) -> Option<&'static str> {
    HELP.iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(cvar))
        .map(|(_, text)| *text)
}

/// `text` cut into at most [`HELP_LINES`] lines of [`HELP_LINE_CHARS`]
/// characters at spaces; the last line takes what is left.
pub(super) fn lines(text: &str) -> [&str; HELP_LINES] {
    let mut out = [""; HELP_LINES];
    let mut rest = text.trim();
    for (index, line) in out.iter_mut().enumerate() {
        if rest.chars().count() <= HELP_LINE_CHARS || index + 1 == HELP_LINES {
            *line = rest;
            break;
        }
        let limit = rest
            .char_indices()
            .nth(HELP_LINE_CHARS)
            .map_or(rest.len(), |(at, _)| at);
        let cut = rest[..limit].rfind(' ').unwrap_or(limit);
        *line = rest[..cut].trim_end();
        rest = rest[cut..].trim_start();
    }
    out
}

const HELP: &[(&str, &str)] = &[
    // VIDEO
    (
        "r_resolution",
        "The window's size, or the screen mode in exclusive fullscreen. Enter opens the list of sizes.",
    ),
    (
        "r_fullscreen",
        "Windowed, borderless (fills the desktop) or exclusive fullscreen (sets the monitor's mode).",
    ),
    (
        "r_vsync",
        "Waits for the monitor's refresh before showing a frame: no tearing, a little more input delay.",
    ),
    (
        "com_maxfps",
        "Most frames drawn per second. AUTO follows the monitor's refresh rate; 0 draws as fast as it can.",
    ),
    (
        "cg_fov",
        "How wide you see, in degrees across the screen: higher shows more at the sides.",
    ),
    (
        "cg_marks",
        "Scorch and blaster marks left on walls and floors by shots and explosions.",
    ),
    ("cg_shadows", "Players' shadows on the ground under them."),
    (
        "cg_drawGun",
        "Shows your weapon in first person; off hides it for a clear view.",
    ),
    (
        "cg_drawFps",
        "Shows frames per second and frame time in a corner of the screen.",
    ),
    (
        "r_gamma",
        "Brightness curve of the whole display, HUD included. 1 is neutral; higher lifts dark areas.",
    ),
    // AUDIO
    ("s_volume", "Loudness of every sound but the music."),
    ("s_musicVolume", "Loudness of the music."),
    (
        "s_doppler",
        "Moving looping sounds, such as passing ships, change pitch as they come and go.",
    ),
    ("cg_footsteps", "Footstep sounds, yours and other players'."),
    // HUD
    (
        "cg_drawHud",
        "Shows the HUD; off hides it, for screenshots and recordings.",
    ),
    ("cg_hudScale", "Size of the HUD: higher is larger."),
    ("cg_drawStatus", "Health, armor and Force on the HUD."),
    (
        "cg_drawWeapon",
        "The weapon bar shown while you change weapons.",
    ),
    ("cg_crosshair", "The crosshair at the centre of the screen."),
    (
        "cg_drawCrosshairNames",
        "Shows the name of the player under your crosshair.",
    ),
    ("cg_drawTimer", "Shows the match time."),
    (
        "cg_lagometer",
        "A small graph of interpolation and snapshot delay, for spotting network trouble.",
    ),
    ("cg_drawChat", "Shows chat messages over the game."),
    (
        crate::menu_hud::STYLE_CVAR,
        "Which HUD shows health, armor, Force and ammo: SJK's modern or classic layout, or the game's own.",
    ),
    (
        crate::menu_hud::FILES_CVAR,
        "The game HUD's menu list (ui/jahud.txt); 1 is the text-only HUD, 3 and 4 EternalJK's HUDs.",
    ),
    (
        "cg_classicHudFont",
        "Draws the HUD's numbers and text in the retail bitmap font.",
    ),
    (
        crate::game_font::CVAR,
        "Draws the menus and console in the original game's fonts instead of SJK's.",
    ),
    (
        crate::ground_hud::CVAR,
        "In third person, shows health, armor and Force on the ground around your character.",
    ),
    // CONTROLS
    (
        "sensitivity",
        "How far the view turns for a given mouse movement.",
    ),
    (
        "m_invert",
        "Moving the mouse forward looks down instead of up.",
    ),
    ("cl_run", "Run by default; the Walk key walks."),
    // GAME
    (
        "cg_simpleItems",
        "Shows pickups as flat icons instead of 3D models, easier to spot from afar.",
    ),
    (
        "cg_forceModel",
        "Draws every player with your own model and skin.",
    ),
    (
        "cg_saberTrail",
        "The light trail behind a swinging saber blade.",
    ),
    (
        "cg_speedTrail",
        "Afterimages behind players using Force Speed.",
    ),
    (
        "cg_auraShell",
        "The glowing shell Force Sight shows around other players.",
    ),
    (
        crate::shader_remaps::CVAR,
        "Lets servers and maps swap textures: 0 never, 1 everywhere but player models, 2 always.",
    ),
    (
        "cg_thirdPersonCameraDamp",
        "How loosely the third-person camera follows you: lower is smoother, 1 sticks to you.",
    ),
    (
        "cg_thirdPersonTargetDamp",
        "How loosely the third-person camera turns where you aim: lower is smoother, 1 is instant.",
    ),
    (
        "cg_errorDecay",
        "Smooths the view over this many milliseconds when the server corrects you; 0 snaps.",
    ),
    ("ui_accent", "Accent colour of the modern menus."),
    (
        "ui_menuContrast",
        "Darkens the backdrop behind the modern menus for easier reading.",
    ),
    (
        crate::menu::style::CVAR,
        "Classic menus after the original game's, or SJK's modern ones.",
    ),
    // NETWORK
    (
        "cl_master",
        "The master server the server browser asks for its list of servers.",
    ),
    (
        "rate",
        "Most data per second the server may send you; raise it on a fast connection.",
    ),
    (
        "snaps",
        "World updates per second asked of the server; it sends at most its own rate.",
    ),
    // HUD+
    ("cg_crosshairSize", "Size of the crosshair."),
    (
        "cg_drawTeamOverlay",
        "In team games, your teammates with their health and where they are.",
    ),
    ("cg_speedometer", "Shows how fast you move."),
    (
        crate::scoreboard::style::CVAR,
        "SJK's modern scoreboard, or the classic one after the original game's.",
    ),
    (
        "cg_showClientIDs",
        "Shows each player's client number on the scoreboard.",
    ),
    (
        "cg_drawScoreboardIcons",
        "Shows each player's head icon on the scoreboard.",
    ),
    (
        "cg_smallScoreboard",
        "Smaller scoreboard rows, so more players fit.",
    ),
    // TEXT
    (
        crate::console::console_options::STYLE_CVAR,
        "The console's look: classic, after EternalJK's, or SJK's modern one.",
    ),
    (crate::text::style::SCALE_CVAR, "Size of the menu text."),
    (
        crate::text::style::TRACKING_CVAR,
        "Extra space between letters in the menus and the console.",
    ),
    ("con_scale", "Size of the console text."),
    (
        "con_lineSpacing",
        "Space between the modern console's lines.",
    ),
    // IMAGE
    (
        "r_sceneHdr",
        "Renders the scene in high dynamic range: brighter lights, smoother bright areas.",
    ),
    (
        "r_hdrExposure",
        "How bright the scene is drawn; eye adaptation works around this value.",
    ),
    (
        crate::frame_target::aa::exposure::ENABLED,
        "Brightness follows what you look at, as eyes adapt between dark rooms and bright sky.",
    ),
    (
        crate::frame_target::aa::exposure::MIN_EV,
        "How far eye adaptation may darken a bright view, in exposure steps (HDR only).",
    ),
    (
        crate::frame_target::aa::exposure::MAX_EV,
        "How far eye adaptation may brighten a dark view, in exposure steps.",
    ),
    (
        "r_toneCurve",
        "A filmic curve that softens the brightest parts of the picture.",
    ),
    (
        "r_sceneBloom",
        "Bright lights bleed a soft glow around them. The HUD and menus never bloom.",
    ),
    (
        "r_DynamicGlow",
        "The glow around sabers and glowing surfaces: 0 off, 1 everything that glows, 2 sabers only.",
    ),
    (
        "r_dynamicGlowStyle",
        "How the glow is blurred: 0 like the original renderer, 1 like EternalJK's Vulkan one.",
    ),
    ("r_fxaa", "Smooths jagged edges with a light filter."),
    (
        "r_superSample",
        "Draws the scene 2 or 3 times larger each way and shrinks it: very sharp, very costly.",
    ),
    (
        "r_softParticles",
        "Smoke and dust fade where they meet walls instead of cutting into them.",
    ),
    (
        crate::dust_motes::CVAR,
        "Dust drifting in sunbeams; needs light shafts. 0 is off.",
    ),
    (
        "r_modelPixelLight",
        "Lights models at every pixel rather than once each, so they match the light around them.",
    ),
    (
        "r_cubeMapping",
        "Captures each map's surroundings so metal and shiny surfaces reflect their room.",
    ),
    (
        "r_floorReflections",
        "Mirror-like reflections on polished floors.",
    ),
    (
        "r_emissiveMaps",
        "Emission maps, which make lamps, screens and signs glow on their own.",
    ),
    (
        "r_emissionStrength",
        "Brightness of emission maps: 1 as made, 0 off.",
    ),
    (
        "r_emissiveGlow",
        "A glow halo around emission-mapped surfaces; needs dynamic glow.",
    ),
    // LIGHTING
    (
        "r_dayNight",
        "Lights maps with SJK's sun and sky as well as their own baked light.",
    ),
    (
        "r_liveLighting",
        "How much light is computed live: 0 keeps the map's baked bounce light, 2 computes it all.",
    ),
    (
        "r_dayHour",
        "The hour of the sun, 0 to 24, when the sun and sky are on.",
    ),
    (
        "r_dayMinutes",
        "Real minutes a whole day lasts; 0 holds the hour still.",
    ),
    (
        "r_dayBrightness",
        "Brightness of the live lighting, relative to the sun.",
    ),
    (
        "r_ambientFill",
        "A faint light in very dark places so they stay readable; 0 is off.",
    ),
    (
        "r_ambientFillOcclusion",
        "How much that fill darkens in corners: 1 as designed, 0 not at all.",
    ),
    (
        "r_indirectBoost",
        "Brightness of bounced and sky light; direct light stays as it is. 1 as designed.",
    ),
    (
        crate::world_materials::material_maps::lights::CVAR,
        "Lets emission-mapped surfaces light their surroundings in live lighting; 0 is off.",
    ),
    (
        "r_volumetrics",
        "Light shafts through the air: 0 off, higher levels finer and more costly.",
    ),
    (
        "r_volumetricClarity",
        "1 keeps sunlit air clear and shows only the shafts; 0 adds an even haze.",
    ),
    // SHADOWS
    (
        "r_worldSunShadows",
        "The sun casts shadows from walls and buildings.",
    ),
    (
        "r_actorSunShadows",
        "Players and NPCs cast sun shadows too.",
    ),
    (
        "r_sunShadowResolution",
        "Detail of the sun's shadows: higher is sharper and takes more video memory.",
    ),
    (
        "r_sunShadowDistance",
        "How far from you sun shadows stay sharp; a coarser map covers the rest of the level.",
    ),
    (
        "r_sunShadowNear",
        "Range of the finest shadows, close around you.",
    ),
    (
        "r_sunShadowTaps",
        "Samples taken along a shadow's edge: more is smoother and more costly.",
    ),
    (
        "r_sunShadowGapClose",
        "Fills light leaking through slits narrower than this many units; 0 is off.",
    ),
    (
        "r_contactShadows",
        "Small shadows where things touch the ground, toward the sun. Still being tuned.",
    ),
];

#[cfg(test)]
mod tests {
    use super::super::catalog::*;
    use super::*;

    fn every_row() -> impl Iterator<Item = &'static Setting> {
        let general = (0..TABS.len()).flat_map(super::super::settings);
        let renderer = [RENDER_IMAGE, RENDER_LIGHTING, RENDER_SHADOWS]
            .into_iter()
            .flatten();
        general.chain(renderer)
    }

    #[test]
    fn every_setting_says_what_it_does_in_two_lines() {
        for setting in every_row() {
            let text =
                help(setting.cvar).unwrap_or_else(|| panic!("{} has no description", setting.cvar));
            let lines = lines(text);
            assert!(
                lines
                    .iter()
                    .all(|line| line.chars().count() <= HELP_LINE_CHARS),
                "{}: {lines:?}",
                setting.cvar
            );
            assert!(
                text.is_ascii(),
                "{}: the menu font draws bytes",
                setting.cvar
            );
        }
        // And nothing describes a setting no row shows.
        for (cvar, _) in HELP {
            assert!(
                every_row().any(|setting| setting.cvar.eq_ignore_ascii_case(cvar)),
                "{cvar} is described but not on any page"
            );
        }
    }

    #[test]
    fn every_row_label_fits_the_label_column() {
        for setting in every_row() {
            let (label, _) = classic_label(setting.cvar, setting.label);
            assert!(
                label.chars().count() <= ROW_LABEL_CHARS,
                "{}: {label}",
                setting.cvar
            );
        }
        for (cvar, _) in SHORT_LABELS {
            assert!(
                every_row().any(|setting| setting.cvar.eq_ignore_ascii_case(cvar)),
                "{cvar} has a short label but no row"
            );
        }
        // A short name keeps the timing of the full label.
        assert_eq!(
            classic_label("cg_errorDecay", "Prediction error smoothing (restart)"),
            ("Error smoothing", Timing::Restart)
        );
    }

    #[test]
    fn lines_break_at_spaces() {
        let text = "Brightness curve of the whole display, HUD included. 1 is neutral; higher lifts dark areas.";
        let [first, second] = lines(text);
        assert!(first.len() <= HELP_LINE_CHARS && !first.ends_with(' '));
        assert_eq!(format!("{first} {second}"), text);
        assert_eq!(lines("Short."), ["Short.", ""]);
    }

    #[test]
    fn labels_lose_their_timing_note() {
        assert_eq!(
            timing("HDR scene (restart)"),
            ("HDR scene", Timing::Restart)
        );
        assert_eq!(
            timing("Live lighting 0-2 (next map)"),
            ("Live lighting 0-2", Timing::NextMap)
        );
        assert_eq!(timing("Bloom"), ("Bloom", Timing::Now));
        assert_eq!(
            timing("Dynamic glow (0 off, 2 sabers)"),
            ("Dynamic glow (0 off, 2 sabers)", Timing::Now)
        );
        // Rows also drop a note in brackets.
        assert_eq!(
            row_label("Dynamic glow (0 off, 2 sabers)"),
            ("Dynamic glow", Timing::Now)
        );
        assert_eq!(
            row_label("Light shafts 0-3 (restart)"),
            ("Light shafts 0-3", Timing::Restart)
        );
        assert_eq!(
            row_label("Adaptation: max darken, EV (HDR)"),
            ("Adaptation: max darken, EV", Timing::Now)
        );
        assert_eq!(row_label("FPS cap (AUTO = monitor, 0 = off)").0, "FPS cap");
        assert_eq!(
            row_label("Supersampling, 1 off (restart)").0,
            "Supersampling, 1 off"
        );
    }
}
