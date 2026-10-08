//! The Profile page's drawing, in the SJK UI's look over the map whatever the menu
//! style: the top bar with the two tabs; on Profile, who the player is and their
//! record on the left, the bio in the middle and their medals and achievements on the
//! right; on Achievements, the board, three columns of cards.

use super::*;
use crate::achievements::{self, Category};
use crate::menu::sjk::{
    Frame, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar, wrap,
};
use crate::menu_widgets::TextFamily;
use crate::text::UiFont;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The columns of the Profile tab (frame pixels).
const LEFT_X: f32 = 96.0;
const LEFT_WIDTH: f32 = 520.0;
const MIDDLE_X: f32 = 680.0;
const MIDDLE_WIDTH: f32 = 620.0;
const RIGHT_X: f32 = 1_364.0;
const RIGHT_WIDTH: f32 = 460.0;
/// Where the columns start, under the top bar.
const TOP: f32 = 170.0;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;
/// The bio's box: its top, height, inner padding and line.
const BIO_TOP: f32 = 216.0;
const BIO_HEIGHT: f32 = 300.0;
const BIO_PAD: f32 = 20.0;
const BIO_LINE: f32 = 26.0;
const BIO_SIZE: f32 = 18.0;
/// The board's cards: three columns, their size and gaps.
const CARD_TOP: f32 = 236.0;
const CARD_WIDTH: f32 = 560.0;
const CARD_HEIGHT: f32 = 94.0;
const CARD_GAP_X: f32 = 24.0;
const CARD_GAP_Y: f32 = 10.0;
const COLUMNS: usize = 3;
/// Most medals the Profile tab lists.
const MEDALS_SHOWN: usize = 4;
/// Most unlocks the Profile tab lists.
const UNLOCKS_SHOWN: usize = 4;

/// What the page says when the player holds no medal yet.
const NO_MEDALS: &str =
    "No medals yet. The SJK team gives medals for testing, contributing and more.";

/// A category's colour on the board.
fn tint(category: Category) -> Color {
    match category {
        Category::Combat => color::EMBER,
        Category::Duels => color::GOLD,
        Category::Journeys => color::HOLO,
        Category::Community => Color::new(0.45, 0.86, 0.62, 1.0),
    }
}

/// The goal as a medallion reads it: `1`, `100`, `1K`, `10h`.
fn goal_label(kind: &achievements::Kind) -> String {
    if kind.source == achievements::Source::Client(achievements::Counter::Minutes) {
        return format!("{}h", kind.goal / 60);
    }
    if kind.goal >= 1_000 && kind.goal.is_multiple_of(1_000) {
        return format!("{}K", kind.goal / 1_000);
    }
    kind.goal.to_string()
}

/// Who the player is, as the Profile tab's left column says it.
struct Who {
    headline: String,
    lines: Vec<(String, Color)>,
}

fn who(inputs: &Inputs<'_>) -> Who {
    let plain = |headline: &str, lines: &[&str]| Who {
        headline: headline.to_owned(),
        lines: lines
            .iter()
            .map(|line| ((*line).to_owned(), color::MUTED))
            .collect(),
    };
    if !inputs.enabled {
        return plain(
            "Identity is off",
            &[
                "Your profile lives on the SJK hub. Switch the identity on in Identity settings to show it to other players.",
                "Your achievements are counted on this PC meanwhile.",
            ],
        );
    }
    let Some(snapshot) = inputs.snapshot else {
        return plain("Starting...", &["Preparing your identity."]);
    };
    match &snapshot.status {
        Status::Disabled => plain("Identity is off", &["The settings were just changed."]),
        Status::NoHub => plain(
            "No hub is set",
            &["Set cl_hubUrl, or use the official hub in Identity settings."],
        ),
        Status::Registering => plain("Contacting the hub...", &[]),
        Status::Failed(error) => Who {
            headline: "Cannot reach the hub".to_owned(),
            lines: vec![
                (error.clone(), color::MUTED),
                ("Retrying automatically.".to_owned(), color::QUIET),
            ],
        },
        Status::Online => {
            let Some(me) = &snapshot.me else {
                return plain("Registered", &[]);
            };
            let mut lines = Vec::new();
            let since = crate::medals::date_text(me.created);
            let badge = if me.verified {
                ("Verified by the SJK team".to_owned(), color::GOLD_BRIGHT)
            } else {
                ("Not verified yet".to_owned(), color::QUIET)
            };
            lines.push(badge);
            if !since.is_empty() {
                lines.push((format!("Member since {since}"), color::MUTED));
            }
            lines.push((format!("Key id {}", me.key_id), color::QUIET));
            let earlier: Vec<&str> = me
                .names
                .iter()
                .map(|worn| worn.name.as_str())
                .filter(|name| {
                    sjk_identity::normal_form(name) != sjk_identity::normal_form(&me.name)
                })
                .take(3)
                .collect();
            if !earlier.is_empty() {
                lines.push((
                    format!("Also known as {}", earlier.join("^7, ")),
                    color::MUTED,
                ));
            }
            Who {
                headline: if me.name.is_empty() {
                    "Registered".to_owned()
                } else {
                    me.name.clone()
                },
                lines,
            }
        }
    }
}

