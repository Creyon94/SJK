//! The what's-new card (`docs/sjk-ui.md`, "What's new card"): the first time the player
//! opens a game that was updated, an SJK pop-up card over the darkened screen tells what
//! the update brought, in a few short lines (the release's recap in `CHANGELOG.md`,
//! `docs/sjk.md` "Changelog"). Close (gold, Enter) puts it away for good; Full changelog
//! (C) opens the changelog page. A player who skipped releases sees the recaps of those
//! releases, newest first, up to `changelog::DIGEST_LIMIT`.
//!
//! Each recap line is its heading over its sentence, and a command or cvar the sentence
//! marks in backticks (`` `whatsnew` ``) is drawn in a chip, as messaging apps draw
//! inline code. The sentences are wrapped by their measured width when the card is
//! drawn, so a chip is never cut and a line ends where the card does.
//!
//! `cl_whatsNewSeen` holds the version whose recap the player closed. The card shows on
//! the main menu or over the game menu, never in the middle of play. A new installation
//! (no marker and First setup still to do) writes the marker without showing the card;
//! a local build, which no release lists, shows nothing. The `whatsnew` command shows the
//! newest recap, the unreleased one included, to preview it. The card is drawn in the SJK
//! UI's look in every menu style, and the menus under it are not drawn and take no input,
//! as under the other cards.

use crate::console::changelog::Digest;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::sjk_chat_look::Measure;
use crate::text::TextFace;
use sjk_ui::{DrawCommand, FontWeight, InputEvent, TextAlign, UiEventKind};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// The version whose recap the player closed.
pub(crate) const SEEN_CVAR: &str = "cl_whatsNewSeen";
/// Console command that shows the newest recap again.
pub(crate) const COMMAND: &str = "whatsnew";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Show what the latest SJK update brought";

/// Close and Full changelog.
const CLOSE_TOKEN: u16 = 1;
const CHANGELOG_TOKEN: u16 = 2;

/// The card's left edge, width and the room above and below it; it is as tall as what it
/// holds and sits in the middle of the frame.
const CARD_X: f32 = 500.0;
const CARD_WIDTH: f32 = 920.0;
const MARGIN: f32 = 44.0;
const TEXT_X: f32 = CARD_X + MARGIN;
const TEXT_WIDTH: f32 = CARD_WIDTH - MARGIN * 2.0;
const FRAME_HEIGHT: f32 = 1080.0;
/// The headline block above the recap, and the recap's rows.
const HEAD_HEIGHT: f32 = 134.0;
const LINE: f32 = 28.0;
const ROW_GAP: f32 = 16.0;
const RELEASE_HEIGHT: f32 = 40.0;
/// A recap line's heading, over its sentence.
const HEADING_HEIGHT: f32 = 28.0;
const HEADING_SIZE: f32 = 21.0;
/// The sentences' text, and a chip's: its text, the room either side of it, its height
/// and corners.
const SIZE: f32 = 18.0;
const CHIP_SIZE: f32 = 17.0;
const CHIP_PAD: f32 = 7.0;
const CHIP_HEIGHT: f32 = 25.0;
const CHIP_RADIUS: f32 = 6.0;
/// Introduction lines shown for a release without a recap.
const INTRO_LINES: usize = 3;
/// How tall the recap may grow before the card points to the changelog.
const BODY_MAX: f32 = 700.0;
/// The buttons along the card's foot, this far under the recap.
const BUTTONS_GAP: f32 = 26.0;
const BUTTON_HEIGHT: f32 = 46.0;
const BUTTON_GAP: f32 = 14.0;
const CLOSE_WIDTH: f32 = 150.0;
const CHANGELOG_WIDTH: f32 = 230.0;
/// The keys' line, as on the other screens.
const KEYS_Y: f32 = 992.0;
const KEYS_RIGHT: f32 = 1824.0;

/// What the player chose on the card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Choice {
    Close,
    Changelog,
}

/// One block of the card as offered, before it is wrapped to the card's width.
#[derive(Debug, Eq, PartialEq)]
enum Block {
    /// A release's name and date, when the card holds several.
    Release(String),
    /// A recap line: its heading and its sentence, code in backticks.
    Line(String, String),
    /// A release's introduction, standing in for a recap it lacks.
    Intro(String),
}

