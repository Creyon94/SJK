//! The Holocrons tab's opening (`docs/cards.md`, "Opening"): Open (the oldest holocron of
//! the tier chosen) or Open all (up to twenty, the oldest first) asks the hub, and while
//! it answers the tab's 3D holocron glides to the middle of the window and hums
//! ([`super::unseal`]). Then each holocron opened plays its ceremony over the darkened
//! page, one after another:
//!
//! - the holocron spins up, blazes and cracks open in a flash of its tier's colour
//!   ([`super::unseal`]);
//! - out of the flash a strip of what its tier can drop runs past a gold marker and slows
//!   to a stop on the prize, as a loot crate does ([`super::strip`]): a tick a tile, a
//!   colour band under each, the prize lit and the others dimmed once it rests;
//! - it bursts open in its tier's light: rings run out, rays turn, the gem flares and
//!   goes;
//! - the sealed card turns in to face the player (a shader instead of a card shows its
//!   blade, framed in its tier's colour, and what it is), in its rarity's colour and its finish's
//!   light ([`crate::cards::look`]);
//! - the credits count up beside the credits' mark ([`crate::cards::mark`]);
//! - its words fade up: the card's rarity and finish, its edition, what the finish is,
//!   that it stays sealed until the first set of cards is out, and "1 of 3".
//!
//! Enter, Space, Right or a click skip to the burst, finish the entrance at once, then
//! take Next (Done on the last); Escape ends the ceremony (what was opened is the hub's
//! already). Openings of more than three crack the first holocron, run their strips five
//! at a time and then lay out everything found ([`super::batch`]). A refusal or no answer
//! shows why, and the page reads the hub again. `debug_open` plays made-up openings the same way
//! ([`crate::cards::rehearsal`]).
//!
//! While the ceremony shows the page under it is not drawn: text draws above every shape,
//! so the page's words would show through the dark.

use super::strip;
use super::*;
use crate::audio::ui_cues::{self, Cue};
use crate::cards::{self, Finish, Rarity};
use crate::holocrons::gem;
use crate::menu::sjk::{Frame, color, key_hint, key_hint_width, kit, text};
use crate::menu_widgets::TextFamily;
use crate::unlockables::Unlockable;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The ceremony's Next (or Done) button.
pub(crate) const NEXT_TOKEN: u16 = 1_322;
/// How long the hub has to answer before the page says it did not.
const ANSWER_WAIT: Duration = Duration::from_secs(15);
/// The holocron cracking open before its strip ([`super::unseal`]), in seconds.
pub(super) const UNSEAL: f32 = 1.7;
/// The crate spin after it, in seconds.
const SPIN: f32 = 6.0;
/// The share of the spin the strip runs; the rest it rests on the prize, lit.
const SPIN_RUN: f32 = 0.88;
/// The unseal and the spin: the entrance begins this far into a holocron's ceremony.
const BEFORE: f32 = UNSEAL + SPIN;
/// A card's entrance: the burst, the card turning in, the credits counting, the words.
const ENTRANCE: f32 = 2.4;
const BURST_END: f32 = 0.75;
const TURN: [f32; 2] = [0.55, 1.25];
const COUNT: [f32; 2] = [1.2, 1.9];
const WORDS: f32 = 1.85;
/// More than this many opened at once run five strips at a time ([`super::batch`]).
pub(super) const ONE_BY_ONE: usize = 3;
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
/// The crate strip: its top and height, and the band it shows through (frame pixels).
const STRIP_Y: f32 = 380.0;
const STRIP_HEIGHT: f32 = strip::TILE * cards::look::ASPECT;
const STRIP_LEFT: f32 = 40.0;
const STRIP_RIGHT: f32 = 1_880.0;
/// The colour band under each tile, as a loot crate's: its gap and height.
pub(super) const BAND_GAP: f32 = 10.0;
pub(super) const BAND: f32 = 6.0;
/// A shader's showpiece: its top, width and height.
const SHADER_TOP: f32 = 250.0;
const SHADER_WIDTH: f32 = 520.0;
const SHADER_HEIGHT: f32 = 300.0;

/// One holocron opened, as the ceremony shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shown {
    /// The holocron's number at the hub (it seeds the crate spin).
    pub(crate) holocron: u64,
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
        // A shader instead of a card is a [`Prize::Shader`].
        let held = opened.card.as_ref()?;
        Some(Self {
            holocron: opened.holocron,
            tier: holocrons::Tier::from_id(&opened.tier)?.index,
            rarity: Rarity::from_id(&held.rarity)?,
            finish: Finish::from_id(&held.finish)?,
            edition: cards::edition_name(&held.edition),
            credits: opened.credits,
            card: held.card.clone(),
        })
    }
}

