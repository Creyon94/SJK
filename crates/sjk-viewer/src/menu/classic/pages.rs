//! The classic main menu's page tables: every page's entries in retail item
//! order (which keyboard focus follows), with their retail positions on the
//! 640x480 canvas. Types and geometry are in [`super::layout`].

use super::layout::{Entry, Page, Size, Slot};
use sjk_ui::TextAlign;

/// A centred button `width` canvas units wide and 30 high.
const fn button(
    entry: Entry,
    label: &'static str,
    hint: &'static str,
    center: [f32; 2],
    width: f32,
    size: Size,
) -> Slot {
    Slot {
        entry,
        label,
        hint,
        center,
        width,
        height: 30.0,
        size,
        align: TextAlign::Center,
    }
}

/// One row of the Controls and Setup option lists: retail rect `80 y 170
/// 24`, its label set against the right edge.
const fn list_row(entry: Entry, label: &'static str, hint: &'static str, y: f32) -> Slot {
    Slot {
        entry,
        label,
        hint,
        center: [165.0, y + 12.0],
        width: 170.0,
        height: 24.0,
        size: Size::List,
        align: TextAlign::End,
    }
}

/// One entry of the start-playing centre list: retail rect `225 y 190 36`.
const fn centre_row(entry: Entry, label: &'static str, hint: &'static str, y: f32) -> Slot {
    Slot {
        entry,
        label,
        hint,
        center: [320.0, y + 18.0],
        width: 190.0,
        height: 34.0,
        size: Size::Medium,
        align: TextAlign::Center,
    }
}

const PLAY_HINT: &str = "Solo game, join a server or start your own";
const PROFILE_HINT: &str = "Name, model, saber and Force";
const CONTROLS_HINT: &str = "Key bindings and mouse";
const SETUP_HINT: &str = "Video, sound and game options";
const EXIT_HINT: &str = "Leave the game";
const CHANGELOG_HINT: &str = "What changed in each SJK release, and who made it";
const BACK_HINT: &str = "Return to the main menu";

/// Retail `main.menu`: two columns either side of the centre window, Exit
/// below.
const MAIN: [Slot; 6] = [
    button(
        Entry::Play,
        "PLAY",
        PLAY_HINT,
        [101.0, 224.0],
        190.0,
        Size::Large,
    ),
    button(
        Entry::Profile,
        "PROFILE",
        PROFILE_HINT,
        [101.0, 322.0],
        190.0,
        Size::Large,
    ),
    button(
        Entry::Controls,
        "CONTROLS",
        CONTROLS_HINT,
        [521.0, 224.0],
        190.0,
        Size::Large,
    ),
    button(
        Entry::Setup,
        "SETUP",
        SETUP_HINT,
        [521.0, 322.0],
        190.0,
        Size::Large,
    ),
    button(
        Entry::Exit,
        "EXIT",
        EXIT_HINT,
        [320.0, 456.0],
        190.0,
        Size::Large,
    ),
    // SJK: under PROFILE, clear of the centre window and Exit.
    button(
        Entry::Changelog,
        "CHANGELOG",
        CHANGELOG_HINT,
        [101.0, 456.0],
        150.0,
        Size::Medium,
    ),
];

/// The navigation row every retail sub-menu repeats along its top (retail
/// rects `7 126`, `170 126`, `340 126`, `502 126`, 130 by 24).
const fn nav_row() -> [Slot; 4] {
    [
        button(
            Entry::Play,
            "PLAY",
            PLAY_HINT,
            [72.0, 138.0],
            130.0,
            Size::Medium,
        ),
        button(
            Entry::Profile,
            "PROFILE",
            PROFILE_HINT,
            [235.0, 138.0],
            130.0,
            Size::Medium,
        ),
        button(
            Entry::Controls,
            "CONTROLS",
            CONTROLS_HINT,
            [405.0, 138.0],
            130.0,
            Size::Medium,
        ),
        button(
            Entry::Setup,
            "SETUP",
            SETUP_HINT,
            [567.0, 138.0],
            130.0,
            Size::Medium,
        ),
    ]
}

/// Back and Exit along the bottom of every sub-page (retail rects `59 444`
/// and `255 444`).
const fn back_exit() -> [Slot; 2] {
    [
        button(
            Entry::Back,
            "BACK",
            BACK_HINT,
            [124.0, 456.0],
            130.0,
            Size::Medium,
        ),
        button(
            Entry::Exit,
            "EXIT",
            EXIT_HINT,
            [320.0, 456.0],
            130.0,
            Size::Medium,
        ),
    ]
}

/// Retail `multiplayer.menu`: the start-playing list in the centre.
const PLAY: [Slot; 11] = {
    let [play, profile, controls, setup] = nav_row();
    let [back, exit] = back_exit();
    [
        play,
        profile,
        controls,
        setup,
        centre_row(
            Entry::SoloGame,
            "SOLO GAME",
            "A local match with bots, set up in Create game",
            191.0,
        ),
        centre_row(
            Entry::JoinServer,
            "JOIN SERVER",
            "Browse servers and join a game",
            226.0,
        ),
        centre_row(
            Entry::CreateServer,
            "CREATE SERVER",
            "Host a match with bots on this machine",
            261.0,
        ),
        centre_row(
            Entry::PlayDemo,
            "PLAY DEMO",
            "Not in SJK yet: use the demo console command",
            296.0,
        ),
        centre_row(
            Entry::Rules,
            "RULES",
            "Not in SJK yet: no rules pages",
            331.0,
        ),
        back,
        exit,
    ]
};

