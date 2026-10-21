//! SJK chat said on the game server too (`docs/hub-chat.md`, "Said on the game server
//! too"), as JoF EternalJK does it: what the player says on the SJK channel during a
//! match goes to the server's chat as well once the hub took it, ended with the hub's
//! id for it (`sjk_client::sjk_copy_marker`), so the server's players on other clients
//! read it. The server's copy of a message, the player's own or another SJK chat
//! player's, waits here for the message itself, which shows in its place: the line is
//! shown, logged and sounded once.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::ChatOverlay;
use sjk_client::{ServerEvent, ServerEventKind};
use sjk_identity::{ChatMessage, ReportOutcome};

/// How long the server's copy of a message waits for the message before it shows.
pub(crate) const HOLD: Duration = Duration::from_millis(2_500);
/// How long a message that showed keeps the server's later copies of it out.
const SEEN: Duration = Duration::from_secs(15);
/// How long a message said in a match waits for the hub before its copy goes without
/// the id.
const ANSWER: Duration = Duration::from_secs(10);
/// Held copies and shown messages followed at most.
const KEPT: usize = 16;

/// What was handed to the SJK chat since the frame last looked, from any of its fields
/// (`player_identity::chat`): the text and the chat outcome's serial then.
static SAID: Mutex<Vec<(String, u64)>> = Mutex::new(Vec::new());

/// Note a message handed to the SJK chat; `outcome` is the chat outcome's serial then.
pub(crate) fn note_said(text: &str, outcome: u64) {
    if let Ok(mut said) = SAID.lock() {
        said.push((text.to_owned(), outcome));
    }
}

/// The messages noted since the last call ([`note_said`]), oldest first.
pub(crate) fn take_said() -> Vec<(String, u64)> {
    SAID.lock()
        .map(|mut said| std::mem::take(&mut *said))
        .unwrap_or_default()
}

#[derive(Debug, Default)]
pub(super) struct ServerCopies {
    held: Vec<Held>,
    seen: VecDeque<(u64, Instant)>,
    waiting: Option<Waiting>,
    /// Commands ready for the server's chat, with the match they are for.
    ready: Vec<(i32, Vec<String>)>,
}

/// A server chat line that is the copy of an SJK chat message, waiting for it.
#[derive(Debug)]
struct Held {
    id: u64,
    until: Instant,
    event: ServerEvent,
}

/// A message said in a match, waiting for the hub's answer.
#[derive(Debug)]
struct Waiting {
    /// The text as the hub takes it (`sjk_identity::chat::check`), to know the
    /// player's own message when the feed brings it back with its id.
    text: String,
    /// The match it was said in (`sv_serverid`).
    server: i32,
    /// The chat outcome's serial when it was sent: a later refusal is this message's.
    outcome: u64,
    since: Instant,
}

impl ServerCopies {
    fn ready(&mut self, waiting: Waiting, id: Option<u64>) {
        self.ready.push((
            waiting.server,
            sjk_client::sjk_copy_commands(&waiting.text, id),
        ));
    }
}

impl ChatOverlay {
    /// A line from the game server: true when it is the server's copy of an SJK chat
    /// message, held for the message or dropped as the message already showed.
    pub(crate) fn hold_server_copy(&mut self, event: &ServerEvent, now: Instant) -> bool {
        let copies = &mut self.server_copies;
        if event.kind != ServerEventKind::Chat || copies.held.len() >= KEPT {
            return false;
        }
        let Some(id) = sjk_client::sjk_copy_id(&event.text) else {
            return false;
        };
        if copies
            .seen
            .iter()
            .any(|&(seen, until)| seen == id && now < until)
        {
            return true;
        }
        copies.held.push(Held {
            id,
            until: now + HOLD,
            event: event.clone(),
        });
        true
    }

    /// The held copies whose message did not come in time, to show as they came.
    pub(crate) fn released_server_copies(&mut self, now: Instant) -> Vec<ServerEvent> {
        let held = std::mem::take(&mut self.server_copies.held);
        let (due, kept): (Vec<_>, Vec<_>) = held.into_iter().partition(|held| now >= held.until);
        self.server_copies.held = kept;
        due.into_iter().map(|held| held.event).collect()
    }

