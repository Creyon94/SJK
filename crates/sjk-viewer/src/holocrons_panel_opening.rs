//! The Holocrons tab's opening (`docs/cards.md`, "Opening"): Open (the oldest holocron of
//! the tier chosen) or Open all (up to twenty, the oldest first) asks the hub, and while
//! it answers the holocron pulses in its tier's colour. Then each holocron opened plays
//! its ceremony over the darkened page, one after another:
//!
//! - it bursts open in its tier's light: rings run out, rays turn, the gem flares and
//!   goes;
//! - the sealed card turns in to face the player, in its rarity's colour and its finish's
//!   light ([`crate::cards::look`]);
//! - the credits count up beside the credits' mark ([`crate::cards::mark`]);
//! - its words fade up: the card's rarity and finish, its edition, what the finish is,
//!   that it stays sealed until the first set of cards is out, and "1 of 3".
//!
//! Enter, Space, Right or a click finish the entrance at once, then take Next (Done on
//! the last); Escape ends the ceremony (what was opened is the hub's already). Openings
//! of more than three play a shorter entrance each. A refusal or no answer shows why, and
//! the page reads the hub again. `debug_open` plays made-up openings the same way
//! ([`crate::cards::rehearsal`]).
//!
//! While the ceremony shows the page under it is not drawn: text draws above every shape,
//! so the page's words would show through the dark.

use super::*;
use crate::audio::ui_cues::{self, Cue};
use crate::cards::{self, Finish, Rarity};
use crate::holocrons::gem;
use crate::menu::sjk::{Frame, color, key_hint, key_hint_width, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The ceremony's Next (or Done) button.
pub(crate) const NEXT_TOKEN: u16 = 1_322;
/// How long the hub has to answer before the page says it did not.
const ANSWER_WAIT: Duration = Duration::from_secs(15);
/// A card's entrance: the burst, the card turning in, the credits counting, the words.
const ENTRANCE: f32 = 2.4;
const BURST_END: f32 = 0.75;
const TURN: [f32; 2] = [0.55, 1.25];
const COUNT: [f32; 2] = [1.2, 1.9];
const WORDS: f32 = 1.85;
/// The entrance of each when more than [`SHORT_AFTER`] were opened at once, as a share.
const SHORT: f32 = 0.55;
const SHORT_AFTER: usize = 3;
/// The card on show: its middle and width (frame pixels); a revealed card's face is
/// bigger, to be read, and higher.
const CARD_X: f32 = 960.0;
const CARD_TOP: f32 = 190.0;
const CARD_WIDTH: f32 = 300.0;
const FACE_TOP: f32 = 62.0;
const FACE_WIDTH: f32 = 400.0;
/// The words' lines under the card.
const CREDITS_Y: f32 = 636.0;
const FACE_CREDITS_Y: f32 = 650.0;
const TITLE_Y: f32 = 700.0;
const EDITION_Y: f32 = 752.0;
const ABOUT_Y: f32 = 790.0;
const SEALED_Y: f32 = 822.0;
const BUTTON_Y: f32 = 880.0;
/// Rays of the burst.
const RAYS: usize = 12;

/// One holocron opened, as the ceremony shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shown {
    /// Position in [`holocrons::TIERS`].
    pub(crate) tier: usize,
    pub(crate) rarity: &'static Rarity,
    pub(crate) finish: &'static Finish,
    /// "Alpha Edition", or empty.
    pub(crate) edition: String,
    pub(crate) credits: u64,
    /// The card it is, picked by the hub; `None` while sealed.
    pub(crate) card: Option<String>,
}

impl Shown {
    /// Where the card stands: its top and width (frame pixels), bigger for a face the
    /// packs have.
    fn place(&self, faces: &cards::faces::Faces) -> (f32, f32) {
        if cards::shown(faces, self.card.as_deref()).is_some() {
            (FACE_TOP, FACE_WIDTH)
        } else {
            (CARD_TOP, CARD_WIDTH)
        }
    }
}