/// Retail `controls.menu`: the binding pages down the left. Each opens the
/// key-binding editor on its tab; Mouse/Joystick opens the mouse options.
/// Retail's two Force Powers pages are one group (classic+).
const CONTROLS: [Slot; 12] = {
    let [play, profile, controls, setup] = nav_row();
    let [back, exit] = back_exit();
    [
        play,
        profile,
        controls,
        setup,
        list_row(
            Entry::Movement,
            "MOVEMENT",
            "Key bindings: moving, jumping, turning and looking",
            185.0,
        ),
        list_row(
            Entry::Interaction,
            "INTERACTION",
            "Key bindings: attacks, saber, use and items",
            209.0,
        ),
        list_row(
            Entry::Weapons,
            "WEAPONS",
            "Key bindings: every weapon",
            233.0,
        ),
        list_row(
            Entry::ForcePowers,
            "FORCE POWERS",
            "Key bindings: every Force power",
            257.0,
        ),
        list_row(
            Entry::MouseJoystick,
            "MOUSE/JOYSTICK",
            "Mouse sensitivity, inversion and always run",
            281.0,
        ),
        list_row(
            Entry::OtherControls,
            "OTHER",
            "Key bindings: chat, scores, votes and emotes",
            305.0,
        ),
        back,
        exit,
    ]
};

/// Retail `setup.menu`: the option pages down the left, then the settings
/// JKR adds after them. Classic+ shows retail's two video pages as one and
/// regroups the rest by subject: the menus and console, the HUD, the
/// scoreboard.
const SETUP: [Slot; 16] = {
    let [play, profile, controls, setup] = nav_row();
    let [back, exit] = back_exit();
    [
        play,
        profile,
        controls,
        setup,
        list_row(
            Entry::Video,
            "VIDEO",
            "Resolution, display, frame rate, field of view and brightness",
            185.0,
        ),
        list_row(
            Entry::Sound,
            "SOUND",
            "Effects and music volume, footsteps",
            209.0,
        ),
        list_row(
            Entry::GameOptions,
            "GAME OPTIONS",
            "Pickups, models, saber and Force trails, camera",
            233.0,
        ),
        list_row(
            Entry::Mods,
            "MODS",
            "Not in SJK yet: set fs_game and restart",
            257.0,
        ),
        list_row(
            Entry::Defaults,
            "DEFAULTS",
            "Not in SJK yet: BACKSPACE on a setting restores its default",
            281.0,
        ),
        list_row(
            Entry::Interface,
            "INTERFACE",
            "Menu style, colours and fonts, the console's look",
            305.0,
        ),
        list_row(
            Entry::Hud,
            "HUD",
            "HUD style and scale, status, crosshair, readouts and chat",
            329.0,
        ),
        list_row(
            Entry::Scoreboard,
            "SCOREBOARD",
            "Scoreboard style, client numbers, head icons and row size",
            353.0,
        ),
        list_row(
            Entry::Network,
            "NETWORK",
            "Master server and connection rates",
            377.0,
        ),
        list_row(
            Entry::Renderer,
            "RENDERER",
            "HDR, bloom, lighting, shadows and day/night",
            401.0,
        ),
        back,
        exit,
    ]
};

/// SJK's renderer page (classic+): `setup.menu`'s layout with the renderer
/// settings' three groups down the left; Back returns to Setup.
const RENDERER: [Slot; 9] = {
    let [play, profile, controls, setup] = nav_row();
    let [back, exit] = back_exit();
    [
        play,
        profile,
        controls,
        setup,
        list_row(
            Entry::RenderImage,
            "IMAGE",
            "HDR, exposure, bloom, glow, edge smoothing, reflections and emission",
            185.0,
        ),
        list_row(
            Entry::RenderLighting,
            "LIGHTING",
            "Sun and sky, live lighting, fill light and light shafts",
            209.0,
        ),
        list_row(
            Entry::RenderShadows,
            "SHADOWS",
            "Sun shadows: on or off, detail, distance and edges",
            233.0,
        ),
        Slot {
            entry: Entry::SetupBack,
            hint: "Return to the setup options",
            ..back
        },
        exit,
    ]
};

/// Retail `quit.menu`: No bottom left, Yes bottom right.
const QUIT: [Slot; 6] = {
    let [play, profile, controls, setup] = nav_row();
    [
        play,
        profile,
        controls,
        setup,
        button(
            Entry::No,
            "NO",
            BACK_HINT,
            [124.0, 456.0],
            130.0,
            Size::Medium,
        ),
        button(
            Entry::Yes,
            "YES",
            "Exit to the desktop",
            [519.0, 456.0],
            130.0,
            Size::Medium,
        ),
    ]
};

/// The entries of `page`, in focus order.
pub(super) fn slots(page: Page) -> &'static [Slot] {
    match page {
        Page::Main => &MAIN,
        Page::Play => &PLAY,
        Page::Controls => &CONTROLS,
        Page::Setup => &SETUP,
        Page::Renderer => &RENDERER,
        Page::Quit => &QUIT,
    }
}
