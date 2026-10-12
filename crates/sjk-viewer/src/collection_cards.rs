//! The Collection's Cards tab (`docs/cards.md`, "In the client"): the cards opening
//! holocrons brought and the player's credits. On the left the copies the player holds,
//! one card per kind (rarity, finish, edition) with how many, the best first; on the
//! right the one chosen up close, in its finish's light, with what it is. Every card is
//! sealed until the first set of cards is out: its face is chosen when it is revealed.
//! The cards come from the player's own hub profile ([`crate::cards::stacks`]).

use super::view::{LEAD_Y, LEFT_X, SHOWCASE_WIDTH, SHOWCASE_X, centred_lines};
use super::*;
use crate::cards::{self, Stack, look};
use crate::menu::sjk::{Frame, color, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The cards on the left: their width, the room each takes across and down, how many to
/// a line and the lines shown.
const CARD_WIDTH: f32 = 128.0;
const STEP_X: f32 = 150.0;
const STEP_Y: f32 = 236.0;
const COLUMNS: usize = 6;
const LINES: usize = 2;
/// The most kinds the tab shows; more are counted under them.
pub(super) const SHOWN_MAX: usize = COLUMNS * LINES;
/// The first line's top.
const GRID_TOP: f32 = 330.0;
/// The card up close.
const BIG_WIDTH: f32 = 290.0;
const BIG_TOP: f32 = 200.0;

/// How many cards the player holds, once the hub answered with the identity on.
pub(super) fn held(inputs: &Inputs<'_>) -> Option<usize> {
    me(inputs).map(|me| cards::total(&cards::stacks(&me.cards)) as usize)
}

/// The own profile, once known with the identity on.
fn me<'a>(inputs: &Inputs<'a>) -> Option<&'a sjk_identity::Profile> {
    inputs
        .snapshot
        .filter(|_| inputs.enabled)
        .and_then(|snapshot| snapshot.me.as_ref())
}

/// What the tab says at its top.
fn headline(inputs: &Inputs<'_>, stacks: Option<&[Stack]>) -> (String, &'static str) {
    match (inputs.enabled, stacks) {
        (false, _) => (
            "Identity is off".to_owned(),
            "Your cards are kept on the SJK hub: switch the identity on in Settings, Network, to see them.",
        ),
        (true, None) => (
            "Contacting the hub...".to_owned(),
            "Your cards show once the SJK hub has answered.",
        ),
        (true, Some([])) => (
            "No cards yet".to_owned(),
            "Open a holocron on the Holocrons tab: each holds a card and credits.",
        ),
        (true, Some(stacks)) => {
            let total = cards::total(stacks);
            (
                if total == 1 {
                    "1 card".to_owned()
                } else {
                    format!("{total} cards")
                },
                "Sealed until the first set of cards is out: each one's face is chosen then.",
            )
        }
    }
}

