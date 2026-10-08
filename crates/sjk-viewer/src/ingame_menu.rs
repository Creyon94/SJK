//! Retained in-game menu: the pages reached with Escape during a match,
//! drawn in the main menu's hero style over the live world, or, with
//! `ui_menuStyle classic`, as the retail bar and pop-ups ([`classic`]), or,
//! with `ui_menuStyle sjk`, as the SJK UI's arc and match card
//! ([`sjk_view`]).

use crate::menu::art::ArtSet;
use crate::menu::sjk::TextTarget;
use crate::menu::style::MenuStyle;
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextVertex, UiFont};
use sjk_protocol::{GameState, InfoString};
use sjk_ui::{AbstractAction, DrawList, InputEvent, UiEventKind};
use std::fmt::Write as _;

mod about;
mod callvote;
pub(crate) mod classic;
mod classic_actions;
mod classic_view;
pub(crate) mod players;
pub(crate) mod shot;
mod siege;
pub(crate) mod siege_data;
pub(crate) mod sjk;
mod sjk_actions;
pub(crate) mod sjk_view;
mod view;
pub(crate) use callvote::Action as CallVoteAction;

const VOTE_SCROLL_TOKEN: u16 = u16::MAX;

/// What players read for the camera and sunlight panel ([`Page::Shot`]): its
/// game-menu entry and title in every style (Shot controls until 08/10/2026).
pub(crate) const CAMERA_CONTROL: &str = "Camera control";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Page {
    Main,
    /// Camera control: camera and sun controls over the live world
    /// ([`shot`]).
    Shot,
    Team,
    /// Map/theme constrained Siege class selection.
    Siege,
    /// SJK's own screens ([`sjk`]): the changelog, more later.
    Sjk,
    /// Host, map, game type and limits (`ingame_about` in the stock UI).
    About,
    /// Disconnect or quit, with the choice as the confirmation (classic:
    /// the retail exit pop-up, confirmed on the next two pages).
    Leave,
    /// Classic only: the retail vote pop-up (Yes, No).
    Vote,
    /// Classic only: "Go to Main Menu?" after the exit pop-up's Main Menu.
    ConfirmLeave,
    /// Classic only: "Quit Program?" after the exit pop-up's Quit Program.
    ConfirmQuit,
    CallVote,
    VoteMap,
    VoteGameType,
    VoteKick,
    VoteClientKick,
    VoteWarmup,
    VoteTimeLimit,
    VoteFragLimit,
    /// Everyone on the server, a small scoreboard; choosing one opens
    /// [`Page::ReportPlayer`] ([`players`]).
    Players,
    /// Why the chosen player is reported; a reason opens the text dialog.
    ReportPlayer,
}

pub(crate) struct View<'a> {
    pub(crate) page: Page,
    pub(crate) selected_row: usize,
    pub(crate) team: u8,
    pub(crate) team_game: bool,
    /// The server runs Siege, where retail swaps two bar buttons.
    pub(crate) siege: bool,
    pub(crate) red_players: usize,
    pub(crate) blue_players: usize,
    pub(crate) vote_active: bool,
    /// Keeps the struct open for per-page data borrowed from the frame.
    pub(crate) _frame: std::marker::PhantomData<&'a ()>,
}

/// The per-entry hint of `view`'s page: only the main page carries hints.
fn hint_for(view: &View<'_>) -> impl Fn(usize) -> &'static str {
    let (page, vote_active) = (view.page, view.vote_active);
    move |row| match page {
        Page::Main => main_hint(row, vote_active),
        Page::Sjk => sjk::ENTRIES.get(row).map_or("", |entry| entry.hint),
        Page::ReportPlayer => players::State::category(row).map_or("", |c| c.hint()),
        _ => "",
    }
}

/// One-line description under the selected entry of the main page.
fn main_hint(row: usize, vote_active: bool) -> &'static str {
    match row {
        0 => "Back to the match",
        1 => "Pick a side or spectate",
        2 => "Find another server; joining leaves this one",
        3 => "Name, model, saber, colours and Force",
        4 => "SJK's changelog, and more to come",
        5 => "Host, map, game type and limits",
        6 => "Video, audio, HUD, game and network",
        7 => "Key bindings",
        8 => "Map, game type, kick, limits",
        9 | 10 if vote_active => "Cast your vote on the current call",
        9 if !vote_active => "Camera framing and smooth sunlight for recording",
        11 if vote_active => "Camera framing and smooth sunlight for recording",
        _ => "Disconnect or quit",
    }
}

