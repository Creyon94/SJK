//! Declarative settings catalog shared by the settings screen.
//!

//! Every row here is backed by a consumer: a cvar that nothing reads yet is
//! not offered. Player identity
//! (name, model, sabers) lives in the Player menu, not here.

pub(super) const TABS: [&str; 9] = [
    "VIDEO", "AUDIO", "HUD", "CONTROLS", "GAME", "NETWORK", "HUD+", "TEXT", "QUICK",
];
/// The first-start tab (`settings/quick.rs`); last, so the other tabs keep their numbers.
pub(super) const QUICK_TAB: usize = 8;
/// The tab whose last row opens the key-binding editor.
pub(super) const KEYBINDS_TAB: usize = 3;
/// The tab whose last row opens the renderer settings ([`RENDERER_TABS`]).
pub(super) const RENDERER_TAB: usize = 0;
/// Tabs of the renderer settings, JKR's own rendering cvars (`jkr_*` in JKR), reached
/// from the last row of [`VIDEO`] as JoF EJK reaches its advanced renderer page
/// from Video.
pub(super) const RENDERER_TABS: [&str; 3] = ["IMAGE", "LIGHTING", "SHADOWS"];

#[derive(Clone, Copy)]
pub(super) enum ValueKind {
    /// On/off; an integer cvar reads nonzero as on and is written 0 or 1.
    Bool,
    Integer {
        min: i64,
        max: i64,
        step: i64,
    },
    Float {
        min: f64,
        max: f64,
        step: f64,
    },
    Choice(&'static [&'static str]),
    Text,
    /// `r_resolution`: steps within the aspect group, Enter opens the list.
    Resolution,
    /// `r_fullscreen` with `r_exclusiveFullscreen`, as named modes.
    DisplayMode,
    /// The HUD in use (`cg_hudStyle`, `cg_hudFiles`, `cg_hudPack`): steps
    /// through the HUDs, Enter opens the HUD picker with its previews.
    HudPicker,
}

#[derive(Clone, Copy)]
pub(super) struct Setting {
    pub(super) label: &'static str,
    pub(super) cvar: &'static str,
    pub(super) kind: ValueKind,
}

pub(crate) const RESOLUTIONS: &[&str] = &[
    "1280x720",
    "1600x900",
    "1920x1080",
    "2560x1440",
    "3840x2160",
];
pub(super) const VIDEO: &[Setting] = &[
    Setting {
        label: "Resolution",
        cvar: "r_resolution",
        kind: ValueKind::Resolution,
    },
    Setting {
        label: "Display mode",
        cvar: "r_fullscreen",
        kind: ValueKind::DisplayMode,
    },
    Setting {
        label: "Vertical sync",
        cvar: "r_vsync",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "FPS cap (AUTO = monitor, 0 = off)",
        cvar: "com_maxfps",
        kind: ValueKind::Integer {
            min: -1,
            max: 2000,
            step: 25,
        },
    },
    Setting {
        label: "Field of view",
        cvar: "cg_fov",
        kind: ValueKind::Float {
            min: 70.0,
            max: 130.0,
            step: 5.0,
        },
    },
    Setting {
        label: "Impact marks",
        cvar: "cg_marks",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Player shadows",
        cvar: "cg_shadows",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "First-person weapon",
        cvar: "cg_drawGun",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "FPS / frame-time readout",
        cvar: "cg_drawFps",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Display gamma",
        cvar: "r_gamma",
        kind: ValueKind::Float {
            min: 0.5,
            max: 3.0,
            step: 0.1,
        },
    },
];
pub(super) const AUDIO: &[Setting] = &[
    Setting {
        label: "Effects volume",
        cvar: "s_volume",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Music volume",
        cvar: "s_musicVolume",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Doppler",
        cvar: "s_doppler",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Footsteps",
        cvar: "cg_footsteps",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Mute in background",
        cvar: "snd_mute_losefocus",
        kind: ValueKind::Bool,
    },
];
pub(super) const HUD_OPTIONS: &[Setting] = &[
    Setting {
        label: "Crosshair size",
        cvar: "cg_crosshairSize",
        kind: ValueKind::Float {
            min: 0.0,
            max: 96.0,
            step: 4.0,
        },
    },
    Setting {
        label: "Team status",
        cvar: "cg_drawTeamOverlay",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Speedometer",
        cvar: "cg_speedometer",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Scoreboard style",
        cvar: crate::scoreboard::style::CVAR,
        kind: ValueKind::Choice(&crate::scoreboard::style::ScoreboardStyle::NAMES),
    },
    Setting {
        label: "Scoreboard client IDs",
        cvar: "cg_showClientIDs",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Scoreboard head icons",
        cvar: "cg_drawScoreboardIcons",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Small scoreboard rows",
        cvar: "cg_smallScoreboard",
        kind: ValueKind::Bool,
    },
];