/// `text` cut to `chars` characters with an ellipsis, colour codes counted as text.
fn cut(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_owned();
    }
    let kept: String = text.chars().take(chars.saturating_sub(3)).collect();
    format!("{kept}...")
}

impl Panel {
    /// Draw the page with what `inputs` says.
    pub(crate) fn append_sjk(
        &mut self,
        inputs: &Inputs<'_>,
        target: TextTarget<'_>,
        viewport: [f32; 2],
    ) {
        // The fonts outlive the target's vertex lists: measure with them, then append.
        let body = match &target {
            TextTarget::Families(fonts, _) => fonts.body.1,
            TextTarget::Inter(_, font) => *font,
        };
        self.build(inputs, body, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out, measuring the bio's lines in `body`.
    pub(super) fn build(&mut self, inputs: &Inputs<'_>, body: &UiFont, viewport: [f32; 2]) {
        self.sync(inputs);
        let frame = Frame::new(viewport);
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        let title = match self.tab {
            Tab::Profile => "Profile",
            Tab::Achievements => "Achievements",
        };
        top_bar(&mut self.ui, &frame, "Back", BACK_TOKEN, title, None);
        kit::segments(
            &mut self.ui,
            &frame,
            1_824.0,
            87.0,
            &["Profile", "Achievements"],
            self.tab.index(),
            self.focus == Focus::Tabs,
            TAB_TOKEN,
        );
        match self.tab {
            Tab::Profile => {
                self.left_column(&frame, inputs);
                self.bio_column(&frame, body);
                self.right_column(&frame, inputs);
            }
            Tab::Achievements => self.board(&frame, inputs.standings),
        }
        self.keys(&frame);
        self.ui.finish(self.focus_token());
    }

    fn left_column(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let who = who(inputs);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", cut(&who.headline, 40)),
            frame.rect(LEFT_X, TOP, LEFT_WIDTH, 56.0),
            44.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut y = TOP + 64.0;
        for (line, colour) in &who.lines {
            for part in wrap(line, 60).take(3) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(LEFT_X, y, LEFT_WIDTH, 26.0),
                    17.0 * s,
                    *colour,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 26.0;
            }
        }
        let button_y = y.max(TOP + 150.0) + 8.0;
        kit::button(
            &mut self.ui,
            frame,
            [LEFT_X, button_y, 220.0, 42.0],
            "Identity settings",
            false,
            true,
            self.focus == Focus::Identity,
            IDENTITY_TOKEN,
        );
        if self.staff {
            kit::button(
                &mut self.ui,
                frame,
                [LEFT_X + 236.0, button_y, 180.0, 42.0],
                "Staff tools",
                true,
                true,
                self.focus == Focus::Staff,
                STAFF_TOKEN,
            );
        }
        y = button_y + 42.0 + 34.0;
        kit::heading(&mut self.ui, frame, LEFT_X, y, LEFT_WIDTH, "Your record");
        y += 30.0;
        for (index, (label, value)) in inputs.record.iter().take(8).enumerate() {
            let x = LEFT_X + (index % 2) as f32 * 260.0;
            let row_y = y + (index / 2) as f32 * 74.0;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{value}"),
                frame.rect(x, row_y, 250.0, 40.0),
                34.0 * s,
                color::TEXT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(x, row_y + 40.0, 250.0, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let mut y = y + 4.0 * 74.0 + 14.0;
        let mut unlocked: Vec<&Standing> = inputs
            .standings
            .iter()
            .filter(|standing| standing.unlocked.is_some())
            .collect();
        unlocked.sort_by_key(|standing| std::cmp::Reverse(standing.unlocked.unwrap_or(0)));
        if unlocked.is_empty() || y + 60.0 > KEYS_Y - 20.0 {
            return;
        }
        kit::heading(
            &mut self.ui,
            frame,
            LEFT_X,
            y,
            LEFT_WIDTH,
            "Unlocked lately",
        );
        y += 26.0;
        for standing in unlocked.into_iter().take(UNLOCKS_SHOWN) {
            if y + 28.0 > KEYS_Y - 20.0 {
                break;
            }
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", standing.kind.name),
                frame.rect(LEFT_X, y, LEFT_WIDTH - 140.0, 28.0),
                21.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}",
                    crate::medals::date_text(standing.unlocked.unwrap_or(0))
                ),
                frame.rect(LEFT_X + LEFT_WIDTH - 140.0, y, 140.0, 28.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            y += 30.0;
        }
    }

    /// The bio's box, its counts, buttons and the rules, measured in `body`.
    fn bio_column(&mut self, frame: &Frame, body: &UiFont) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            TOP + 20.0,
            MIDDLE_WIDTH,
            "About you",
        );
        let focused = self.focus == Focus::Bio;
        let rect = frame.rect(MIDDLE_X, BIO_TOP, MIDDLE_WIDTH, BIO_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 16.0 * s,
            color: color::alpha(color::SPACE, 0.6),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 16.0 * s,
            width: 1.5 * s,
            color: if focused {
                Color::new(1.0, 1.0, 1.0, 0.7)
            } else {
                color::alpha(color::HOLO, 0.4)
            },
        });
        self.ui.hit_region(BIO_TOKEN, rect);
        let caret = focused && (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let room = MIDDLE_WIDTH - BIO_PAD * 2.0;
        let shown = if self.bio.is_empty() && !focused {
            None
        } else {
            Some(self.bio.clone())
        };
        let rows = BIO_HEIGHT - BIO_PAD * 2.0;
        let most = (rows / BIO_LINE) as usize;
        match shown {
            None => {
                let prompt = if self.writable {
                    "Say something about yourself: your clan, your style, where you play."
                } else {
                    "Your bio is kept on the SJK hub: switch the identity on to write one."
                };
                for (index, part) in wrap(prompt, 60).enumerate() {
                    text(
                        &mut self.ui,
                        TextFamily::Body,
                        format_args!("{part}"),
                        frame.rect(
                            MIDDLE_X + BIO_PAD,
                            BIO_TOP + BIO_PAD + index as f32 * BIO_LINE,
                            room,
                            BIO_LINE,
                        ),
                        BIO_SIZE * s,
                        color::QUIET,
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                }
            }
            Some(bio_text) => {
                let lines = bio_lines(&bio_text, body, room);
                // The end being written stays in view.
                let skip = lines.len().saturating_sub(most);
                let last = lines.len().saturating_sub(1);
                for (index, line) in lines.iter().enumerate().skip(skip) {
                    let row = (index - skip) as f32;
                    let mark = if caret && index == last { "|" } else { "" };
                    text(
                        &mut self.ui,
                        TextFamily::Body,
                        format_args!("{line}{mark}"),
                        frame.rect(
                            MIDDLE_X + BIO_PAD,
                            BIO_TOP + BIO_PAD + row * BIO_LINE,
                            room,
                            BIO_LINE,
                        ),
                        BIO_SIZE * s,
                        if self.writable {
                            color::TEXT
                        } else {
                            color::MUTED
                        },
                        FontWeight::Regular,
                        TextAlign::Start,
                    );
                }
            }
        }
        let below = BIO_TOP + BIO_HEIGHT + 18.0;
        let characters = self.bio.chars().count();
        let lines = if self.bio.is_empty() {
            0
        } else {
            self.bio.split('\n').count()
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{characters} / {} characters, {lines} / {} lines",
                bio::BIO_MAX,
                bio::LINES_MAX
            ),
            frame.rect(MIDDLE_X, below, 330.0, 26.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let mut x = MIDDLE_X + MIDDLE_WIDTH;
        for (label, primary, token, focus) in [
            ("Save", true, SAVE_TOKEN, Focus::Save),
            ("Revert", false, REVERT_TOKEN, Focus::Revert),
        ] {
            let width = 130.0;
            x -= width;
            kit::button(
                &mut self.ui,
                frame,
                [x, below - 10.0, width, 46.0],
                label,
                primary,
                self.writable && (primary || self.edited),
                self.focus == focus,
                token,
            );
            x -= 14.0;
        }
        let mut y = below + 56.0;
        if !self.message.is_empty() {
            let message = cut(&self.message, 80);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{message}"),
                frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 26.0),
                17.0 * s,
                if self.message == "Saved" {
                    color::GOLD_BRIGHT
                } else {
                    color::EMBER
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 34.0;
        }
        for part in wrap(
            "Letters, digits, spaces and simple punctuation, up to 6 lines. No emoji or symbols. A caret and a digit colour the text. Everyone can read it.",
            92,
        ) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 24.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 24.0;
        }
    }

    fn right_column(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            RIGHT_X,
            TOP + 20.0,
            RIGHT_WIDTH,
            "Medals",
        );
        let mut y = TOP + 48.0;
        let medals = inputs
            .snapshot
            .and_then(|snapshot| snapshot.me.as_ref())
            .map(|me| crate::medals::awards(&me.medals))
            .unwrap_or_default();
        if medals.is_empty() {
            for part in wrap(NO_MEDALS, 48) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{part}"),
                    frame.rect(RIGHT_X, y, RIGHT_WIDTH, 24.0),
                    16.0 * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 24.0;
            }
            y += 16.0;
        }
        for award in medals.iter().take(MEDALS_SHOWN) {
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(RIGHT_X, y, 84.0, 84.0),
                texture: award.medal.art(),
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", award.label()),
                frame.rect(RIGHT_X + 98.0, y + 8.0, RIGHT_WIDTH - 98.0, 30.0),
                23.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let given = award.given();
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}",
                    if given.is_empty() {
                        award.medal.description()
                    } else {
                        &given
                    }
                ),
                frame.rect(RIGHT_X + 98.0, y + 40.0, RIGHT_WIDTH - 98.0, 24.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 96.0;
        }
        if medals.len() > MEDALS_SHOWN {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("and {} more", medals.len() - MEDALS_SHOWN),
                frame.rect(RIGHT_X, y, RIGHT_WIDTH, 24.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 30.0;
        }
        let y = y.max(TOP + 300.0) + 20.0;
        kit::heading(&mut self.ui, frame, RIGHT_X, y, RIGHT_WIDTH, "Achievements");
        let total = inputs.standings.len();
        let unlocked = inputs
            .standings
            .iter()
            .filter(|standing| standing.unlocked.is_some())
            .count();
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{unlocked} of {total} unlocked"),
            frame.rect(RIGHT_X, y + 26.0, RIGHT_WIDTH, 40.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        bar(
            &mut self.ui,
            frame,
            [RIGHT_X, y + 76.0, RIGHT_WIDTH, 8.0],
            if total == 0 {
                0.0
            } else {
                unlocked as f32 / total as f32
            },
            color::GOLD,
        );
        kit::button(
            &mut self.ui,
            frame,
            [RIGHT_X, y + 104.0, 240.0, 46.0],
            "See the board",
            false,
            true,
            self.focus == Focus::Board,
            BOARD_TOKEN,
        );
    }

    /// The achievements board: how many are unlocked, then every achievement's card.
    fn board(&mut self, frame: &Frame, standings: &[Standing]) {
        let s = frame.s;
        let unlocked = standings
            .iter()
            .filter(|standing| standing.unlocked.is_some())
            .count();
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{unlocked} of {} unlocked", standings.len()),
            frame.rect(LEFT_X, TOP - 4.0, 520.0, 44.0),
            34.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        bar(
            &mut self.ui,
            frame,
            [LEFT_X + 360.0, TOP + 14.0, 560.0, 8.0],
            if standings.is_empty() {
                0.0
            } else {
                unlocked as f32 / standings.len() as f32
            },
            color::GOLD,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Counted on this PC in your matches on servers, and kept on the SJK hub with your identity on."
            ),
            frame.rect(LEFT_X, TOP + 36.0, 1_200.0, 24.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        for (index, standing) in standings.iter().enumerate() {
            let column = index % COLUMNS;
            let row = index / COLUMNS;
            let x = LEFT_X + column as f32 * (CARD_WIDTH + CARD_GAP_X);
            let y = CARD_TOP + row as f32 * (CARD_HEIGHT + CARD_GAP_Y);
            if y + CARD_HEIGHT > KEYS_Y - 16.0 {
                break;
            }
            self.card(frame, standing, x, y);
        }
    }

    /// One achievement's card at (`x`, `y`).
    fn card(&mut self, frame: &Frame, standing: &Standing, x: f32, y: f32) {
        let s = frame.s;
        let kind = standing.kind;
        let done = standing.unlocked.is_some();
        let hue = tint(kind.category);
        let rect = frame.rect(x, y, CARD_WIDTH, CARD_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 14.0 * s,
            color: if done {
                Color::new(0.07, 0.09, 0.16, 0.92)
            } else {
                color::alpha(color::SPACE, 0.62)
            },
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 14.0 * s,
            width: 1.2 * s,
            color: if done {
                color::alpha(color::GOLD, 0.55)
            } else {
                color::alpha(color::HOLO, 0.18)
            },
        });
        // The medallion: a ring filling with the count, gold and lit once unlocked.
        let centre = frame.point(x + 50.0, y + CARD_HEIGHT * 0.5);
        let radius = 31.0;
        if done {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(
                    x + 50.0 - radius,
                    y + CARD_HEIGHT * 0.5 - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                radius: radius * s,
                color: color::alpha(hue, 0.28),
            });
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
            center: centre,
            radius: radius * s,
            width: 3.0 * s,
            start: 0.0,
            sweep: std::f32::consts::TAU,
            color: color::alpha(color::HOLO, 0.16),
            knockout: None,
        });
        let fraction = standing.fraction();
        if fraction > 0.0 {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                center: centre,
                radius: radius * s,
                width: 4.0 * s,
                start: -std::f32::consts::FRAC_PI_2,
                sweep: std::f32::consts::TAU * fraction,
                color: if done { color::GOLD_BRIGHT } else { hue },
                knockout: None,
            });
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", goal_label(kind)),
            frame.rect(
                x + 50.0 - radius,
                y + CARD_HEIGHT * 0.5 - 16.0,
                radius * 2.0,
                32.0,
            ),
            22.0 * s,
            if done {
                color::GOLD_BRIGHT
            } else {
                color::QUIET
            },
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let text_x = x + 98.0;
        let text_width = CARD_WIDTH - 98.0 - 18.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", kind.name),
            frame.rect(text_x, y + 10.0, text_width - 130.0, 30.0),
            24.0 * s,
            if done {
                color::GOLD_BRIGHT
            } else {
                color::TEXT
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", kind.category.name()),
            frame.rect(text_x + text_width - 150.0, y + 12.0, 150.0, 24.0),
            15.0 * s,
            color::alpha(hue, 0.9),
            FontWeight::Regular,
            TextAlign::End,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", kind.description),
            frame.rect(text_x, y + 40.0, text_width, 22.0),
            15.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let status = match standing.unlocked {
            Some(at) if at > 0 => format!("Unlocked {}", crate::medals::date_text(at)),
            Some(_) => "Unlocked".to_owned(),
            None => format!(
                "{} / {}",
                kind.amount(standing.progress),
                kind.amount(kind.goal)
            ),
        };
        bar(
            &mut self.ui,
            frame,
            [text_x, y + 72.0, text_width - 170.0, 6.0],
            fraction,
            if done { color::GOLD } else { hue },
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(text_x + text_width - 160.0, y + 64.0, 160.0, 22.0),
            14.0 * s,
            if done {
                color::GOLD_BRIGHT
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::End,
        );
    }

    /// The keys of what has the keyboard, right-aligned at the bottom.
    fn keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let typing = self.focus == Focus::Bio && self.writable;
        let enter = match self.focus {
            Focus::Tabs => "switch tab",
            Focus::Identity | Focus::Staff => "open",
            Focus::Bio | Focus::Save => "save",
            Focus::Revert => "revert",
            Focus::Board => "open",
        };
        let mut keys: Vec<(&[&str], &str)> = vec![(&["Tab"], "next"), (&["Enter"], enter)];
        if typing {
            keys.push((&["Shift", "Enter"], "new line"));
        }
        keys.push((&["Esc"], "back"));
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(1_824.0, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

/// A progress bar over `rect` (frame pixels), filled to `fraction` in `fill`.
fn bar(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    rect: [f32; 4],
    fraction: f32,
    fill: Color,
) {
    let [x, y, width, height] = rect;
    let s = frame.s;
    let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
        rect: frame.rect(x, y, width, height),
        radius: height * 0.5 * s,
        color: color::alpha(color::HOLO, 0.12),
    });
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y, (width * fraction).max(height), height),
            radius: height * 0.5 * s,
            color: fill,
        });
    }
}