/// A piece of a wrapped line: plain text, or a chip's text. `x` and `width` are frame
/// pixels from the text's left edge.
#[derive(Debug, PartialEq)]
struct Run {
    x: f32,
    width: f32,
    text: String,
    code: bool,
}

type Row = Vec<Run>;

/// One block of the card's recap, wrapped.
#[derive(Debug, PartialEq)]
enum Item {
    Release(String),
    /// A recap line: its heading and the sentence's rows.
    Line(String, Vec<Row>),
    /// An introduction's rows, in muted text.
    Intro(Vec<Row>),
    /// What did not fit.
    More,
}

impl Item {
    fn height(&self) -> f32 {
        match self {
            Self::Release(_) => RELEASE_HEIGHT,
            Self::Line(_, rows) => HEADING_HEIGHT + rows.len() as f32 * LINE + ROW_GAP,
            Self::Intro(rows) => rows.len() as f32 * LINE + ROW_GAP,
            Self::More => LINE + ROW_GAP,
        }
    }
}

pub(crate) struct Card {
    /// The marker was read this run.
    checked: bool,
    /// The version the card is about, written to the marker on Close.
    version: String,
    /// What it shows: the first release's name, and its blocks.
    title: String,
    kicker: &'static str,
    blocks: Vec<Block>,
    /// The blocks wrapped, and the measure they were wrapped with
    /// ([`Card::lay_out`]).
    items: Vec<Item>,
    laid_for: Option<u32>,
    /// Shown this frame.
    open: bool,
    ui: MenuCanvas,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            checked: false,
            version: String::new(),
            title: String::new(),
            kicker: "SJK updated",
            blocks: Vec::new(),
            items: Vec::new(),
            laid_for: None,
            open: false,
            ui: MenuCanvas::with_capacities(96, 200, 96),
        }
    }
}

impl Card {
    /// Whether the marker was read this run; reading it sets this.
    pub(crate) fn take_check(&mut self) -> bool {
        !std::mem::replace(&mut self.checked, true)
    }

    /// Put `digests` (newest first) on the card for the player's `version`, or take the
    /// card away with none. `preview` is a recap shown on request, not an update.
    pub(crate) fn offer(&mut self, version: &str, digests: &[Digest], preview: bool) {
        self.blocks.clear();
        self.items.clear();
        self.laid_for = None;
        self.version = version.to_owned();
        self.kicker = if preview { "Preview" } else { "SJK updated" };
        let Some(first) = digests.first() else {
            self.title.clear();
            return;
        };
        self.title = first.title.clone();
        for digest in digests {
            if digests.len() > 1 {
                self.blocks.push(Block::Release(release_label(digest)));
            }
            if digest.lines.is_empty() {
                if !digest.intro.trim().is_empty() {
                    self.blocks.push(Block::Intro(digest.intro.clone()));
                }
                continue;
            }
            for (heading, sentence) in &digest.lines {
                self.blocks
                    .push(Block::Line(heading.clone(), sentence.clone()));
            }
        }
    }

    /// Whether the card has something to show.
    pub(crate) fn wants(&self) -> bool {
        !self.blocks.is_empty()
    }

    /// The card is answered: it puts the recap away.
    fn put_away(&mut self) {
        self.blocks.clear();
        self.items.clear();
        self.laid_for = None;
    }

