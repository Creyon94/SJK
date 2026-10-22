//! End-to-end encrypted whispers (`docs/hub-chat.md`, "Encrypted whispers"): a
//! whisper to another SJK player goes through the SJK hub sealed to their identity key
//! (`sjk_identity::whisper`), so only the two of them can read it, while
//! `cl_e2eWhispers` is on (the default).
//!
//! A `tell` (the composer's whisper, `tell`, `reply`) to a player whose claim on the
//! server carries a whisper key is sealed and handed to the hub instead of the game
//! server ([`GpuState::route_encrypted_whisper`]). When the hub takes it, the chat feed
//! and the console show it in its pink, labelled ENCRYPTED; when it does not, the card
//! ([`e2e_whisper_card`](crate::e2e_whisper_card)) says why and asks before sending it
//! unencrypted through the game server. Whispers that come in are shown the same way,
//! with their own sound, and `reply` answers them.
//!
//! Each player's keys are remembered in `whisper-keys.txt` beside `config.cfg`
//! ([`sjk_identity::whisper::KeyPins`]): a key not seen before for a player is said in
//! the console, since a lying hub could hand out its own.

use crate::e2e_whisper_card::{Choice, Failure};
use crate::{GpuState, player_identity};
use sjk_identity::whisper::{KeyPins, OpenError, Outgoing, PinCheck, Received};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// `cl_e2eWhispers`: whether whispers to SJK players go encrypted through the hub.
pub(crate) const CVAR: &str = "cl_e2eWhispers";
/// The remembered keys, in the settings folder.
const PINS_FILE: &str = "whisper-keys.txt";
/// How long a sent whisper may wait for the hub's answer before it counts as failed.
const ANSWER_WAIT: Duration = Duration::from_secs(10);

/// A whisper handed to the hub, until it answers.
struct Pending {
    serial: u64,
    server: SocketAddr,
    /// The recipient's name as the game showed it.
    name: String,
    text: String,
    /// The `tell` the game server would have had, for sending it unencrypted.
    command: String,
    sent: Instant,
}

#[derive(Default)]
pub(crate) struct State {
    serial: u64,
    pending: Vec<Pending>,
    pins: Option<(PathBuf, KeyPins)>,
    pub(crate) card: crate::e2e_whisper_card::Card,
    /// The card opened the game menu, which goes with it.
    opened_menu: bool,
    /// The server and identity revision the slots that take whispers were found for.
    slots_for: Option<(SocketAddr, u64)>,
}

impl State {
    /// Whether `key` is one seen for `key_id` (remembering it), reading and writing the
    /// file in `directory`. A key seen before, or the first for a player, is no news.
    fn pin(&mut self, directory: &Path, key_id: &str, key: &str) -> PinCheck {
        let path = directory.join(PINS_FILE);
        if self.pins.as_ref().is_none_or(|(at, _)| *at != path) {
            let pins = std::fs::read_to_string(&path)
                .map(|text| KeyPins::parse(&text))
                .unwrap_or_default();
            self.pins = Some((path.clone(), pins));
        }
        let (_, pins) = self.pins.as_mut().expect("loaded above");
        let check = pins.check(key_id, key);
        if check != PinCheck::Known {
            let _ = std::fs::write(&path, pins.to_text());
        }
        check
    }
}

/// The console line telling that `name`'s key is new on this PC.
fn key_changed_line(name: &str) -> String {
    format!(
        "^3Encrypted whispers: {name}^3 uses a key this PC has not seen for them before (another \
         PC of theirs, a new key, or the SJK hub handing out its own)."
    )
}

/// How a whisper's text shows: EternalJK's escapes for `%` and `"` as what was typed,
/// as for every chat line.
fn shown_text(text: &str) -> String {
    sjk_client::chat_display_text(&sjk_client::chat_unescape(text))
}

/// The whisper's text: what follows `tell <slot>`, unquoted as the server reads it.
fn whisper_text(command: &str) -> Option<(u16, String)> {
    let tokens = sjk_shell::tokenize(command).ok()?;
    let [verb, slot, body @ ..] = tokens.as_slice() else {
        return None;
    };
    let text = body.join(" ");
    (verb.eq_ignore_ascii_case("tell") && !text.trim().is_empty())
        .then(|| Some((slot.parse().ok()?, text)))
        .flatten()
}