/// The bio's lines as the box draws them: each of its lines broken at spaces to fit
/// `room` pixels in `body` at the box's size; an empty line stays a line.
fn bio_lines(bio_text: &str, body: &UiFont, room: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for line in bio_text.split('\n') {
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }
        lines.extend(crate::text_dialog::wrap_to(line, body, BIO_SIZE, 0.0, room));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::super::tests::snapshot;
    use super::*;
    use sjk_identity::{Medal, Profile, WornName};

    struct Fonts {
        display: crate::text::FontAtlas,
        body: crate::text::FontAtlas,
    }

    fn fonts() -> Fonts {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a bundled family");
        Fonts {
            display: load(&crate::text::DISPLAY),
            body: load(&crate::text::BODY),
        }
    }

    fn full_profile() -> Profile {
        Profile {
            key_id: "0123456789abcdef".to_owned(),
            key: String::new(),
            name: "^1Ш^7Sol^3Vulpes^7 the Long Named".to_owned(),
            bio: format!(
                "{}\n\nline three\n{}",
                "Äöüß word ".repeat(30),
                "x".repeat(60)
            ),
            verified: true,
            staff: false,
            created: 1_759_708_800,
            names: ["^2Fox", "Sol", "^4Another Long Name", "Fourth"]
                .iter()
                .map(|name| WornName {
                    name: (*name).to_owned(),
                    first_seen: 0,
                    last_seen: 0,
                })
                .collect(),
            medals: [
                "early_tester",
                "early_contributor",
                "bug_hunter",
                "jof_clan",
                "unknown",
            ]
            .iter()
            .map(|id| Medal {
                id: (*id).to_owned(),
                count: 3,
                awarded: 1_759_900_000,
                note: "a note".to_owned(),
            })
            .collect(),
            achievements: Vec::new(),
        }
    }

    fn record() -> Vec<(&'static str, String)> {
        [
            "Players defeated",
            "Saber kills",
            "Best streak",
            "Duels won",
            "Flags captured",
            "Maps played",
            "Servers",
            "Time played",
        ]
        .iter()
        .map(|label| (*label, "123456".to_owned()))
        .collect()
    }

    /// Half unlocked, half part-way.
    fn standings() -> Vec<Standing> {
        achievements::ALL
            .iter()
            .enumerate()
            .map(|(index, kind)| Standing {
                kind,
                progress: if index % 2 == 0 {
                    kind.goal
                } else {
                    kind.goal / 3
                },
                unlocked: (index % 2 == 0).then_some(1_759_900_000),
            })
            .collect()
    }

    /// Every tab, focus and state, at its longest, fits the canvas at 1080 lines, 4K,
    /// 4:3 and 21:9, in the families and in Inter.
    #[test]
    fn every_state_fits_the_canvas() {
        let families = fonts();
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let record = record();
        let standings = standings();
        let online = snapshot(Some(full_profile()), Some("saved"));
        let offline = Snapshot {
            status: Status::Failed("the hub took too long to answer, again and again".into()),
            ..snapshot(None, None)
        };
        for (enabled, shot) in [(true, Some(&online)), (true, Some(&offline)), (false, None)] {
            for tab in Tab::ALL {
                for focus in [
                    Focus::Tabs,
                    Focus::Identity,
                    Focus::Bio,
                    Focus::Save,
                    Focus::Revert,
                    Focus::Board,
                ] {
                    for body in [&families.body.font, &inter.font] {
                        for viewport in [
                            [1920.0, 1080.0],
                            [3840.0, 2160.0],
                            [1440.0, 1080.0],
                            [2560.0, 1080.0],
                        ] {
                            let mut panel = Panel::new();
                            panel.open(tab, true);
                            panel.focus = focus;
                            panel.message = "a bio uses letters, digits, spaces and simple punctuation (no emoji or symbols)".into();
                            let inputs = Inputs {
                                enabled,
                                snapshot: shot,
                                standings: &standings,
                                record: &record,
                            };
                            panel.build(&inputs, body, viewport);
                            assert!(
                                !panel.ui.overflowed(),
                                "{tab:?} {focus:?} {enabled} at {viewport:?}"
                            );
                        }
                    }
                }
            }
        }
        let _ = &families.display;
    }

    /// The bio's lines fit the box's width and a long bio shows its end.
    #[test]
    fn the_bio_wraps_inside_its_box() {
        let fonts = fonts();
        let body = &fonts.body.font;
        let room = MIDDLE_WIDTH - BIO_PAD * 2.0;
        let lines = bio_lines(&full_profile().bio, body, room);
        assert!(lines.len() > 4);
        assert_eq!(lines.iter().filter(|line| line.is_empty()).count(), 1);
        let scale = BIO_SIZE / body.height;
        for line in &lines {
            assert!(
                crate::text::visible_text_width(body, line, scale) <= room,
                "{line}"
            );
        }
    }

    #[test]
    fn the_board_shows_every_achievement_and_the_tabs_answer_the_pointer() {
        let fonts = fonts();
        let standings = standings();
        let record = record();
        let online = snapshot(Some(full_profile()), None);
        let mut panel = Panel::new();
        panel.open(Tab::Achievements, true);
        let inputs = Inputs {
            enabled: true,
            snapshot: Some(&online),
            standings: &standings,
            record: &record,
        };
        panel.build(&inputs, &fonts.body.font, [1920.0, 1080.0]);
        let names: usize = panel
            .ui
            .draw_list()
            .commands()
            .iter()
            .filter(|command| matches!(command, DrawCommand::Arc { .. }))
            .count();
        assert!(
            names >= achievements::ALL.len(),
            "every card has its medallion"
        );
        let tab = panel.ui.rect_for(TAB_TOKEN).expect("the Profile tab");
        let at = sjk_ui::Vec2::new(tab.x + tab.width * 0.5, tab.y + tab.height * 0.5);
        for event in [
            InputEvent::PointerMove(at),
            InputEvent::PointerPress {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
            InputEvent::PointerRelease {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
        ] {
            let _ = panel.handle_pointer(event, None);
        }
        assert_eq!(panel.tab(), Tab::Profile);
    }

    #[test]
    fn medallions_read_the_goal() {
        let label = |id| goal_label(achievements::find(id).unwrap());
        assert_eq!(label("first_blood"), "1");
        assert_eq!(label("kills_1000"), "1K");
        assert_eq!(label("hours_100"), "100h");
        assert_eq!(label("arsenal"), "8");
    }
}
