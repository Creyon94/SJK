//! The encrypted whisper card (`docs/sjk-ui.md`, "Encrypted whisper failed"): when an
//! encrypted whisper could not go through the SJK hub (`e2e_whispers.rs`), an SJK pop-up
//! card over the game menu says to whom, what and why, and offers Send unencrypted
//! (Enter), which sends it through the game server as a plain whisper, or Don't send
//! (Escape). Nothing goes unencrypted without Send unencrypted. Several failures show
//! one after the other, oldest first. It is drawn in the SJK UI's look in every menu
//! style, and the menus under it are not drawn and take no input, as under the other
//! cards.

use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, wrap};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{FontWeight, InputEvent, TextAlign, UiEventKind};
use std::collections::VecDeque;
use std::net::SocketAddr;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Send unencrypted and Don't send.
const SEND_TOKEN: u16 = 1;
const CANCEL_TOKEN: u16 = 2;

/// The card's left edge, top and width; it is as tall as what it holds.
const CARD_X: f32 = 560.0;
const CARD_TOP: f32 = 330.0;
const CARD_WIDTH: f32 = 800.0;
const MARGIN: f32 = 44.0;
const TEXT_X: f32 = CARD_X + MARGIN;
const TEXT_WIDTH: f32 = CARD_WIDTH - MARGIN * 2.0;
/// The explanation's first line, its step and most lines, and characters a line.
const BODY_TOP: f32 = CARD_TOP + 134.0;
const LINE: f32 = 28.0;
const LINES: usize = 7;
const LINE_CHARS: usize = 90;
/// The most of the whisper quoted.
const QUOTE_CHARS: usize = 120;
/// The buttons along the card's foot, this far under the explanation.
const BUTTONS_GAP: f32 = 30.0;
const BUTTON_HEIGHT: f32 = 46.0;
const BUTTON_GAP: f32 = 14.0;
const SEND_WIDTH: f32 = 250.0;
const CANCEL_WIDTH: f32 = 170.0;
/// The keys' line, as on the other screens.
const KEYS_Y: f32 = 992.0;
const KEYS_RIGHT: f32 = 1824.0;
/// The most failures waiting.
const WAITING: usize = 8;

/// An encrypted whisper the hub did not take.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Failure {
    /// The game server it was written on.
    pub(crate) server: SocketAddr,
    /// The recipient's name as the game showed it.
    pub(crate) name: String,
    pub(crate) text: String,
    /// The `tell` the game server would have had.
    pub(crate) command: String,
    /// Why it did not go, in a player's words.
    pub(crate) why: String,
}

/// What the player chose on the card.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Choice {
    SendPlain(Failure),
    Cancel,
}

pub(crate) struct Card {
    failures: VecDeque<Failure>,
    /// Shown this frame.
    open: bool,
    ui: MenuCanvas,
    /// A world shot's failures, kept with no server.
    #[cfg(test)]
    pub(crate) preview: bool,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            failures: VecDeque::new(),
            open: false,
            ui: MenuCanvas::with_capacities(16, 200, 64),
            #[cfg(test)]
            preview: false,
        }
    }
}

impl Card {
    /// Ask about `failure`, after those already waiting.
    pub(crate) fn offer(&mut self, failure: Failure) {
        if self.failures.len() >= WAITING {
            self.failures.pop_front();
        }
        self.failures.push_back(failure);
    }

    /// Keep only the failures written on `server`, the one the player is on.
    pub(crate) fn keep_for(&mut self, server: Option<SocketAddr>) {
        #[cfg(test)]
        if self.preview {
            return;
        }
        self.failures
            .retain(|failure| Some(failure.server) == server);
    }

    /// Whether the card has something to show.
    pub(crate) fn wants(&self) -> bool {
        !self.failures.is_empty()
    }