/// A shader an opening brought, instead of a card.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Found {
    /// The holocron's number at the hub (it seeds the crate spin).
    pub(crate) holocron: u64,
    /// Position in [`holocrons::TIERS`].
    pub(crate) tier: usize,
    /// The shader, when this client knows it; `None` for one newer than this client
    /// (the hub's catalogue grows before every player updates): it is shown by the
    /// hub's name for it, and its blade if the hub's pack brought one.
    pub(crate) skin: Option<&'static Unlockable>,
    /// Its id and name as the hub sent them.
    pub(crate) id: String,
    pub(crate) name: String,
    /// How many of it the player holds now, this one included.
    pub(crate) count: u32,
}

impl Found {
    /// The colour it is shown in: its tier's, or the holocron's for one this client
    /// does not know.
    fn colour(&self) -> Color {
        self.skin
            .map_or(holocrons::TIERS[self.tier].colour, |skin| {
                skin.tier.colour()
            })
    }
}

/// What one holocron brought: a card with credits, or a shader.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Prize {
    Card(Shown),
    Shader(Found),
}

impl Prize {
    /// What the hub said one holocron brought; `None` for a tier, rarity or finish the
    /// client does not know. A shader it does not know still shows, by the hub's name.
    pub(crate) fn of(opened: &sjk_identity::Opened) -> Option<Self> {
        if let Some(unlock) = &opened.unlock {
            let name = if unlock.name.trim().is_empty() {
                "A new saber shader".to_owned()
            } else {
                unlock.name.trim().to_owned()
            };
            return Some(Self::Shader(Found {
                holocron: opened.holocron,
                tier: holocrons::Tier::from_id(&opened.tier)?.index,
                skin: crate::unlockables::blade_skin(&unlock.id),
                id: unlock.id.clone(),
                name,
                count: unlock.count,
            }));
        }
        Shown::of(opened).map(Self::Card)
    }

    pub(super) fn tier(&self) -> usize {
        match self {
            Self::Card(shown) => shown.tier,
            Self::Shader(found) => found.tier,
        }
    }

    pub(super) fn holocron(&self) -> u64 {
        match self {
            Self::Card(shown) => shown.holocron,
            Self::Shader(found) => found.holocron,
        }
    }

    /// The tile the crate spin ends on.
    pub(super) fn item(&self) -> strip::Item {
        match self {
            Self::Card(shown) => strip::Item::Card {
                rarity: shown.rarity.index,
                finish: shown.finish.index,
            },
            Self::Shader(found) => found.skin.map_or(strip::Item::New, strip::Item::Shader),
        }
    }
}

/// Where the ceremony stands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ceremony {
    /// The hub was asked to open holocrons of tier `tier` (the first of them), at `since`.
    Waiting { tier: usize, since: Instant },
    /// Showing `shown[index]`, its entrance begun at `since`.
    Showing {
        shown: Vec<Prize>,
        index: usize,
        since: Instant,
    },
    /// More than [`ONE_BY_ONE`] opened at once: the first cracks open, their strips run
    /// five at a time, then everything found is laid out ([`super::batch`]), all on one
    /// clock begun at `since`.
    Batch { shown: Vec<Prize>, since: Instant },
    /// Nothing was opened: why.
    Failed { message: String, since: Instant },
}

impl Ceremony {
    /// How far into the holocron (or the batch) on show the ceremony is, in seconds.
    fn elapsed(&self, now: Instant) -> f32 {
        let since = match self {
            Self::Waiting { since, .. }
            | Self::Showing { since, .. }
            | Self::Batch { since, .. }
            | Self::Failed { since, .. } => *since,
        };
        now.saturating_duration_since(since).as_secs_f32()
    }

    /// The tier of the holocron the 3D one stands for: the one asked for, or opened.
    pub(crate) fn stage_tier(&self) -> Option<usize> {
        match self {
            Self::Waiting { tier, .. } => Some(*tier),
            Self::Showing { shown, index, .. } => shown.get(*index).map(Prize::tier),
            Self::Batch { shown, .. } => shown.first().map(Prize::tier),
            Self::Failed { .. } => None,
        }
    }
}

/// The cues the holocron (or round of strips) on show has played, so each plays once, and
/// the tile under each strip's marker last frame (a tick a tile).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Played {
    pub(super) charged: bool,
    pub(super) cracked: bool,
    pub(super) tick: [usize; super::batch::ROWS],
    /// A bit a strip that has come to rest.
    pub(super) landed: u8,
    pub(super) entered: bool,
    /// The batch's round these are of.
    pub(super) round: usize,
}