/// Fixed-storage retained UI state shared by every in-game page.
pub(crate) struct InGameMenu {
    canvas: MenuCanvas,
    kicker: String,
    rows: [String; 24],
    enabled: [bool; 24],
    row_count: usize,
    callvote: callvote::State,
    about: about::State,
    active_page: Page,
    siege: siege::State,
    pub(crate) shot: shot::Panel,
    /// The Players and Report pages' roster and chosen player.
    pub(crate) players: players::State,
    /// Layout family (`ui_menuStyle`).
    style: MenuStyle,
    /// Retail artwork the classic layout can draw.
    art: ArtSet,
    /// The SJK UI's line under each row (what it opens, who is on a team).
    hints: [String; 24],
    /// The SJK UI's match card.
    card: sjk_view::Card,
    /// The SJK UI's gold mark along its arc.
    motion: sjk_view::Motion,
    /// The SJK UI's main-page row that handed over to another screen, which
    /// the menu comes back on.
    return_row: usize,
    /// The world shots' made-up match facts in place of the frame's.
    #[cfg(test)]
    shot_view: Option<ShotView>,
}

/// The facts a world shot puts in place of the frame's (it has no server).
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShotView {
    pub(crate) team: u8,
    pub(crate) team_game: bool,
    pub(crate) red_players: usize,
    pub(crate) blue_players: usize,
    pub(crate) vote_active: bool,
}

impl InGameMenu {
    pub(crate) fn new() -> Self {
        Self {
            canvas: MenuCanvas::new(),
            kicker: String::with_capacity(48),
            rows: std::array::from_fn(|_| String::with_capacity(96)),
            enabled: [true; 24],
            row_count: 0,
            callvote: callvote::State::new(),
            about: about::State::new(),
            active_page: Page::Main,
            siege: siege::State::default(),
            shot: shot::Panel::default(),
            players: players::State::default(),
            style: MenuStyle::default(),
            art: ArtSet::default(),
            hints: std::array::from_fn(|_| String::with_capacity(96)),
            card: sjk_view::Card::default(),
            motion: sjk_view::Motion::default(),
            return_row: 0,
            #[cfg(test)]
            shot_view: None,
        }
    }

    /// Follow the player's `ui_menuStyle`, with the retail artwork `art`
    /// the classic layout can draw.
    pub(crate) fn set_style(&mut self, style: MenuStyle, art: ArtSet) {
        self.style = style;
        self.art = art;
    }

    /// Whether the classic (retail bar and pop-ups) layout is in use.
    pub(crate) fn is_classic(&self) -> bool {
        self.style.classic_screens() && !self.is_sjk()
    }

    /// Whether the SJK UI's layout is in use ([`sjk_view`]).
    pub(crate) fn is_sjk(&self) -> bool {
        self.style == MenuStyle::Sjk
    }

    /// The SJK UI's main-page `row` hands over to another screen (Character,
    /// Settings, Servers): the menu comes back on it.
    pub(crate) fn remember_return(&mut self, row: usize) {
        self.return_row = row;
    }

    /// The row the main page shows when a screen opened from it returns: the
    /// entry that opened it in the SJK UI, the first in the other looks.
    pub(crate) fn return_row(&mut self) -> usize {
        if self.is_sjk() {
            std::mem::take(&mut self.return_row)
        } else {
            0
        }
    }

