//! The Staff page's drawing, in the SJK UI's look: the search pill in the top bar,
//! the players found down the left, the chosen player's medals with Give and Take
//! back in the middle, and their achievements with Clear on the right.

use super::*;
use crate::achievements;
use crate::medals::Medal;
use crate::menu::sjk::{
    Frame, SearchPill, TextTarget, color, key_hint, key_hint_width, kit, text, top_bar, wrap,
};
use crate::menu_widgets::TextFamily;
use crate::text::UiFont;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The columns (frame pixels).
const LEFT_X: f32 = 96.0;
const LEFT_WIDTH: f32 = 560.0;
const MIDDLE_X: f32 = 720.0;
const MIDDLE_WIDTH: f32 = 520.0;
const RIGHT_X: f32 = 1_300.0;
const RIGHT_WIDTH: f32 = 524.0;
const TOP: f32 = 170.0;
/// The players' rows.
const PLAYERS_TOP: f32 = 236.0;
const PLAYER_ROW: f32 = 50.0;
/// The medals' rows and the achievements' rows.
const SECTION_TOP: f32 = 330.0;
const MEDAL_ROW: f32 = 76.0;
const ACHIEVEMENT_ROW: f32 = 34.0;
const ACHIEVEMENTS_SHOWN: usize = 18;
/// The keys' line.
const KEYS_Y: f32 = 1_010.0;

