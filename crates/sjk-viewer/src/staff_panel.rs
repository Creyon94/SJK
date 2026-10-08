//! The Staff page: for a player the hub's operator made staff, the SJK team's tools
//! in the game (`docs/identity.md`, "Staff"): find a player, give or take back their
//! medals, clear their achievements. Every action is a request signed by the
//! player's own key (`sjk_identity::StaffRequest`); the hub refuses it from any other
//! key, and the page only opens for a key whose profile says staff.
//!
//! Opened by the Profile page's Staff tools button or the `staff` command. Like the
//! Profile page it lives in the console and has the SJK UI's look ([`view`]). Tab
//! and Shift+Tab walk every control; Enter or Space works the one focused; the
//! search pill and the note take typing; Up and Down move through the players found;
//! Escape goes back.

use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use sjk_identity::{Profile, StaffRequest, StaffState};
use sjk_ui::{InputEvent, UiEventKind};
use std::time::{Duration, Instant};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "staff_panel_view.rs"]
mod view;

const SEARCH_TOKEN: u16 = 1_100;
const ME_TOKEN: u16 = 1_101;
const RECENT_TOKEN: u16 = 1_102;
const NOTE_TOKEN: u16 = 1_103;
const CLEAR_ALL_TOKEN: u16 = 1_104;
/// Players found, one token each.
const PLAYER_BASE: u16 = 1_110;
const PLAYERS_SHOWN: usize = 14;
/// Give and Take back, one token per medal of the catalogue.
const GIVE_BASE: u16 = 1_150;
const TAKE_BASE: u16 = 1_160;
/// Clear, one token per achievement of the catalogue.
const CLEAR_BASE: u16 = 1_170;
/// Longest search, as the hub takes it.
const QUERY_MAX: usize = 64;
/// Longest note with a medal, as the hub takes it.
const NOTE_MAX: usize = 200;
/// How long Clear all waits for its second press.
const CONFIRM_FOR: Duration = Duration::from_secs(3);

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Send this to the hub.
    Request(StaffRequest),
}

/// What the page knows this frame.
pub(crate) struct Inputs<'a> {
    /// The player's own profile, once the hub sent it.
    pub(crate) me: Option<&'a Profile>,
    pub(crate) staff: &'a StaffState,
}

/// What the last frame showed, for keys and clicks.
#[derive(Debug, Default)]
struct Shown {
    /// The players' keys in the list, in order.
    players: Vec<String>,
    me: Option<String>,
    /// The chosen player's key, and how many of each medal they hold.
    target: Option<String>,
    medals: [u32; crate::medals::Medal::COUNT],
}

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    ui: MenuCanvas,
    query: String,
    note: String,
    /// The chosen player's key; the player's own until another is chosen.
    selected: Option<String>,
    /// The control the keyboard is on, by its token.
    focus: u16,
    /// The controls in the order Tab visits them, laid out by the last frame.
    order: Vec<u16>,
    /// When Clear all was pressed once.
    confirm_all: Option<Instant>,
    shown: Shown,
    epoch: Instant,
    /// What a world shot shows in place of the live profile and answers.
    #[cfg(test)]
    pub(crate) preview: Option<(Profile, StaffState)>,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