    /// The SJK UI's next row from `current` of `count` on `page`, `forward`
    /// or back, wrapping and passing over rows that cannot be taken (the team
    /// the player is on, a ballot with no vote on).
    pub(crate) fn sjk_step(
        &self,
        page: Page,
        current: usize,
        count: usize,
        forward: bool,
    ) -> usize {
        if count == 0 {
            return 0;
        }
        let takeable =
            |row: usize| page != self.active_page || self.enabled.get(row).copied().unwrap_or(true);
        let mut row = current.min(count - 1);
        for _ in 0..count {
            row = if forward {
                (row + 1) % count
            } else {
                (row + count - 1) % count
            };
            if takeable(row) {
                return row;
            }
        }
        current.min(count - 1)
    }

    /// Draw `view`'s page in the SJK UI's look, its text to `target`.
    pub(crate) fn append_sjk(
        &mut self,
        view: View<'_>,
        target: TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        if view.page == Page::Shot {
            // Camera control has its own column, not the arc.
            self.active_page = Page::Shot;
            self.shot.build_sjk(&mut self.canvas, viewport);
        } else {
            self.build_sjk(view, viewport);
        }
        target.append(&self.canvas, viewport);
    }

    /// Lay `view`'s page out on the canvas in the SJK UI's look.
    fn build_sjk(&mut self, view: View<'_>, viewport: [f32; 2]) {
        #[cfg(test)]
        let view = match self.shot_view {
            Some(shot) => View {
                team: shot.team,
                team_game: shot.team_game,
                red_players: shot.red_players,
                blue_players: shot.blue_players,
                vote_active: shot.vote_active,
                ..view
            },
            None => view,
        };
        self.prepare_rows(&view);
        self.active_page = view.page;
        let count = self.row_count;
        let rows = sjk_view::Rows {
            labels: &self.rows[..count],
            hints: &self.hints[..count],
            enabled: &self.enabled[..count],
        };
        sjk_view::build(
            &mut self.canvas,
            &view,
            &rows,
            &sjk_view::Sides {
                card: &self.card,
                players: &self.players,
            },
            &mut self.motion,
            viewport,
        );
    }