impl Shown {
    /// What the hub said one holocron brought; `None` for a tier, rarity or finish the
    /// client does not know.
    pub(crate) fn of(opened: &sjk_identity::Opened) -> Option<Self> {
        // A shader instead of a card has no ceremony yet ([`found_words`] says it).
        let held = opened.card.as_ref()?;
        Some(Self {
            tier: holocrons::Tier::from_id(&opened.tier)?.index,
            rarity: Rarity::from_id(&held.rarity)?,
            finish: Finish::from_id(&held.finish)?,
            edition: cards::edition_name(&held.edition),
            credits: opened.credits,
            card: held.card.clone(),
        })
    }
}

/// What shaders an opening brought, in a line ("You found Tidal blade, 2 held."); `None`
/// when it brought none.
fn found_words(opened: &[sjk_identity::Opened]) -> Option<String> {
    let found: Vec<String> = opened
        .iter()
        .filter_map(|opened| opened.unlock.as_ref())
        .map(|unlock| {
            let name = if unlock.name.is_empty() {
                &unlock.id
            } else {
                &unlock.name
            };
            format!("{name} ({} held)", unlock.count)
        })
        .collect();
    (!found.is_empty()).then(|| format!("You found {}.", found.join(", ")))
}

/// Where the ceremony stands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ceremony {
    /// The hub was asked to open holocrons of tier `tier` (the first of them), at `since`.
    Waiting { tier: usize, since: Instant },
    /// Showing `shown[index]`, its entrance begun at `since`.
    Showing {
        shown: Vec<Shown>,
        index: usize,
        since: Instant,
    },
    /// Nothing was opened: why.
    Failed { message: String, since: Instant },
}

impl Ceremony {
    /// The share of a full entrance each card takes.
    fn pace(&self) -> f32 {
        match self {
            Self::Showing { shown, .. } if shown.len() > SHORT_AFTER => SHORT,
            _ => 1.0,
        }
    }

    /// How far into its entrance the card on show is, in seconds of a full entrance.
    fn elapsed(&self, now: Instant) -> f32 {
        let since = match self {
            Self::Waiting { since, .. }
            | Self::Showing { since, .. }
            | Self::Failed { since, .. } => *since,
        };
        now.saturating_duration_since(since).as_secs_f32() / self.pace()
    }
}

/// The cue a card's entrance plays: the medals' fanfare for a Solar finish or a Legendary
/// card, the unlocks' chime otherwise.
fn cue(shown: &Shown) -> Cue {
    if shown.finish.index == 2 || shown.rarity.index == 3 {
        Cue::Medal
    } else {
        Cue::Achievement
    }
}

impl Panel {
    /// Whether the ceremony (or the wait for the hub) shows.
    pub(crate) fn opening(&self) -> bool {
        self.ceremony.is_some()
    }

    /// The holocrons Open would open: the oldest of the tier chosen; none while the hub
    /// keeps opening to staff.
    pub(super) fn open_one(&self) -> Option<Vec<u64>> {
        if !self.data.can_open {
            return None;
        }
        self.data.sealed[self.selected].first().map(|id| vec![*id])
    }

    /// The holocrons Open all would open: every one listed, the oldest first, at most
    /// as many as one opening takes.
    pub(super) fn open_every(&self) -> Option<Vec<u64>> {
        if !self.data.can_open {
            return None;
        }
        let mut ids: Vec<u64> = self.data.sealed.iter().flatten().copied().collect();
        ids.sort_unstable();
        ids.truncate(sjk_identity::OPEN_MAX);
        (!ids.is_empty()).then_some(ids)
    }

    /// Ask the hub to open `ids` (`Open` or `Open all`): the page waits for its answer.
    pub(crate) fn ask(&mut self, ids: Vec<u64>, now: Instant) -> PanelAction {
        if self.ceremony.is_some() || self.data.state != State::Known || !self.data.can_open {
            return PanelAction::None;
        }
        let first = ids.first().copied();
        let tier = (0..holocrons::COUNT)
            .find(|&tier| first.is_some_and(|id| self.data.sealed[tier].contains(&id)))
            .unwrap_or(self.selected);
        self.ceremony = Some(Ceremony::Waiting { tier, since: now });
        ui_cues::post(Cue::Click);
        PanelAction::Open(ids)
    }

