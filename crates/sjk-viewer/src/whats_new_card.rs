//! The what's-new card (`docs/sjk-ui.md`, "What's new card"): the first time the player
//! opens a game that was updated, an SJK pop-up card over the darkened screen tells what
//! the update brought, in a few short lines (the release's recap in `CHANGELOG.md`,
//! `docs/sjk.md` "Changelog"). Close (gold, Enter) puts it away for good; Full changelog
//! (C) opens the changelog page. A player who skipped releases sees the recaps of those
//! releases, newest first, up to `changelog::DIGEST_LIMIT`.
//!
//! `cl_whatsNewSeen` holds the version whose recap the player closed. The card shows on
//! the main menu or over the game menu, never in the middle of play. A new installation
//! (no marker and First setup still to do) writes the marker without showing the card;
//! a local build, which no release lists, shows nothing. The `whatsnew` command shows the
//! newest recap, the unreleased one included, to preview it. The card is drawn in the SJK
//! UI's look in every menu style, and the menus under it are not drawn and take no input,
//! as under the other cards.

use crate::console::changelog::Digest;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, wrap};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{FontWeight, InputEvent, TextAlign, UiEventKind};
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
const ROW_GAP: f32 = 12.0;
const RELEASE_HEIGHT: f32 = 40.0;
/// A recap row's heading column, and the characters a line of its sentence holds.
const HEADING_WIDTH: f32 = 210.0;
const TEXT_CHARS: usize = 76;
/// Characters a line of a release's introduction holds (the full width).
const INTRO_CHARS: usize = 84;
/// Introduction lines shown for a release without a recap.
const INTRO_LINES: usize = 3;
/// Sentence lines the card holds before it points to the changelog.
const LINES: usize = 16;
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

/// One block of the card's recap, wrapped.
#[derive(Debug, Eq, PartialEq)]
enum Item {
    /// A release's name and date, when the card holds several.
    Release(String),
    /// A recap line: its heading and the sentence's lines.
    Line(String, Vec<String>),
    /// A release's introduction, standing in for a recap it lacks.
    Intro(Vec<String>),
    /// What did not fit.
    More,
}

impl Item {
    fn height(&self) -> f32 {
        match self {
            Self::Release(_) => RELEASE_HEIGHT,
            Self::Line(_, lines) | Self::Intro(lines) => lines.len() as f32 * LINE + ROW_GAP,
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
    items: Vec<Item>,
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
            items: Vec::new(),
            open: false,
            ui: MenuCanvas::with_capacities(64, 200, 96),
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
        self.items.clear();
        self.version = version.to_owned();
        self.kicker = if preview { "Preview" } else { "SJK updated" };
        let Some(first) = digests.first() else {
            self.title.clear();
            return;
        };
        self.title = first.title.clone();
        let mut spent = 0;
        let mut more = false;
        'releases: for digest in digests {
            if digests.len() > 1 {
                self.items.push(Item::Release(release_label(digest)));
            }
            if digest.lines.is_empty() {
                let mut lines: Vec<String> = wrap(&digest.intro, INTRO_CHARS)
                    .map(str::to_owned)
                    .collect();
                if lines.len() > INTRO_LINES {
                    // Cut short, so it says so.
                    lines.truncate(INTRO_LINES);
                    if let Some(last) = lines.last_mut() {
                        let keep = INTRO_CHARS.saturating_sub(3);
                        let cut: String = last.chars().take(keep).collect();
                        *last = format!("{}...", cut.trim_end());
                    }
                }
                spent += lines.len();
                if !lines.is_empty() {
                    self.items.push(Item::Intro(lines));
                }
                continue;
            }
            for (heading, sentence) in &digest.lines {
                let lines: Vec<String> = wrap(sentence, TEXT_CHARS).map(str::to_owned).collect();
                if spent + lines.len().max(1) > LINES {
                    more = true;
                    break 'releases;
                }
                spent += lines.len().max(1);
                self.items.push(Item::Line(heading.clone(), lines));
            }
        }
        if more {
            self.items.push(Item::More);
        }
    }