/// The cue a card's entrance plays: the medals' fanfare for a Solar finish or a Legendary
/// card, the unlocks' chime otherwise.
fn cue(prize: &Prize) -> Cue {
    match prize {
        Prize::Card(shown) if shown.finish.index == 2 || shown.rarity.index == 3 => Cue::Medal,
        Prize::Shader(found)
            if found
                .skin
                .is_some_and(|skin| skin.tier >= crate::unlockables::Rarity::Legendary) =>
        {
            Cue::Medal
        }
        _ => Cue::Achievement,
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
        self.glide_from = Some(now);
        ui_cues::post(Cue::Click);
        PanelAction::Open(ids)
    }

    /// The identity could not take the request: say why.
    pub(crate) fn refused(&mut self, message: &str, now: Instant) {
        self.glide_from = None;
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
                let shown: Vec<Prize> = outcome.opened.iter().filter_map(Prize::of).collect();
                if shown.is_empty() {
                    let message = if outcome.message.is_empty() {
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
    pub(crate) fn show(&mut self, shown: Vec<Prize>, now: Instant) {
        self.played = Played::default();
        // The glide and the hum carry on from the wait (none for a rehearsal).
        self.waited = self.glide_from.map_or(0.0, |from| {
            now.saturating_duration_since(from).as_secs_f32()
        });
        self.ceremony = Some(if shown.len() > ONE_BY_ONE {
            Ceremony::Batch { shown, since: now }
        } else {
            Ceremony::Showing {
                shown,
                index: 0,
                since: now,
            }
        });
    }

    /// Enter, Space, Right or a click: finish the entrance, else go to the next card or
    /// end the ceremony after the last (or after a refusal).
    pub(super) fn advance(&mut self, now: Instant) {
        let back = |since: &mut Instant, to: f32| {
            *since = now
                .checked_sub(Duration::from_secs_f32(to))
                .unwrap_or(*since);
        };
        match &mut self.ceremony {
            Some(Ceremony::Showing {
                shown,
                index,
                since,
            }) => {
                let elapsed = now.saturating_duration_since(*since).as_secs_f32();
                if elapsed < BEFORE {
                    // The holocron opens and the crate stops where it was going: on to
                    // the burst.
                    back(since, BEFORE);
                    self.played.charged = true;
                    self.played.cracked = true;
                } else if elapsed < BEFORE + ENTRANCE {
                    back(since, BEFORE + ENTRANCE);
                } else if *index + 1 < shown.len() {
                    *index += 1;
                    *since = now;
                    self.played = Played::default();
                    ui_cues::post(Cue::Click);
                } else {
                    self.end_ceremony();
                }
            }
            Some(Ceremony::Batch { shown, since }) => {
                let elapsed = now.saturating_duration_since(*since).as_secs_f32();
                match super::batch::skip(elapsed, shown.len()) {
                    Some(to) => {
                        back(since, to);
                        self.played.charged = true;
                        self.played.cracked = true;
                    }
                    None => self.end_ceremony(),
                }
            }
            Some(Ceremony::Failed { .. }) => self.end_ceremony(),
            _ => {}
        }
    }

    /// End the ceremony (Escape, or Done): the page shows again, read afresh.
    pub(crate) fn end_ceremony(&mut self) {
        self.glide_from = None;
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
        let elapsed = self.ceremony_elapsed(&ceremony, now);
        // The world shows round the holocron while it is asked for and cracks open.
        let unsealing = match &ceremony {
            Ceremony::Waiting { .. } => true,
            Ceremony::Showing { .. } | Ceremony::Batch { .. } => elapsed < UNSEAL,
            Ceremony::Failed { .. } => false,
        };
        if !unsealing {
            let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
                rect: sjk_ui::Rect::new(0.0, 0.0, viewport[0], viewport[1]),
                color: color::alpha(color::SPACE, 0.93),
            });
        }
        let seconds = self.seconds(now);
        self.follow_skins();
        match &ceremony {
            Ceremony::Waiting { tier, .. } => self.draw_waiting(frame, viewport, *tier),
            Ceremony::Failed { message, .. } => self.failed(frame, message),
            Ceremony::Batch { shown, .. } => {
                self.draw_batch(frame, viewport, shown, elapsed, seconds);
            }
            Ceremony::Showing { shown, index, .. } if elapsed < UNSEAL => {
                let tier = shown[*index].tier();
                let of = (shown.len() > 1).then_some((*index, shown.len()));
                self.draw_unseal(
                    frame,
                    viewport,
                    tier,
                    elapsed,
                    format_args!("{}", holocrons::TIERS[tier].name),
                    of,
                );
            }
            Ceremony::Showing { shown, index, .. } if elapsed < BEFORE => {
                self.spin(frame, viewport, &shown[*index], elapsed - UNSEAL, seconds);
            }
            Ceremony::Showing { shown, index, .. } => {
                let entered = elapsed - BEFORE;
                if !self.played.entered {
                    self.played.entered = true;
                    ui_cues::post(cue(&shown[*index]));
                }
                match &shown[*index] {
                    Prize::Card(card) => self.entrance(frame, card, entered, seconds),
                    Prize::Shader(found) => self.entrance_shader(frame, found, entered, seconds),
                }
                if entered >= WORDS {
                    match &shown[*index] {
                        Prize::Card(_) => self.words(frame, shown, *index, entered),
                        Prize::Shader(found) => {
                            self.shader_words(frame, found, shown.len(), *index, entered);
                        }
                    }
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

    /// How far into the holocron (or batch) on show `ceremony` is at `now`, in seconds. A
    /// world shot's time is into the entrance (negative: into the unseal and spin before
    /// it), or for a batch from its start.
    pub(super) fn ceremony_elapsed(&self, ceremony: &Ceremony, now: Instant) -> f32 {
        let elapsed = ceremony.elapsed(now);
        #[cfg(test)]
        if let Some(shot) = self.shot_elapsed {
            return match ceremony {
                Ceremony::Batch { .. } => shot,
                _ => BEFORE + shot,
            };
        }
        elapsed
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

    /// The burst: the tier's gem flares and goes while rings run out and rays turn, round
    /// `middle` (frame pixels), `elapsed` seconds in.
    fn burst(&mut self, frame: &Frame, middle: [f32; 2], tier: &holocrons::Tier, elapsed: f32) {
        let s = frame.s;
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
    }

    /// The burst, the card turning in and the credits counting up.
    fn entrance(&mut self, frame: &Frame, shown: &Shown, elapsed: f32, seconds: f32) {
        let tier = &holocrons::TIERS[shown.tier];
        let faces = cards::faces::current();
        let (top, card_width) = shown.place(&faces);
        let height = card_width * cards::look::ASPECT;
        let middle = [CARD_X, top + height * 0.47];
        self.burst(frame, middle, tier, elapsed);
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
    fn words(&mut self, frame: &Frame, shown: &[Prize], index: usize, elapsed: f32) {
        let s = frame.s;
        let Prize::Card(card) = &shown[index] else {
            return;
        };
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

    /// The crate spin, `elapsed` seconds in: out of the holocron's flash a strip of what
    /// its tier can drop runs past the marker, a tick a tile, each tile trailing its
    /// colour at speed over its colour band, and slows to a stop on the prize; at rest the
    /// prize lifts and lights up (rays behind a grand one) while the others dim.
    fn spin(
        &mut self,
        frame: &Frame,
        viewport: [f32; 2],
        prize: &Prize,
        elapsed: f32,
        seconds: f32,
    ) {
        let s = frame.s;
        let tier = &holocrons::TIERS[prize.tier()];
        let geometry = strip::SINGLE;
        let item = prize.item();
        let tiles = strip::strip(prize.tier(), prize.holocron(), item);
        let run = strip::run(prize.holocron(), geometry);
        let running = SPIN * SPIN_RUN;
        let t = elapsed / running;
        let distance = run * strip::travelled(t);
        let speed = run * strip::speed(t, running);
        let under = strip::under_marker(distance, geometry);
        if under != self.played.tick[0] {
            self.played.tick[0] = under;
            ui_cues::post(Cue::CrateTick);
        }
        let settled = elapsed >= running;
        if settled && self.played.landed & 1 == 0 {
            self.played.landed |= 1;
            ui_cues::post(Cue::CrateLand);
        }
        // How lit the prize is at rest (0 to 1, quickly).
        let lit = if settled {
            smooth((elapsed - running) / (0.35 * (SPIN - running)))
        } else {
            0.0
        };
        let middle = [CARD_X, STRIP_Y + STRIP_HEIGHT * 0.5];
        glow(&mut self.ui, frame, middle, 420.0, tier.colour, 0.16);
        let appear = (elapsed / 0.35).clamp(0.0, 1.0);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", tier.name),
            frame.rect(CARD_X - 500.0, STRIP_Y - 120.0, 1_000.0, 50.0),
            38.0 * s,
            tier.colour_alpha(appear),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        if lit > 0.0 {
            prize_light(&mut self.ui, frame, middle, item, lit, seconds, 1.0);
        }
        let pitch = geometry.pitch();
        let height = STRIP_HEIGHT;
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::PushOpacity(appear));
        let _ = list.push(DrawCommand::PushClip(frame.rect(
            STRIP_LEFT,
            STRIP_Y - 30.0,
            STRIP_RIGHT - STRIP_LEFT,
            height + 60.0,
        )));
        let faces = cards::faces::current();
        for (at, tile) in tiles.iter().enumerate() {
            let x = CARD_X - strip::TILE * 0.5 + at as f32 * pitch - distance;
            if x + strip::TILE + SMEAR_MAX < STRIP_LEFT || x > STRIP_RIGHT {
                continue;
            }
            // The tile under the marker lifts a little; at rest the prize more.
            let near = (1.0 - ((x + strip::TILE * 0.5 - CARD_X).abs() / pitch)).clamp(0.0, 1.0);
            let won = at == strip::WIN;
            let lift = if won && settled {
                10.0 + 8.0 * lit
            } else {
                10.0 * near
            };
            let dim = if won { 1.0 } else { 1.0 - 0.65 * lit };
            self.strip_tile(
                frame,
                &faces,
                *tile,
                [x, STRIP_Y - lift, strip::TILE],
                near,
                dim,
                speed,
                seconds,
            );
        }
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::PopClip);
        // The strip fades into the dark at both ends.
        strip_ends(
            &mut self.ui,
            frame,
            [STRIP_LEFT, STRIP_RIGHT],
            STRIP_Y - 30.0,
            height + 60.0,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::PopOpacity);
        marker(
            &mut self.ui,
            frame,
            CARD_X,
            [STRIP_Y - 36.0, STRIP_Y + height + 36.0],
            color::alpha(color::GOLD_BRIGHT, 0.9),
            1.0,
        );
        // The holocron's flash fades over the strip.
        super::unseal::flash(
            &mut self.ui,
            viewport,
            tier.colour,
            1.0 - elapsed / super::unseal::FLASH_OUT,
        );
    }

    /// One tile of a strip at `[x, y, width]` (frame pixels), `dim` its opacity, trailing
    /// its colour behind it at `speed` (frame pixels a second, the strip running left),
    /// over its colour band.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn strip_tile(
        &mut self,
        frame: &Frame,
        faces: &cards::faces::Faces,
        item: strip::Item,
        [x, y, width]: [f32; 3],
        near: f32,
        dim: f32,
        speed: f32,
        seconds: f32,
    ) {
        let s = frame.s;
        let height = width * cards::look::ASPECT;
        let colour = item.colour();
        let list = self.ui.draw_list_mut();
        if dim < 1.0 {
            let _ = list.push(DrawCommand::PushOpacity(dim));
        }
        let _ = list.push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y + height + BAND_GAP * width / strip::TILE, width, BAND),
            radius: BAND * 0.5 * s,
            color: color::alpha(colour, 0.9),
        });
        self.tile(frame, faces, item, [x, y, width], near, seconds);
        // At speed its colour streaks across it and trails it (the strip runs left),
        // longer and stronger the faster it runs.
        let trail = (speed * 0.03).min(SMEAR_MAX) * width / strip::TILE;
        if trail > 4.0 {
            let strength = (speed / 3_000.0).min(1.0) * 0.35;
            let list = self.ui.draw_list_mut();
            for (rect, start, end) in [
                (
                    frame.rect(x + width * 0.15, y, width * 0.85, height),
                    0.0,
                    strength,
                ),
                (
                    frame.rect(x + width, y + height * 0.1, trail, height * 0.8),
                    strength,
                    0.0,
                ),
            ] {
                let _ = list.push(DrawCommand::GradientRect {
                    rect,
                    radius: 0.0,
                    gradient: sjk_ui::Gradient {
                        start: color::alpha(colour, start),
                        end: color::alpha(colour, end),
                        vertical: false,
                    },
                });
            }
        }
        if dim < 1.0 {
            let _ = self.ui.draw_list_mut().push(DrawCommand::PopOpacity);
        }
    }

    /// One tile of the strip at `[x, y, width]` (frame pixels): a card in its rarity and
    /// finish, or a shader's blade framed in its tier's colour with its name. `near` (0 to
    /// 1) is how close it is to the marker.
    pub(super) fn tile(
        &mut self,
        frame: &Frame,
        faces: &cards::faces::Faces,
        item: strip::Item,
        [x, y, width]: [f32; 3],
        near: f32,
        seconds: f32,
    ) {
        if width < MINI {
            self.mini_tile(frame, item, [x, y, width]);
            return;
        }
        let s = frame.s;
        // Everything inside a tile is laid out for the strip's own width.
        let k = width / strip::TILE;
        let height = width * cards::look::ASPECT;
        let rect = frame.rect(x, y, width, height);
        match item {
            strip::Item::Card { rarity, finish } => {
                cards::look::draw(
                    &mut self.ui,
                    frame,
                    [x, y, width],
                    cards::look::Card {
                        rarity: &cards::RARITIES[rarity],
                        finish: &cards::FINISHES[finish],
                        edition: "",
                        face: cards::shown(faces, None),
                        tilt: None,
                    },
                    seconds,
                    0.75 + 0.25 * near,
                    1.0,
                );
            }
            strip::Item::Shader(skin) => {
                let colour = skin.tier.colour();
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 14.0 * k * s,
                    color: color::alpha(color::SPACE, 0.9),
                });
                let look = self.skins.get(skin.id);
                let _ = crate::console::collection_panel::swatch::small_swatch(
                    self.ui.draw_list_mut(),
                    frame,
                    [
                        x + 10.0 * k,
                        y + height * 0.5 - 38.0 * k,
                        width - 20.0 * k,
                        76.0 * k,
                    ],
                    look,
                    true,
                    seconds,
                    0.0,
                );
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 14.0 * k * s,
                    width: (2.0 + near) * s,
                    color: color::alpha(colour, 0.7 + 0.3 * near),
                });
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{} {}", skin.tier.symbol(), skin.tier.label()),
                    frame.rect(x, y + 12.0 * k, width, 24.0 * k),
                    16.0 * k * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{}", skin.name),
                    frame.rect(x, y + height - 40.0 * k, width, 26.0 * k),
                    17.0 * k * s,
                    color::TEXT,
                    FontWeight::Regular,
                    TextAlign::Center,
                );
            }
            strip::Item::New => {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 14.0 * k * s,
                    color: color::alpha(color::SPACE, 0.9),
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 14.0 * k * s,
                    width: (2.0 + near) * s,
                    color: color::alpha(color::GOLD, 0.7 + 0.3 * near),
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("?"),
                    frame.rect(x, y + height * 0.5 - 40.0 * k, width, 70.0 * k),
                    60.0 * k * s,
                    color::GOLD,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("New shader"),
                    frame.rect(x, y + height - 40.0 * k, width, 26.0 * k),
                    17.0 * k * s,
                    color::TEXT,
                    FontWeight::Regular,
                    TextAlign::Center,
                );
            }
        }
    }

    /// A shader's burst and blade: the tier's light, then the blade swatch growing in
    /// large, framed in the shader's tier colour.
    fn entrance_shader(&mut self, frame: &Frame, found: &Found, elapsed: f32, seconds: f32) {
        let s = frame.s;
        let tier = &holocrons::TIERS[found.tier];
        let colour = found.colour();
        let middle = [CARD_X, SHADER_TOP + SHADER_HEIGHT * 0.5];
        self.burst(frame, middle, tier, elapsed);
        let turn = ((elapsed - TURN[0]) / (TURN[1] - TURN[0])).clamp(0.0, 1.0);
        if turn <= 0.0 {
            return;
        }
        let eased = 1.0 - (1.0 - turn).powi(3);
        let width = SHADER_WIDTH * (0.85 + 0.15 * eased);
        let height = SHADER_HEIGHT * (0.85 + 0.15 * eased);
        let rect = [
            CARD_X - width * 0.5,
            SHADER_TOP + (SHADER_HEIGHT - height) * 0.5,
            width,
            height,
        ];
        glow(&mut self.ui, frame, middle, 300.0, colour, 0.3 * eased);
        let area = frame.rect(rect[0], rect[1], rect[2], rect[3]);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 22.0 * s,
            color: color::alpha(color::SPACE, 0.95 * eased),
        });
        // The blade the hub's pack brought, even for a shader this client does not list.
        let look = self.skins.get(&found.id);
        let _ = crate::console::collection_panel::swatch::small_swatch(
            self.ui.draw_list_mut(),
            frame,
            [
                rect[0] + 24.0,
                rect[1] + height * 0.5 - 90.0,
                rect[2] - 48.0,
                180.0,
            ],
            look,
            true,
            seconds,
            0.0,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 22.0 * s,
            width: 3.0 * s,
            color: color::alpha(colour, eased),
        });
    }

    /// What the shader is, fading up, and the button.
    fn shader_words(
        &mut self,
        frame: &Frame,
        found: &Found,
        count: usize,
        index: usize,
        entered: f32,
    ) {
        let s = frame.s;
        let alpha = ((entered - WORDS) / 0.4).clamp(0.0, 1.0);
        let fade = |colour: Color| Color::new(colour.r, colour.g, colour.b, colour.a * alpha);
        let tier = &holocrons::TIERS[found.tier];
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", found.name),
            frame.rect(CARD_X - 500.0, TITLE_Y, 1_000.0, 50.0),
            42.0 * s,
            fade(found.colour()),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let kind = found.skin.map_or_else(
            || "A saber shader new to this version of SJK".to_owned(),
            |skin| format!("{} {} saber shader", skin.tier.symbol(), skin.tier.label()),
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{kind}. From {} {}", tier.article(), tier.name),
            frame.rect(CARD_X - 500.0, EDITION_Y, 1_000.0, 28.0),
            19.0 * s,
            fade(color::TEXT),
            FontWeight::Regular,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}",
                found.skin.map_or(
                    "It is yours: update SJK to see it in your Collection and wear it.",
                    |skin| skin.description
                )
            ),
            frame.rect(CARD_X - 600.0, ABOUT_Y, 1_200.0, 26.0),
            17.0 * s,
            fade(color::MUTED),
            FontWeight::Regular,
            TextAlign::Center,
        );
        let held = match (found.count, found.skin) {
            (count, _) if count > 1 => format!("You hold {count} of it now."),
            (_, Some(_)) => "Yours now: wear it in the Collection.".to_owned(),
            // Its line above says where it will show.
            (_, None) => String::new(),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{held}"),
            frame.rect(CARD_X - 500.0, SEALED_Y, 1_000.0, 26.0),
            17.0 * s,
            fade(color::QUIET),
            FontWeight::Regular,
            TextAlign::Center,
        );
        if count > 1 {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{} of {count}", index + 1),
                frame.rect(CARD_X - 300.0, 120.0, 600.0, 34.0),
                24.0 * s,
                fade(color::MUTED),
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        kit::button(
            &mut self.ui,
            frame,
            [CARD_X - 110.0, BUTTON_Y, 220.0, 48.0],
            if index + 1 == count { "Done" } else { "Next" },
            true,
            true,
            true,
            NEXT_TOKEN,
        );
    }

    /// Take the loaded blade skins when a pack brought new ones (the tiles' blades).
    fn follow_skins(&mut self) {
        if self.skins.generation() != crate::sjk_packs::generation() {
            self.skins = crate::sjk_packs::skins();
        }
    }

    /// The keys under the ceremony.
    fn ceremony_keys(&mut self, frame: &Frame, ceremony: &Ceremony, elapsed: f32) {
        let s = frame.s;
        let action = match ceremony {
            Ceremony::Waiting { .. } => None,
            Ceremony::Failed { .. } => Some("close"),
            Ceremony::Showing { shown, index, .. } => Some(if elapsed < BEFORE + ENTRANCE {
                "skip"
            } else if index + 1 < shown.len() {
                "next"
            } else {
                "done"
            }),
            Ceremony::Batch { shown, .. } => {
                Some(if super::batch::summing_up(elapsed, shown.len()) {
                    "done"
                } else {
                    "skip"
                })
            }
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

/// The longest streak a tile trails at speed (frame pixels, for the strip's own tile).
const SMEAR_MAX: f32 = 150.0;
/// Tiles narrower than this (frame pixels; the stacked strips' of an opening of many) are
/// drawn light: their colour, a mark and a blade or a symbol, not a whole card.
const MINI: f32 = 120.0;

impl Panel {
    /// A light tile at `[x, y, width]` (frame pixels): a dark plate washed from below in
    /// its colour, edged in it, with a card's rarity symbol (ringed in gold for Solar, a
    /// rainbow under it for Holo) or a shader's blade, or "?" for a shader this client
    /// does not know.
    fn mini_tile(&mut self, frame: &Frame, item: strip::Item, [x, y, width]: [f32; 3]) {
        let s = frame.s;
        let k = width / strip::STACKED.tile;
        let height = width * cards::look::ASPECT;
        let colour = item.colour();
        let rect = frame.rect(x, y, width, height);
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect,
            radius: 8.0 * k * s,
            color: color::alpha(color::SPACE, 0.92),
        });
        let _ = list.push(DrawCommand::GradientRect {
            rect,
            radius: 8.0 * k * s,
            gradient: sjk_ui::Gradient {
                start: color::alpha(colour, 0.04),
                end: color::alpha(colour, 0.34),
                vertical: true,
            },
        });
        let [cx, cy] = frame.point(x + width * 0.5, y + height * 0.5);
        match item {
            strip::Item::Card { rarity, finish } => {
                if finish == 2 {
                    let _ = list.push(DrawCommand::Arc {
                        center: [cx, cy],
                        radius: 26.0 * k * s,
                        width: 3.0 * k * s,
                        start: 0.0,
                        sweep: std::f32::consts::TAU,
                        color: color::alpha(color::GOLD_BRIGHT, 0.9),
                        knockout: None,
                    });
                }
                if finish == 1 {
                    for (band, hue) in [0xFF_6B9D, 0x6B_D4FF, 0xB8_FF6B].into_iter().enumerate() {
                        let left = x + width * (0.22 + 0.19 * band as f32);
                        let _ = list.push(DrawCommand::RoundedRect {
                            rect: frame.rect(left, y + height * 0.78, width * 0.18, 4.0 * k),
                            radius: 2.0 * k * s,
                            color: crate::holocrons::rgb(hue),
                        });
                    }
                }
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * k * s,
                    width: 2.0 * s,
                    color: color::alpha(colour, 0.8),
                });
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{}", crate::symbols::RARITY[rarity]),
                    frame.rect(x, y + height * 0.5 - 20.0 * k, width, 40.0 * k),
                    30.0 * k * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
            strip::Item::Shader(_) => {
                // A blade across the tile: its glow, its white core, a short hilt.
                let from = frame.point(x + width * 0.34, y + height * 0.76);
                let to = frame.point(x + width * 0.70, y + height * 0.2);
                let hilt = frame.point(x + width * 0.26, y + height * 0.88);
                let _ = list.push(DrawCommand::Line {
                    from,
                    to,
                    width: 12.0 * k * s,
                    color: color::alpha(colour, 0.45),
                });
                let _ = list.push(DrawCommand::Line {
                    from,
                    to,
                    width: 3.5 * k * s,
                    color: color::alpha(sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0), 0.95),
                });
                let _ = list.push(DrawCommand::Line {
                    from: hilt,
                    to: from,
                    width: 5.0 * k * s,
                    color: color::alpha(color::MUTED, 0.9),
                });
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * k * s,
                    width: 2.0 * s,
                    color: color::alpha(colour, 0.8),
                });
            }
            strip::Item::New => {
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * k * s,
                    width: 2.0 * s,
                    color: color::alpha(colour, 0.8),
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("?"),
                    frame.rect(x, y + height * 0.5 - 22.0 * k, width, 44.0 * k),
                    36.0 * k * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
        }
    }
}