    pub(crate) fn append(
        &mut self,
        view: View<'_>,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        if view.page == Page::Shot {
            self.active_page = Page::Shot;
            self.shot.build(&mut self.canvas, viewport);
            self.canvas.append_text(vertices, font, viewport);
            return;
        }
        self.prepare_rows(&view);
        self.active_page = view.page;
        let rows = view::Rows {
            labels: &self.rows[..self.row_count],
            enabled: &self.enabled[..self.row_count],
            scroll: self.callvote.scroll_metrics(view.page),
            info: info_lines(&self.about, &self.players, view.page),
        };
        if self.is_classic() {
            classic_view::build(&mut self.canvas, &view, rows, self.art, viewport);
        } else {
            view::build(
                &mut self.canvas,
                &view,
                &self.kicker,
                rows,
                hint_for(&view),
                viewport,
            );
        }
        self.canvas.append_text(vertices, font, viewport);
    }

    pub(crate) fn draw_list(&self) -> &DrawList {
        self.canvas.draw_list()
    }
    pub(crate) fn navigate(&mut self, action: AbstractAction) -> Option<usize> {
        if self.active_page.is_vote_page() {
            return None;
        }
        self.canvas.action(action).map(usize::from)
    }
    pub(crate) fn pointer(&mut self, event: InputEvent) -> Option<(UiEventKind, usize)> {
        let wheel = match event {
            InputEvent::PointerWheel { delta, .. } => Some(delta),
            _ => None,
        };
        let event = self.canvas.pointer(event);
        if let Some(delta) = wheel
            && self.active_page.is_vote_page()
        {
            self.callvote
                .scroll(self.active_page, -delta.y.signum() as isize * 3);
            return None;
        }
        let event = event?;
        if event.kind == UiEventKind::Drag && event.token == Some(VOTE_SCROLL_TOKEN) {
            if let (Some(point), Some(track)) =
                (event.position, self.canvas.rect_for(VOTE_SCROLL_TOKEN))
            {
                let ratio = ((point.y - track.y) / track.height).clamp(0.0, 1.0);
                self.callvote.scroll_to(self.active_page, ratio);
            }
            return None;
        }
        Some((event.kind, usize::from(event.token?)))
    }
    pub(crate) fn activation_allowed(&self, row: usize) -> bool {
        self.enabled.get(row).copied().unwrap_or(false)
    }

    /// Rebuild the server-info page from the live game state.
    pub(crate) fn refresh_about(&mut self, game_state: Option<&GameState>, address: Option<&str>) {
        self.about.refresh(game_state, address);
    }

    pub(crate) fn refresh_callvote(
        &mut self,
        game_state: Option<&GameState>,
        vfs: Option<&sjk_vfs::VirtualFileSystem>,
    ) {
        self.callvote.refresh(game_state, vfs);
    }

    pub(crate) fn callvote_action(&mut self, page: Page, row: usize) -> CallVoteAction {
        self.callvote.activate(page, row)
    }

    pub(crate) fn row_count(&self, page: Page, team_game: bool, vote_active: bool) -> usize {
        if matches!(page, Page::Players | Page::ReportPlayer) {
            self.players.row_count(page)
        } else if let Some(count) =
            classic::row_count(page, team_game).filter(|_| self.is_classic())
        {
            count
        } else if let Some(count) = sjk_view::row_count(page, team_game).filter(|_| self.is_sjk()) {
            count
        } else if page == Page::Siege {
            self.siege.row_count()
        } else if page.is_vote_page() {
            self.callvote.row_count(page)
        } else {
            row_count(page, team_game, vote_active)
        }
    }

    fn prepare_rows(&mut self, view: &View<'_>) {
        self.kicker.clear();
        self.kicker.push_str(match view.team {
            1 => "JEDI ACADEMY   /   RED TEAM",
            2 => "JEDI ACADEMY   /   BLUE TEAM",
            3 => "JEDI ACADEMY   /   SPECTATING",
            _ => "JEDI ACADEMY   /   PLAYING",
        });
        for row in &mut self.rows {
            row.clear();
        }
        for hint in &mut self.hints {
            hint.clear();
        }
        self.enabled = [true; 24];
        let own_rows = if self.is_classic() {
            classic::prepare(view, &mut self.rows, &mut self.enabled)
        } else if self.is_sjk() {
            sjk_view::prepare(
                view,
                &mut self.rows,
                &mut self.hints,
                &mut self.enabled,
                &self.card,
            )
        } else {
            None
        };
        self.row_count = if let Some(count) = own_rows {
            count
        } else if matches!(view.page, Page::Players | Page::ReportPlayer) {
            self.players.prepare(
                view.page,
                &mut self.rows,
                &mut self.hints,
                &mut self.enabled,
            )
        } else if view.page == Page::Siege {
            self.siege.prepare(&mut self.rows)
        } else if view.page.is_vote_page() {
            self.callvote.prepare_rows(view.page, &mut self.rows)
        } else {
            self.prepare_standard_rows(view)
        };
        for row in 0..self.row_count {
            let current = current_team_row(view, row);
            self.enabled[row] &= !current;
            // The modern list says so in the row; the classic pop-up dims it
            // and the SJK UI says so under it.
            if current && !self.style.classic_screens() {
                self.rows[row].push_str("  /  current");
            }
        }
        if self.is_sjk() {
            sjk_view::split_hints(
                view.page,
                &mut self.rows[..self.row_count],
                &mut self.hints[..self.row_count],
            );
        }
    }

    fn prepare_standard_rows(&mut self, view: &View<'_>) -> usize {
        match view.page {
            Page::Main => {
                let mut count = 0;
                for value in [
                    "Resume",
                    "Join / change team",
                    "Server browser",
                    "Player profile",
                    "SJK",
                    "Server info",
                    "Settings",
                    "Controls",
                ] {
                    self.rows[count].push_str(value);
                    count += 1;
                }
                self.rows[count].push_str("Call vote");
                count += 1;
                if view.vote_active {
                    self.rows[count].push_str("Vote yes");
                    self.rows[count + 1].push_str("Vote no");
                    count += 2;
                }
                self.rows[count].push_str(CAMERA_CONTROL);
                count += 1;
                self.rows[count].push_str("Leave");
                count + 1
            }
            Page::About => {
                self.rows[0].push_str("Back");
                1
            }
            Page::Sjk => {
                for (row, entry) in sjk::ENTRIES.iter().enumerate() {
                    self.rows[row].push_str(entry.label);
                }
                self.rows[sjk::ENTRIES.len()].push_str("Back");
                sjk::ENTRIES.len() + 1
            }
            Page::Leave => {
                self.rows[0].push_str("Disconnect  /  back to the main menu");
                self.rows[1].push_str("Quit Sol JK");
                self.rows[2].push_str("Back");
                3
            }
            Page::Team => {
                if view.team_game {
                    let _ = write!(
                        self.rows[0],
                        "Auto-join  /  {} red / {} blue",
                        view.red_players, view.blue_players
                    );
                    let _ = write!(self.rows[1], "Red team  /  {} players", view.red_players);
                    let _ = write!(self.rows[2], "Blue team  /  {} players", view.blue_players);
                    self.rows[3].push_str("Spectate");
                    self.rows[4].push_str("Back");
                    5
                } else {
                    self.rows[0].push_str("Join game");
                    self.rows[1].push_str("Spectate");
                    self.rows[2].push_str("Back");
                    3
                }
            }
            _ => 0,
        }
    }
}