    pub(crate) fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Enter, Space, Escape or Backspace close the card, C opens the changelog; every
    /// other key is swallowed while the card shows.
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
        let choice = match code {
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => Choice::Close,
            KeyCode::Escape | KeyCode::Backspace => Choice::Close,
            KeyCode::KeyC => Choice::Changelog,
            _ => return None,
        };
        self.put_away();
        Some(choice)
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> Option<Choice> {
        let event = self.ui.pointer(event)?;
        if event.kind != UiEventKind::Activate {
            return None;
        }
        let choice = match event.token {
            Some(CLOSE_TOKEN) => Choice::Close,
            Some(CHANGELOG_TOKEN) => Choice::Changelog,
            _ => return None,
        };
        self.put_away();
        Some(choice)
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    fn body_height(&self) -> f32 {
        self.items.iter().map(Item::height).sum()
    }

    /// Wrap the blocks to the card's width as `measure` draws text, once for each
    /// measure (the menu text style changes it), and stop with a pointer to the changelog
    /// when the recap would grow taller than [`BODY_MAX`].
    fn lay_out(&mut self, measure: &Measure<'_>) {
        // How the font and the text style measure, to tell when they changed.
        let key = measure.width("Mm `", SIZE, TextFace::Regular).to_bits();
        if self.laid_for == Some(key) {
            return;
        }
        self.laid_for = Some(key);
        self.items.clear();
        let room = BODY_MAX - Item::More.height();
        let mut height = 0.0;
        for block in &self.blocks {
            let item = match block {
                Block::Release(label) => Item::Release(label.clone()),
                Block::Line(heading, sentence) => {
                    Item::Line(heading.clone(), flow(sentence, TEXT_WIDTH, measure))
                }
                Block::Intro(intro) => Item::Intro(intro_rows(intro, measure)),
            };
            if height + item.height() > room {
                self.items.push(Item::More);
                break;
            }
            height += item.height();
            self.items.push(item);
        }
        // A release name with nothing under it says nothing.
        if let Some(Item::Release(_)) = self.items.last() {
            self.items.pop();
            self.items.push(Item::More);
        }
    }

    /// Draw the card over the whole frame.
    pub(crate) fn append(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        self.lay_out(&target.body_measure());
        let frame = Frame::new(viewport);
        let s = frame.s;
        let body = self.body_height();
        let height = HEAD_HEIGHT + body + BUTTONS_GAP + BUTTON_HEIGHT + MARGIN;
        let top = ((FRAME_HEIGHT - height) / 2.0).max(24.0);
        let headline = if self.title == "Unreleased" {
            "What's coming next".to_owned()
        } else {
            format!("What's new in {}", self.title)
        };
        let ui = &mut self.ui;
        ui.begin_transparent(viewport);
        kit::scrim(ui, viewport);
        kit::card(ui, &frame, [CARD_X, top, CARD_WIDTH, height]);
        text(
            ui,
            TextFamily::Display,
            format_args!("{}", self.kicker),
            frame.rect(TEXT_X, top + 30.0, TEXT_WIDTH, 30.0),
            22.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            ui,
            TextFamily::Display,
            format_args!("{headline}"),
            frame.rect(TEXT_X, top + 66.0, TEXT_WIDTH, 48.0),
            36.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut y = top + HEAD_HEIGHT;
        for item in &self.items {
            match item {
                Item::Release(label) => {
                    // Muted, with a rule to the card's edge, so it reads as a divider
                    // above the gold headings rather than as one of them.
                    text(
                        ui,
                        TextFamily::Display,
                        format_args!("{label}"),
                        frame.rect(TEXT_X, y + 6.0, TEXT_WIDTH, 28.0),
                        19.0 * s,
                        color::MUTED,
                        FontWeight::Semibold,
                        TextAlign::Start,
                    );
                    let rule_x = TEXT_X + crate::text::display_width(label, 19.0) + 16.0;
                    let _ = ui.draw_list_mut().push(DrawCommand::SolidRect {
                        rect: frame.rect(rule_x, y + 20.0, TEXT_X + TEXT_WIDTH - rule_x, 1.0),
                        color: color::alpha(color::HOLO, 0.25),
                    });
                }
                Item::Line(heading, rows) => {
                    text(
                        ui,
                        TextFamily::Display,
                        format_args!("{heading}"),
                        frame.rect(TEXT_X, y, TEXT_WIDTH, HEADING_HEIGHT),
                        HEADING_SIZE * s,
                        color::GOLD_BRIGHT,
                        FontWeight::Semibold,
                        TextAlign::Start,
                    );
                    draw_rows(ui, &frame, rows, y + HEADING_HEIGHT, color::TEXT);
                }
                Item::Intro(rows) => draw_rows(ui, &frame, rows, y, color::MUTED),
                Item::More => {
                    text(
                        ui,
                        TextFamily::Body,
                        format_args!("More in the full changelog."),
                        frame.rect(TEXT_X, y, TEXT_WIDTH, LINE),
                        SIZE * s,
                        color::MUTED,
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                }
            }
            y += item.height();
        }
        let buttons_y = y + BUTTONS_GAP;
        let right = CARD_X + CARD_WIDTH - MARGIN;
        kit::button(
            ui,
            &frame,
            [right - CLOSE_WIDTH, buttons_y, CLOSE_WIDTH, BUTTON_HEIGHT],
            "Close",
            true,
            true,
            false,
            CLOSE_TOKEN,
        );
        kit::button(
            ui,
            &frame,
            [
                right - CLOSE_WIDTH - BUTTON_GAP - CHANGELOG_WIDTH,
                buttons_y,
                CHANGELOG_WIDTH,
                BUTTON_HEIGHT,
            ],
            "Full changelog",
            false,
            true,
            false,
            CHANGELOG_TOKEN,
        );
        let [right, keys_y] = frame.point(KEYS_RIGHT, KEYS_Y);
        let close = key_hint_width(&["Enter"], "close", s);
        let changelog = key_hint_width(&["C"], "full changelog", s);
        let x = right - close - changelog - 18.0 * s;
        let x = key_hint(ui, &["C"], "full changelog", x, keys_y, s);
        key_hint(ui, &["Enter"], "close", x + 18.0 * s, keys_y, s);
        ui.finish(CLOSE_TOKEN);
        target.append(&self.ui, viewport);
    }
}

/// Draw wrapped `rows` from the line whose top is `y` (frame pixels), plain text in
/// `colour` and code in chips.
fn draw_rows(ui: &mut MenuCanvas, frame: &Frame, rows: &[Row], y: f32, colour: sjk_ui::Color) {
    let s = frame.s;
    for (index, row) in rows.iter().enumerate() {
        let line_y = y + index as f32 * LINE;
        for run in row {
            if !run.code {
                // A little room past the measured end, so no letter is clipped.
                text(
                    ui,
                    TextFamily::Body,
                    format_args!("{}", run.text),
                    frame.rect(TEXT_X + run.x, line_y, run.width + 8.0, LINE),
                    SIZE * s,
                    colour,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                continue;
            }
            let chip = frame.rect(
                TEXT_X + run.x,
                line_y + (LINE - CHIP_HEIGHT) * 0.5,
                run.width,
                CHIP_HEIGHT,
            );
            let _ = ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: chip,
                radius: CHIP_RADIUS * s,
                color: color::alpha(color::HOLO, 0.12),
            });
            let _ = ui.draw_list_mut().push(DrawCommand::Border {
                rect: chip,
                radius: CHIP_RADIUS * s,
                width: s.max(1.0),
                color: color::alpha(color::HOLO, 0.4),
            });
            text(
                ui,
                TextFamily::Body,
                format_args!("{}", run.text),
                frame.rect(TEXT_X + run.x, line_y, run.width, LINE),
                CHIP_SIZE * s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
    }
}

/// A word of a sentence, or a code span; `glued` when nothing parts it from the one
/// before (a full stop after a chip), so a line never breaks between them.
struct Token<'a> {
    text: &'a str,
    code: bool,
    glued: bool,
}

/// `sentence`'s words and code spans: what backticks enclose is code, kept whole.
fn tokens(sentence: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    // Whether a space came since the last token.
    let mut spaced = true;
    for (index, part) in sentence.split('`').enumerate() {
        if index % 2 == 1 {
            let code = part.trim();
            if !code.is_empty() {
                tokens.push(Token {
                    text: code,
                    code: true,
                    glued: !spaced,
                });
                spaced = false;
            }
            continue;
        }
        let mut rest = part;
        loop {
            let word = rest.trim_start();
            spaced |= word.len() != rest.len();
            if word.is_empty() {
                break;
            }
            let end = word.find(char::is_whitespace).unwrap_or(word.len());
            tokens.push(Token {
                text: &word[..end],
                code: false,
                glued: !spaced,
            });
            spaced = false;
            rest = &word[end..];
        }
    }
    tokens
}

/// `sentence` wrapped into rows `width` wide (frame pixels) as `measure` draws it: at
/// spaces, never inside a code span or between glued tokens; a group wider than a row
/// stands alone. Neighbouring words share one run.
fn flow(sentence: &str, width: f32, measure: &Measure<'_>) -> Vec<Row> {
    let space = measure.width(" ", SIZE, TextFace::Regular);
    let token_width = |token: &Token<'_>| {
        if token.code {
            measure.width(token.text, CHIP_SIZE, TextFace::Semibold) + CHIP_PAD * 2.0
        } else {
            measure.width(token.text, SIZE, TextFace::Regular)
        }
    };
    let tokens = tokens(sentence);
    let mut rows: Vec<Row> = vec![Vec::new()];
    let mut x = 0.0;
    let mut start = 0;
    while start < tokens.len() {
        // The group: this token and those glued after it.
        let end = tokens[start + 1..]
            .iter()
            .position(|token| !token.glued)
            .map_or(tokens.len(), |at| start + 1 + at);
        let group = &tokens[start..end];
        let group_width: f32 = group.iter().map(token_width).sum();
        let mut gap = if x > 0.0 { space } else { 0.0 };
        if x > 0.0 && x + gap + group_width > width {
            rows.push(Vec::new());
            x = 0.0;
            gap = 0.0;
        }
        let row = rows.last_mut().expect("one row at least");
        for (index, token) in group.iter().enumerate() {
            let token_gap = if index == 0 { gap } else { 0.0 };
            let token_width = token_width(token);
            match row.last_mut() {
                Some(run) if !run.code && !token.code => {
                    if token_gap > 0.0 {
                        run.text.push(' ');
                    }
                    run.text.push_str(token.text);
                    run.width += token_gap + token_width;
                }
                _ => row.push(Run {
                    x: x + token_gap,
                    width: token_width,
                    text: token.text.to_owned(),
                    code: token.code,
                }),
            }
            x += token_gap + token_width;
        }
        start = end;
    }
    rows
}

/// An introduction's rows: [`INTRO_LINES`] at most, the last ending in "..." when it was
/// cut short. Code marks are dropped, as introductions draw no chips.
fn intro_rows(intro: &str, measure: &Measure<'_>) -> Vec<Row> {
    let plain = intro.replace('`', "");
    let ellipsis = measure.width("...", SIZE, TextFace::Regular);
    let mut rows = flow(&plain, TEXT_WIDTH - ellipsis, measure);
    if rows.len() > INTRO_LINES {
        rows.truncate(INTRO_LINES);
        if let Some(run) = rows.last_mut().and_then(|row| row.last_mut()) {
            run.text.push_str("...");
            run.width += ellipsis;
        }
    }
    rows
}

/// "2026.1011.3 (Alpha)   /   11/10/2026", or just the name when it has no date.
fn release_label(digest: &Digest) -> String {
    if digest.date.is_empty() {
        digest.title.clone()
    } else {
        format!("{}   /   {}", digest.title, digest.date)
    }
}

impl crate::GpuState {
    /// Read the marker once and put the recap of an update on the card; a new
    /// installation, and a build no release lists, show nothing. Then say whether the
    /// card shows this frame: the player on the main menu or the game menu with nothing
    /// else over it. `covered` is another card or the console over the frame.
    pub(crate) fn prepare_whats_new_card(&mut self, covered: bool) -> bool {
        if self.whats_new_card.take_check() {
            self.read_whats_new_marker();
        }
        let menu_visible = self
            .client_menu
            .as_ref()
            .is_some_and(crate::menu::ClientMenu::is_visible);
        let main_menu = self
            .client_menu
            .as_ref()
            .is_some_and(|menu| menu.is_visible() && menu.on_main_menu());
        let place = main_menu || (self.game_menu && !menu_visible);
        let console_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::is_open);
        let away = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::window_away);
        let open = self.whats_new_card.wants()
            && place
            && !covered
            && !console_open
            && !away
            && !self.text_dialog.is_open()
            && self.world_load_task.is_none()
            && self.world_install_task.is_none()
            && !self.pending_map_reload;
        self.whats_new_card.set_open(open);
        open
    }