impl Panel {
    /// The Cards tab.
    pub(super) fn cards(&mut self, frame: &Frame, inputs: &Inputs<'_>, seconds: f32) {
        let (stacks, credits) = self.holdings(inputs);
        let (headline, line) = headline(inputs, stacks.as_deref());
        self.lead(frame, &headline, line);
        if let Some(credits) = credits {
            self.balance(frame, credits);
        }
        let stacks = stacks.unwrap_or_default();
        self.cards_shown = stacks.len().min(SHOWN_MAX);
        self.card = self.card.min(self.cards_shown.saturating_sub(1));
        if stacks.is_empty() {
            // A dark outline where the first card will lie.
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(LEFT_X, GRID_TOP, CARD_WIDTH, CARD_WIDTH * look::ASPECT),
                radius: CARD_WIDTH * 0.07 * frame.s,
                width: 1.5 * frame.s,
                color: color::alpha(color::HOLO, 0.22),
            });
            return;
        }
        for (index, stack) in stacks.iter().take(SHOWN_MAX).enumerate() {
            self.small_card(frame, stack, index, seconds);
        }
        if stacks.len() > SHOWN_MAX {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("And {} more kinds", stacks.len() - SHOWN_MAX),
                frame.rect(LEFT_X, GRID_TOP + STEP_Y * LINES as f32, 600.0, 24.0),
                15.0 * frame.s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        if let Some(chosen) = stacks.get(self.card) {
            self.card_up_close(frame, chosen, seconds);
        }
    }

    /// The player's cards and credits, once known (a world shot's own in its place).
    pub(super) fn holdings(&self, inputs: &Inputs<'_>) -> (Option<Vec<Stack>>, Option<u64>) {
        #[cfg(test)]
        if let Some((list, credits)) = &self.preview_cards {
            return (Some(cards::stacks(list)), Some(*credits));
        }
        let me = me(inputs);
        (
            me.map(|me| cards::stacks(&me.cards)),
            me.map(|me| me.credits),
        )
    }

    /// The player's credits, at the right of the tab's headline.
    fn balance(&mut self, frame: &Frame, credits: u64) {
        let s = frame.s;
        let words = cards::credits_text(credits);
        let size = 30.0;
        let width = crate::text::display_width(&words, size) * crate::text::style::current().scale;
        let right = SHOWCASE_X - 40.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{words}"),
            frame.rect(right - width - 8.0, LEAD_Y, width + 8.0, 40.0),
            size * s,
            color::GOLD_BRIGHT,
            FontWeight::Semibold,
            TextAlign::End,
        );
        let mark = 24.0;
        let [x, y] = frame.point(
            right - width - 8.0 - 10.0 - cards::mark::width(mark),
            LEAD_Y + 8.0,
        );
        cards::mark::draw(self.ui.draw_list_mut(), [x, y], mark * s, color::GOLD);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("credits"),
            frame.rect(right - 200.0, LEAD_Y + 42.0, 200.0, 24.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::End,
        );
    }

    /// Kind `index` on the left: its card, how many, and its finish.
    fn small_card(&mut self, frame: &Frame, stack: &Stack, index: usize, seconds: f32) {
        let s = frame.s;
        let x = LEFT_X + (index % COLUMNS) as f32 * STEP_X;
        let y = GRID_TOP + (index / COLUMNS) as f32 * STEP_Y;
        let chosen = index == self.card;
        let token = CARD_BASE + index as u16;
        let height = CARD_WIDTH * look::ASPECT;
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(x - 5.0, y - 5.0, CARD_WIDTH + 10.0, height + 10.0),
                radius: (CARD_WIDTH * 0.07 + 5.0) * s,
                width: 2.0 * s,
                color: color::GOLD_BRIGHT,
            });
        }
        look::draw(
            &mut self.ui,
            frame,
            [x, y, CARD_WIDTH],
            look::Card {
                rarity: stack.rarity,
                finish: stack.finish,
                edition: "",
            },
            seconds,
            1.0,
            1.0,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("x{}", stack.count),
            frame.rect(x, y + height + 6.0, CARD_WIDTH * 0.5, 24.0),
            19.0 * s,
            if chosen {
                color::GOLD_BRIGHT
            } else {
                color::TEXT
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", stack.finish.name),
            frame.rect(
                x + CARD_WIDTH * 0.4,
                y + height + 8.0,
                CARD_WIDTH * 0.6,
                20.0,
            ),
            14.0 * s,
            if stack.finish.index == 2 {
                color::GOLD
            } else {
                color::MUTED
            },
            FontWeight::Regular,
            TextAlign::End,
        );
        self.ui.hit_region(
            token,
            frame.rect(x - 5.0, y - 5.0, CARD_WIDTH + 10.0, height + 36.0),
        );
    }

    /// The kind chosen, up close on the right.
    fn card_up_close(&mut self, frame: &Frame, stack: &Stack, seconds: f32) {
        let s = frame.s;
        let middle = SHOWCASE_X + SHOWCASE_WIDTH * 0.5;
        look::draw(
            &mut self.ui,
            frame,
            [middle - BIG_WIDTH * 0.5, BIG_TOP, BIG_WIDTH],
            look::Card {
                rarity: stack.rarity,
                finish: stack.finish,
                edition: &stack.edition,
            },
            seconds,
            1.0,
            1.0,
        );
        let mut y = BIG_TOP + BIG_WIDTH * look::ASPECT + 22.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", stack.title()),
            frame.rect(SHOWCASE_X, y, SHOWCASE_WIDTH, 40.0),
            32.0 * s,
            if stack.finish.index == 2 {
                color::GOLD_BRIGHT
            } else {
                stack.rarity.colour
            },
            FontWeight::Semibold,
            TextAlign::Center,
        );
        y += 46.0;
        let first = crate::medals::date_text(stack.first);
        let held = match (stack.count, first.is_empty()) {
            (1, true) => "1 held".to_owned(),
            (1, false) => format!("1 held, found {first}"),
            (count, true) => format!("{count} held"),
            (count, false) => format!("{count} held, the first found {first}"),
        };
        let edition = if stack.edition.is_empty() {
            held
        } else {
            format!("{}. {held}", stack.edition)
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{edition}"),
            frame.rect(SHOWCASE_X, y, SHOWCASE_WIDTH, 26.0),
            18.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::Center,
        );
        y += 34.0;
        let y = centred_lines(
            &mut self.ui,
            frame,
            stack.finish.about,
            [SHOWCASE_X, y, SHOWCASE_WIDTH],
            70,
            2,
            16.0,
            color::MUTED,
        );
        if stack.sealed() {
            centred_lines(
                &mut self.ui,
                frame,
                "Sealed: its face is chosen when the first set of cards comes out.",
                [SHOWCASE_X, y + 4.0, SHOWCASE_WIDTH],
                70,
                2,
                15.0,
                color::QUIET,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::inputs;
    use super::*;
    use crate::unlockables::Holdings;

    /// Every kind of sealed card: each rarity in each finish, the counts rising.
    pub(crate) fn every_kind() -> Vec<sjk_identity::CardStack> {
        let mut list = Vec::new();
        for finish in &cards::FINISHES {
            for rarity in &cards::RARITIES {
                list.push(sjk_identity::CardStack {
                    card: None,
                    rarity: rarity.id.to_owned(),
                    finish: finish.id.to_owned(),
                    edition: "alpha".to_owned(),
                    count: (list.len() as u32 % 4) + 1,
                    first: 1_791_641_100,
                    last: 1_791_641_100,
                });
            }
        }
        list
    }

    /// Every state fits the canvas: no identity, waiting, none, one kind and more kinds
    /// than the tab shows; the arrows walk the kinds shown.
    #[test]
    fn the_tab_shows_the_kinds_and_fits() {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let shown = inputs(Holdings::Known(&[]), "");
        let mut many = every_kind();
        many.push(sjk_identity::CardStack {
            edition: "beta".to_owned(),
            ..many[0].clone()
        });
        assert!(many.len() > SHOWN_MAX);
        let states = [
            None,
            Some((Vec::new(), 0)),
            Some((many[..1].to_vec(), 35)),
            Some((many.clone(), 123_456)),
        ];
        for state in &states {
            let mut panel = Panel::new();
            panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
            panel.preview_cards = state.clone();
            for viewport in [[1920.0, 1080.0], [3840.0, 2160.0], [1440.0, 1080.0]] {
                panel.build(&shown, &fonts.font, viewport);
                assert!(!panel.overflowed(), "{viewport:?}");
            }
        }
        let mut panel = Panel::new();
        panel.open(Tab::Cards, true, true, ReturnTarget::MainMenu);
        panel.preview_cards = states[3].clone();
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(panel.cards_shown, SHOWN_MAX);
        let _ = panel.key(KeyCode::ArrowRight, false, true);
        let _ = panel.key(KeyCode::ArrowRight, false, true);
        assert_eq!(panel.card, 2);
        for _ in 0..40 {
            let _ = panel.key(KeyCode::ArrowRight, false, true);
        }
        assert_eq!(
            panel.card,
            SHOWN_MAX - 1,
            "the arrows stop at the last shown"
        );
        assert!(
            panel.ui.rect_for(CARD_BASE).is_some(),
            "a card answers the pointer"
        );
    }
}
