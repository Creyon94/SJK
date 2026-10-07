//! The text dialog: a panel in the middle of the screen with a text box, Send and Cancel,
//! for a world note (`world_notes`, the second `inspect` press), a bug report
//! (`bug_report`, the Report a bug button under the game menu) or a player report
//! (`player_report`, a reason on the game menu's Report page). Escape or Cancel drops
//! it; Enter or Send sends it.
//!
//! All keep only what the hub accepts as it is typed or pasted (letters and digits of
//! any script, spaces and `. , ! ? ' - : ( )`, line breaks turned into spaces), a report
//! at most 600 characters, a note at most [`NOTE_MAX`], a player report at most 300.
//!
//! The launcher is the Report a bug button drawn centred at the bottom of the screen
//! while the game menu is open. With the classic menus both take the classic+ look
//! ([`classic`]).

use crate::menu::art::ArtSet;
use crate::menu_widgets::{ButtonStyle, MenuCanvas};
use crate::text::{TextVertex, UiFont};
use sjk_ui::{Color, DrawCommand, FontWeight, InputEvent, Rect, TextAlign, UiEventKind};
use std::time::Instant;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "text_dialog_classic.rs"]
mod classic;

/// Longest world note, in characters.
pub(crate) const NOTE_MAX: usize = sjk_identity::report::NOTE_MAX;
/// Lines of text the box shows; a longer text shows its end.
const LINES: usize = 6;

const FIELD_TOKEN: u16 = 960;
const SEND_TOKEN: u16 = 961;
const CANCEL_TOKEN: u16 = 962;
const LAUNCH_TOKEN: u16 = 963;

/// What the dialog is for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// A note about the selection named by `subject`.
    Note { subject: String },
    /// A bug report for the hub.
    Report,
    /// A report about a player for the hub: `subject` names them and the reason.
    PlayerReport {
        subject: String,
        category: sjk_identity::Category,
    },
}

/// What the caller does after the dialog handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Action {
    None,
    /// Closed without sending.
    Cancel,
    /// Send this text (the dialog has closed).
    Send(Kind, String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Focus {
    Field,
    Send,
    Cancel,
}

pub(crate) struct TextDialog {
    kind: Option<Kind>,
    text: String,
    focus: Focus,
    /// Why the last Send was refused.
    message: String,
    ui: MenuCanvas,
    launcher: MenuCanvas,
    epoch: Instant,
    /// Shift is held (Shift+Tab goes back).
    shift: bool,
    /// The classic+ look is drawn, with the retail menu art it can use.
    classic: bool,
    art: ArtSet,
}

impl Default for TextDialog {
    fn default() -> Self {
        Self {
            kind: None,
            text: String::with_capacity(2_400),
            focus: Focus::Field,
            message: String::new(),
            ui: MenuCanvas::with_capacities(24, 192, 96),
            launcher: MenuCanvas::with_capacities(4, 32, 16),
            epoch: Instant::now(),
            shift: false,
            classic: false,
            art: ArtSet::default(),
        }
    }
}

/// The longest text `kind` takes, in characters.
fn limit(kind: &Kind) -> usize {
    match kind {
        Kind::Note { .. } => NOTE_MAX,
        Kind::Report => sjk_identity::report::TEXT_MAX,
        Kind::PlayerReport { .. } => sjk_identity::report::PLAYER_MAX,
    }
}

/// `text` as `kind` keeps it when typed or pasted onto `field`.
fn typed(kind: &Kind, field: &str, text: &str) -> String {
    let room = limit(kind).saturating_sub(field.chars().count());
    text.chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| sjk_identity::report::allowed(*c))
        .take(room)
        .collect()
}

/// `text` cut to its first `max` characters.
fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// `text` broken into lines of at most `width` characters, at spaces where it can be.
#[cfg(test)]
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    wrap_by(text, |line| line.chars().count() <= width)
}

/// `text` broken into lines that `fits` accepts, at spaces where it can be; a word longer
/// than a line is cut where the line ends.
fn wrap_by(text: &str, fits: impl Fn(&str) -> bool) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let mut word = word.to_owned();
        loop {
            let candidate = if line.is_empty() {
                word.clone()
            } else {
                format!("{line} {word}")
            };
            if fits(&candidate) {
                line = candidate;
                break;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                continue;
            }
            let mut head = String::new();
            for character in word.chars() {
                head.push(character);
                if !fits(&head) {
                    head.pop();
                    break;
                }
            }
            if head.is_empty() {
                head = word.chars().take(1).collect();
            }
            word = word[head.len()..].to_owned();
            lines.push(head);
        }
    }
    lines.push(line);
    lines
}

