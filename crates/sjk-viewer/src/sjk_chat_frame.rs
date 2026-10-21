//! The per-frame hook of the SJK chat (`docs/hub-chat.md`): when the identity
//! service's chat changed, bring the game's chat feed in line with it (new messages,
//! deleted ones, why a message was refused), optionally put live messages in the
//! console, and play the SJK chat's sound for a new message, the player's own included
//! (`audio/sjk_chat_sound.rs`). Each frame costs one look at the chat's revision and the
//! mutes'; nothing is copied unless one changed.

use super::*;

impl GpuState {
    /// Follow the SJK chat in the chat feed while `cl_sjkChat` is on.
    pub(crate) fn update_sjk_chat(&mut self) {
        self.send_server_copies(Instant::now());
        let Some(console) = self.console.as_ref() else {
            return;
        };
        if console.bool_cvar("cl_sjkChat") == Some(false) {
            return;
        }
        let feed = console.bool_cvar(FEED_CVAR) != Some(false);
        let in_console = console.bool_cvar(CONSOLE_CVAR) != Some(false);
        self.chat.set_sjk_feed_visible(feed);
        let sound = sound_on(console);
        let mutes = player_mutes::revision();
        let Some(mark) = player_identity::with_chat(|chat| {
            (
                chat.revision,
                chat.outcome.as_ref().map_or(0, |outcome| outcome.serial),
                mutes,
            )
        }) else {
            return;
        };
        if !self.chat.sjk_changed(mark) {
            return;
        }
        // The mutes live under the service's lock too: read them before it is taken.
        let muted = player_mutes::muted_keys();
        let is_muted = |key: &str| muted.iter().any(|m| m == key);
        let now = Instant::now();
        let own_key = player_identity::own_key_id();
        let chat = &mut self.chat;
        let update = player_identity::with_chat(|state| {
            let after = chat.sync_sjk_in_feed(state, is_muted, now, feed);
            chat.sjk_outcome(state.outcome.as_ref(), now);
            let joined = state
                .messages
                .iter()
                .filter(|message| after.is_some_and(|after| message.id > after));
            chat.follow_server_copies(joined, own_key.as_deref(), state.outcome.as_ref(), now);
            let news = after
                .is_some_and(|after| chat::sjk::worth_a_sound(&state.messages, after, is_muted));
            let console_lines = if in_console {
                after.map_or_else(Vec::new, |after| {
                    chat::sjk::console_lines(&state.messages, after, is_muted)
                })
            } else {
                Vec::new()
            };
            (news, console_lines)
        });
        let Some((news, console_lines)) = update else {
            return;
        };
        if let Some(console) = self.console.as_mut() {
            for line in console_lines {
                console.push_chat_line(line.clone(), &line, None);
            }
        }
        if sound && news && self.chat.sjk_sound_due(now) {
            audio::ui_cues::post(audio::ui_cues::Cue::SjkChat);
        }
    }

    /// What the player said on the SJK chat during a match goes to the game server's
    /// chat too, once the hub answered (`chat/server_copy.rs`); in a local game or the
    /// menus it goes nowhere else.
    fn send_server_copies(&mut self, now: Instant) {
        let server = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .map(|session| session.server_id());
        let said = chat::server_copy::take_said();
        for command in self.chat.due_server_copies(said, server, now) {
            if let Some(session) = self.live_session.as_mut()
                && let Err(error) = session.send_reliable_command_from(
                    command.as_bytes(),
                    sjk_client::ReliableCommandOrigin::Chat,
                )
            {
                eprintln!("failed to say an SJK chat message on the server: {error}");
            }
        }
    }
}

/// Whether the SJK chat's sound plays: `cl_sjkChatSound`, under the master chat sound
/// switch `cg_chatSounds` (`cgame_options.rs`), which silences every chat sound.
fn sound_on(console: &console::ViewerConsole) -> bool {
    console.bool_cvar(SOUND_CVAR) != Some(false) && cgame_options::chat_sounds(Some(console))
}

/// `cl_sjkChatSound`: whether a new SJK chat message plays a sound.
pub(crate) const SOUND_CVAR: &str = "cl_sjkChatSound";

/// `cl_sjkChatFeed`: whether live SJK messages join the in-game general chat feed.
pub(crate) const FEED_CVAR: &str = "cl_sjkChatFeed";

/// `cl_sjkChatConsole`: whether live SJK messages enter console scrollback.
pub(crate) const CONSOLE_CVAR: &str = "cl_sjkChatConsole";