pub(super) const HUD: &[Setting] = &[
    Setting {
        label: "HUD",
        cvar: "cg_drawHud",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HUD scale",
        cvar: "cg_hudScale",
        kind: ValueKind::Float {
            min: 0.5,
            max: 1.5,
            step: 0.05,
        },
    },
    Setting {
        label: "Status (health / armour / force)",
        cvar: "cg_drawStatus",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Weapon bar",
        cvar: "cg_drawWeapon",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Crosshair (0 off)",
        cvar: "cg_crosshair",
        kind: ValueKind::Integer {
            min: 0,
            max: 10,
            step: 1,
        },
    },
    Setting {
        label: "Crosshair names",
        cvar: "cg_drawCrosshairNames",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Overhead names",
        cvar: "cg_drawPlayerNames",
        kind: ValueKind::Integer {
            min: 0,
            max: 2,
            step: 1,
        },
    },
    Setting {
        label: "Name size",
        cvar: "cg_drawPlayerNamesScale",
        kind: ValueKind::Float {
            min: 0.2,
            max: 1.5,
            step: 0.1,
        },
    },
    Setting {
        label: "Ally markers",
        cvar: "cg_drawFriend",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Player card",
        cvar: "cg_playerCard",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Card delay (s)",
        cvar: "cg_playerCardDelay",
        kind: ValueKind::Float {
            min: 0.5,
            max: 5.0,
            step: 0.5,
        },
    },
    Setting {
        label: "Nameplates",
        cvar: "cg_nameplate",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Nameplate range",
        cvar: "cg_nameplateRange",
        kind: ValueKind::Integer {
            min: 500,
            max: 10000,
            step: 250,
        },
    },
    Setting {
        label: "Bars distance",
        cvar: "cg_nameplateNear",
        kind: ValueKind::Integer {
            min: 0,
            max: 5000,
            step: 100,
        },
    },
    Setting {
        label: "Nameplate size",
        cvar: "cg_nameplateScale",
        kind: ValueKind::Float {
            min: 0.2,
            max: 1.5,
            step: 0.1,
        },
    },
    Setting {
        label: "Nameplate bars",
        cvar: "cg_nameplateBars",
        kind: ValueKind::Integer {
            min: 0,
            max: 2,
            step: 1,
        },
    },
    Setting {
        label: "Power icons",
        cvar: "cg_nameplateIcons",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Force bar (estimated)",
        cvar: "cg_nameplateForce",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Health estimate",
        cvar: "cg_nameplatePredict",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Weapon icon",
        cvar: "cg_nameplateWeapon",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Plates through walls",
        cvar: "cg_nameplateWalls",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "NPC nameplates",
        cvar: "cg_nameplateNpcs",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Match timer",
        cvar: "cg_drawTimer",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Version and date",
        cvar: crate::version_overlay::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Lagometer",
        cvar: "cg_lagometer",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Chat",
        cvar: "cg_drawChat",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HUD look (Enter: pick)",
        cvar: crate::menu_hud::STYLE_CVAR,
        kind: ValueKind::HudPicker,
    },
    Setting {
        label: "Game HUD files",
        cvar: crate::menu_hud::FILES_CVAR,
        kind: ValueKind::Text,
    },
    Setting {
        label: "Classic HUD font",
        cvar: "cg_classicHudFont",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Classic game fonts",
        cvar: crate::game_font::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Ground HUD (third person)",
        cvar: crate::ground_hud::CVAR,
        kind: ValueKind::Bool,
    },
];
pub(super) const CONTROLS: &[Setting] = &[
    Setting {
        label: "Mouse sensitivity",
        cvar: "sensitivity",
        kind: ValueKind::Float {
            min: 0.1,
            max: 20.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Invert mouse",
        cvar: "m_invert",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Always run",
        cvar: "cl_run",
        kind: ValueKind::Bool,
    },
];
pub(super) const GAME: &[Setting] = &[
    Setting {
        label: "Simple pickup icons",
        cvar: "cg_simpleItems",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show everyone as my model",
        cvar: "cg_forceModel",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Saber trail",
        cvar: "cg_saberTrail",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Force Speed trail",
        cvar: "cg_speedTrail",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Shield hit sphere",
        cvar: "cg_shieldSphere",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Shield hit brightness",
        cvar: "cg_shieldBrightness",
        kind: ValueKind::Integer {
            min: 1,
            max: 12,
            step: 1,
        },
    },
    Setting {
        label: "Force Seeing aura",
        cvar: "cg_auraShell",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Protect+Absorb combo",
        cvar: "cg_spProtAbsColor",
        kind: ValueKind::Bool,
    },
    Setting {
        // SJK defaults to 2 (EternalJK); 1 excludes player-texture configstring remaps.
        label: "Shader remaps (0 off / 1 map / 2 all)",
        cvar: "cg_remaps",
        kind: ValueKind::Integer {
            min: 0,
            max: 2,
            step: 1,
        },
    },
    Setting {
        label: "Third-person camera damping",
        cvar: "cg_thirdPersonCameraDamp",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Third-person target damping",
        cvar: "cg_thirdPersonTargetDamp",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Prediction error smoothing (ms)",
        cvar: "cg_errorDecay",
        kind: ValueKind::Float {
            min: 0.0,
            max: 500.0,
            step: 25.0,
        },
    },
    Setting {
        label: "Menu accent",
        cvar: "ui_accent",
        kind: ValueKind::Choice(&["ember", "amber", "blue", "green", "violet", "neutral"]),
    },
    Setting {
        label: "Menu contrast",
        cvar: "ui_menuContrast",
        kind: ValueKind::Choice(&["off", "standard", "strong"]),
    },
    Setting {
        label: "Menu style",
        cvar: crate::menu::style::CVAR,
        kind: ValueKind::Choice(&crate::menu::style::MenuStyle::NAMES),
    },
];
pub(super) const NETWORK: &[Setting] = &[
    Setting {
        label: "Master server",
        cvar: "cl_master",
        kind: ValueKind::Text,
    },
    Setting {
        label: "Check for updates",
        cvar: "cl_autoUpdate",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK identity",
        cvar: "cl_identity",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK hub",
        cvar: "cl_hubUrl",
        kind: ValueKind::Text,
    },
    Setting {
        label: "Rate (bytes/s)",
        cvar: "rate",
        kind: ValueKind::Integer {
            min: 1000,
            max: 100000,
            step: 1000,
        },
    },
    Setting {
        label: "Snapshots per second",
        cvar: "snaps",
        kind: ValueKind::Integer {
            min: 10,
            max: 125,
            step: 5,
        },
    },
];
/// Text size and spacing, and the console's style. Menu rows keep their layout;
/// modern console rows follow `con_lineSpacing`.
pub(super) const TEXT: &[Setting] = &[
    Setting {
        label: "Console style",
        cvar: crate::console::console_options::STYLE_CVAR,
        kind: ValueKind::Choice(&crate::console::console_options::ConsoleStyle::NAMES),
    },
    Setting {
        label: "Menu text size",
        cvar: crate::text::style::SCALE_CVAR,
        kind: ValueKind::Float {
            min: 0.8,
            max: 1.2,
            step: 0.05,
        },
    },
    Setting {
        label: "Letter spacing (menus, console)",
        cvar: crate::text::style::TRACKING_CVAR,
        kind: ValueKind::Float {
            min: -0.05,
            max: 0.15,
            step: 0.01,
        },
    },
    Setting {
        label: "Console text size",
        cvar: "con_scale",
        kind: ValueKind::Float {
            min: 0.5,
            max: 2.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Console line spacing",
        cvar: "con_lineSpacing",
        kind: ValueKind::Float {
            min: 0.8,
            max: 2.0,
            step: 0.05,
        },
    },
];

// Renderer settings. "(restart)" marks cvars the renderer reads only at startup
// (their registrations print a restart notice on change); "(next map)" those read
// when a map loads. The rest apply immediately. Ranges follow each consumer's clamp.

pub(super) const RENDER_IMAGE: &[Setting] = &[
    Setting {
        label: "HDR scene (restart)",
        cvar: "r_sceneHdr",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HDR exposure",
        cvar: "r_hdrExposure",
        kind: ValueKind::Float {
            min: 0.25,
            max: 4.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Eye adaptation",
        cvar: crate::frame_target::aa::exposure::ENABLED,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Adaptation: max darken, EV (HDR)",
        cvar: crate::frame_target::aa::exposure::MIN_EV,
        kind: ValueKind::Float {
            min: -2.0,
            max: 0.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Adaptation: max brighten, EV",
        cvar: crate::frame_target::aa::exposure::MAX_EV,
        kind: ValueKind::Float {
            min: 0.0,
            max: 2.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Filmic tone curve",
        cvar: "r_toneCurve",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Bloom",
        cvar: "r_sceneBloom",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Dynamic glow (0 off, 2 sabers)",
        cvar: "r_DynamicGlow",
        kind: ValueKind::Integer {
            min: 0,
            max: 3,
            step: 1,
        },
    },
    Setting {
        label: "Glow style (0 retail, 1 Vulkan)",
        cvar: "r_dynamicGlowStyle",
        kind: ValueKind::Integer {
            min: 0,
            max: 1,
            step: 1,
        },
    },
    Setting {
        label: "FXAA (restart)",
        cvar: "r_fxaa",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Supersampling, 1 off (restart)",
        cvar: "r_superSample",
        kind: ValueKind::Integer {
            min: 1,
            max: 3,
            step: 1,
        },
    },
    Setting {
        label: "Soft particles",
        cvar: "r_softParticles",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Sunbeam dust (0 off)",
        cvar: crate::dust_motes::CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Weather (rain, snow, mist)",
        cvar: crate::weather::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Weather density (1 original)",
        cvar: crate::weather::DENSITY_CVAR,
        kind: ValueKind::Float {
            min: 0.25,
            max: 4.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Weather quality (0 low, 3 ultra)",
        cvar: crate::weather::QUALITY_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: 3,
            step: 1,
        },
    },
    Setting {
        label: "Force weather (0 the map's)",
        cvar: crate::weather::FORCE_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: crate::weather::settings::FORCE_MAX,
            step: 1,
        },
    },
    Setting {
        label: "Ground fog (0 off, 1 map, 2 always)",
        cvar: crate::weather::FOG_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: 2,
            step: 1,
        },
    },
    Setting {
        label: "Volumetric clouds",
        cvar: crate::weather::CLOUDS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Per-pixel model lighting",
        cvar: "r_modelPixelLight",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Reflection probes (restart)",
        cvar: "r_cubeMapping",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Floor mirrors",
        cvar: "r_floorReflections",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Emission maps (restart)",
        cvar: "r_emissiveMaps",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Emission strength (0 off)",
        cvar: "r_emissionStrength",
        kind: ValueKind::Float {
            min: 0.0,
            max: 4.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Emission glow halo",
        cvar: "r_emissiveGlow",
        kind: ValueKind::Bool,
    },
];

pub(super) const RENDER_LIGHTING: &[Setting] = &[
    Setting {
        label: "Sun and sky (restart)",
        cvar: "r_dayNight",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Live lighting 0-2 (next map)",
        cvar: "r_liveLighting",
        kind: ValueKind::Integer {
            min: 0,
            max: 2,
            step: 1,
        },
    },
    Setting {
        label: "Time of day (hour)",
        cvar: "r_dayHour",
        kind: ValueKind::Float {
            min: 0.0,
            max: 24.0,
            step: 0.5,
        },
    },
    Setting {
        label: "Day length, min (0 holds)",
        cvar: "r_dayMinutes",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1440.0,
            step: 5.0,
        },
    },
    Setting {
        label: "Sunlight brightness",
        cvar: "r_dayBrightness",
        kind: ValueKind::Float {
            min: 0.1,
            max: 10.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Ambient fill",
        cvar: "r_ambientFill",
        kind: ValueKind::Float {
            min: 0.0,
            max: 0.2,
            step: 0.005,
        },
    },
    Setting {
        label: "Ambient fill corner shading",
        cvar: "r_ambientFillOcclusion",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Indirect light boost",
        cvar: "r_indirectBoost",
        kind: ValueKind::Float {
            min: 0.0,
            max: 4.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Emission-map lights (next map)",
        cvar: "r_emissiveLights",
        kind: ValueKind::Float {
            min: 0.0,
            max: 4.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Light shafts 0-3 (restart)",
        cvar: "r_volumetrics",
        kind: ValueKind::Integer {
            min: 0,
            max: 3,
            step: 1,
        },
    },
    Setting {
        label: "Light shaft clarity",
        cvar: "r_volumetricClarity",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
];

pub(super) const RENDER_SHADOWS: &[Setting] = &[
    Setting {
        label: "World sun shadows (restart)",
        cvar: "r_worldSunShadows",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Character sun shadows (restart)",
        cvar: "r_actorSunShadows",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Shadow resolution (restart)",
        cvar: "r_sunShadowResolution",
        kind: ValueKind::Integer {
            min: 512,
            max: 4096,
            step: 512,
        },
    },
    Setting {
        label: "Sharp shadow distance (restart)",
        cvar: "r_sunShadowDistance",
        kind: ValueKind::Integer {
            min: 128,
            max: 4096,
            step: 128,
        },
    },
    Setting {
        label: "Close shadow distance (restart)",
        cvar: "r_sunShadowNear",
        kind: ValueKind::Integer {
            min: 64,
            max: 1024,
            step: 64,
        },
    },
    Setting {
        label: "Shadow filter taps (restart)",
        cvar: "r_sunShadowTaps",
        kind: ValueKind::Integer {
            min: 4,
            max: 32,
            step: 4,
        },
    },
    Setting {
        label: "Close shadow slits (units)",
        cvar: "r_sunShadowGapClose",
        kind: ValueKind::Float {
            min: 0.0,
            max: 64.0,
            step: 1.0,
        },
    },
    Setting {
        label: "Contact shadows",
        cvar: "r_contactShadows",
        kind: ValueKind::Bool,
    },
];
