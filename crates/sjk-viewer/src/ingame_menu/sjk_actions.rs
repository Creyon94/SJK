//! What the SJK UI's in-game menu ([`super::sjk_view`]) does where it differs
//! from the modern menu: its main page's entries, its Vote and Leave pages,
//! Escape returning to the entry that opened a page, and the match card read
//! from the live session. Siege's classes, the call-vote lists, Team's rows and
//! Sol JK's keep the shared actions (`game_menu_actions.rs`).

use super::Page;
use super::sjk_view::{self, Entry, Local, leave, vote};
use crate::GpuState;
use sjk_client::ClientSession;

/// `persistant[PERS_SCORE]` and `persistant[PERS_RANK]`.
const PERS_SCORE: usize = 0;
const PERS_RANK: usize = 2;

impl GpuState {
    /// Activate the focused row of an SJK UI page whose rows are its own.
    /// Returns false for rows that act as in the modern menu.
    pub(crate) fn activate_sjk_ui_row(&mut self) -> bool {
        let row = self.game_menu_row;
        match self.game_menu_page {
            Page::Main => self.activate_sjk_entry(row),
            Page::Vote => match row {
                vote::YES | vote::NO => {
                    // As retail's vote pop-up: vote, and back to the match.
                    self.send_vote(row == vote::YES);
                    self.close_game_menu();
                }
                vote::CALL => self.open_call_vote(),
                _ => self.sjk_back(),
            },
            // Team's last row is Back; the others choose a side.
            Page::Team if row + 1 >= self.game_menu_row_count() => self.sjk_back(),
            Page::Leave => match row {
                leave::SERVER => self.disconnect_to_menu(),
                leave::QUIT => self.quit_requested = true,
                _ => self.sjk_back(),
            },
            Page::ConfirmLeave | Page::ConfirmQuit if row != 0 => self.sjk_back(),
            Page::ConfirmLeave => self.disconnect_to_menu(),
            Page::ConfirmQuit => self.quit_requested = true,
            Page::About => self.sjk_back(),
            page if page.is_vote_page() => self.activate_sjk_call_vote(page, row),
            _ => return false,
        }
        true
    }

    /// Take entry `row` of the SJK UI's main page.
    fn activate_sjk_entry(&mut self, row: usize) {
        let Some(entry) = Entry::at(row) else {
            return;
        };
        match entry {
            Entry::Resume => self.close_game_menu(),
            Entry::Team => {
                if !self.open_siege_classes() {
                    self.open_game_menu_page(Page::Team);
                }
            }
            Entry::Players => self.open_players_page(),
            Entry::Vote if self.vote_active() => self.open_game_menu_page(Page::Vote),
            Entry::Vote => self.open_call_vote(),
            Entry::Character => {
                self.in_game_menu.remember_return(row);
                self.open_player_menu_from_game();
            }
            Entry::Settings => {
                self.in_game_menu.remember_return(row);
                if let (Some(menu), Some(console)) = (&mut self.client_menu, &self.console) {
                    menu.open_sjk_settings_from_game(console);
                    self.game_menu = false;
                }
            }
            Entry::Servers => {
                self.in_game_menu.remember_return(row);
                self.open_browser_from_game();
            }
            Entry::Shot => self.open_shot_panel(),
            Entry::SolJk => self.open_game_menu_page(Page::Sjk),
            Entry::Leave => {
                // As the main page's Quit, it opens on Stay: its rows act at once.
                self.open_game_menu_page(Page::Leave);
                self.game_menu_row = leave::STAY;
            }
        }
    }

    /// A row of the call-vote page or one of its lists: open a list, call the
    /// vote and return to the match, or go back.
    fn activate_sjk_call_vote(&mut self, page: Page, row: usize) {
        match self.in_game_menu.callvote_action(page, row) {
            super::CallVoteAction::Open(list) => self.open_game_menu_page(list),
            super::CallVoteAction::Back => self.sjk_back(),
            super::CallVoteAction::Send(command) => {
                self.send_menu_reliable(&command);
                self.close_game_menu();
            }
            super::CallVoteAction::None => {}
        }
    }

    /// The call-vote page, its maps and players read now.
    fn open_call_vote(&mut self) {
        let game_state = self.live_session.as_ref().map(ClientSession::game_state);
        self.in_game_menu
            .refresh_callvote(game_state, self.vfs.as_deref());
        self.open_game_menu_page(Page::CallVote);
    }

    /// Escape in the SJK UI: back to the entry that opened the page, or out
    /// of the menu from the main page.
    pub(crate) fn sjk_back(&mut self) {
        if self.game_menu_page == Page::ReportPlayer {
            // Back on the reported player's row.
            self.back_to_players();
            return;
        }
        match sjk_view::parent(self.game_menu_page) {
            Some((page, row)) => {
                self.game_menu_page = page;
                self.game_menu_row = row;
            }
            None => self.close_game_menu(),
        }
    }

    /// Back to the match.
    fn close_game_menu(&mut self) {
        self.game_menu = false;
        self.capture_pointer();
    }

    /// Read the match card's facts from the live session this frame; without
    /// one (a map explored alone) the card hides.
    pub(crate) fn refresh_game_menu_card(&mut self) {
        let Some(session) = self.live_session.as_ref() else {
            self.in_game_menu.card.forget();
            return;
        };
        let snapshot = session.latest_snapshot();
        let player = &snapshot.player;
        let local = Local {
            team: player.team(),
            spectator: player.is_spectator(),
            score: player.persistent[PERS_SCORE] as i32,
            rank: player.persistent[PERS_RANK],
        };
        self.in_game_menu.card.refresh(
            session.game_state(),
            local,
            snapshot.server_time,
            session.team_scores(),
            self.last_connect_address.as_deref(),
        );
    }
}