#[cfg(test)]
impl InGameMenu {
    /// The SJK UI's in-game menu over a made-up match: `card` for the match
    /// card, `view` in place of the frame's facts (world shots and tests).
    pub(crate) fn sjk_for_shot(&mut self, card: sjk_view::Card, view: ShotView) {
        self.style = MenuStyle::Sjk;
        self.card = card;
        self.shot_view = Some(view);
    }
}

impl Page {
    pub(crate) const fn is_vote_page(self) -> bool {
        matches!(
            self,
            Self::CallVote
                | Self::VoteMap
                | Self::VoteGameType
                | Self::VoteKick
                | Self::VoteClientKick
                | Self::VoteWarmup
                | Self::VoteTimeLimit
                | Self::VoteFragLimit
        )
    }
}

pub(crate) fn weapon_name(weapon: u8) -> &'static str {
    match weapon {
        0 => "Unarmed",
        1 => "Stun Baton",
        2 => "Melee",
        3 => "Lightsaber",
        4 => "Blaster Pistol",
        5 => "E-11 Blaster Rifle",
        6 => "Disruptor Rifle",
        7 => "Bowcaster",
        8 => "Heavy Repeater",
        9 => "DEMP2",
        10 => "Flechette",
        11 => "Rocket Launcher",
        12 => "Thermal Detonator",
        13 => "Trip Mine",
        14 => "Det Pack",
        15 => "Concussion Rifle",
        16 => "Bryar Pistol (legacy)",
        17 => "Emplaced Gun",
        18 => "Turret",
        _ => "Unknown weapon",
    }
}

pub(crate) fn row_count(page: Page, team_game: bool, vote_active: bool) -> usize {
    match page {
        Page::Main => {
            if vote_active {
                13
            } else {
                11
            }
        }
        Page::Team if team_game => 5,
        Page::Team => 3,
        Page::About => 1,
        Page::Sjk => sjk::ENTRIES.len() + 1,
        Page::Leave => 3,
        _ => 0,
    }
}