/// `text` cut to `chars` characters with an ellipsis.
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
        let body = match &target {
            TextTarget::Families(fonts, _) => fonts.body.1,
            TextTarget::Inter(_, font) => *font,
        };
        self.build(inputs, body, viewport);
        target.append(&self.ui, viewport);
    }

    /// Lay the page out; `_body` is the body family, kept for measured layouts.
    pub(super) fn build(&mut self, inputs: &Inputs<'_>, _body: &UiFont, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        self.order.clear();
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        let caret = (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let query = if self.focus == SEARCH_TOKEN && caret {
            format!("{}|", self.query)
        } else {
            self.query.clone()
        };
        top_bar(
            &mut self.ui,
            &frame,
            "Back",
            BACK_TOKEN,
            "Staff",
            Some(SearchPill {
                query: &query,
                active: self.focus == SEARCH_TOKEN,
                prompt: "Find a player by name or key id",
                found: (!inputs.staff.players.is_empty()).then_some(inputs.staff.players.len()),
                token: SEARCH_TOKEN,
            }),
        );
        self.order.push(SEARCH_TOKEN);
        self.shown.me = inputs.me.map(|me| me.key_id.clone());
        self.players(&frame, inputs);
        let target = self.target(inputs).cloned();
        self.shown.target = target.as_ref().map(|profile| profile.key_id.clone());
        self.shown.medals = [0; Medal::COUNT];
        if let Some(profile) = &target {
            for medal in &profile.medals {
                if let Some(known) = Medal::from_id(&medal.id) {
                    self.shown.medals[known.index()] = medal.count.max(1);
                }
            }
            let mine = self.shown.me.as_deref() == Some(profile.key_id.as_str());
            self.header(&frame, profile, mine);
            self.medals(&frame, profile);
            self.achievements(&frame, profile, mine);
        }
        self.status(&frame, inputs.staff);
        self.keys(&frame);
        if !self.order.contains(&self.focus) {
            self.focus = SEARCH_TOKEN;
        }
        self.ui.finish(self.focus);
    }

    /// The players found, with Me and Recent over them.
    fn players(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        for (index, (label, token)) in [("Me", ME_TOKEN), ("Seen lately", RECENT_TOKEN)]
            .into_iter()
            .enumerate()
        {
            let width = if index == 0 { 90.0 } else { 160.0 };
            let x = LEFT_X + index as f32 * 104.0;
            kit::button(
                &mut self.ui,
                frame,
                [x, TOP - 2.0, width, 42.0],
                label,
                false,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
        }
        self.shown.players.clear();
        let selected = self.shown_selection(inputs);
        for (index, player) in inputs.staff.players.iter().take(PLAYERS_SHOWN).enumerate() {
            let token = PLAYER_BASE + index as u16;
            let y = PLAYERS_TOP + index as f32 * PLAYER_ROW;
            let row = [LEFT_X, y, LEFT_WIDTH, PLAYER_ROW - 4.0];
            if self.focus == token || selected.as_deref() == Some(player.key_id.as_str()) {
                kit::band(&mut self.ui, frame, row);
            }
            let name = if player.name.is_empty() {
                "(no name yet)".to_owned()
            } else {
                cut(&player.name, 34)
            };
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(LEFT_X + 18.0, y + 2.0, LEFT_WIDTH - 150.0, 26.0),
                21.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}", player.key_id),
                frame.rect(LEFT_X + 18.0, y + 26.0, 220.0, 18.0),
                13.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let tags = match (player.staff, player.verified) {
                (true, true) => "Staff, verified",
                (true, false) => "Staff",
                (false, true) => "Verified",
                (false, false) => "",
            };
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{tags}"),
                frame.rect(LEFT_X + LEFT_WIDTH - 150.0, y + 10.0, 136.0, 24.0),
                16.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Regular,
                TextAlign::End,
            );
            self.ui
                .hit_region(token, frame.rect(row[0], row[1], row[2], row[3]));
            self.order.push(token);
            self.shown.players.push(player.key_id.clone());
        }
        if inputs.staff.players.len() > PLAYERS_SHOWN {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "and {} more: refine the search",
                    inputs.staff.players.len() - PLAYERS_SHOWN
                ),
                frame.rect(
                    LEFT_X + 18.0,
                    PLAYERS_TOP + PLAYERS_SHOWN as f32 * PLAYER_ROW,
                    LEFT_WIDTH,
                    22.0,
                ),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// The key of the chosen player, as the list marks it.
    fn shown_selection(&self, inputs: &Inputs<'_>) -> Option<String> {
        self.selected
            .clone()
            .or_else(|| inputs.me.map(|me| me.key_id.clone()))
    }

    /// The chosen player's name and facts over the middle and right columns.
    fn header(&mut self, frame: &Frame, profile: &Profile, mine: bool) {
        let s = frame.s;
        let name = if profile.name.is_empty() {
            "(no name yet)".to_owned()
        } else {
            cut(&profile.name, 40)
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(MIDDLE_X, TOP - 6.0, 1_100.0, 48.0),
            38.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut facts = vec![format!("Key id {}", profile.key_id)];
        if profile.verified {
            facts.push("verified".to_owned());
        }
        if profile.staff {
            facts.push("staff".to_owned());
        }
        let since = crate::medals::date_text(profile.created);
        if !since.is_empty() {
            facts.push(format!("member since {since}"));
        }
        if mine {
            facts.push("this is you".to_owned());
        }
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", facts.join(", ")),
            frame.rect(MIDDLE_X, TOP + 46.0, 1_100.0, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// Every medal of the catalogue with what the player holds, Give and Take back,
    /// then the note sent with the next one.
    fn medals(&mut self, frame: &Frame, profile: &Profile) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            MIDDLE_X,
            SECTION_TOP - 20.0,
            MIDDLE_WIDTH,
            "Medals",
        );
        for medal in Medal::ALL {
            let y = SECTION_TOP + medal.index() as f32 * MEDAL_ROW;
            let held = self.shown.medals[medal.index()];
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(MIDDLE_X, y + 4.0, 60.0, 60.0),
                texture: medal.icon(),
                color: Color::new(1.0, 1.0, 1.0, if held > 0 { 1.0 } else { 0.35 }),
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", medal.label(held)),
                frame.rect(MIDDLE_X + 74.0, y + 6.0, 220.0, 28.0),
                21.0 * s,
                if held > 0 {
                    color::GOLD_BRIGHT
                } else {
                    color::MUTED
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            let given = profile
                .medals
                .iter()
                .find(|entry| entry.id == medal.id())
                .map(|entry| crate::medals::date_text(entry.awarded))
                .filter(|date| !date.is_empty());
            let line = match (held, given) {
                (0, _) => "Not held".to_owned(),
                (_, Some(date)) => format!("Given {date}"),
                (_, None) => "Held".to_owned(),
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(MIDDLE_X + 74.0, y + 36.0, 200.0, 22.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let can_give = held == 0 || medal.repeatable();
            let give = GIVE_BASE + medal.index() as u16;
            let take = TAKE_BASE + medal.index() as u16;
            let right = MIDDLE_X + MIDDLE_WIDTH;
            kit::button(
                &mut self.ui,
                frame,
                [right - 236.0, y + 12.0, 96.0, 40.0],
                if held > 0 && medal.repeatable() {
                    "Give +1"
                } else {
                    "Give"
                },
                true,
                can_give,
                self.focus == give,
                give,
            );
            kit::button(
                &mut self.ui,
                frame,
                [right - 128.0, y + 12.0, 128.0, 40.0],
                "Take back",
                false,
                held > 0,
                self.focus == take,
                take,
            );
            if can_give {
                self.order.push(give);
            }
            if held > 0 {
                self.order.push(take);
            }
        }
        let y = SECTION_TOP + Medal::COUNT as f32 * MEDAL_ROW + 14.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Note with the next medal (optional, everyone reads it)"),
            frame.rect(MIDDLE_X, y, MIDDLE_WIDTH, 22.0),
            15.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        let field = [MIDDLE_X, y + 28.0, MIDDLE_WIDTH, 44.0];
        let focused = self.focus == NOTE_TOKEN;
        let caret = focused && (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let shown = if self.note.chars().count() > 56 {
            let tail: String = self
                .note
                .chars()
                .skip(self.note.chars().count() - 53)
                .collect();
            format!("...{tail}")
        } else {
            self.note.clone()
        };
        kit::field(
            &mut self.ui,
            frame,
            field,
            format_args!("{shown}{}", if caret { "|" } else { "" }),
            focused,
            false,
        );
        self.ui.hit_region(
            NOTE_TOKEN,
            frame.rect(field[0], field[1], field[2], field[3]),
        );
        self.order.push(NOTE_TOKEN);
    }

    /// The player's achievements with a count, each with Clear, and Clear all.
    fn achievements(&mut self, frame: &Frame, profile: &Profile, mine: bool) {
        let s = frame.s;
        kit::heading(
            &mut self.ui,
            frame,
            RIGHT_X,
            SECTION_TOP - 20.0,
            RIGHT_WIDTH - 150.0,
            "Achievements",
        );
        let confirming = self.confirming();
        kit::button(
            &mut self.ui,
            frame,
            [
                RIGHT_X + RIGHT_WIDTH - 140.0,
                SECTION_TOP - 40.0,
                140.0,
                38.0,
            ],
            if confirming {
                "Press again"
            } else {
                "Clear all"
            },
            false,
            !profile.achievements.is_empty(),
            self.focus == CLEAR_ALL_TOKEN,
            CLEAR_ALL_TOKEN,
        );
        if !profile.achievements.is_empty() {
            self.order.push(CLEAR_ALL_TOKEN);
        }
        let mut y = SECTION_TOP + 10.0;
        let listed: Vec<(usize, &sjk_identity::Achievement)> = achievements::ALL
            .iter()
            .enumerate()
            .filter_map(|(index, kind)| {
                profile
                    .achievements
                    .iter()
                    .find(|held| held.id == kind.id && held.progress > 0)
                    .map(|held| (index, held))
            })
            .collect();
        if listed.is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("No achievements at the hub yet."),
                frame.rect(RIGHT_X, y, RIGHT_WIDTH, 24.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 34.0;
        }
        for (index, held) in listed.iter().take(ACHIEVEMENTS_SHOWN) {
            let kind = &achievements::ALL[*index];
            let token = CLEAR_BASE + *index as u16;
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", kind.name),
                frame.rect(RIGHT_X, y, 230.0, 28.0),
                19.0 * s,
                if held.unlocked > 0 {
                    color::GOLD_BRIGHT
                } else {
                    color::TEXT
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            let status = if held.unlocked > 0 {
                crate::medals::date_text(held.unlocked)
            } else {
                format!(
                    "{} / {}",
                    kind.amount(held.progress),
                    kind.amount(kind.goal)
                )
            };
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{status}"),
                frame.rect(RIGHT_X + 236.0, y + 2.0, 170.0, 24.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            kit::button(
                &mut self.ui,
                frame,
                [RIGHT_X + RIGHT_WIDTH - 96.0, y - 2.0, 96.0, 30.0],
                "Clear",
                false,
                true,
                self.focus == token,
                token,
            );
            self.order.push(token);
            y += ACHIEVEMENT_ROW;
        }
        if listed.len() > ACHIEVEMENTS_SHOWN {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("and {} more", listed.len() - ACHIEVEMENTS_SHOWN),
                frame.rect(RIGHT_X, y, RIGHT_WIDTH, 22.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 26.0;
        }
        let note = if mine {
            "Clearing your own also resets your game's count, so the next kill, duel or map unlocks it again."
        } else {
            "Their game sends its own counts again: only what the hub counts stays cleared."
        };
        for part in wrap(note, 72) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(RIGHT_X, y + 6.0, RIGHT_WIDTH, 22.0),
                14.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 22.0;
        }
    }

    /// What the last request came to, bottom left.
    fn status(&mut self, frame: &Frame, staff: &StaffState) {
        let s = frame.s;
        let (line, colour) = if staff.busy {
            ("Working...".to_owned(), color::MUTED)
        } else if staff.failed {
            (cut(&staff.message, 90), color::EMBER)
        } else {
            (cut(&staff.message, 90), color::GOLD_BRIGHT)
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(LEFT_X, KEYS_Y, 900.0, 24.0),
            16.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
    }

    /// The keys of what has the keyboard, right-aligned at the bottom.
    fn keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let enter = match self.focus {
            SEARCH_TOKEN => "search",
            NOTE_TOKEN => "",
            _ => "do",
        };
        let mut keys: Vec<(&[&str], &str)> = vec![(&["Tab"], "next")];
        if !enter.is_empty() {
            keys.push((&["Enter"], enter));
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

#[cfg(test)]
mod tests {
    use super::super::tests::profile;
    use super::*;
    use sjk_identity::{Achievement, Medal as WireMedal};

    fn crowded() -> Profile {
        Profile {
            staff: true,
            verified: true,
            name: "^1Sol^7Vulpes the Very Long Named One^5!".into(),
            medals: Medal::ALL
                .iter()
                .map(|medal| WireMedal {
                    id: medal.id().into(),
                    count: 3,
                    awarded: 1_791_336_225,
                    note: String::new(),
                })
                .collect(),
            achievements: achievements::ALL
                .iter()
                .map(|kind| Achievement {
                    id: kind.id.into(),
                    progress: kind.goal / 2 + 1,
                    goal: kind.goal,
                    unlocked: 0,
                })
                .collect(),
            ..profile("aaaaaaaaaaaaaaaa", "x")
        }
    }

    /// Every focus, a full list and a crowded player fit the canvas at 1080p, 4K, 4:3
    /// and 21:9, in the families and in Inter.
    #[test]
    fn every_state_fits_the_canvas() {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a family");
        let body = load(&crate::text::BODY);
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let me = crowded();
        let players: Vec<Profile> = (0..30)
            .map(|index| Profile {
                staff: index % 3 == 0,
                verified: index % 2 == 0,
                ..profile(
                    &format!("{index:016x}"),
                    "^3Someone^7 with a long name here",
                )
            })
            .collect();
        for staff in [
            StaffState {
                players: players.clone(),
                message: "x".repeat(200),
                failed: true,
                ..StaffState::default()
            },
            StaffState::default(),
        ] {
            let mut panel = Panel::new();
            panel.open(true);
            panel.note = "n".repeat(200);
            panel.query = "q".repeat(64);
            let inputs = Inputs {
                me: Some(&me),
                staff: &staff,
            };
            let font = &inter.font;
            panel.build(&inputs, font, [1920.0, 1080.0]);
            let order = panel.order.clone();
            for focus in order {
                for font in [&body.font, &inter.font] {
                    for viewport in [
                        [1920.0, 1080.0],
                        [3840.0, 2160.0],
                        [1440.0, 1080.0],
                        [2560.0, 1080.0],
                    ] {
                        panel.focus = focus;
                        panel.build(&inputs, font, viewport);
                        assert!(!panel.ui.overflowed(), "{focus} at {viewport:?}");
                    }
                }
            }
        }
    }
}