    /// Follow the SJK chat for the server's copies: `messages` just joined, so the
    /// server's copies of each go unshown, and the player's own (`own_key`), back with
    /// its id, has its copy ready; a refusal (`outcome`) readies it without the id.
    pub(crate) fn follow_server_copies<'m>(
        &mut self,
        messages: impl IntoIterator<Item = &'m ChatMessage>,
        own_key: Option<&str>,
        outcome: Option<&ReportOutcome>,
        now: Instant,
    ) {
        let copies = &mut self.server_copies;
        for message in messages {
            copies.held.retain(|held| held.id != message.id);
            if copies.seen.len() == KEPT {
                copies.seen.pop_front();
            }
            copies.seen.push_back((message.id, now + SEEN));
            let own = own_key == Some(message.key_id.as_str());
            if own
                && let Some(waiting) = copies
                    .waiting
                    .take_if(|waiting| waiting.text == message.text)
            {
                copies.ready(waiting, Some(message.id));
            }
        }
        if let Some(outcome) = outcome.filter(|outcome| !outcome.sent)
            && let Some(waiting) = copies
                .waiting
                .take_if(|waiting| outcome.serial > waiting.outcome)
        {
            copies.ready(waiting, None);
        }
    }

    /// The commands for the server's chat that are due in the match `server` (`None`
    /// outside a match on a game server): what was `said` there ([`take_said`]), once
    /// the hub answered, or after [`ANSWER`] without it. Copies for another match are
    /// dropped.
    pub(crate) fn due_server_copies(
        &mut self,
        said: Vec<(String, u64)>,
        server: Option<i32>,
        now: Instant,
    ) -> Vec<String> {
        let copies = &mut self.server_copies;
        if let Some(server) = server {
            for (text, outcome) in said {
                if let Some(earlier) = copies.waiting.take() {
                    copies.ready(earlier, None);
                }
                copies.waiting = Some(Waiting {
                    text: sjk_identity::chat::check(&text).unwrap_or(text),
                    server,
                    outcome,
                    since: now,
                });
            }
        }
        if let Some(waiting) = copies
            .waiting
            .take_if(|waiting| now.saturating_duration_since(waiting.since) >= ANSWER)
        {
            copies.ready(waiting, None);
        }
        std::mem::take(&mut copies.ready)
            .into_iter()
            .filter(|(at, _)| Some(*at) == server)
            .flat_map(|(_, commands)| commands)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> ServerEvent {
        ServerEvent {
            kind: ServerEventKind::Chat,
            text: text.to_owned(),
            sender: Some(3),
            is_private: false,
        }
    }

    fn message(id: u64, key_id: &str, text: &str) -> ChatMessage {
        ChatMessage {
            id,
            at: 0,
            key_id: key_id.to_owned(),
            name: "Sol".to_owned(),
            verified: false,
            staff: false,
            creator: false,
            trust: None,
            emoji: String::new(),
            client: "ejk".to_owned(),
            text: text.to_owned(),
            holocron: None,
        }
    }

    #[test]
    fn a_servers_copy_waits_for_its_message_and_goes_unshown() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        assert!(chat.hold_server_copy(&line("Sol^7: ^2hi^9^2^1^2"), now));
        assert!(
            !chat.hold_server_copy(&line("Sol^7: ^2hi"), now),
            "a plain line shows"
        );
        let mut team = line("Sol^7: ^2hi^9^2^1^2");
        team.kind = ServerEventKind::TeamChat;
        assert!(
            !chat.hold_server_copy(&team, now),
            "team chat is never a copy"
        );
        chat.follow_server_copies([&message(12, "a", "hi")], None, None, now);
        assert!(
            chat.released_server_copies(now + HOLD).is_empty(),
            "the message showed"
        );
        // A copy that comes after its message is dropped too.
        assert!(chat.hold_server_copy(&line("Sol^7: ^2more^9^2^1^2"), now));
        assert!(chat.released_server_copies(now + HOLD).is_empty());
    }

    #[test]
    fn a_copy_whose_message_does_not_come_shows_as_it_came() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        let copy = line("Sol^7: ^2hi^9^1^7");
        assert!(chat.hold_server_copy(&copy, now));
        assert!(chat.released_server_copies(now).is_empty(), "not yet");
        assert_eq!(chat.released_server_copies(now + HOLD), [copy]);
    }

    #[test]
    fn the_players_line_said_in_a_match_goes_to_the_server_with_its_id() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        let said = |text: &str, outcome| vec![(text.to_owned(), outcome)];
        assert!(
            chat.due_server_copies(said("  hello   there ", 4), Some(77), now)
                .is_empty(),
            "waits for the hub"
        );
        // Another player's message with the same words is not the player's own.
        chat.follow_server_copies([&message(11, "b", "hello there")], Some("a"), None, now);
        assert!(chat.due_server_copies(Vec::new(), Some(77), now).is_empty());
        chat.follow_server_copies([&message(12, "a", "hello there")], Some("a"), None, now);
        assert_eq!(
            chat.due_server_copies(Vec::new(), Some(77), now),
            ["say \"^3SJK^2 hello there^9^2^1^2\""]
        );
        // In the menus nothing is said anywhere.
        assert!(
            chat.due_server_copies(said("from the menus", 5), None, now + ANSWER)
                .is_empty()
        );
    }

    #[test]
    fn a_refused_or_unanswered_line_goes_to_the_server_without_the_id() {
        let mut chat = ChatOverlay::new();
        let now = Instant::now();
        let said = |text: &str, outcome| vec![(text.to_owned(), outcome)];
        assert!(
            chat.due_server_copies(said("refused", 4), Some(77), now)
                .is_empty()
        );
        let refused = ReportOutcome {
            serial: 4,
            sent: false,
            message: "too fast".to_owned(),
        };
        chat.follow_server_copies([], Some("a"), Some(&refused), now);
        assert!(
            chat.due_server_copies(Vec::new(), Some(77), now).is_empty(),
            "an earlier outcome"
        );
        let refused = ReportOutcome {
            serial: 5,
            ..refused
        };
        chat.follow_server_copies([], Some("a"), Some(&refused), now);
        assert_eq!(
            chat.due_server_copies(Vec::new(), Some(77), now),
            ["say \"refused\""]
        );
        assert!(
            chat.due_server_copies(said("unanswered", 5), Some(77), now)
                .is_empty()
        );
        assert_eq!(
            chat.due_server_copies(Vec::new(), Some(77), now + ANSWER),
            ["say \"unanswered\""]
        );
        // A copy for a match that ended is dropped.
        chat.due_server_copies(said("left", 5), Some(77), now);
        assert!(
            chat.due_server_copies(Vec::new(), Some(78), now + ANSWER)
                .is_empty()
        );
    }
}
