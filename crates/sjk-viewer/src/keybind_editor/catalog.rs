//! Stable bindable-action catalog and default key map.
//!
//! Commands are the stock client's bind names (`codemp/client/cl_input.cpp`
//! command table, cgame console commands) so pasted JKA configs keep working.
//! Default keys follow retail `mpdefault.cfg` where the key is free; the
//! JKR-only rows (votes, team menu, camera) take keys retail leaves unbound.
//! `forcenext`/`forceprev`/`invnext`/`invprev` wait on local force and
//! inventory selection.
//!
//! Rows are grouped the way retail's `ui/jamp/controls.menu` groups them
//! (Movement, Interaction, Weapons, Force Powers, Other); the mouse page
//! lives on the Settings CONTROLS tab instead.

use Category::*;
use sjk_shell::BindTable;

/// Retail's controls pages, in tab order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Category {
    Movement,
    Interaction,
    Weapons,
    Force,
    Other,
}

/// Tab captions, indexed by `Category as usize`.
pub(crate) const CATEGORIES: [&str; 5] = ["MOVEMENT", "INTERACTION", "WEAPONS", "FORCE", "OTHER"];

pub(crate) struct BindableAction {
    pub(crate) category: Category,
    pub(crate) label: &'static str,
    pub(crate) command: &'static str,
    pub(crate) default_key: &'static str,
}

const fn action(
    category: Category,
    label: &'static str,
    command: &'static str,
    default_key: &'static str,
) -> BindableAction {
    BindableAction {
        category,
        label,
        command,
        default_key,
    }
}

/// Every bindable action, grouped by category in tab order.
pub(crate) const ACTIONS: &[BindableAction] = &[
    // MOVEMENT
    action(Movement, "Move forward", "+forward", "w"),
    action(Movement, "Move back", "+back", "s"),
    action(Movement, "Strafe left", "+moveleft", "a"),
    action(Movement, "Strafe right", "+moveright", "d"),
    action(Movement, "Jump", "+moveup", "SPACE"),
    action(Movement, "Crouch", "+movedown", "c"),
    action(Movement, "Walk", "+speed", "SHIFT"),
    action(Movement, "Center view", "centerview", "END"),
    action(Movement, "Turn left", "+left", "LEFTARROW"),
    action(Movement, "Turn right", "+right", "RIGHTARROW"),
    action(Movement, "Look up", "+lookup", "PGUP"),
    action(Movement, "Look down", "+lookdown", "PGDN"),
    action(Movement, "Strafe modifier", "+strafe", "z"),
    action(Movement, "Mouse look", "+mlook", "/"),
    // JoF EJK's `controls.menu` "FlipKick:" row: one press, a run of jump taps.
    action(Movement, "Flip kick (JoF)", "flipkick", ""),
    // INTERACTION
    action(Interaction, "Attack", "+attack", "MOUSE1"),
    action(Interaction, "Alternate attack", "+altattack", "MOUSE2"),
    action(Interaction, "Use", "+use", "ENTER"),
    action(Interaction, "Use selected inventory item", "+button2", ""),
    action(Interaction, "Next inventory item", "invnext", ""),
    action(Interaction, "Previous inventory item", "invprev", ""),
    action(Interaction, "Toggle saber", "sv_saberswitch", ""),
    action(Interaction, "Saber style", "saberAttackCycle", "l"),
    action(Interaction, "Engage duel", "engage_duel", "k"),
    action(Interaction, "Binoculars", "zoom", ""),
    // SJK: pin the player card to the player under the crosshair; press again to hide.
    action(Interaction, "Inspect player", "inspect", "x"),
    action(Interaction, "Item: Bacta", "use_bacta", ""),
    action(Interaction, "Item: Seeker drone", "use_seeker", ""),
    action(Interaction, "Item: Sentry gun", "use_sentry", ""),
    action(Interaction, "Item: Force field", "use_field", "KP_HOME"),
    action(
        Interaction,
        "Item: Electrobinoculars",
        "use_electrobinoculars",
        "",
    ),
    // WEAPONS
    action(Weapons, "Next weapon", "weapnext", "MWHEELDOWN"),
    action(Weapons, "Previous weapon", "weapprev", "MWHEELUP"),
    // `weapon N` selects weapon `N + 2` of `weapon_t`; the labels name it.
    action(Weapons, "Saber / melee", "weapon 1", "1"),
    action(Weapons, "Blaster pistol", "weapon 2", "2"),
    action(Weapons, "Blaster rifle", "weapon 3", "3"),
    action(Weapons, "Disruptor rifle", "weapon 4", "4"),
    action(Weapons, "Bowcaster", "weapon 5", "5"),
    action(Weapons, "Heavy repeater", "weapon 6", "6"),
    action(Weapons, "DEMP 2", "weapon 7", "7"),
    action(Weapons, "Flechette", "weapon 8", "8"),
    action(Weapons, "Concussion rifle", "weapon 13", "9"),
    action(Weapons, "Rocket launcher", "weapon 9", "0"),
    action(Weapons, "Explosives", "weapon 10", "-"),
    // FORCE
    action(Force, "Use selected force power", "+useforce", "f"),
    action(Force, "Next force power", "forcenext", ""),
    action(Force, "Previous force power", "forceprev", ""),
    action(Force, "Push", "force_throw", "F1"),
    action(Force, "Pull", "force_pull", "F2"),
    action(Force, "Speed", "force_speed", "F3"),
    action(Force, "Seeing", "force_seeing", "F4"),
    action(Force, "Heal", "force_heal", "F5"),
    action(Force, "Protect", "force_protect", "F6"),
    action(Force, "Absorb", "force_absorb", "F7"),
    action(Force, "Mind trick", "force_distract", "F8"),
    action(Force, "Grip (hold)", "+force_grip", "F9"),
    action(Force, "Lightning (hold)", "+force_lightning", "F10"),
    action(Force, "Rage", "force_rage", "F11"),
    action(Force, "Drain (hold)", "+force_drain", "F12"),
    action(Force, "Team heal", "force_healother", "]"),
    action(Force, "Team energize", "force_forcepowerother", "\\"),
    // JoF JA+ abilities, as JoF EJK's `controls.menu` lists them: Dash and Repulse
    // are server commands, Stasis the held usercmd button (`input::GameButton`).
    action(Force, "Dash (JoF)", "force_dash", ""),
    action(Force, "Stasis (JoF, hold)", "+force_stasis", ""),
    action(Force, "Repulse (JoF)", "force_repulse", ""),
    // OTHER
    // Server commands (`codemp/game/g_cmds.c:3402-3404`); the console forwards
    // them, but they were unbindable from this screen until now.
    action(Other, "Follow next player", "follownext", ""),
    action(Other, "Follow previous player", "followprev", ""),
    action(Other, "Console (also Shift+Esc)", "toggleconsole", ""),
    // Generic mod buttons (`cl_input.cpp:1686-1717`, MAX_KBUTTONS 16). Bits
    // 0/5/6/7/9/10/11 already have named actions above and bit 4 is
    // BUTTON_WALKING, which PM_CmdScale rewrites every frame.
    action(Other, "Mod button 1", "+button1", ""),
    action(Other, "Mod button 3", "+button3", ""),
    action(Other, "Mod button 8", "+button8", ""),
    action(Other, "Mod button 12 (JA+ grapple)", "+button12", ""),
    action(Other, "Mod button 13", "+button13", ""),
    action(Other, "Mod button 14", "+button14", ""),
    action(Other, "Mod button 15", "+button15", ""),
    action(Other, "Scoreboard", "+scores", "TAB"),
    action(Other, "Chat", "messagemode", "y"),
    action(Other, "Team chat", "messagemode2", "t"),
    action(Other, "Whisper to crosshair player", "messagemode3", ""),
    action(Other, "Whisper to last attacker", "messagemode4", ""),
    action(Other, "Join / team menu", "teammenu", "j"),
    action(Other, "Vote yes", "vote yes", ""),
    action(Other, "Vote no", "vote no", ""),
    action(Other, "Camera mode", "togglecamera", "p"),
    action(Other, "Free camera", "freecam", ""),
    // SJK: off, names only, bars on the target and duel opponent, bars on everyone.
    action(Other, "Nameplate mode", "nameplates", "v"),
    action(Other, "Taunt", "taunt", "g"),
    action(Other, "Bow", "bow", "b"),
    action(Other, "Meditate", "meditate", ""),
    action(Other, "Flourish", "flourish", "n"),
    action(Other, "Gloat", "gloat", "h"),
];