/// `text` broken into lines no wider than `width` pixels when drawn `size` pixels high
/// in `font` with `spacing` pixels between glyphs, leaving room for the caret.
fn wrap_to(text: &str, font: &UiFont, size: f32, spacing: f32, width: f32) -> Vec<String> {
    let scale = size / font.height.max(1.0);
    let room = width - size * 0.5;
    wrap_by(text, |line| {
        crate::text::visible_text_width(font, line, scale) + spacing * line.chars().count() as f32
            <= room
    })
}

impl TextDialog {
    pub(crate) fn is_open(&self) -> bool {
        self.kind.is_some()
    }

    /// Open empty for `kind`.
    pub(crate) fn open(&mut self, kind: Kind) {
        self.kind = Some(kind);
        self.text.clear();
        self.message.clear();
        self.focus = Focus::Field;
        self.epoch = Instant::now();
    }

    fn close(&mut self) -> Action {
        self.kind = None;
        Action::Cancel
    }

    /// Enter or Send: hand the text over, or say why it cannot go.
    fn send(&mut self) -> Action {
        let Some(kind) = self.kind.clone() else {
            return Action::None;
        };
        let refusal = match &kind {
            Kind::Report => sjk_identity::report::text(&self.text).err(),
            Kind::Note { .. } => sjk_identity::report::note_text(&self.text).err(),
            Kind::PlayerReport { .. } => sjk_identity::report::player_text(&self.text).err(),
        };
        if let Some(why) = refusal {
            self.message = why.to_owned();
            self.focus = Focus::Field;
            return Action::None;
        }
        self.kind = None;
        Action::Send(kind, std::mem::take(&mut self.text))
    }

    /// Fill the dialog for the menu snapshots: its `text`, the focus on Send or the field,
    /// and a refusal `message`.
    #[cfg(test)]
    pub(crate) fn preview(&mut self, text: &str, on_send: bool, message: &str) {
        self.text = text.to_owned();
        self.focus = if on_send { Focus::Send } else { Focus::Field };
        self.message = message.to_owned();
    }

    /// Choose the look: classic+ with the retail `art` it can draw, or modern.
    pub(crate) fn set_look(&mut self, classic: bool, art: ArtSet) {
        self.classic = classic;
        self.art = art;
    }