    /// What the marker says about this run: nothing to show, a new installation to mark
    /// as up to date, or an update to recap.
    fn read_whats_new_marker(&mut self) {
        let installed = crate::build_info::VERSION;
        let Some(console) = self.console.as_mut() else {
            return;
        };
        let seen = console.text_cvar(SEEN_CVAR).unwrap_or_default().to_owned();
        if seen == installed {
            return;
        }
        // A player who never saw First setup installed just now: nothing was updated.
        if seen.is_empty() && console.bool_cvar(crate::settings::quick::HIDE_CVAR) != Some(true) {
            console.set_cvar(SEEN_CVAR, installed);
            return;
        }
        let digests = crate::console::changelog::digests(installed, &seen);
        self.whats_new_card.offer(installed, &digests, false);
    }

    /// `whatsnew`: put the newest recap on the card, the unreleased one included.
    pub(crate) fn preview_whats_new(&mut self) {
        let digests: Vec<Digest> = crate::console::changelog::newest_digest()
            .into_iter()
            .collect();
        self.whats_new_card.offer("", &digests, true);
    }

    /// Draw the card. It darkens everything, so every font batch appended before it is
    /// dropped (text draws above all shapes).
    pub(crate) fn append_whats_new_card(&mut self, viewport: [f32; 2]) {
        self.text_vertices.clear();
        self.classic_text_vertices.clear();
        self.game_fonts.clear_text();
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        self.whats_new_card.append(target, viewport);
    }