    /// The identity could not take the request: say why.
    pub(crate) fn refused(&mut self, message: &str, now: Instant) {
        self.ceremony = Some(Ceremony::Failed {
            message: message.to_owned(),
            since: now,
        });
    }

    /// Follow the hub's answer: `outcome` is the identity's last opening (read every
    /// frame while the page waits; a new serial is this page's answer).
    pub(crate) fn follow_opening(
        &mut self,
        now: Instant,
        outcome: Option<&sjk_identity::OpenOutcome>,
    ) {
        let serial = outcome.map_or(0, |outcome| outcome.serial);
        let Some(Ceremony::Waiting { since, .. }) = &self.ceremony else {
            self.opening_serial = serial;
            return;
        };
        let since = *since;
        match outcome.filter(|_| serial > self.opening_serial) {
            Some(outcome) => {
                self.opening_serial = serial;
                // The page reads the hub's counts again at once.
                self.refreshed = None;
                let shown: Vec<Shown> = outcome.opened.iter().filter_map(Shown::of).collect();
                if shown.is_empty() {
                    let message = if let Some(words) = found_words(&outcome.opened) {
                        words
                    } else if outcome.message.is_empty() {
                        "Nothing this client knows came out.".to_owned()
                    } else {
                        outcome.message.clone()
                    };
                    self.refused(&message, now);
                } else {
                    self.show(shown, now);
                }
            }
            None if now.saturating_duration_since(since) >= ANSWER_WAIT => {
                self.refreshed = None;
                self.refused("The SJK hub did not answer. Try again later.", now);
            }
            None => {}
        }
    }

    /// Play the ceremony of `shown` from the first.
    pub(crate) fn show(&mut self, shown: Vec<Shown>, now: Instant) {
        if let Some(first) = shown.first() {
            ui_cues::post(cue(first));
        }
        self.ceremony = Some(Ceremony::Showing {
            shown,
            index: 0,
            since: now,
        });
    }

    /// Enter, Space, Right or a click: finish the entrance, else go to the next card or
    /// end the ceremony after the last (or after a refusal).
    pub(super) fn advance(&mut self, now: Instant) {
        let pace = self.ceremony.as_ref().map_or(1.0, Ceremony::pace);
        match &mut self.ceremony {
            Some(Ceremony::Showing {
                shown,
                index,
                since,
            }) => {
                let elapsed = now.saturating_duration_since(*since).as_secs_f32() / pace;
                if elapsed < ENTRANCE {
                    *since = now
                        .checked_sub(Duration::from_secs_f32(ENTRANCE * pace))
                        .unwrap_or(*since);
                } else if *index + 1 < shown.len() {
                    *index += 1;
                    *since = now;
                    ui_cues::post(cue(&shown[*index]));
                } else {
                    self.end_ceremony();
                }
            }
            Some(Ceremony::Failed { .. }) => self.end_ceremony(),
            _ => {}
        }
    }

    /// End the ceremony (Escape, or Done): the page shows again, read afresh.
    pub(crate) fn end_ceremony(&mut self) {
        if self.ceremony.take().is_some() {
            ui_cues::post(Cue::Back);
            self.refreshed = None;
        }
    }

    /// A key while the ceremony shows.
    pub(super) fn ceremony_key(&mut self, key: KeyCode, now: Instant) -> PanelAction {
        match key {
            KeyCode::Escape => self.end_ceremony(),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::ArrowRight => {
                self.advance(now);
            }
            _ => {}
        }
        PanelAction::None
    }