/// Read-only lines above the entries of `page` (the server-info page, and the
/// Players and Report pages' who and why).
fn info_lines<'a>(
    about: &'a about::State,
    players: &'a players::State,
    page: Page,
) -> &'a [String] {
    match page {
        Page::About => about.lines(),
        Page::Players | Page::ReportPlayer => players.info(page),
        _ => &[],
    }
}

/// Whether `row` of the join page is the team the player is already on.
fn current_team_row(view: &View<'_>, row: usize) -> bool {
    if view.page != Page::Team {
        return false;
    }
    if view.team_game {
        matches!((row, view.team), (1, 1) | (2, 2) | (3, 3))
    } else {
        matches!((row, view.team), (0, 0..=2) | (1, 3))
    }
}

/// Count populated red/blue clientinfo records for the join modal.
pub(crate) fn team_sizes(game_state: &GameState) -> [usize; 2] {
    let mut sizes = [0; 2];
    for client in 0..32 {
        let Some(bytes) = game_state.config_string(1_131 + client) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        let Ok(info) = InfoString::parse(text) else {
            continue;
        };
        match info.get_i32("t") {
            Some(1) => sizes[0] += 1,
            Some(2) => sizes[1] += 1,
            _ => {}
        }
    }
    sizes
}

#[cfg(test)]
mod sjk_tests {
    use super::*;