    /// Act on the player's choice: remember the version, and open the changelog.
    pub(crate) fn whats_new_card_choice(&mut self, choice: Option<Choice>) {
        let Some(choice) = choice else {
            return;
        };
        let version = self.whats_new_card.version.clone();
        if let Some(console) = self.console.as_mut() {
            if !version.is_empty() {
                console.set_cvar(SEEN_CVAR, &version);
            }
            if choice == Choice::Changelog {
                console.open_changelog();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{TextStyle, test_font};

    fn digest(title: &str, lines: &[(&str, &str)]) -> Digest {
        Digest {
            title: title.to_owned(),
            date: "12/10/2026".to_owned(),
            intro: "A release introduction that is not a recap.".to_owned(),
            lines: lines
                .iter()
                .map(|(heading, sentence)| ((*heading).to_owned(), (*sentence).to_owned()))
                .collect(),
        }
    }

    /// Lay `card` out with test glyphs, 12 frame pixels a character at the text size.
    fn laid_out(card: &mut Card) {
        let font = test_font();
        card.lay_out(&Measure::new(&font, TextStyle::NEUTRAL));
    }

    #[test]
    fn a_recap_is_shown_until_it_is_closed() {
        let mut card = Card::default();
        assert!(!card.wants());
        card.offer(
            "2.0",
            &[digest("2.0", &[("Servers", "Downloads work.")])],
            false,
        );
        assert!(card.wants());
        assert_eq!(card.title, "2.0");
        assert_eq!(card.key(KeyCode::KeyA), None);
        assert!(card.wants(), "other keys are swallowed");
        assert_eq!(card.key(KeyCode::Enter), Some(Choice::Close));
        assert!(!card.wants());
        card.offer("2.0", &[digest("2.0", &[("Servers", "x")])], false);
        assert_eq!(card.key(KeyCode::KeyC), Some(Choice::Changelog));
        card.offer("2.0", &[digest("2.0", &[("Servers", "x")])], false);
        assert_eq!(card.key(KeyCode::Escape), Some(Choice::Close));
        card.offer("2.0", &[], false);
        assert!(!card.wants());
    }

    #[test]
    fn the_marker_is_read_once_a_run() {
        let mut card = Card::default();
        assert!(card.take_check());
        assert!(!card.take_check());
    }

    #[test]
    fn several_releases_are_named_and_a_missing_recap_falls_back_to_the_introduction() {
        let mut card = Card::default();
        card.offer(
            "3.0",
            &[
                digest("3.0", &[("Cards", "Two cards.")]),
                digest("2.0", &[]),
            ],
            false,
        );
        laid_out(&mut card);
        assert!(matches!(card.items[0], Item::Release(_)));
        assert!(matches!(card.items[1], Item::Line(..)));
        assert!(matches!(card.items[2], Item::Release(_)));
        assert!(matches!(card.items[3], Item::Intro(_)));
        assert_eq!(card.title, "3.0");
        // A long introduction is cut at three lines, with an ellipsis.
        let long = Digest {
            intro: "word ".repeat(80),
            ..digest("1.0", &[])
        };
        card.offer("1.0", &[long], false);
        laid_out(&mut card);
        let Some(Item::Intro(rows)) = card.items.first() else {
            panic!("{:?}", card.items);
        };
        assert_eq!(rows.len(), INTRO_LINES);
        let last = rows[INTRO_LINES - 1].last().unwrap();
        assert!(last.text.ends_with("..."), "{rows:?}");
        assert!(last.x + last.width <= TEXT_WIDTH);
    }

    #[test]
    fn the_heading_sits_over_a_sentence_that_takes_the_card_width() {
        let mut card = Card::default();
        let sentence = "word ".repeat(30);
        card.offer("2.0", &[digest("2.0", &[("Servers", &sentence)])], false);
        laid_out(&mut card);
        let Some(Item::Line(heading, rows)) = card.items.first() else {
            panic!("{:?}", card.items);
        };
        assert_eq!(heading, "Servers");
        // 150 characters of 12 pixels in rows 832 wide: three rows, starting at the
        // text's left edge, each one run.
        assert_eq!(rows.len(), 3, "{rows:?}");
        for row in rows {
            assert_eq!(row.len(), 1);
            assert_eq!(row[0].x, 0.0);
            assert!(row[0].width <= TEXT_WIDTH, "{row:?}");
        }
        assert_eq!(
            card.items[0].height(),
            HEADING_HEIGHT + 3.0 * LINE + ROW_GAP
        );
    }

    #[test]
    fn code_in_backticks_is_drawn_in_chips_that_are_never_cut() {
        let font = test_font();
        let measure = Measure::new(&font, TextStyle::NEUTRAL);
        let rows = flow(
            "Type `whatsnew`, or `cl_allowDownload 1`.",
            TEXT_WIDTH,
            &measure,
        );
        assert_eq!(rows.len(), 1);
        let texts: Vec<(&str, bool)> = rows[0]
            .iter()
            .map(|run| (run.text.as_str(), run.code))
            .collect();
        assert_eq!(
            texts,
            [
                ("Type", false),
                ("whatsnew", true),
                (", or", false),
                ("cl_allowDownload 1", true),
                (".", false),
            ]
        );
        let chip = &rows[0][1];
        // Its letters (8 of 17 / 12 * 8 wide) and the padding either side.
        let letters = 8.0 * 8.0 * CHIP_SIZE / 12.0;
        assert!(
            (chip.width - letters - CHIP_PAD * 2.0).abs() < 0.01,
            "{chip:?}"
        );
        // The comma is glued to the chip and shares a run with the next word; the next
        // chip is a space after it.
        assert_eq!(rows[0][2].x, chip.x + chip.width);
        let space = 12.0;
        assert_eq!(rows[0][3].x, rows[0][2].x + rows[0][2].width + space);

        // A chip that does not fit the rest of a row moves to the next one whole, with
        // the full stop glued to it.
        let narrow = flow("aaaa `bbbb cccc`.", 150.0, &measure);
        assert_eq!(narrow.len(), 2, "{narrow:?}");
        assert_eq!(narrow[0][0].text, "aaaa");
        assert_eq!(narrow[1][0].text, "bbbb cccc");
        assert!(narrow[1][0].code);
        assert_eq!(narrow[1][1].text, ".");
    }

    #[test]
    fn a_long_recap_stops_with_a_pointer_to_the_changelog() {
        let long = "word ".repeat(60);
        let lines: Vec<(&str, &str)> = (0..8).map(|_| ("Heading", long.as_str())).collect();
        let mut card = Card::default();
        card.offer("3.0", &[digest("3.0", &lines)], false);
        laid_out(&mut card);
        assert_eq!(card.items.last(), Some(&Item::More));
        assert!(card.body_height() <= BODY_MAX, "{}", card.body_height());
        assert!(card.body_height() < FRAME_HEIGHT - HEAD_HEIGHT - 200.0);
    }

    #[test]
    fn the_layout_is_kept_until_the_measure_changes() {
        let font = test_font();
        let mut card = Card::default();
        card.offer(
            "2.0",
            &[digest("2.0", &[("Servers", "Downloads work.")])],
            false,
        );
        card.lay_out(&Measure::new(&font, TextStyle::NEUTRAL));
        let first = card.laid_for;
        card.items.clear();
        card.lay_out(&Measure::new(&font, TextStyle::NEUTRAL));
        assert!(card.items.is_empty(), "laid out once for a measure");
        let larger = TextStyle {
            scale: 1.25,
            ..TextStyle::NEUTRAL
        };
        card.lay_out(&Measure::new(&font, larger));
        assert_ne!(card.laid_for, first);
        assert!(!card.items.is_empty());
    }

    #[test]
    fn the_real_changelog_recap_fits_the_card() {
        let newest = crate::console::changelog::newest_digest().unwrap();
        assert!(!newest.lines.is_empty(), "{} has no recap", newest.title);
        let mut card = Card::default();
        card.offer("", &[newest], true);
        laid_out(&mut card);
        assert!(card.wants());
        assert!(!card.items.contains(&Item::More), "{:?}", card.items);
        assert!(card.body_height() < FRAME_HEIGHT - HEAD_HEIGHT - 200.0);
    }
}