/// Ease in and out.
pub(super) fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The light behind a prize at rest at `middle` (frame pixels), `lit` (0 to 1) of the way
/// up, `scale` against the strip's own: a glow of its colour breathing, and for a grand
/// one slow rays turning.
pub(super) fn prize_light(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    middle: [f32; 2],
    item: strip::Item,
    lit: f32,
    seconds: f32,
    scale: f32,
) {
    let s = frame.s;
    let colour = item.colour();
    let pulse = 0.5 + 0.5 * (seconds * 4.0).sin();
    glow(
        canvas,
        frame,
        middle,
        230.0 * scale,
        colour,
        (0.35 + 0.15 * pulse) * lit,
    );
    if !item.grand() {
        return;
    }
    const GRAND_RAYS: usize = 16;
    let [cx, cy] = frame.point(middle[0], middle[1]);
    for ray in 0..GRAND_RAYS {
        let angle = ray as f32 / GRAND_RAYS as f32 * std::f32::consts::TAU + seconds * 0.4;
        let (sin, cos) = angle.sin_cos();
        let (inner, outer) = (140.0 * scale * s, (300.0 + 80.0 * pulse) * scale * s);
        let _ = canvas.draw_list_mut().push(DrawCommand::Line {
            from: [cx + cos * inner, cy + sin * inner],
            to: [cx + cos * outer, cy + sin * outer],
            width: 4.0 * scale * s,
            color: color::alpha(colour, 0.4 * lit),
        });
    }
}