/// Index range of `ACTIONS` shown on tab `tab`; rows are grouped, so the
/// range is contiguous.
pub(crate) fn category_range(tab: usize) -> std::ops::Range<usize> {
    let start = ACTIONS
        .iter()
        .position(|action| action.category as usize == tab)
        .unwrap_or(ACTIONS.len());
    let end = ACTIONS[start..]
        .iter()
        .position(|action| action.category as usize != tab)
        .map_or(ACTIONS.len(), |length| start + length);
    start..end
}

/// Install only missing defaults, preserving a loaded user configuration.
pub(crate) fn default_bindings() -> BindTable {
    let mut binds = BindTable::new();
    for action in ACTIONS {
        if !action.default_key.is_empty() {
            let _ = binds.bind(action.default_key, action.command);
        }
    }
    binds
}

/// Add defaults absent from a pre-M2 config without replacing custom keys.
/// Pre-step-73e configs spell `+useforce` as `+force`; rename so the editor
/// shows the key.
pub(crate) fn migrate_missing_defaults(binds: &mut BindTable) {
    let legacy_force: Vec<String> = binds
        .iter()
        .filter(|(_, command)| command.eq_ignore_ascii_case("+force"))
        .map(|(key, _)| key.to_owned())
        .collect();
    for key in legacy_force {
        let _ = binds.bind(&key, "+useforce");
    }
    for action in ACTIONS {
        let has_action = binds
            .iter()
            .any(|(_, command)| command.eq_ignore_ascii_case(action.command));
        if !has_action && !action.default_key.is_empty() && binds.get(action.default_key).is_none()
        {
            let _ = binds.bind(action.default_key, action.command);
        }
    }
}