    pub(crate) fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Enter takes Send unencrypted, Escape or Backspace Don't send; every other key is
    /// swallowed while the card shows. Space does nothing: sending needs a deliberate key.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> Option<Choice> {
        if event.state != ElementState::Pressed || event.repeat {
            return None;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return None;
        };
        self.key(code)
    }

    fn key(&mut self, code: KeyCode) -> Option<Choice> {
        match code {
            KeyCode::Enter | KeyCode::NumpadEnter => self.choose(true),
            KeyCode::Escape | KeyCode::Backspace => self.choose(false),
            _ => None,
        }
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> Option<Choice> {
        let event = self.ui.pointer(event)?;
        if event.kind != UiEventKind::Activate {
            return None;
        }
        match event.token {
            Some(SEND_TOKEN) => self.choose(true),
            Some(CANCEL_TOKEN) => self.choose(false),
            _ => None,
        }
    }

    /// The failure on show is answered: the next one shows, if any.
    fn choose(&mut self, send: bool) -> Option<Choice> {
        let failure = self.failures.pop_front()?;
        Some(if send {
            Choice::SendPlain(failure)
        } else {
            Choice::Cancel
        })
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// What the card says under its headline: to whom, why it failed, what it says, and
    /// what sending it unencrypted means.
    fn words(failure: &Failure) -> String {
        let name = crate::medals::plain_note(&failure.name);
        let quoted = crate::medals::plain_note(&failure.text);
        let quoted = match quoted.char_indices().nth(QUOTE_CHARS) {
            Some((end, _)) => format!("{}...", &quoted[..end]),
            None => quoted,
        };
        format!(
            "Your whisper to {name} could not go through the SJK hub: {}. \u{201c}{quoted}\u{201d} \
             Send it unencrypted through the game server instead? The server, and whoever \
             runs it, can then read it.",
            failure.why
        )
    }

    /// Draw the card over the whole frame.
    pub(crate) fn append(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let Some(failure) = self.failures.front().cloned() else {
            return;
        };
        let frame = Frame::new(viewport);
        let s = frame.s;
        let explanation = Self::words(&failure);
        let lines: Vec<&str> = wrap(&explanation, LINE_CHARS).take(LINES).collect();
        let body_bottom = BODY_TOP + lines.len() as f32 * LINE;
        let buttons_y = body_bottom + BUTTONS_GAP;
        let height = buttons_y + BUTTON_HEIGHT + MARGIN - CARD_TOP;
        let ui = &mut self.ui;
        ui.begin_transparent(viewport);
        kit::scrim(ui, viewport);
        kit::card(ui, &frame, [CARD_X, CARD_TOP, CARD_WIDTH, height]);
        text(
            ui,
            TextFamily::Display,
            format_args!("Encrypted whisper"),
            frame.rect(TEXT_X, CARD_TOP + 30.0, TEXT_WIDTH, 30.0),
            22.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            ui,
            TextFamily::Display,
            format_args!("Send it unencrypted?"),
            frame.rect(TEXT_X, CARD_TOP + 66.0, TEXT_WIDTH, 48.0),
            36.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        for (index, line) in lines.iter().enumerate() {
            text(
                ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(TEXT_X, BODY_TOP + index as f32 * LINE, TEXT_WIDTH, LINE),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let right = CARD_X + CARD_WIDTH - MARGIN;
        kit::button(
            ui,
            &frame,
            [right - SEND_WIDTH, buttons_y, SEND_WIDTH, BUTTON_HEIGHT],
            "Send unencrypted",
            true,
            true,
            false,
            SEND_TOKEN,
        );
        kit::button(
            ui,
            &frame,
            [
                right - SEND_WIDTH - BUTTON_GAP - CANCEL_WIDTH,
                buttons_y,
                CANCEL_WIDTH,
                BUTTON_HEIGHT,
            ],
            "Don't send",
            false,
            true,
            false,
            CANCEL_TOKEN,
        );
        let [right, keys_y] = frame.point(KEYS_RIGHT, KEYS_Y);
        let cancel = key_hint_width(&["Esc"], "don't send", s);
        let send = key_hint_width(&["Enter"], "send unencrypted", s);
        let x = right - cancel - send - 18.0 * s;
        let x = key_hint(ui, &["Enter"], "send unencrypted", x, keys_y, s);
        key_hint(ui, &["Esc"], "don't send", x + 18.0 * s, keys_y, s);
        // Focus starts on Don't send: a stray click or key sends nothing.
        ui.finish(CANCEL_TOKEN);
        target.append(&self.ui, viewport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(name: &str, text: &str, server: &str) -> Failure {
        Failure {
            server: server.parse().unwrap(),
            name: name.into(),
            text: text.into(),
            command: format!("tell 4 \"{text}\""),
            why: "the SJK hub could not be reached".into(),
        }
    }

    #[test]
    fn failures_are_asked_about_oldest_first() {
        let mut card = Card::default();
        assert!(!card.wants());
        card.offer(failure("Fox", "one", "1.2.3.4:29070"));
        card.offer(failure("Fox", "two", "1.2.3.4:29070"));
        assert_eq!(card.key(KeyCode::Space), None);
        let Some(Choice::SendPlain(sent)) = card.key(KeyCode::Enter) else {
            panic!("Enter sends");
        };
        assert_eq!(sent.command, "tell 4 \"one\"");
        assert_eq!(card.key(KeyCode::Escape), Some(Choice::Cancel));
        assert!(!card.wants());
        assert_eq!(card.key(KeyCode::Enter), None);
    }

    #[test]
    fn leaving_the_server_drops_its_failures() {
        let mut card = Card::default();
        card.offer(failure("Fox", "one", "1.2.3.4:29070"));
        card.offer(failure("Fox", "two", "5.6.7.8:29070"));
        card.keep_for(Some("5.6.7.8:29070".parse().unwrap()));
        assert_eq!(card.failures.len(), 1);
        card.keep_for(None);
        assert!(!card.wants());
    }

    #[test]
    fn the_words_say_to_whom_why_and_what() {
        let words = Card::words(&failure("^1Fox", "^3meet me", "1.2.3.4:29070"));
        assert!(
            words.starts_with(
                "Your whisper to Fox could not go through the SJK hub: the SJK hub could not \
                 be reached. \u{201c}meet me\u{201d}"
            ),
            "{words}"
        );
        let longest = Card::words(&failure(&"N".repeat(36), &"a".repeat(900), "1.2.3.4:1"));
        assert!(wrap(&longest, LINE_CHARS).count() <= LINES, "{longest}");
    }
}