/// A strip's ends fading into the dark: from `left` to `right` (frame pixels), the band
/// from `top`, `height` tall.
pub(super) fn strip_ends(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [left, right]: [f32; 2],
    top: f32,
    height: f32,
) {
    let width = (right - left) * 0.28;
    let dark = color::alpha(color::SPACE, 0.93);
    let clear = color::alpha(color::SPACE, 0.0);
    for (x, start, end) in [(left, dark, clear), (right - width, clear, dark)] {
        let _ = canvas.draw_list_mut().push(DrawCommand::GradientRect {
            rect: frame.rect(x, top, width, height),
            radius: 0.0,
            gradient: sjk_ui::Gradient {
                start,
                end,
                vertical: false,
            },
        });
    }
}

/// A strip's marker at `x` from `top` to `bottom` (frame pixels): a line with a point
/// above and below, the points `scale` against the strip's own.
pub(super) fn marker(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    x: f32,
    [top, bottom]: [f32; 2],
    colour: Color,
    scale: f32,
) {
    let s = frame.s;
    let [mx, top] = frame.point(x, top);
    let [_, bottom] = frame.point(x, bottom);
    let list = canvas.draw_list_mut();
    let _ = list.push(DrawCommand::Line {
        from: [mx, top],
        to: [mx, bottom],
        width: 3.0 * s,
        color: colour,
    });
    for (y, down) in [(top, 1.0_f32), (bottom, -1.0)] {
        for side in [-1.0_f32, 1.0] {
            let _ = list.push(DrawCommand::Line {
                from: [mx, y],
                to: [mx + side * 14.0 * scale * s, y - down * 16.0 * scale * s],
                width: 3.0 * s,
                color: colour,
            });
        }
    }
}

/// A soft round glow of `colour` at `centre` (frame pixels), `radius` across its half,
/// `strength` its alpha at the middle: discs stacked from the rim inwards.
pub(super) fn glow(
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