    /// Draw the ceremony over the whole window.
    pub(super) fn draw_ceremony(&mut self, frame: &Frame, viewport: [f32; 2], now: Instant) {
        let Some(ceremony) = self.ceremony.clone() else {
            return;
        };
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: sjk_ui::Rect::new(0.0, 0.0, viewport[0], viewport[1]),
            color: color::alpha(color::SPACE, 0.93),
        });
        let elapsed = ceremony.elapsed(now);
        #[cfg(test)]
        let elapsed = self.shot_elapsed.unwrap_or(elapsed);
        let seconds = self.seconds(now);
        match &ceremony {
            Ceremony::Waiting { tier, .. } => self.waiting(frame, *tier, elapsed),
            Ceremony::Failed { message, .. } => self.failed(frame, message),
            Ceremony::Showing { shown, index, .. } => {
                self.entrance(frame, &shown[*index], elapsed, seconds);
                if elapsed >= WORDS {
                    self.words(frame, shown, *index, elapsed);
                }
            }
        }
        self.ceremony_keys(frame, &ceremony, elapsed);
    }

    /// The page's clock in seconds, for the finishes' motion.
    fn seconds(&self, now: Instant) -> f32 {
        #[cfg(test)]
        if let Some(seconds) = self.shot_seconds {
            return seconds;
        }
        now.saturating_duration_since(self.epoch).as_secs_f32()
    }

    /// The hub has been asked: the tier's gem pulses.
    fn waiting(&mut self, frame: &Frame, tier: usize, elapsed: f32) {
        let s = frame.s;
        let colour = holocrons::TIERS[tier].colour;
        let pulse = 0.5 + 0.5 * (elapsed * 5.0).sin();
        let [cx, cy] = frame.point(CARD_X, 400.0);
        glow(
            &mut self.ui,
            frame,
            [CARD_X, 400.0],
            150.0,
            colour,
            0.25 + 0.2 * pulse,
        );
        gem::draw(
            self.ui.draw_list_mut(),
            [cx, cy],
            (70.0 + 8.0 * pulse) * s,
            colour,
            1.0,
            gem::ROWS,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Opening..."),
            frame.rect(CARD_X - 400.0, 560.0, 800.0, 44.0),
            34.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
    }

    /// Nothing was opened, and why.
    fn failed(&mut self, frame: &Frame, message: &str) {
        let s = frame.s;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Nothing was opened"),
            frame.rect(CARD_X - 500.0, 420.0, 1_000.0, 48.0),
            38.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{message}"),
            frame.rect(CARD_X - 500.0, 480.0, 1_000.0, 28.0),
            19.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Center,
        );
        kit::button(
            &mut self.ui,
            frame,
            [CARD_X - 110.0, BUTTON_Y, 220.0, 48.0],
            "Close",
            true,
            true,
            true,
            NEXT_TOKEN,
        );
    }

    /// The burst, the card turning in and the credits counting up.
    fn entrance(&mut self, frame: &Frame, shown: &Shown, elapsed: f32, seconds: f32) {
        let s = frame.s;
        let tier = &holocrons::TIERS[shown.tier];
        let faces = cards::faces::current();
        let (top, card_width) = shown.place(&faces);
        let height = card_width * cards::look::ASPECT;
        let middle = [CARD_X, top + height * 0.47];
        let [cx, cy] = frame.point(middle[0], middle[1]);
        // The burst: the gem flares and goes while rings run out and rays turn.
        if elapsed < BURST_END + 0.6 {
            let t = (elapsed / BURST_END).clamp(0.0, 1.0);
            let fade = 1.0 - ((elapsed - BURST_END) / 0.6).clamp(0.0, 1.0);
            glow(
                &mut self.ui,
                frame,
                middle,
                120.0 + 260.0 * t,
                tier.colour,
                0.5 * fade,
            );
            for ring in 0..3 {
                let r = (t - ring as f32 * 0.18).clamp(0.0, 1.0);
                if r <= 0.0 {
                    continue;
                }
                let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                    center: [cx, cy],
                    radius: (60.0 + 340.0 * r) * s,
                    width: (6.0 - 4.0 * r) * s,
                    start: 0.0,
                    sweep: std::f32::consts::TAU,
                    color: tier.colour_alpha((1.0 - r) * 0.8 * fade),
                    knockout: None,
                });
            }
            for ray in 0..RAYS {
                let angle = ray as f32 / RAYS as f32 * std::f32::consts::TAU + elapsed * 0.8;
                let (sin, cos) = angle.sin_cos();
                let inner = (40.0 + 80.0 * t) * s;
                let outer = (120.0 + 420.0 * t) * s;
                let _ = self.ui.draw_list_mut().push(DrawCommand::Line {
                    from: [cx + cos * inner, cy + sin * inner],
                    to: [cx + cos * outer, cy + sin * outer],
                    width: 3.0 * s,
                    color: tier.colour_alpha(0.55 * fade * (1.0 - t * 0.5)),
                });
            }
            let gem_fade = 1.0 - ((elapsed - 0.25) / 0.45).clamp(0.0, 1.0);
            if gem_fade > 0.0 {
                gem::draw(
                    self.ui.draw_list_mut(),
                    [cx, cy],
                    (70.0 + 50.0 * t) * s,
                    tier.colour,
                    gem_fade,
                    gem::ROWS,
                );
            }
        }
        // The card turns in.
        let turn = ((elapsed - TURN[0]) / (TURN[1] - TURN[0])).clamp(0.0, 1.0);
        if turn > 0.0 {
            // Ease out: fast at first, settling.
            let eased = 1.0 - (1.0 - turn).powi(3);
            let grow = 0.85 + 0.15 * eased;
            let width = card_width * grow;
            cards::look::draw(
                &mut self.ui,
                frame,
                [
                    CARD_X - width * 0.5,
                    top + (card_width - width) * cards::look::ASPECT * 0.5,
                    width,
                ],
                cards::look::Card {
                    rarity: shown.rarity,
                    finish: shown.finish,
                    edition: &shown.edition,
                    face: cards::shown(&faces, shown.card.as_deref()),
                    tilt: None,
                },
                seconds,
                eased,
                eased,
            );
        }
        // The credits count up.
        if elapsed >= COUNT[0] {
            let t = ((elapsed - COUNT[0]) / (COUNT[1] - COUNT[0])).clamp(0.0, 1.0);
            let counted = (shown.credits as f32 * t).round() as u64;
            credits_line(
                &mut self.ui,
                frame,
                CARD_X,
                if top == FACE_TOP {
                    FACE_CREDITS_Y
                } else {
                    CREDITS_Y
                },
                &format!("+{}", cards::credits_text(counted)),
                38.0,
                (t * 3.0).min(1.0),
            );
        }
    }

    /// The card's words, fading up, and the button.
    fn words(&mut self, frame: &Frame, shown: &[Shown], index: usize, elapsed: f32) {
        let s = frame.s;
        let card = &shown[index];
        let alpha = ((elapsed - WORDS) / 0.4).clamp(0.0, 1.0);
        let fade = |colour: Color| Color::new(colour.r, colour.g, colour.b, colour.a * alpha);
        let faces = cards::faces::current();
        let face = cards::shown(&faces, card.card.as_deref());
        let stack = cards::Stack {
            rarity: card.rarity,
            finish: card.finish,
            edition: card.edition.clone(),
            card: card.card.clone(),
            count: 1,
            first: 0,
        };
        let title = stack.name_in(&faces);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{title}"),
            frame.rect(CARD_X - 500.0, TITLE_Y, 1_000.0, 50.0),
            42.0 * s,
            fade(if card.finish.index == 2 {
                color::GOLD_BRIGHT
            } else {
                card.rarity.colour
            }),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let tier = &holocrons::TIERS[card.tier];
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}{}From {} {}",
                if stack.sealed() {
                    String::new()
                } else {
                    format!("{}, {}. ", card.rarity.name, card.finish.name)
                },
                if card.edition.is_empty() {
                    String::new()
                } else {
                    format!("{}. ", card.edition)
                },
                tier.article(),
                tier.name
            ),
            frame.rect(CARD_X - 500.0, EDITION_Y, 1_000.0, 28.0),
            19.0 * s,
            fade(color::TEXT),
            FontWeight::Regular,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", card.finish.about),
            frame.rect(CARD_X - 500.0, ABOUT_Y, 1_000.0, 26.0),
            17.0 * s,
            fade(color::MUTED),
            FontWeight::Regular,
            TextAlign::Center,
        );
        let last_line = match face {
            Some(face) => format!(
                "{}, {}: card {} of {}",
                face.face.kind.name(),
                face.face.set_name,
                face.face.number,
                face.of
            ),
            None if stack.sealed() => "Sealed: its card is picked when it is revealed.".to_owned(),
            None => "Its face is not in your card pack yet.".to_owned(),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{last_line}"),
            frame.rect(CARD_X - 500.0, SEALED_Y, 1_000.0, 26.0),
            17.0 * s,
            fade(color::QUIET),
            FontWeight::Regular,
            TextAlign::Center,
        );
        if shown.len() > 1 {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{} of {}", index + 1, shown.len()),
                frame.rect(
                    CARD_X - 300.0,
                    if face.is_some() { 20.0 } else { 120.0 },
                    600.0,
                    34.0,
                ),
                24.0 * s,
                fade(color::MUTED),
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        let last = index + 1 == shown.len();
        kit::button(
            &mut self.ui,
            frame,
            [CARD_X - 110.0, BUTTON_Y, 220.0, 48.0],
            if last { "Done" } else { "Next" },
            true,
            true,
            true,
            NEXT_TOKEN,
        );
    }

    /// The keys under the ceremony.
    fn ceremony_keys(&mut self, frame: &Frame, ceremony: &Ceremony, elapsed: f32) {
        let s = frame.s;
        let action = match ceremony {
            Ceremony::Waiting { .. } => None,
            Ceremony::Failed { .. } => Some("close"),
            Ceremony::Showing { shown, index, .. } => Some(if elapsed < ENTRANCE {
                "skip"
            } else if index + 1 < shown.len() {
                "next"
            } else {
                "done"
            }),
        };
        let mut keys: [(&[&str], &str); 2] = [(&["Esc"], "close"); 2];
        let mut count = 0;
        if let Some(action) = action {
            keys[count] = (&["Enter"], action);
            count += 1;
        }
        keys[count] = (&["Esc"], "close");
        count += 1;
        let keys = &keys[..count];
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(1_824.0, super::view::KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

/// A soft round glow of `colour` at `centre` (frame pixels), `radius` across its half,
/// `strength` its alpha at the middle: discs stacked from the rim inwards.
fn glow(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    radius: f32,
    colour: Color,
    strength: f32,
) {
    const LAYERS: usize = 8;
    for layer in 0..LAYERS {
        let r = radius * (1.0 - layer as f32 / LAYERS as f32);
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x - r, y - r, r * 2.0, r * 2.0),
            radius: r * frame.s,
            color: color::alpha(colour, strength / LAYERS as f32),
        });
    }
}