    fn view(page: Page, selected_row: usize, team_game: bool, team: u8) -> View<'static> {
        View {
            page,
            selected_row,
            team,
            team_game,
            siege: false,
            red_players: 4,
            blue_players: 3,
            vote_active: false,
            _frame: std::marker::PhantomData,
        }
    }

    fn sjk_menu() -> InGameMenu {
        let mut menu = InGameMenu::new();
        menu.set_style(MenuStyle::Sjk, ArtSet::default());
        menu
    }

    #[test]
    fn each_style_keeps_its_own_menu() {
        let mut menu = InGameMenu::new();
        for (style, classic, sjk, main_rows) in [
            (MenuStyle::Classic, true, false, classic::Tab::ALL.len()),
            (MenuStyle::Modern, false, false, 11),
            (MenuStyle::Sjk, false, true, sjk_view::Entry::MAIN.len()),
        ] {
            menu.set_style(style, ArtSet::default());
            assert_eq!(
                (menu.is_classic(), menu.is_sjk()),
                (classic, sjk),
                "{style:?}"
            );
            assert_eq!(menu.row_count(Page::Main, false, false), main_rows);
        }
        // Pages the SJK UI shares keep the shared row counts.
        let menu = sjk_menu();
        assert_eq!(
            menu.row_count(Page::CallVote, false, false),
            menu.callvote.row_count(Page::CallVote)
        );
        assert_eq!(menu.row_count(Page::Vote, false, true), 4);
    }

    #[test]
    fn every_page_draws_its_rows_within_the_canvas() {
        let mut game = GameState::empty_local(0);
        let maps: String = (0..40).map(|index| format!("mp/map{index} ")).collect();
        game.replace_config_string(0, format!("\\sv_maplist\\{maps}").into_bytes())
            .expect("serverinfo");
        let mut menu = sjk_menu();
        menu.refresh_callvote(Some(&game), None);
        menu.card = sjk_view::Card::for_shot(true, false);
        for viewport in [[1_920.0, 1_080.0], [3_840.0, 2_160.0], [1_024.0, 768.0]] {
            for (page, team_game) in [
                (Page::Main, false),
                (Page::Team, true),
                (Page::Team, false),
                (Page::Vote, false),
                (Page::CallVote, false),
                (Page::VoteMap, false),
                (Page::VoteGameType, false),
                (Page::Sjk, false),
                (Page::Leave, false),
            ] {
                let count = menu.row_count(page, team_game, false);
                assert!(count > 0, "{page:?}");
                menu.build_sjk(view(page, count - 1, team_game, 2), viewport);
                assert_eq!(menu.row_count, count, "{page:?}: prepared rows");
                assert!(!menu.canvas.overflowed(), "{page:?} {viewport:?}");
                for row in 0..count {
                    assert!(
                        menu.canvas.rect_for(row as u16).is_some(),
                        "{page:?} row {row}"
                    );
                }
                assert!(menu.canvas.rect_for(count as u16).is_none(), "{page:?}");
            }
        }
        // Forty maps page through sixteen at a time, with More and Back.
        assert_eq!(menu.row_count(Page::VoteMap, false, false), 18);
        menu.build_sjk(view(Page::VoteMap, 0, false, 0), [1_920.0, 1_080.0]);
        assert_eq!(menu.rows[16], "More maps...");
    }

    #[test]
    fn the_arc_passes_over_rows_that_cannot_be_taken() {
        let mut menu = sjk_menu();
        // On blue in a team game: Blue team cannot be taken.
        menu.build_sjk(view(Page::Team, 1, true, 2), [1_920.0, 1_080.0]);
        assert!(!menu.activation_allowed(2));
        assert_eq!(menu.sjk_step(Page::Team, 1, 5, true), 3);
        assert_eq!(menu.sjk_step(Page::Team, 3, 5, false), 1);
        assert_eq!(menu.sjk_step(Page::Team, 4, 5, true), 0, "wraps");
        // No vote on: the ballot is passed over, Call a vote is reached.
        menu.build_sjk(view(Page::Vote, 2, false, 0), [1_920.0, 1_080.0]);
        assert_eq!(menu.sjk_step(Page::Vote, 3, 4, true), 2);
        assert!(!menu.activation_allowed(0));
        // Another page than the one drawn: every row counts.
        assert_eq!(menu.sjk_step(Page::Main, 0, 9, true), 1);
        assert_eq!(menu.sjk_step(Page::Main, 0, 0, true), 0);
    }

    #[test]
    fn every_style_calls_the_panel_camera_control() {
        for style in [MenuStyle::Modern, MenuStyle::Sjk] {
            let mut menu = InGameMenu::new();
            menu.set_style(style, ArtSet::default());
            for vote_active in [false, true] {
                let mut main = view(Page::Main, 0, false, 0);
                main.vote_active = vote_active;
                menu.prepare_rows(&main);
                let rows = &menu.rows[..menu.row_count];
                // The row the main page's actions open the panel from.
                let row = match style {
                    MenuStyle::Sjk => sjk_view::Entry::Shot.index(),
                    _ if vote_active => 11,
                    _ => 9,
                };
                assert_eq!(rows[row], CAMERA_CONTROL, "{style:?} {vote_active}");
                assert!(
                    rows.iter()
                        .all(|row| !row.to_ascii_lowercase().contains("shot")),
                    "{style:?}: {rows:?}"
                );
            }
        }
        // The classic bar has no entry: F8 opens the panel there.
        let mut classic = InGameMenu::new();
        classic.set_style(MenuStyle::Classic, ArtSet::default());
        classic.prepare_rows(&view(Page::Main, 0, false, 0));
        assert!(
            classic.rows[..classic.row_count]
                .iter()
                .all(|row| !row.to_ascii_lowercase().contains("shot"))
        );
    }

    #[test]
    fn a_screen_opened_from_the_main_page_returns_to_its_entry() {
        let mut menu = sjk_menu();
        menu.remember_return(sjk_view::Entry::Settings.index());
        assert_eq!(menu.return_row(), sjk_view::Entry::Settings.index());
        assert_eq!(menu.return_row(), 0, "taken once");
        menu.set_style(MenuStyle::Modern, ArtSet::default());
        menu.remember_return(4);
        assert_eq!(menu.return_row(), 0, "the other looks open on the first");
    }
}
