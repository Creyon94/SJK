//! Bug reports for the SJK team: the Report a bug button under the game menu (or Escape,
//! SJK, Report a bug) opens the text dialog (`text_dialog`); Send hands the text to the
//! hub (`cl_hubUrl`) through the identity service, signed with the player's key, with the
//! map, the build and the server.
//!
//! The text keeps to the hub's rules as it is typed (letters, digits, spaces and
//! `. , ! ? ' - : ( )`, at most 600 characters) and is checked again here
//! before it leaves; the hub checks it a third time and limits how often a key and an
//! address may report (`PROTOCOL.md` in the hub, "Bug reports"). The outcome comes back
//! as a centre print.

use sjk_client::ServerEventKind;
use std::time::Instant;

impl crate::GpuState {
    /// Report a bug: close the game menu and open the report dialog.
    pub(crate) fn open_bug_report(&mut self) {
        self.game_menu = false;
        self.game_menu_page = crate::ingame_menu::Page::Main;
        self.game_menu_row = 0;
        self.gameplay_input.release_keys();
        self.text_dialog.open(crate::text_dialog::Kind::Report);
        self.sync_cursor_policy();
    }

    /// Carry out what the text dialog asked for.
    pub(crate) fn apply_dialog_action(&mut self, action: crate::text_dialog::Action) {
        use crate::text_dialog::{Action, Kind};
        match action {
            Action::None => return,
            Action::Cancel => self.world_notes.composer_closed(),
            Action::Send(Kind::Note { .. }, text) => self.save_world_note(&text),
            Action::Send(Kind::Report, text) => self.send_bug_report(&text),
        }
        self.sync_cursor_policy();
    }

    /// The report line's Enter: check the text and hand it to the identity service.
    pub(crate) fn send_bug_report(&mut self, text: &str) {
        let message = match sjk_identity::report::text(text) {
            Err(why) => format!("Bug report not sent: {why}"),
            Ok(text) => {
                let map = if self.resident.exploring() {
                    self.resident.map.clone()
                } else {
                    self.world_load_map.clone()
                };
                let server = self
                    .live_session
                    .as_ref()
                    .filter(|session| !session.is_local())
                    .map(|session| session.server().to_string())
                    .unwrap_or_default();
                let report = sjk_identity::BugReport {
                    text,
                    map,
                    build: crate::build_info::VERSION.to_owned(),
                    server,
                };
                if crate::player_identity::report(report) {
                    self.bug_report_waiting = true;
                    "Sending the bug report...".to_owned()
                } else {
                    "Bug report not sent: the SJK identity is off (cl_identity 1 turns it on)"
                        .to_owned()
                }
            }
        };
        self.chat
            .receive(ServerEventKind::CenterPrint, message, None, Instant::now());
    }

    /// Once a frame: show the outcome of a report sent with [`Self::send_bug_report`].
    pub(crate) fn poll_bug_report(&mut self) {
        if !self.bug_report_waiting {
            return;
        }
        let Some(outcome) = crate::player_identity::report_outcome()
            .filter(|outcome| outcome.serial != self.bug_report_serial)
        else {
            return;
        };
        self.bug_report_serial = outcome.serial;
        self.bug_report_waiting = false;
        let message = if outcome.sent {
            format!("Bug report sent ({}). Thank you!", outcome.message)
        } else {
            format!("Bug report not sent: {}", outcome.message)
        };
        crate::log::progress(format_args!("{message}"));
        self.chat
            .receive(ServerEventKind::CenterPrint, message, None, Instant::now());
    }
}
