//! The alpha notice (`docs/sjk-ui.md`, "Alpha notice"): the first time the game starts,
//! an SJK pop-up card over the darkened screen says SJK is an alpha build of a free and
//! open-source project, that bugs and unfinished parts are to be expected, how to report
//! them (the SJK Discord, Report a bug in a game), and that SJK comes with no warranty and
//! owes the player nothing. Continue (gold, Enter) only works once the player ticks "I
//! understand" (Space); Quit leaves the game. There is no way past it but those two.
//!
//! `ui_alphaNotice` holds the notice's version the player accepted: raising
//! [`VERSION`] when the words change shows the card again. It shows over the menus, never
//! in the middle of play, before every other card and First setup, in the SJK UI's look
//! in every menu style; the menus under it are not drawn and take no input.

use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, wrap};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{DrawCommand, FontWeight, InputEvent, TextAlign, UiEventKind};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// The notice's version the player accepted.
pub(crate) const CVAR: &str = "ui_alphaNotice";
/// This notice's version: raise it when the words change, to ask again.
pub(crate) const VERSION: i64 = 1;
/// The SJK Discord's permanent invite (to #welcome; it never expires).
pub(crate) const DISCORD_URL: &str = "https://discord.gg/dGBdMygvJw";

const CONTINUE_TOKEN: u16 = 1;
const QUIT_TOKEN: u16 = 2;
const TICK_TOKEN: u16 = 3;
const DISCORD_TOKEN: u16 = 4;

/// The card's left edge and width; it is as tall as what it holds, centred.
const CARD_X: f32 = 510.0;
const CARD_WIDTH: f32 = 900.0;
const MARGIN: f32 = 44.0;
const TEXT_X: f32 = CARD_X + MARGIN;
const TEXT_WIDTH: f32 = CARD_WIDTH - MARGIN * 2.0;
/// The explanation's first line under the card's top, its step, and characters a line.
const BODY: f32 = 134.0;
const LINE: f32 = 28.0;
const LINE_CHARS: usize = 98;
const PARAGRAPH_GAP: f32 = 12.0;
/// The report row: the Report a bug button as it looks in a game, and Join the Discord.
const ROW_GAP: f32 = 22.0;
const ROW_HEIGHT: f32 = 46.0;
const REPORT_WIDTH: f32 = 190.0;
const DISCORD_WIDTH: f32 = 230.0;
/// The tick's lines and the buttons.
const TICK_GAP: f32 = 30.0;
const TICK_CHARS: usize = 84;
const FOOT_GAP: f32 = 26.0;
const BUTTON_HEIGHT: f32 = 46.0;
const BUTTON_GAP: f32 = 14.0;
const CONTINUE_WIDTH: f32 = 200.0;
const QUIT_WIDTH: f32 = 130.0;
const KEYS_Y: f32 = 1020.0;
const KEYS_RIGHT: f32 = 1824.0;

/// What the card says above the report row.
const PARAGRAPHS: [&str; 2] = [
    "This is an alpha build of Sol JK, a free and open-source Jedi Academy client. Expect \
     bugs, and parts that are not finished yet.",
    "Found a bug, or something that feels wrong? Tell us on the SJK Discord, or send it from \
     a game with Report a bug: press Escape, then SJK.",
];
/// What it says under the report row.
const TERMS: &str = "SJK is a fun project that Sol and its contributors make in their free \
     time. It comes with no warranty of any kind (as its GPL licence says), and nobody owes \
     you support, fixes or features.";
/// What the player ticks.
const UNDERSTAND: &str = "I understand that SJK is a work in progress that may have bugs, that \
     it comes with no warranty, and that I am owed nothing.";

/// What the player chose on the card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Choice {
    /// Flip "I understand".
    Tick,
    /// Accepted: the card goes for good.
    Continue,
    Quit,
    Discord,
}

pub(crate) struct Card {
    /// Not accepted yet.
    pending: bool,
    ticked: bool,
    /// Shown this frame.
    open: bool,
    ui: MenuCanvas,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            pending: false,
            ticked: false,
            open: false,
            ui: MenuCanvas::with_capacities(32, 320, 96),
        }
    }
}

impl Card {
    /// The version the player accepted, read from `ui_alphaNotice`.
    pub(crate) fn offer(&mut self, accepted: i64) {
        self.pending = accepted < VERSION;
    }

    /// Whether the card has something to show.
    pub(crate) fn wants(&self) -> bool {
        self.pending
    }