    /// Follow the Shift key (`ModifiersChanged`).
    pub(crate) fn set_shift(&mut self, shift: bool) {
        self.shift = shift;
    }

    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> Action {
        let shift = self.shift;
        if event.state != ElementState::Pressed || self.kind.is_none() {
            return Action::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return Action::None;
        };
        match key {
            KeyCode::Escape => return self.close(),
            KeyCode::Tab => {
                let order = [Focus::Field, Focus::Send, Focus::Cancel];
                let at = order.iter().position(|f| *f == self.focus).unwrap_or(0);
                self.focus = order[if shift { at + 2 } else { at + 1 } % 3];
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                return if self.focus == Focus::Cancel {
                    self.close()
                } else {
                    self.send()
                };
            }
            KeyCode::Space if self.focus != Focus::Field => {
                return if self.focus == Focus::Cancel {
                    self.close()
                } else {
                    self.send()
                };
            }
            KeyCode::Backspace if self.focus == Focus::Field => {
                self.text.pop();
                self.message.clear();
            }
            _ if self.focus == Focus::Field => {
                let pasted;
                let text = match event.text.as_deref() {
                    Some("\u{16}") => {
                        pasted = crate::console::clipboard::paste().unwrap_or_default();
                        pasted.as_str()
                    }
                    Some(text) => text,
                    None => return Action::None,
                };
                let kind = self.kind.as_ref().expect("open");
                let kept = typed(kind, &self.text, text);
                if !kept.is_empty() {
                    self.text.push_str(&kept);
                    self.message.clear();
                }
            }
            _ => {}
        }
        Action::None
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> Action {
        let Some(event) = self.ui.pointer(event) else {
            return Action::None;
        };
        if event.kind != UiEventKind::Activate {
            return Action::None;
        }
        match event.token {
            Some(FIELD_TOKEN) => {
                self.focus = Focus::Field;
                Action::None
            }
            Some(SEND_TOKEN) => self.send(),
            Some(CANCEL_TOKEN) => self.close(),
            _ => Action::None,
        }
    }

    /// The launcher's pointer: true when its button was clicked.
    pub(crate) fn launcher_pointer(&mut self, event: InputEvent) -> bool {
        self.launcher.pointer(event).is_some_and(|event| {
            event.kind == UiEventKind::Activate && event.token == Some(LAUNCH_TOKEN)
        })
    }

    /// Draw the Report a bug button centred at the bottom of the screen.
    pub(crate) fn append_launcher(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        if self.classic {
            self.append_launcher_classic(vertices, font, viewport);
            return;
        }
        let s = crate::ui_scale::height_scale(viewport[1]);
        let ui = &mut self.launcher;
        ui.begin_transparent(viewport);
        let width = 220.0 * s;
        let height = 46.0 * s;
        let rect = Rect::new(
            (viewport[0] - width) * 0.5,
            viewport[1] - height - 36.0 * s,
            width,
            height,
        );
        ui.button_styled(
            LAUNCH_TOKEN,
            "Report a bug",
            rect,
            ButtonStyle {
                selected: false,
                enabled: true,
                accent: Some(Color::new(1.0, 0.45, 0.42, 1.0)),
                badge: None,
            },
        );
        ui.finish(0);
        ui.append_text(vertices, font, viewport);
    }

    pub(crate) fn launcher_draw_list(&self) -> &sjk_ui::DrawList {
        self.launcher.draw_list()
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Draw the dialog over the whole frame; text other overlays appended earlier this
    /// frame is dropped rather than shown through the panel.
    pub(crate) fn append(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let Some(kind) = self.kind.clone() else {
            return;
        };
        vertices.clear();
        if self.classic {
            self.append_classic(&kind, vertices, font, viewport);
            return;
        }
        let s = crate::ui_scale::height_scale(viewport[1]);
        let ui = &mut self.ui;
        ui.begin_transparent(viewport);
        let theme = ui.theme();
        let _ = ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: Rect::new(0.0, 0.0, viewport[0], viewport[1]),
            color: Color::new(0.0, 0.0, 0.0, 0.45),
        });
        let width = (viewport[0] - 32.0 * s).min(780.0 * s);
        let pad = 24.0 * s;
        let line = 24.0 * s;
        let box_height = LINES as f32 * line + 20.0 * s;
        let height = pad * 2.0 + 40.0 * s + 26.0 * s + box_height + 30.0 * s + 22.0 * s + 56.0 * s;
        let card = Rect::new(
            (viewport[0] - width) * 0.5,
            (viewport[1] - height) * 0.5,
            width,
            height,
        );
        ui.panel(card);
        let left = card.x + pad;
        let inner = card.width - pad * 2.0;
        let mut y = card.y + pad;
        let (title, subject, accent) = match &kind {
            Kind::Report => (
                "Report a bug",
                "What went wrong? Where were you, and what did you expect?".to_owned(),
                Color::new(1.0, 0.45, 0.42, 1.0),
            ),
            Kind::Note { subject } => (
                "Note for Claude",
                subject.clone(),
                Color::new(1.0, 0.78, 0.36, 1.0),
            ),
            Kind::PlayerReport { subject, .. } => (
                "Report a player",
                subject.clone(),
                Color::new(1.0, 0.478, 0.239, 1.0),
            ),
        };
        let _ = ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: Rect::new(card.x, card.y + 14.0 * s, 4.0 * s, 40.0 * s),
            color: accent,
        });
        ui.text(
            title,
            Rect::new(left, y, inner, 36.0 * s),
            26.0 * s,
            theme.foreground,
            FontWeight::Semibold,
            0.0,
        );
        y += 40.0 * s;
        let room = (inner / (14.0 * s * 0.55)) as usize;
        let subject = clip(&subject, room);
        ui.text(
            &subject,
            Rect::new(left, y, inner, 22.0 * s),
            14.0 * s,
            theme.muted,
            FontWeight::Regular,
            0.0,
        );
        y += 26.0 * s;
        let field = Rect::new(left, y, inner, box_height);
        let focused = self.focus == Focus::Field;
        ui.text_field(field, focused);
        ui.hit_region(FIELD_TOKEN, field);
        let size = 16.0 * s;
        let caret = focused && (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        if self.text.is_empty() && !focused {
            ui.text(
                "Type here",
                Rect::new(left + 14.0 * s, y + 10.0 * s, inner - 28.0 * s, line),
                size,
                theme.muted,
                FontWeight::Regular,
                0.0,
            );
        } else {
            let mut lines = wrap_to(&self.text, font, size, 0.0, inner - 28.0 * s);
            if caret && let Some(last) = lines.last_mut() {
                last.push('|');
            }
            let first = lines.len().saturating_sub(LINES);
            for (row, text) in lines[first..].iter().enumerate() {
                ui.text(
                    text,
                    Rect::new(
                        left + 14.0 * s,
                        y + 10.0 * s + row as f32 * line,
                        inner - 28.0 * s,
                        line,
                    ),
                    size,
                    theme.foreground,
                    FontWeight::Regular,
                    0.0,
                );
            }
        }
        y += box_height + 8.0 * s;
        let count = format!("{} / {}", self.text.chars().count(), limit(&kind));
        ui.text_aligned(
            &count,
            Rect::new(left, y, inner, 20.0 * s),
            12.0 * s,
            theme.muted,
            FontWeight::Regular,
            0.0,
            TextAlign::End,
        );
        let hint = match kind {
            Kind::Report => "Letters, digits, spaces and . , ! ? ' - : ( ) only",
            Kind::Note { .. } => "Saved and sent with a screenshot to the SJK team",
            Kind::PlayerReport { .. } => {
                "What happened? Sent to the SJK team with who, where and when"
            }
        };
        ui.text(
            hint,
            Rect::new(left, y, inner - 120.0 * s, 20.0 * s),
            12.0 * s,
            theme.muted,
            FontWeight::Regular,
            0.0,
        );
        y += 30.0 * s;
        if !self.message.is_empty() {
            ui.text(
                &self.message,
                Rect::new(left, y, inner, 20.0 * s),
                13.0 * s,
                Color::new(1.0, 0.45, 0.4, 1.0),
                FontWeight::Semibold,
                0.0,
            );
        }
        y += 22.0 * s;
        let button = 150.0 * s;
        let cancel = Rect::new(card.right() - pad - button, y, button, 44.0 * s);
        let send = Rect::new(cancel.x - 12.0 * s - button, y, button, 44.0 * s);
        ui.button_styled(
            SEND_TOKEN,
            "Send",
            send,
            ButtonStyle {
                selected: self.focus == Focus::Send,
                enabled: true,
                accent: Some(accent),
                badge: None,
            },
        );
        ui.button(CANCEL_TOKEN, "Cancel", cancel, self.focus == Focus::Cancel);
        ui.text(
            "Enter sends, Escape cancels",
            Rect::new(left, y, send.x - left - 12.0 * s, 44.0 * s),
            12.0 * s,
            theme.muted,
            FontWeight::Regular,
            0.0,
        );
        ui.finish(match self.focus {
            Focus::Field => FIELD_TOKEN,
            Focus::Send => SEND_TOKEN,
            Focus::Cancel => CANCEL_TOKEN,
        });
        ui.append_text(vertices, font, viewport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_and_notes_keep_only_what_the_hub_accepts() {
        assert_eq!(
            typed(
                &Kind::Report,
                "",
                "Door <b>flickers</b>\n\u{1F642} \"left\"!"
            ),
            "Door bflickersb  left!"
        );
        let note = Kind::Note {
            subject: String::new(),
        };
        assert_eq!(typed(&note, "", "a <b> \"x\"\tz?"), "a b x z?");
        // Nothing past the limit.
        let full = "x".repeat(NOTE_MAX);
        assert_eq!(typed(&note, &full, "more"), "");
        assert_eq!(
            typed(&Kind::Report, "", &"word ".repeat(200))
                .chars()
                .count(),
            sjk_identity::report::TEXT_MAX
        );
    }

    #[test]
    fn long_text_wraps_at_spaces_and_cuts_long_words() {
        assert_eq!(wrap("one two three four", 9), ["one two", "three", "four"]);
        assert_eq!(wrap("abcdefghijklmnop", 8), ["abcdefgh", "ijklmnop"]);
        assert_eq!(wrap("", 10), [""]);
    }

    #[test]
    fn send_checks_the_text_and_escape_cancels() {
        let mut dialog = TextDialog::default();
        dialog.open(Kind::Report);
        dialog.text = "short".into();
        assert_eq!(dialog.send(), Action::None);
        assert!(dialog.is_open() && !dialog.message.is_empty());
        dialog.text = "The door flickers on ffa3".into();
        assert_eq!(
            dialog.send(),
            Action::Send(Kind::Report, "The door flickers on ffa3".into())
        );
        assert!(!dialog.is_open());
        dialog.open(Kind::Note {
            subject: "wall".into(),
        });
        assert_eq!(dialog.send(), Action::None, "an empty note waits");
        dialog.text = "too shiny".into();
        assert_eq!(
            dialog.send(),
            Action::Send(
                Kind::Note {
                    subject: "wall".into()
                },
                "too shiny".into()
            )
        );
        dialog.open(Kind::Note {
            subject: "wall".into(),
        });
        assert_eq!(dialog.close(), Action::Cancel);
        assert!(!dialog.is_open());
    }

    #[test]
    fn a_player_report_takes_a_few_words_up_to_its_limit() {
        let kind = Kind::PlayerReport {
            subject: "Troll: Griefing".into(),
            category: sjk_identity::Category::Griefing,
        };
        assert_eq!(
            typed(&kind, "", &"word ".repeat(100)).chars().count(),
            sjk_identity::report::PLAYER_MAX
        );
        let mut dialog = TextDialog::default();
        dialog.open(kind.clone());
        dialog.text = "troll".into();
        assert_eq!(dialog.send(), Action::None, "too short");
        dialog.text = "Team killing all match long".into();
        assert_eq!(
            dialog.send(),
            Action::Send(kind, "Team killing all match long".into())
        );
    }
}