impl GpuState {
    /// Send `command`, a `tell`, encrypted through the hub when its recipient takes
    /// encrypted whispers and `cl_e2eWhispers` is on: true when it went that way, and
    /// the game server must not have it.
    pub(crate) fn route_encrypted_whisper(&mut self, command: &str) -> bool {
        let Some(console) = self.console.as_ref() else {
            return false;
        };
        if console.bool_cvar(CVAR) == Some(false) || console.bool_cvar("cl_identity") != Some(true)
        {
            return false;
        }
        let directory = console.config_directory().to_owned();
        let Some((slot, text)) = whisper_text(command) else {
            return false;
        };
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            return false;
        };
        let game_state = session.game_state();
        if u16::try_from(game_state.client_num).ok() == Some(slot)
            || text.len() > sjk_identity::whisper::TEXT_MAX
        {
            return false;
        }
        let server = session.server();
        let Some(shown) = player_identity::shown_name(game_state, usize::from(slot)) else {
            return false;
        };
        let Ok(target) = u8::try_from(slot) else {
            return false;
        };
        let Some((key_id, key)) = player_identity::with_snapshot(|snapshot| {
            let player = snapshot.badge(target, &shown)?;
            Some((player.key_id.clone(), player.whisper_key.clone()?))
        })
        .flatten() else {
            return false;
        };
        self.e2e_whispers.serial += 1;
        let serial = self.e2e_whispers.serial;
        let sent = player_identity::send_whisper(Outgoing {
            serial,
            server: server.to_string(),
            target,
            to: key.clone(),
            text: text.clone(),
        });
        if !sent {
            return false;
        }
        if self.e2e_whispers.pin(&directory, &key_id, &key) == PinCheck::Changed
            && let Some(console) = self.console.as_mut()
        {
            console.push_log(key_changed_line(&shown));
        }
        self.e2e_whispers.pending.push(Pending {
            serial,
            server,
            name: shown,
            text: shown_text(&text),
            command: command.to_owned(),
            sent: Instant::now(),
        });
        true
    }

    /// Once a frame: listen for encrypted whispers while on a server with them on, show
    /// what the hub did with ours and the ones that came, and tell the chat which slots
    /// take them (`due`: the identity's twice-a-second turn).
    pub(crate) fn update_e2e_whispers(&mut self) {
        let on = self.console.as_ref().is_some_and(|console| {
            console.bool_cvar(CVAR) != Some(false) && console.bool_cvar("cl_identity") == Some(true)
        });
        let server = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .map(|session| session.server());
        player_identity::set_whisper_listen(on && server.is_some());
        self.update_encrypted_slots(server.filter(|_| on));
        self.e2e_whisper_answers(server);
        let received = player_identity::take_whispers(server);
        for whisper in received {
            self.show_received_whisper(whisper);
        }
        // A failure for a server the player left cannot be sent unencrypted there.
        self.e2e_whispers.card.keep_for(server);
    }

    /// Tell the chat which slots' players take encrypted whispers, when the known
    /// players or the server changed.
    fn update_encrypted_slots(&mut self, server: Option<SocketAddr>) {
        let Some(server) = server else {
            if self.e2e_whispers.slots_for.take().is_some() {
                self.chat.set_encrypted_slots(0);
            }
            return;
        };
        let revision = player_identity::revision();
        if self.e2e_whispers.slots_for == Some((server, revision)) {
            return;
        }
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let game_state = session.game_state();
        let slots = player_identity::with_snapshot(|snapshot| {
            let mut slots = 0_u64;
            for player in snapshot.players.iter().filter(|p| p.whisper_key.is_some()) {
                let shown = player_identity::shown_name(game_state, usize::from(player.slot));
                if player.slot < 64
                    && shown.is_some_and(|shown| {
                        sjk_identity::names_match(&player.claimed_name, &shown)
                    })
                {
                    slots |= 1 << player.slot;
                }
            }
            slots
        })
        .unwrap_or(0);
        self.chat.set_encrypted_slots(slots);
        self.e2e_whispers.slots_for = Some((server, revision));
    }

    /// What the hub did with the whispers handed to it: shown when taken, else the card.
    fn e2e_whisper_answers(&mut self, server: Option<SocketAddr>) {
        let answers = player_identity::take_whispers_sent();
        let now = Instant::now();
        let mut done: Vec<(Pending, Result<(), String>)> = Vec::new();
        for answer in answers {
            if let Some(index) = self
                .e2e_whispers
                .pending
                .iter()
                .position(|pending| pending.serial == answer.serial)
            {
                done.push((self.e2e_whispers.pending.remove(index), answer.result));
            }
        }
        // No answer in time (the service stopped): failed.
        while let Some(index) = self
            .e2e_whispers
            .pending
            .iter()
            .position(|pending| now.duration_since(pending.sent) >= ANSWER_WAIT)
        {
            done.push((
                self.e2e_whispers.pending.remove(index),
                Err("the SJK hub did not answer".to_owned()),
            ));
        }
        for (pending, result) in done {
            match result {
                Ok(()) => {
                    self.chat
                        .receive_encrypted(None, &pending.name, &pending.text, now);
                    if let Some(console) = self.console.as_mut() {
                        let line = format!("^6To {}^6 (encrypted): {}", pending.name, pending.text);
                        console.push_chat_line(line.clone(), &line, None);
                    }
                }
                Err(why) if Some(pending.server) == server => {
                    self.e2e_whispers.card.offer(Failure {
                        server: pending.server,
                        name: pending.name,
                        text: pending.text,
                        command: pending.command,
                        why,
                    });
                    self.open_e2e_whisper_card();
                }
                // Written on a server the player has left: said, never sent.
                Err(why) => {
                    if let Some(console) = self.console.as_mut() {
                        console.push_log(format!(
                            "^3Your encrypted whisper to {}^3 did not go ({why}).",
                            pending.name
                        ));
                    }
                }
            }
        }
    }

    /// Show a whisper that came in: from the player the game shows in its slot under
    /// the name its claim was made with, never from one blocked here.
    fn show_received_whisper(&mut self, whisper: Received) {
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let game_state = session.game_state();
        let own = u16::try_from(game_state.client_num).ok();
        let slot = u16::from(whisper.slot);
        let Some(shown) = player_identity::shown_name(game_state, usize::from(whisper.slot))
            .filter(|shown| sjk_identity::names_match(&whisper.claimed_name, shown))
        else {
            return;
        };
        if Some(slot) == own || crate::player_mutes::is_muted(Some(&whisper.key_id), &shown) {
            return;
        }
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let directory = console.config_directory().to_owned();
        let sound = crate::cgame_options::chat_sounds(Some(console));
        let text = match whisper.text {
            Ok(text) => shown_text(&text),
            Err(failure) => {
                let why = match failure {
                    OpenError::Unreadable => "it was not sealed to your key",
                    OpenError::Forged => "it was not signed by them",
                    OpenError::Stale => "it is too old, or a clock is wrong",
                };
                if let Some(console) = self.console.as_mut() {
                    console.push_log(format!(
                        "^3An encrypted whisper from {shown}^3 was thrown away: {why}."
                    ));
                }
                return;
            }
        };
        let changed = self
            .e2e_whispers
            .pin(&directory, &whisper.key_id, &whisper.from)
            == PinCheck::Changed;
        if !self
            .chat
            .receive_encrypted(Some(slot), "", &text, Instant::now())
        {
            return;
        }
        if let Some(console) = self.console.as_mut() {
            if changed {
                console.push_log(key_changed_line(&shown));
            }
            let line = format!("^6{shown}^6 (encrypted): {text}");
            console.push_chat_line(line.clone(), &line, None);
        }
        if sound {
            crate::audio::ui_cues::post(crate::audio::ui_cues::Cue::EncryptedWhisper);
        }
    }

    /// Show the card over the game menu, opening it when it is not.
    fn open_e2e_whisper_card(&mut self) {
        let menu_visible = self
            .client_menu
            .as_ref()
            .is_some_and(crate::menu::ClientMenu::is_visible);
        if self.chat.is_typing() {
            self.chat.close_composer();
        }
        if !menu_visible && self.live_session.is_some() && !self.game_menu {
            self.release_pointer();
            self.game_menu = true;
            self.game_menu_page = crate::ingame_menu::Page::Main;
            self.game_menu_row = 0;
            self.e2e_whispers.opened_menu = true;
        }
        self.sync_cursor_policy();
    }

    /// Whether the card shows this frame: a failure to answer, and the player on the
    /// game menu with nothing else over it. `covered` is another card or the console.
    pub(crate) fn prepare_e2e_whisper_card(&mut self, covered: bool) -> bool {
        let console_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::is_open);
        let menu_visible = self
            .client_menu
            .as_ref()
            .is_some_and(crate::menu::ClientMenu::is_visible);
        let open = self.e2e_whispers.card.wants()
            && (self.game_menu || menu_visible)
            && !covered
            && !console_open
            && !self.text_dialog.is_open()
            && self.world_load_task.is_none()
            && self.world_install_task.is_none()
            && !self.pending_map_reload;
        self.e2e_whispers.card.set_open(open);
        open
    }

    /// Draw the card. It darkens everything, so every font batch appended before it is
    /// dropped (text draws above all shapes).
    pub(crate) fn append_e2e_whisper_card(&mut self, viewport: [f32; 2]) {
        self.text_vertices.clear();
        self.classic_text_vertices.clear();
        self.game_fonts.clear_text();
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        self.e2e_whispers.card.append(target, viewport);
    }

    /// Act on the player's choice: send the whisper unencrypted, or not at all. The
    /// game menu the card opened closes once no failure is left.
    pub(crate) fn e2e_whisper_card_choice(&mut self, choice: Option<Choice>) {
        let Some(choice) = choice else {
            return;
        };
        if let Choice::SendPlain(failure) = choice {
            self.send_chat_to_server(&failure.command, Some(failure.name));
        }
        if !self.e2e_whispers.card.wants() && std::mem::take(&mut self.e2e_whispers.opened_menu) {
            self.game_menu = false;
            self.sync_cursor_policy();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_is_what_follows_tell_and_its_slot() {
        assert_eq!(
            whisper_text("tell 7 \"^7hello there\""),
            Some((7, "^7hello there".to_owned()))
        );
        assert_eq!(
            whisper_text("tell 7 hello there"),
            Some((7, "hello there".to_owned()))
        );
        assert_eq!(whisper_text("TELL 12 x"), Some((12, "x".to_owned())));
        for other in ["say hi", "tell bob hi", "tell 7", "tell 7 \"  \""] {
            assert_eq!(whisper_text(other), None, "{other}");
        }
    }

    /// Encrypted whispers among the game's chat lines, and the card when one fails, over
    /// duel6, rendered off screen at 1080p: `JKA_GAME_DATA=<GameData> cargo test -p
    /// sjk-viewer encrypted_whisper_shots -- --ignored --nocapture`.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn encrypted_whisper_shots() {
        crate::world_shot::on_big_stack(|| {
            let menu = crate::menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
                ("cg_chatbox", "60000"),
                ("cg_chatboxlines", "8"),
            ];
            let Some((mut gpu, _profile)) =
                crate::world_shot::open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
            else {
                return;
            };
            let _ = crate::world_shot::frame(&mut gpu, 10);
            // The card, when the hub could not take a whisper.
            gpu.e2e_whispers.card.preview = true;
            gpu.e2e_whispers.card.offer(Failure {
                server: "127.0.0.1:29070".parse().unwrap(),
                name: "^5Creyon".to_owned(),
                text: "on my way, only you can read this".to_owned(),
                command: "tell 4 \"on my way\"".to_owned(),
                why: "the SJK hub could not be reached".to_owned(),
            });
            let path = crate::world_shot::shoot(&mut gpu, 6, "encrypted-whisper-card");
            println!("{}", path.display());
            assert!(gpu.e2e_whispers.card.is_open(), "the card shows");
        });
        crate::world_shot::on_big_stack(|| {
            let Some((mut gpu, _profile)) = crate::world_shot::open(
                "maps/mp/duel6.bsp",
                [1920, 1080],
                None,
                &[("cg_chatbox", "60000"), ("cg_chatboxlines", "8")],
            ) else {
                return;
            };
            gpu.chat.for_shot = true;
            let mut game = sjk_protocol::GameState::empty_local(0);
            for (slot, name) in [(0, "^1Sol^7Vulpes"), (4, "^5Creyon"), (6, "Padawan^1Fox")] {
                game.replace_config_string(1131 + slot, format!("n\\{name}\\t\\0").into_bytes())
                    .unwrap();
            }
            gpu.chat.update_roster(&game);
            let now = Instant::now();
            gpu.chat.receive(
                sjk_client::ServerEventKind::Chat,
                "Padawan^1Fox^7: ^2gg".to_owned(),
                Some(6),
                now,
            );
            gpu.chat.receive(
                sjk_client::ServerEventKind::Chat,
                "\u{19}[Padawan^1Fox^7\u{19}]: ^6a plain whisper, through the server".to_owned(),
                Some(6),
                now,
            );
            gpu.chat
                .receive_encrypted(Some(4), "", "meet me at the bridge, bring the hilt", now);
            gpu.chat
                .receive_encrypted(None, "^5Creyon", "on my way, only you can read this", now);
            let path = crate::world_shot::shoot(&mut gpu, 10, "encrypted-whispers-chat");
            println!("{}", path.display());
        });
    }

    #[test]
    fn keys_are_remembered_in_the_settings_folder() {
        let directory = tempfile::tempdir().unwrap();
        let key = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
        let other = "HxweHRwbGhkYFxYVFBMSERAPDg0MCwoJCAcGBQQDAgE";
        let mut state = State::default();
        assert_eq!(state.pin(directory.path(), "fox", key), PinCheck::First);
        // Read back by another run.
        let mut later = State::default();
        assert_eq!(later.pin(directory.path(), "fox", key), PinCheck::Known);
        assert_eq!(later.pin(directory.path(), "fox", other), PinCheck::Changed);
        assert!(
            std::fs::read_to_string(directory.path().join(PINS_FILE))
                .unwrap()
                .contains(other)
        );
    }
}