    pub(crate) fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Space ticks, Enter continues once ticked; every other key, Escape too, is
    /// swallowed while the card shows.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> Option<Choice> {
        if event.state != ElementState::Pressed || event.repeat {
            return None;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return None;
        };
        self.key(code)
    }

    /// [`Self::handle_key`] for a key code, for the world shots.
    #[cfg(test)]
    pub(crate) fn handle_key_code_for_test(&mut self, code: KeyCode) -> Option<Choice> {
        self.key(code)
    }

    fn key(&mut self, code: KeyCode) -> Option<Choice> {
        let choice = match code {
            KeyCode::Space => Choice::Tick,
            KeyCode::Enter | KeyCode::NumpadEnter => Choice::Continue,
            _ => return None,
        };
        self.choose(choice)
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> Option<Choice> {
        let event = self.ui.pointer(event)?;
        if event.kind != UiEventKind::Activate {
            return None;
        }
        let choice = match event.token {
            Some(CONTINUE_TOKEN) => Choice::Continue,
            Some(QUIT_TOKEN) => Choice::Quit,
            Some(TICK_TOKEN) => Choice::Tick,
            Some(DISCORD_TOKEN) => Choice::Discord,
            _ => return None,
        };
        self.choose(choice)
    }

    /// The tick flips; Continue needs it and puts the card away.
    fn choose(&mut self, choice: Choice) -> Option<Choice> {
        match choice {
            Choice::Tick => self.ticked = !self.ticked,
            Choice::Continue if !self.ticked => return None,
            Choice::Continue => self.pending = false,
            Choice::Quit | Choice::Discord => {}
        }
        Some(choice)
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Draw the card over the whole frame.
    pub(crate) fn append(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        let s = frame.s;
        // Lay out from the card's top, then centre it.
        let mut y = BODY;
        let mut lines: Vec<(f32, &str)> = Vec::new();
        for paragraph in PARAGRAPHS {
            for line in wrap(paragraph, LINE_CHARS) {
                lines.push((y, line));
                y += LINE;
            }
            y += PARAGRAPH_GAP;
        }
        let row = y - PARAGRAPH_GAP + ROW_GAP;
        y = row + ROW_HEIGHT + ROW_GAP;
        for line in wrap(TERMS, LINE_CHARS) {
            lines.push((y, line));
            y += LINE;
        }
        let tick_top = y + TICK_GAP;
        let tick_lines: Vec<&str> = wrap(UNDERSTAND, TICK_CHARS).collect();
        let foot = tick_top + tick_lines.len() as f32 * LINE + FOOT_GAP;
        let height = foot + BUTTON_HEIGHT + MARGIN;
        let top = ((1080.0 - height) * 0.5).max(20.0);
        let ui = &mut self.ui;
        ui.begin_transparent(viewport);
        kit::scrim(ui, viewport);
        kit::card(ui, &frame, [CARD_X, top, CARD_WIDTH, height]);
        text(
            ui,
            TextFamily::Display,
            format_args!("Alpha build"),
            frame.rect(TEXT_X, top + 30.0, TEXT_WIDTH, 30.0),
            22.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            ui,
            TextFamily::Display,
            format_args!("Welcome to Sol JK"),
            frame.rect(TEXT_X, top + 66.0, TEXT_WIDTH, 48.0),
            36.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        for (line_y, line) in &lines {
            text(
                ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(TEXT_X, top + line_y, TEXT_WIDTH, LINE),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        // The Report a bug button as the game menu shows it: a picture, not a button.
        let row = top + row;
        let report = frame.rect(TEXT_X, row, REPORT_WIDTH, ROW_HEIGHT);
        let _ = ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: report,
            radius: report.height * 0.5,
            color: color::alpha(color::HOLO, 0.05),
        });
        let _ = ui.draw_list_mut().push(DrawCommand::Border {
            rect: report,
            radius: report.height * 0.5,
            width: 1.5 * s,
            color: color::alpha(color::HOLO, 0.4),
        });
        text(
            ui,
            TextFamily::Display,
            format_args!("Report a bug"),
            frame.rect(TEXT_X, row + (ROW_HEIGHT - 26.0) * 0.5, REPORT_WIDTH, 26.0),
            20.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Center,
        );
        text(
            ui,
            TextFamily::Body,
            format_args!("This button, in a game: Escape, SJK"),
            frame.rect(TEXT_X + REPORT_WIDTH + 16.0, row + 9.0, 260.0, LINE),
            16.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let right = CARD_X + CARD_WIDTH - MARGIN;
        kit::button(
            ui,
            &frame,
            [right - DISCORD_WIDTH, row, DISCORD_WIDTH, ROW_HEIGHT],
            "Join the SJK Discord",
            false,
            true,
            false,
            DISCORD_TOKEN,
        );
        text(
            ui,
            TextFamily::Body,
            format_args!("discord.gg/dGBdMygvJw"),
            frame.rect(right - DISCORD_WIDTH - 236.0, row + 9.0, 220.0, LINE),
            16.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::End,
        );
        // "I understand", which Continue needs.
        let tick_top = top + tick_top;
        let after = kit::tick(
            ui,
            &frame,
            TEXT_X,
            tick_top + LINE * 0.5,
            self.ticked,
            false,
        );
        for (index, line) in tick_lines.iter().enumerate() {
            text(
                ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(
                    after + 14.0,
                    tick_top + index as f32 * LINE,
                    TEXT_WIDTH,
                    LINE,
                ),
                18.0 * s,
                if self.ticked {
                    color::TEXT
                } else {
                    color::GOLD_BRIGHT
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        ui.hit_region(
            TICK_TOKEN,
            frame.rect(
                TEXT_X - 8.0,
                tick_top - 8.0,
                TEXT_WIDTH + 16.0,
                tick_lines.len() as f32 * LINE + 16.0,
            ),
        );
        let foot = top + foot;
        kit::button(
            ui,
            &frame,
            [right - CONTINUE_WIDTH, foot, CONTINUE_WIDTH, BUTTON_HEIGHT],
            "Continue",
            true,
            self.ticked,
            false,
            CONTINUE_TOKEN,
        );
        kit::button(
            ui,
            &frame,
            [
                right - CONTINUE_WIDTH - BUTTON_GAP - QUIT_WIDTH,
                foot,
                QUIT_WIDTH,
                BUTTON_HEIGHT,
            ],
            "Quit",
            false,
            true,
            false,
            QUIT_TOKEN,
        );
        let [right, keys_y] = frame.point(KEYS_RIGHT, KEYS_Y);
        let hints: [(&[&str], &str); 2] = [(&["Space"], "I understand"), (&["Enter"], "continue")];
        let width: f32 = hints
            .iter()
            .map(|(keys, label)| key_hint_width(keys, label, s) + 18.0 * s)
            .sum();
        let mut x = right - width;
        for (keys, label) in hints {
            x = key_hint(ui, keys, label, x, keys_y, s) + 18.0 * s;
        }
        ui.finish(if self.ticked {
            CONTINUE_TOKEN
        } else {
            TICK_TOKEN
        });
        target.append(&self.ui, viewport);
    }
}

impl crate::GpuState {
    /// Whether the card shows this frame: not accepted yet, and the player on a menu or
    /// the game menu. Only the console covers it; every other card waits for it.
    pub(crate) fn prepare_alpha_card(&mut self) -> bool {
        let accepted = self
            .console
            .as_ref()
            .and_then(|console| console.integer_cvar(CVAR))
            .unwrap_or(0);
        self.alpha_card.offer(accepted);
        let menu_visible = self
            .client_menu
            .as_ref()
            .is_some_and(crate::menu::ClientMenu::is_visible);
        let console_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::is_open);
        let open = self.alpha_card.wants()
            && (menu_visible || self.game_menu)
            && !console_open
            && !self.text_dialog.is_open()
            && self.world_load_task.is_none()
            && self.world_install_task.is_none()
            && !self.pending_map_reload;
        self.alpha_card.set_open(open);
        open
    }

    /// Draw the card. It darkens everything, so every font batch appended before it is
    /// dropped (text draws above all shapes).
    pub(crate) fn append_alpha_card(&mut self, viewport: [f32; 2]) {
        self.text_vertices.clear();
        self.classic_text_vertices.clear();
        self.game_fonts.clear_text();
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        self.alpha_card.append(target, viewport);
    }

    /// Act on the player's choice on the card.
    pub(crate) fn alpha_card_choice(&mut self, choice: Option<Choice>) {
        match choice {
            Some(Choice::Continue) => {
                if let Some(console) = &mut self.console {
                    console.set_cvar(CVAR, &VERSION.to_string());
                }
                crate::log::progress(format_args!("alpha notice: accepted"));
            }
            Some(Choice::Quit) => self.quit_requested = true,
            Some(Choice::Discord) => crate::update::open_page(DISCORD_URL),
            Some(Choice::Tick) | None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continue_needs_the_tick_and_puts_the_card_away() {
        let mut card = Card::default();
        card.offer(0);
        assert!(card.wants());
        // Escape and other keys do nothing; Enter does nothing before the tick.
        assert_eq!(card.key(KeyCode::Escape), None);
        assert_eq!(card.key(KeyCode::KeyA), None);
        assert_eq!(card.key(KeyCode::Enter), None);
        assert!(card.wants());
        assert_eq!(card.key(KeyCode::Space), Some(Choice::Tick));
        assert_eq!(card.key(KeyCode::Space), Some(Choice::Tick));
        assert_eq!(card.key(KeyCode::Enter), None);
        assert_eq!(card.key(KeyCode::Space), Some(Choice::Tick));
        assert_eq!(card.key(KeyCode::Enter), Some(Choice::Continue));
        assert!(!card.wants());
    }

    #[test]
    fn an_accepted_version_hides_the_card_and_a_newer_one_asks_again() {
        let mut card = Card::default();
        card.offer(VERSION);
        assert!(!card.wants());
        card.offer(VERSION - 1);
        assert!(card.wants());
    }

    #[test]
    fn the_cvar_is_defined_and_starts_unaccepted() {
        let directory = tempfile::tempdir().unwrap();
        let console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(console.integer_cvar(CVAR), Some(0));
    }

    #[test]
    fn the_invite_is_the_permanent_one_and_https() {
        assert!(DISCORD_URL.starts_with("https://discord.gg/"));
    }
}