/// Add `text` to `field` up to `limit` characters, control characters left out;
/// whether it changed.
fn type_into(field: &mut String, text: &str, limit: usize) -> bool {
    let mut changed = false;
    for c in text.chars().filter(|c| !c.is_control()) {
        if field.chars().count() >= limit {
            break;
        }
        field.push(c);
        changed = true;
    }
    changed
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            ui: MenuCanvas::with_capacities(260, 160, 1_200),
            query: String::new(),
            note: String::new(),
            selected: None,
            focus: SEARCH_TOKEN,
            order: Vec::with_capacity(64),
            confirm_all: None,
            shown: Shown::default(),
            epoch: Instant::now(),
            #[cfg(test)]
            preview: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page; `owns_console` when the console was closed before it.
    pub(crate) fn open(&mut self, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        self.focus = SEARCH_TOKEN;
        self.confirm_all = None;
    }

    /// Closing the page closes the console too.
    pub(crate) fn own_console(&mut self) {
        self.owns_console = true;
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// The chosen player's profile: from the players found, or the player's own.
    fn target<'a>(&self, inputs: &Inputs<'a>) -> Option<&'a Profile> {
        let me = inputs.me;
        let key = self
            .selected
            .as_deref()
            .or(me.map(|me| me.key_id.as_str()))?;
        inputs
            .staff
            .players
            .iter()
            .find(|player| player.key_id == key)
            .or(me.filter(|me| me.key_id == key))
    }

    /// Whether Clear all is waiting for its second press.
    fn confirming(&self) -> bool {
        self.confirm_all
            .is_some_and(|at| at.elapsed() < CONFIRM_FOR)
    }

    /// What Enter (or a click) on `token` does.
    fn activate(&mut self, token: u16) -> PanelAction {
        self.focus = token;
        if token != CLEAR_ALL_TOKEN {
            self.confirm_all = None;
        }
        let target = self.shown.target.clone();
        match token {
            SEARCH_TOKEN => {
                PanelAction::Request(StaffRequest::Search(self.query.trim().to_owned()))
            }
            ME_TOKEN => {
                self.selected.clone_from(&self.shown.me);
                PanelAction::None
            }
            RECENT_TOKEN => {
                self.query.clear();
                PanelAction::Request(StaffRequest::Search(String::new()))
            }
            NOTE_TOKEN => PanelAction::None,
            CLEAR_ALL_TOKEN => {
                let Some(key_id) = target else {
                    return PanelAction::None;
                };
                if self.confirming() {
                    self.confirm_all = None;
                    PanelAction::Request(StaffRequest::ClearAchievements {
                        key_id,
                        id: String::new(),
                    })
                } else {
                    self.confirm_all = Some(Instant::now());
                    PanelAction::None
                }
            }
            token if (PLAYER_BASE..PLAYER_BASE + PLAYERS_SHOWN as u16).contains(&token) => {
                if let Some(key) = self.shown.players.get(usize::from(token - PLAYER_BASE)) {
                    self.selected = Some(key.clone());
                }
                PanelAction::None
            }
            token
                if (GIVE_BASE..GIVE_BASE + crate::medals::Medal::COUNT as u16).contains(&token) =>
            {
                let medal = crate::medals::Medal::ALL[usize::from(token - GIVE_BASE)];
                let held = self.shown.medals[medal.index()];
                match target {
                    Some(key_id) if held == 0 || medal.repeatable() => {
                        PanelAction::Request(StaffRequest::Award {
                            key_id,
                            medal: medal.id().to_owned(),
                            note: self.note.trim().to_owned(),
                        })
                    }
                    _ => PanelAction::None,
                }
            }
            token
                if (TAKE_BASE..TAKE_BASE + crate::medals::Medal::COUNT as u16).contains(&token) =>
            {
                let medal = crate::medals::Medal::ALL[usize::from(token - TAKE_BASE)];
                match target {
                    Some(key_id) if self.shown.medals[medal.index()] > 0 => {
                        PanelAction::Request(StaffRequest::Unaward {
                            key_id,
                            medal: medal.id().to_owned(),
                        })
                    }
                    _ => PanelAction::None,
                }
            }
            token
                if (CLEAR_BASE..CLEAR_BASE + crate::achievements::ALL.len() as u16)
                    .contains(&token) =>
            {
                let kind = &crate::achievements::ALL[usize::from(token - CLEAR_BASE)];
                match target {
                    Some(key_id) => PanelAction::Request(StaffRequest::ClearAchievements {
                        key_id,
                        id: kind.id.to_owned(),
                    }),
                    None => PanelAction::None,
                }
            }
            _ => PanelAction::None,
        }
    }

    /// Move the keyboard `by` controls along the order the last frame laid out.
    fn step(&mut self, forward: bool) {
        if self.order.is_empty() {
            return;
        }
        let at = self.order.iter().position(|token| *token == self.focus);
        let next = match (at, forward) {
            (None, _) => 0,
            (Some(at), true) => (at + 1) % self.order.len(),
            (Some(at), false) => (at + self.order.len() - 1) % self.order.len(),
        };
        self.focus = self.order[next];
    }

    /// A key.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent, shift: bool) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        let typing = matches!(self.focus, SEARCH_TOKEN | NOTE_TOKEN);
        let in_list = (PLAYER_BASE..PLAYER_BASE + PLAYERS_SHOWN as u16).contains(&self.focus);
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Tab => self.step(!shift),
            KeyCode::ArrowDown | KeyCode::ArrowUp if in_list => {
                let count = self.shown.players.len().min(PLAYERS_SHOWN);
                if count > 0 {
                    let at = usize::from(self.focus - PLAYER_BASE);
                    let next = if key == KeyCode::ArrowDown {
                        (at + 1) % count
                    } else {
                        (at + count - 1) % count
                    };
                    return self.activate(PLAYER_BASE + next as u16);
                }
            }
            KeyCode::ArrowDown if !typing => self.step(true),
            KeyCode::ArrowUp if !typing => self.step(false),
            KeyCode::Enter | KeyCode::NumpadEnter => return self.activate(self.focus),
            KeyCode::Space if !typing => return self.activate(self.focus),
            KeyCode::Backspace if typing => {
                let field = if self.focus == SEARCH_TOKEN {
                    &mut self.query
                } else {
                    &mut self.note
                };
                field.pop();
            }
            _ if typing && !event.repeat => {
                let pasted;
                let text = match event.text.as_deref() {
                    Some("\u{16}") => {
                        pasted = crate::console::clipboard::paste().unwrap_or_default();
                        pasted.as_str()
                    }
                    Some(text) => text,
                    None => return PanelAction::None,
                };
                if self.focus == SEARCH_TOKEN {
                    type_into(&mut self.query, text, QUERY_MAX);
                } else {
                    type_into(&mut self.note, text, NOTE_MAX);
                }
            }
            _ => {}
        }
        PanelAction::None
    }

    /// A pointer event.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            // A click in a field puts the keyboard there; it searches on Enter.
            Some(token @ (SEARCH_TOKEN | NOTE_TOKEN)) => {
                self.focus = token;
                PanelAction::None
            }
            Some(token) => self.activate(token),
            None => PanelAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::{Achievement, Medal};

    pub(super) fn profile(key_id: &str, name: &str) -> Profile {
        Profile {
            key_id: key_id.to_owned(),
            key: String::new(),
            name: name.to_owned(),
            bio: String::new(),
            verified: false,
            staff: false,
            created: 1_791_250_000,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
        }
    }

    fn me() -> Profile {
        Profile {
            staff: true,
            medals: vec![Medal {
                id: "early_tester".into(),
                count: 1,
                awarded: 1,
                note: String::new(),
            }],
            achievements: vec![Achievement {
                id: "first_blood".into(),
                progress: 1,
                goal: 1,
                unlocked: 5,
            }],
            ..profile("aaaaaaaaaaaaaaaa", "^1Sol")
        }
    }

    /// The panel after one frame with `players` found, so its controls are known.
    fn drawn(players: Vec<Profile>) -> (Panel, Profile, StaffState) {
        let me = me();
        let staff = StaffState {
            players,
            ..StaffState::default()
        };
        let mut panel = Panel::new();
        panel.open(true);
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &Inputs {
                me: Some(&me),
                staff: &staff,
            },
            &fonts.font,
            [1920.0, 1080.0],
        );
        (panel, me, staff)
    }

    #[test]
    fn the_player_is_their_own_target_until_another_is_chosen() {
        let (mut panel, me, staff) = drawn(vec![profile("bbbbbbbbbbbbbbbb", "Fox")]);
        let inputs = Inputs {
            me: Some(&me),
            staff: &staff,
        };
        assert_eq!(panel.target(&inputs).unwrap().key_id, me.key_id);
        assert_eq!(panel.activate(PLAYER_BASE), PanelAction::None);
        assert_eq!(panel.target(&inputs).unwrap().name, "Fox");
        let _ = panel.activate(ME_TOKEN);
        assert_eq!(panel.target(&inputs).unwrap().key_id, me.key_id);
    }

    #[test]
    fn medals_are_given_and_taken_back_as_the_catalogue_allows() {
        let (mut panel, me, _) = drawn(Vec::new());
        panel.note = "  found the fog bug ".into();
        // Early Tester is held and not repeatable: no second one.
        assert_eq!(panel.activate(GIVE_BASE), PanelAction::None);
        assert_eq!(
            panel.activate(TAKE_BASE),
            PanelAction::Request(StaffRequest::Unaward {
                key_id: me.key_id.clone(),
                medal: "early_tester".into()
            })
        );
        assert_eq!(
            panel.activate(GIVE_BASE + 2),
            PanelAction::Request(StaffRequest::Award {
                key_id: me.key_id.clone(),
                medal: "bug_hunter".into(),
                note: "found the fog bug".into()
            })
        );
        // Not held: nothing to take back.
        assert_eq!(panel.activate(TAKE_BASE + 2), PanelAction::None);
    }

    #[test]
    fn clear_all_needs_a_second_press() {
        let (mut panel, me, _) = drawn(Vec::new());
        assert_eq!(panel.activate(CLEAR_ALL_TOKEN), PanelAction::None);
        assert!(panel.confirming());
        assert_eq!(
            panel.activate(CLEAR_ALL_TOKEN),
            PanelAction::Request(StaffRequest::ClearAchievements {
                key_id: me.key_id.clone(),
                id: String::new()
            })
        );
        // Anything else in between starts again.
        let _ = panel.activate(CLEAR_ALL_TOKEN);
        let _ = panel.activate(ME_TOKEN);
        assert_eq!(panel.activate(CLEAR_ALL_TOKEN), PanelAction::None);
        assert_eq!(
            panel.activate(CLEAR_BASE),
            PanelAction::Request(StaffRequest::ClearAchievements {
                key_id: me.key_id,
                id: "first_blood".into()
            })
        );
    }

    #[test]
    fn tab_walks_every_control_and_typing_goes_to_the_field() {
        let (mut panel, _, _) = drawn(vec![profile("bbbbbbbbbbbbbbbb", "Fox")]);
        assert!(panel.order.contains(&SEARCH_TOKEN));
        assert!(panel.order.contains(&PLAYER_BASE));
        assert!(panel.order.contains(&NOTE_TOKEN));
        assert!(panel.order.contains(&CLEAR_BASE), "first_blood is listed");
        let start = panel.focus;
        for _ in 0..panel.order.len() {
            panel.step(true);
        }
        assert_eq!(panel.focus, start, "Tab comes round");
        assert!(type_into(&mut panel.query, "So\u{7}l", QUERY_MAX));
        assert_eq!(panel.query, "Sol");
        panel.focus = SEARCH_TOKEN;
        assert_eq!(
            panel.activate(SEARCH_TOKEN),
            PanelAction::Request(StaffRequest::Search("Sol".into()))
        );
        let mut long = String::new();
        type_into(&mut long, &"x".repeat(300), NOTE_MAX);
        assert_eq!(long.len(), NOTE_MAX);
    }
}