/// The credits' mark and `words` centred on `x` (frame pixels), the mark as tall as the
/// words' capitals, at `alpha`.
pub(super) fn credits_line(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    x: f32,
    y: f32,
    words: &str,
    size: f32,
    alpha: f32,
) {
    let s = frame.s;
    let mark = size * 0.8;
    let gap = size * 0.28;
    let words_width = crate::text::display_width(words, size) * crate::text::style::current().scale;
    let total = cards::mark::width(mark) + gap + words_width;
    let left = x - total * 0.5;
    let gold = Color::new(color::GOLD.r, color::GOLD.g, color::GOLD.b, alpha);
    let [mx, my] = frame.point(left, y + (size * 1.25 - mark) * 0.5);
    cards::mark::draw(canvas.draw_list_mut(), [mx, my], mark * s, gold);
    text(
        canvas,
        TextFamily::Display,
        format_args!("{words}"),
        frame.rect(
            left + cards::mark::width(mark) + gap,
            y,
            words_width + size,
            size * 1.25,
        ),
        size * s,
        Color::new(
            color::GOLD_BRIGHT.r,
            color::GOLD_BRIGHT.g,
            color::GOLD_BRIGHT.b,
            alpha,
        ),
        FontWeight::Semibold,
        TextAlign::Start,
    );
}