    /// Whether the card has something to show.
    pub(crate) fn wants(&self) -> bool {
        !self.items.is_empty()
    }

    /// The card is answered: it puts the recap away.
    fn put_away(&mut self) {
        self.items.clear();
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

    /// Draw the card over the whole frame.
    pub(crate) fn append(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
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
                    text(
                        ui,
                        TextFamily::Display,
                        format_args!("{label}"),
                        frame.rect(TEXT_X, y + 6.0, TEXT_WIDTH, 28.0),
                        20.0 * s,
                        color::GOLD,
                        FontWeight::Semibold,
                        TextAlign::Start,
                    );
                }
                Item::Line(heading, lines) => {
                    text(
                        ui,
                        TextFamily::Body,
                        format_args!("{heading}"),
                        frame.rect(TEXT_X, y, HEADING_WIDTH - 12.0, LINE),
                        18.0 * s,
                        color::GOLD_BRIGHT,
                        FontWeight::Semibold,
                        TextAlign::Start,
                    );
                    for (index, line) in lines.iter().enumerate() {
                        text(
                            ui,
                            TextFamily::Body,
                            format_args!("{line}"),
                            frame.rect(
                                TEXT_X + HEADING_WIDTH,
                                y + index as f32 * LINE,
                                TEXT_WIDTH - HEADING_WIDTH,
                                LINE,
                            ),
                            18.0 * s,
                            color::TEXT,
                            FontWeight::Regular,
                            TextAlign::Start,
                        );
                    }
                }
                Item::Intro(lines) => {
                    for (index, line) in lines.iter().enumerate() {
                        text(
                            ui,
                            TextFamily::Body,
                            format_args!("{line}"),
                            frame.rect(TEXT_X, y + index as f32 * LINE, TEXT_WIDTH, LINE),
                            18.0 * s,
                            color::MUTED,
                            FontWeight::Regular,
                            TextAlign::Start,
                        );
                    }
                }
                Item::More => {
                    text(
                        ui,
                        TextFamily::Body,
                        format_args!("More in the full changelog."),
                        frame.rect(TEXT_X, y, TEXT_WIDTH, LINE),
                        18.0 * s,
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

/// "2026.1011.3 (Alpha)   -   11/10/2026", or just the name when it has no date.
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
        let Some(Item::Intro(lines)) = card.items.first() else {
            panic!("{:?}", card.items);
        };
        assert_eq!(lines.len(), INTRO_LINES);
        assert!(lines[INTRO_LINES - 1].ends_with("..."), "{lines:?}");
        assert!(lines[INTRO_LINES - 1].chars().count() <= INTRO_CHARS);
    }

    #[test]
    fn a_long_recap_stops_with_a_pointer_to_the_changelog() {
        let long = "word ".repeat(60);
        let lines: Vec<(&str, &str)> = (0..8).map(|_| ("Heading", long.as_str())).collect();
        let mut card = Card::default();
        card.offer("3.0", &[digest("3.0", &lines)], false);
        assert_eq!(card.items.last(), Some(&Item::More));
        let spent: usize = card
            .items
            .iter()
            .map(|item| match item {
                Item::Line(_, lines) => lines.len(),
                _ => 0,
            })
            .sum();
        assert!(spent <= LINES, "{spent}");
        assert!(card.body_height() < FRAME_HEIGHT - HEAD_HEIGHT - 200.0);
    }

    #[test]
    fn the_real_changelog_recap_fits_the_card() {
        let newest = crate::console::changelog::newest_digest().unwrap();
        assert!(!newest.lines.is_empty(), "{} has no recap", newest.title);
        let mut card = Card::default();
        card.offer("", &[newest], true);
        assert!(card.wants());
        assert!(!card.items.contains(&Item::More), "{:?}", card.items);
        assert!(card.body_height() < FRAME_HEIGHT - HEAD_HEIGHT - 200.0);
    }
}
