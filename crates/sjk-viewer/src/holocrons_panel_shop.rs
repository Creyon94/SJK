//! The Holocrons tab's shop (`docs/cards.md`, "The shop"): the day's four saber shaders,
//! the same for every player, sold for kyber shards (credits on hubs before 17/10/2026),
//! over the darkened page as the loot table is ([`super::loot`]). One Common, one
//! Uncommon, one Rare and a Legendary (a Mythical one day in four), dealt by the hub each
//! day at midnight in Paris; a countdown says when the next come. Under them, holocrons for
//! credits: an Uncommon and a Rare one, at the hub's prices, while the hub sells them.
//!
//! Each offer is a card: its tier's frame and name, its blade, its name, its price beside
//! the shards' mark, and Buy. A purchase asks twice (Buy, then Confirm within a few
//! seconds) since it spends for good; the hub refuses a shader the player holds and a
//! second purchase of an offer the same day, and the card says so before asking. A
//! holocron asks twice the same way. The player's credits and kyber shards stand at the
//! top right.
//!
//! Shop (beside Open and Open all, while the hub lets the player open holocrons) or S shows
//! it, and reads the hub's day; Left and Right choose an offer or a holocron, Enter buys
//! (twice), the mouse chooses and buys; Escape, S or Back go back to the page.

use super::*;
use crate::audio::ui_cues::{self, Cue};
use crate::menu::sjk::{Frame, color, key_hint, key_hint_width, kit, text};
use crate::menu_widgets::TextFamily;
use crate::unlockables::Rarity;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The page's edges.
const LEFT: f32 = 96.0;
const RIGHT: f32 = 1_824.0;
const WIDTH: f32 = RIGHT - LEFT;
/// The title, the two lines under it, and the day's heading.
const TITLE_Y: f32 = 60.0;
const LINE_Y: f32 = 122.0;
const RULES_Y: f32 = 148.0;
const DAY_Y: f32 = 206.0;
/// The offers' cards: their top, size and the gap between.
const CARDS_TOP: f32 = 262.0;
const CARD_WIDTH: f32 = 384.0;
const CARD_HEIGHT: f32 = 540.0;
const CARD_GAP: f32 = 30.0;
/// The line under the cards: what came of the last purchase, or why the shop is not read.
const MESSAGE_Y: f32 = 832.0;
/// The holocrons for credits: their row's top, each button's size and the gap between.
const HOLOCRONS_Y: f32 = 886.0;
const HOLOCRON_WIDTH: f32 = 420.0;
const HOLOCRON_HEIGHT: f32 = 52.0;
const HOLOCRON_GAP: f32 = 30.0;
/// Back, and the keys' line.
const BACK_Y: f32 = 988.0;
const BACK_WIDTH: f32 = 150.0;
/// How long Confirm waits for the second press.
const CONFIRM: Duration = Duration::from_secs(5);

/// The shader tiers by the hub's ids, Common to Mythical.
const TIER_IDS: [(&str, Rarity); 5] = [
    ("common", Rarity::Common),
    ("uncommon", Rarity::Uncommon),
    ("rare", Rarity::Rare),
    ("legendary", Rarity::Legendary),
    ("mythical", Rarity::Mythical),
];

/// The tier of an offer: the catalogue's for a shader this client knows, else the hub's
/// word, else Common.
fn tier_of(offer: &sjk_identity::Offer) -> Rarity {
    crate::unlockables::find(&offer.id).map_or_else(
        || {
            TIER_IDS
                .iter()
                .find(|(id, _)| *id == offer.tier)
                .map_or(Rarity::Common, |(_, tier)| *tier)
        },
        |skin| skin.tier,
    )
}

/// "17/10/2026" from the hub's "2026-10-17"; the hub's words when they are not a date.
pub(super) fn day_text(day: &str) -> String {
    let mut parts = day.split('-');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(year), Some(month), Some(day), None)
            if year.len() == 4 && month.len() == 2 && day.len() == 2 =>
        {
            format!("{day}/{month}/{year}")
        }
        _ => day.to_owned(),
    }
}

/// How long until the next offers, as players read it: "5 h 12 min", "12 min", "less
/// than a minute".
pub(super) fn countdown_text(seconds: u64) -> String {
    let minutes = seconds.div_ceil(60);
    match (minutes / 60, minutes % 60) {
        (0, 0) | (0, 1) if seconds < 60 => "less than a minute".to_owned(),
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

/// What an offer's button may do for the player now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Can {
    /// Buy it (it asks to confirm first).
    Buy,
    /// Bought today already.
    Bought,
    /// Held already: the hub sells no duplicate.
    Owned,
    /// Less than its price.
    Short(u64),
}

/// What `offer` allows a player holding `purse` of the shop's currency.
pub(super) fn can(offer: &sjk_identity::Offer, purse: u64) -> Can {
    if offer.bought {
        Can::Bought
    } else if offer.owned {
        Can::Owned
    } else if purse < offer.price {
        Can::Short(offer.price - purse)
    } else {
        Can::Buy
    }
}

/// `amount` of the shop's currency in words: "75 kyber shards", "250 credits".
fn priced(amount: u64) -> String {
    if crate::economy::shop_in_shards() {
        format!("{} kyber shards", crate::economy::shards_text(amount))
    } else {
        format!("{} credits", crate::cards::credits_text(amount))
    }
}

/// The holocron tier called `id` (this client's list), if it knows it.
fn holocron_tier(id: &str) -> Option<&'static crate::holocrons::Tier> {
    crate::holocrons::TIERS.iter().find(|tier| tier.id == id)
}

impl Panel {
    /// Whether the shop shows.
    #[cfg(test)]
    pub(crate) fn showing_shop(&self) -> bool {
        self.shop
    }

    /// Show the shop (`true`, reading the hub's day: [`PanelAction::ReadShop`]) or go back
    /// to the page. It shows only while the hub lets the player open holocrons.
    pub(crate) fn show_shop(&mut self, show: bool) -> PanelAction {
        let show = show && self.data.can_open;
        if self.shop != show {
            ui_cues::post(if show { Cue::Click } else { Cue::Back });
        }
        self.shop = show;
        self.confirm = None;
        if show {
            self.shop_selected = self.shop_selected.min(3);
            PanelAction::ReadShop
        } else {
            PanelAction::None
        }
    }

    /// Whether the shop's day has turned since it was read, so it should be read again
    /// (once a day: it asks for each day once).
    pub(crate) fn shop_due(&mut self, now: Instant) -> bool {
        if !self.shop {
            return false;
        }
        let Some(state) = &self.data.shop else {
            return false;
        };
        if !state.message.is_empty() || state.seconds_left(now) > 0 {
            return false;
        }
        if self.shop_reread == Some(state.serial) {
            return false;
        }
        self.shop_reread = Some(state.serial);
        true
    }

    /// The day's offers, once read.
    fn offers(&self) -> &[sjk_identity::Offer] {
        self.data
            .shop
            .as_ref()
            .map_or(&[], |state| state.day.offers.as_slice())
    }

    /// The player's credits: the hub's latest word.
    fn shop_credits(&self) -> u64 {
        self.data.credits.unwrap_or(0)
    }

    /// What the player holds of the shop's currency: kyber shards, or credits on hubs
    /// before.
    fn shop_purse(&self) -> u64 {
        if crate::economy::shop_in_shards() {
            self.data.shards.unwrap_or(0)
        } else {
            self.shop_credits()
        }
    }

    /// How many things the shop offers to choose from: the day's offers, then the
    /// holocrons for credits (once the day is read).
    fn shop_choices(&self) -> usize {
        let offers = self.offers().len();
        if self
            .data
            .shop
            .as_ref()
            .is_some_and(|state| state.message.is_empty())
        {
            offers + crate::economy::holocrons_for_sale().len()
        } else {
            offers
        }
    }

    /// Press choice `index`: an offer's Buy or a holocron's.
    fn press_choice(&mut self, index: usize, now: Instant) -> PanelAction {
        let offers = self.offers().len();
        if index < offers {
            self.press_buy(index, now)
        } else {
            self.press_holocron(index - offers, now)
        }
    }

    /// Buy holocron `index` of [`crate::economy::holocrons_for_sale`] with credits: the
    /// first press asks to confirm, the second (within [`CONFIRM`]) asks the hub.
    fn press_holocron(&mut self, index: usize, now: Instant) -> PanelAction {
        let Some((tier, price)) = crate::economy::holocrons_for_sale().get(index).copied() else {
            return PanelAction::None;
        };
        let choice = self.offers().len() + index;
        self.shop_selected = choice;
        if self.buying() || self.shop_credits() < price {
            return PanelAction::None;
        }
        match self.confirm {
            Some((at, since)) if at == choice && now.saturating_duration_since(since) < CONFIRM => {
                self.confirm = None;
                self.buying = Some(self.data.trade.as_ref().map_or(0, |trade| trade.serial));
                ui_cues::post(Cue::Click);
                PanelAction::BuyHolocron(tier.to_owned())
            }
            _ => {
                self.confirm = Some((choice, now));
                ui_cues::post(Cue::Click);
                PanelAction::None
            }
        }
    }

    /// Whether a purchase is on its way to the hub.
    fn buying(&self) -> bool {
        self.buying
            .is_some_and(|asked| self.data.trade.as_ref().map_or(0, |trade| trade.serial) == asked)
    }

    /// Buy offer `index`: the first press asks to confirm, the second (within
    /// [`CONFIRM`]) asks the hub.
    fn press_buy(&mut self, index: usize, now: Instant) -> PanelAction {
        let Some(offer) = self.offers().get(index).cloned() else {
            return PanelAction::None;
        };
        self.shop_selected = index;
        if self.buying() || can(&offer, self.shop_purse()) != Can::Buy {
            return PanelAction::None;
        }
        match self.confirm {
            Some((at, since)) if at == index && now.saturating_duration_since(since) < CONFIRM => {
                self.confirm = None;
                self.buying = Some(self.data.trade.as_ref().map_or(0, |trade| trade.serial));
                ui_cues::post(Cue::Click);
                PanelAction::Buy(offer.id)
            }
            _ => {
                self.confirm = Some((index, now));
                ui_cues::post(Cue::Click);
                PanelAction::None
            }
        }
    }

    /// Wait for the second press on offer `index`'s Buy, chosen, for a world shot.
    #[cfg(test)]
    pub(crate) fn confirm_for_shot(&mut self, index: usize) {
        self.buying = None;
        self.shop_selected = index;
        self.confirm = Some((index, Instant::now()));
    }

    /// Take the trade the page holds as asked from the shop, so its line shows, for a
    /// world shot.
    #[cfg(test)]
    pub(crate) fn asked_for_shot(&mut self) {
        self.buying = Some(0);
    }

    /// A key while the shop shows.
    pub(super) fn shop_key(&mut self, key: KeyCode) -> PanelAction {
        let now = Instant::now();
        let count = self.shop_choices();
        match key {
            KeyCode::Escape | KeyCode::KeyS => return self.show_shop(false),
            KeyCode::ArrowLeft | KeyCode::ArrowUp => {
                self.shop_selected = self.shop_selected.saturating_sub(1);
                self.confirm = None;
            }
            KeyCode::ArrowRight | KeyCode::ArrowDown | KeyCode::Tab => {
                self.shop_selected = (self.shop_selected + 1).min(count.saturating_sub(1));
                self.confirm = None;
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                return self.press_choice(self.shop_selected, now);
            }
            _ => {}
        }
        PanelAction::None
    }

    /// A pointer event while the shop shows: a card chooses its offer, its button buys,
    /// Back goes back.
    pub(super) fn shop_pointer(&mut self, event: InputEvent) -> PanelAction {
        if matches!(event, InputEvent::PointerWheel { .. }) {
            return PanelAction::None;
        }
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(SHOP_BACK_TOKEN) => self.show_shop(false),
            Some(token) if (SHOP_BUY_BASE..SHOP_BUY_BASE + 4).contains(&token) => {
                self.press_buy(usize::from(token - SHOP_BUY_BASE), Instant::now())
            }
            Some(token) if (SHOP_HOLOCRON_BASE..SHOP_HOLOCRON_BASE + 2).contains(&token) => {
                self.press_holocron(usize::from(token - SHOP_HOLOCRON_BASE), Instant::now())
            }
            Some(token) if (SHOP_CARD_BASE..SHOP_CARD_BASE + 4).contains(&token) => {
                let index = usize::from(token - SHOP_CARD_BASE);
                if index != self.shop_selected {
                    self.confirm = None;
                }
                self.shop_selected = index;
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }

    /// Draw the shop over the whole window at `now`.
    pub(super) fn draw_shop(&mut self, frame: &Frame, viewport: [f32; 2], now: Instant) {
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: sjk_ui::Rect::new(0.0, 0.0, viewport[0], viewport[1]),
            color: color::alpha(color::SPACE, 0.93),
        });
        self.follow_skins_for_shop();
        self.shop_head(frame, now);
        let seconds = crate::menu::art::motion::seconds() as f32;
        for index in 0..self.offers().len().min(4) {
            self.offer_card(frame, index, now, seconds);
        }
        self.shop_message(frame);
        self.holocrons_for_credits(frame, now);
        self.shop_keys(frame);
    }

    /// Under the message: the holocrons credits buy, each a button with its tier's mark,
    /// name and price, asking twice. Nothing until the day is read, or when the hub sells
    /// none.
    fn holocrons_for_credits(&mut self, frame: &Frame, now: Instant) {
        if !self
            .data
            .shop
            .as_ref()
            .is_some_and(|state| state.message.is_empty())
        {
            return;
        }
        let sale = crate::economy::holocrons_for_sale();
        if sale.is_empty() {
            return;
        }
        let s = frame.s;
        let count = sale.len() as f32;
        let total = count * HOLOCRON_WIDTH + (count - 1.0) * HOLOCRON_GAP;
        let left = 960.0 - total * 0.5;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Holocrons for credits"),
            frame.rect(LEFT, HOLOCRONS_Y + 13.0, left - LEFT - 24.0, 26.0),
            18.0 * s,
            color::MUTED,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let credits = self.shop_credits();
        let offers = self.offers().len();
        let on_its_way = self.buying();
        for (index, (tier, price)) in sale.into_iter().enumerate() {
            let choice = offers + index;
            let chosen = self.shop_selected == choice;
            let confirming = self.confirm.is_some_and(|(at, since)| {
                at == choice && now.saturating_duration_since(since) < CONFIRM
            });
            let name = holocron_tier(tier).map_or(tier, |known| known.name);
            let affordable = credits >= price;
            let price_words = crate::cards::credits_text(price);
            let label = if on_its_way && chosen {
                "Buying...".to_owned()
            } else if confirming && affordable {
                format!("Confirm: {price_words} credits")
            } else {
                format!("{} {name}  {price_words} credits", crate::symbols::HOLOCRON)
            };
            let x = left + index as f32 * (HOLOCRON_WIDTH + HOLOCRON_GAP);
            if let Some(known) = holocron_tier(tier) {
                let [bx, by] = frame.point(x, HOLOCRONS_Y);
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect: sjk_ui::Rect::new(
                        bx - 3.0 * s,
                        by - 3.0 * s,
                        (HOLOCRON_WIDTH + 6.0) * s,
                        (HOLOCRON_HEIGHT + 6.0) * s,
                    ),
                    radius: 28.0 * s,
                    width: if chosen { 2.5 * s } else { 1.5 * s },
                    color: color::alpha(known.colour, if chosen { 0.95 } else { 0.55 }),
                });
            }
            kit::button(
                &mut self.ui,
                frame,
                [x, HOLOCRONS_Y, HOLOCRON_WIDTH, HOLOCRON_HEIGHT],
                &label,
                confirming && affordable,
                affordable && !on_its_way,
                chosen,
                SHOP_HOLOCRON_BASE + index as u16,
            );
            if !affordable {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!(
                        "{} more credits needed.",
                        crate::cards::credits_text(price - credits)
                    ),
                    frame.rect(x, HOLOCRONS_Y + HOLOCRON_HEIGHT + 8.0, HOLOCRON_WIDTH, 22.0),
                    15.0 * s,
                    color::QUIET,
                    FontWeight::Regular,
                    TextAlign::Center,
                );
            }
        }
    }

    /// Take the loaded blade skins when a pack brought new ones (the offers' blades).
    fn follow_skins_for_shop(&mut self) {
        if self.skins.generation() != crate::sjk_packs::generation() {
            self.skins = crate::sjk_packs::skins();
        }
    }

    /// The title, what the shop is, its rules, the balances, the day and the countdown.
    fn shop_head(&mut self, frame: &Frame, now: Instant) {
        let s = frame.s;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Shop"),
            frame.rect(LEFT, TITLE_Y, WIDTH, 54.0),
            44.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}",
                if crate::economy::shop_in_shards() {
                    "Four saber shaders a day, the same for every player, for kyber shards."
                } else {
                    "Four saber shaders a day, the same for every player, for the credits holocrons bring."
                }
            ),
            frame.rect(LEFT, LINE_Y, WIDTH - 520.0, 26.0),
            17.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "Only shaders holocrons drop, never a medal's. Each once a day; a shader you hold is not sold to you."
            ),
            frame.rect(LEFT, RULES_Y, WIDTH - 520.0, 24.0),
            16.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Start,
        );
        self.balances(frame, RIGHT, TITLE_Y + 6.0);
        let (day, countdown) = match &self.data.shop {
            Some(state) if state.message.is_empty() => (
                format!("Offers of {}", day_text(&state.day.day)),
                match state.seconds_left(now) {
                    0 => "New offers are here: reading them...".to_owned(),
                    left => format!(
                        "New offers in {}, at midnight in Paris",
                        countdown_text(left)
                    ),
                },
            ),
            _ => ("Today's offers".to_owned(), String::new()),
        };
        kit::heading(&mut self.ui, frame, LEFT, DAY_Y, WIDTH - 560.0, &day);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{countdown}"),
            frame.rect(RIGHT - 560.0, DAY_Y - 14.0, 560.0, 28.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::End,
        );
    }

    /// One offer's card: its tier's frame and wash, the tier, its blade, its name, its
    /// price and its button.
    fn offer_card(&mut self, frame: &Frame, index: usize, now: Instant, seconds: f32) {
        let s = frame.s;
        let Some(offer) = self.offers().get(index).cloned() else {
            return;
        };
        let tier = tier_of(&offer);
        let colour = tier.colour();
        let count = self.offers().len().min(4) as f32;
        let total = count * CARD_WIDTH + (count - 1.0) * CARD_GAP;
        let x = 960.0 - total * 0.5 + index as f32 * (CARD_WIDTH + CARD_GAP);
        let y = CARDS_TOP;
        let rect = frame.rect(x, y, CARD_WIDTH, CARD_HEIGHT);
        let chosen = index == self.shop_selected;
        let token = SHOP_CARD_BASE + index as u16;
        let hovered = self.ui.token_hovered(token);
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect,
            radius: 18.0 * s,
            color: color::alpha(color::SPACE, 0.92),
        });
        let _ = list.push(DrawCommand::GradientRect {
            rect,
            radius: 18.0 * s,
            gradient: sjk_ui::Gradient {
                start: color::alpha(colour, 0.03),
                end: color::alpha(colour, 0.22),
                vertical: true,
            },
        });
        crate::rarity_fx::draw(
            self.ui.draw_list_mut(),
            crate::rarity_fx::Item::of(tier, true, crate::rarity_fx::Size::Full),
            rect,
            18.0 * s,
            s,
            seconds,
        );
        self.ui.hit_region(token, rect);
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 18.0 * s,
            width: if chosen { 3.0 * s } else { 2.0 * s },
            color: if chosen {
                Color::new(1.0, 1.0, 1.0, 0.75)
            } else if hovered {
                color::alpha(colour, 1.0)
            } else {
                color::alpha(colour, 0.7)
            },
        });
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{} {}", tier.symbol(), tier.label()),
            frame.rect(x, y + 22.0, CARD_WIDTH, 26.0),
            19.0 * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let look = self.skins.get(&offer.id);
        let _ = crate::console::collection_panel::swatch::small_swatch(
            self.ui.draw_list_mut(),
            frame,
            [x + 28.0, y + 74.0, CARD_WIDTH - 56.0, 170.0],
            look,
            true,
            seconds,
            0.0,
        );
        let name = if offer.name.trim().is_empty() {
            crate::unlockables::find(&offer.id).map_or("A saber shader", |skin| skin.name)
        } else {
            offer.name.trim()
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(x + 12.0, y + 268.0, CARD_WIDTH - 24.0, 38.0),
            30.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Saber shader"),
            frame.rect(x, y + 308.0, CARD_WIDTH, 22.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Center,
        );
        if crate::economy::shop_in_shards() {
            super::opening::shards_line(
                &mut self.ui,
                frame,
                x + CARD_WIDTH * 0.5,
                y + 352.0,
                &crate::economy::shards_text(offer.price),
                32.0,
                1.0,
            );
        } else {
            super::opening::credits_line(
                &mut self.ui,
                frame,
                x + CARD_WIDTH * 0.5,
                y + 352.0,
                &crate::cards::credits_text(offer.price),
                32.0,
                1.0,
            );
        }
        let state = can(&offer, self.shop_purse());
        let status = match state {
            Can::Buy => String::new(),
            Can::Bought => "You bought it today.".to_owned(),
            Can::Owned => "You hold it already.".to_owned(),
            Can::Short(missing) => format!("{} more needed.", priced(missing)),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(x, y + 408.0, CARD_WIDTH, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Center,
        );
        let confirming = self.confirm.is_some_and(|(at, since)| {
            at == index && now.saturating_duration_since(since) < CONFIRM
        });
        let on_its_way = self.buying();
        let buying = on_its_way && chosen;
        let label = match state {
            _ if buying => "Buying...".to_owned(),
            Can::Buy if confirming => format!("Confirm: {}", priced(offer.price)),
            Can::Buy | Can::Short(_) => "Buy".to_owned(),
            Can::Bought => "Bought".to_owned(),
            Can::Owned => "Owned".to_owned(),
        };
        kit::button(
            &mut self.ui,
            frame,
            [x + 40.0, y + 456.0, CARD_WIDTH - 80.0, 50.0],
            &label,
            state == Can::Buy,
            state == Can::Buy && !on_its_way,
            chosen && confirming,
            SHOP_BUY_BASE + index as u16,
        );
    }

    /// Under the cards: what came of the last purchase, or why the shop shows nothing.
    fn shop_message(&mut self, frame: &Frame) {
        let s = frame.s;
        let (words, colour) = match &self.data.shop {
            None => ("Reading the shop...".to_owned(), color::MUTED),
            Some(state) if !state.message.is_empty() => (state.message.clone(), color::MUTED),
            Some(state) if state.day.offers.is_empty() => {
                ("The SJK hub offers nothing today.".to_owned(), color::MUTED)
            }
            Some(_) => match (&self.data.trade, self.buying) {
                // Only a purchase asked from this page, once the hub answered it.
                (Some(trade), Some(asked))
                    if trade.serial > asked
                        && matches!(
                            trade.trade,
                            sjk_identity::Trade::Buy | sjk_identity::Trade::Holocron
                        ) =>
                {
                    let name = if trade.trade == sjk_identity::Trade::Holocron {
                        holocron_tier(&trade.id)
                            .map_or_else(|| trade.id.clone(), |tier| tier.name.to_owned())
                    } else {
                        self.offers()
                            .iter()
                            .find(|offer| offer.id == trade.id)
                            .map_or_else(|| trade.id.clone(), |offer| offer.name.clone())
                    };
                    (
                        crate::economy::outcome_words(trade, &name),
                        if trade.done() {
                            color::GOLD_BRIGHT
                        } else {
                            color::EMBER
                        },
                    )
                }
                _ => (String::new(), color::MUTED),
            },
        };
        let y = if self.offers().is_empty() {
            CARDS_TOP + 180.0
        } else {
            MESSAGE_Y
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{words}"),
            frame.rect(LEFT, y, WIDTH, 30.0),
            20.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Center,
        );
    }

    /// Back, and the keys.
    fn shop_keys(&mut self, frame: &Frame) {
        let s = frame.s;
        kit::button(
            &mut self.ui,
            frame,
            [LEFT, BACK_Y, BACK_WIDTH, 44.0],
            "Back",
            false,
            true,
            false,
            SHOP_BACK_TOKEN,
        );
        let keys: [(&[&str], &str); 3] = [
            (&["Left", "Right"], "choose"),
            (&["Enter"], "buy"),
            (&["Esc"], "back"),
        ];
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(RIGHT, super::view::KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }

    /// The player's kyber shards and credits, right-aligned to `right` at `y` (frame
    /// pixels): each mark and sum, the word under it. Nothing before the hub said.
    pub(super) fn balances(&mut self, frame: &Frame, right: f32, y: f32) {
        let (Some(credits), shards) = (self.data.credits, self.data.shards) else {
            return;
        };
        let s = frame.s;
        let size = 30.0;
        let scale = crate::text::style::current().scale;
        let mut right = right;
        let mut column = |ui: &mut MenuCanvas,
                          words: String,
                          label: &str,
                          colour: Color,
                          mark: &dyn Fn(&mut sjk_ui::DrawList, [f32; 2], f32) -> f32,
                          mark_width: f32| {
            let width = crate::text::display_width(&words, size) * scale;
            text(
                ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(right - width - 8.0, y, width + 8.0, 38.0),
                size * s,
                colour,
                FontWeight::Semibold,
                TextAlign::End,
            );
            let left = right - width - 8.0 - 10.0 - mark_width;
            let [mx, my] = frame.point(left, y + 5.0);
            let _ = mark(ui.draw_list_mut(), [mx, my], s);
            text(
                ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(right - 200.0, y + 50.0, 200.0, 24.0),
                15.0 * s,
                color::QUIET,
                FontWeight::Regular,
                TextAlign::End,
            );
            right = left - 44.0;
        };
        let mark_height = 26.0;
        column(
            &mut self.ui,
            crate::cards::credits_text(credits),
            "credits",
            color::GOLD_BRIGHT,
            &|list, at, s| crate::cards::mark::draw(list, at, mark_height * s, color::GOLD),
            crate::cards::mark::width(mark_height),
        );
        if let Some(shards) = shards {
            column(
                &mut self.ui,
                crate::economy::shards_text(shards),
                "kyber shards",
                crate::cards::shard::KYBER,
                &|list, at, s| {
                    crate::cards::shard::draw(list, at, mark_height * s, crate::cards::shard::KYBER)
                },
                crate::cards::shard::width(mark_height),
            );
        }
    }
}

#[cfg(test)]
fn offer(id: &str, tier: &str, price: u64) -> sjk_identity::Offer {
    sjk_identity::Offer {
        id: id.to_owned(),
        name: crate::unlockables::find(id).map_or(String::new(), |skin| skin.name.to_owned()),
        tier: tier.to_owned(),
        price,
        bought: false,
        owned: false,
    }
}

/// The day the shop shows in a world shot and the tests: four offers, one of each
/// slot, the next day `left` seconds away.
#[cfg(test)]
pub(crate) fn sample_day(left: i64) -> sjk_identity::ShopState {
    sjk_identity::ShopState {
        serial: 1,
        day: sjk_identity::ShopDay {
            day: "2026-10-17".to_owned(),
            offers: vec![
                offer("saber_aurora", "common", 30),
                offer("saber_sakura", "uncommon", 75),
                offer("saber_lavalamp", "rare", 225),
                offer("saber_phoenix", "mythical", 1_500),
            ],
            next: 1_791_000_000 + left,
            server_time: 1_791_000_000,
            credits: 300,
            shards: 300,
        },
        message: String::new(),
        read_at: Instant::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(shop: Option<sjk_identity::ShopState>) -> Data {
        Data {
            counts: Some([1, 0, 0, 0]),
            credits: Some(300),
            shards: Some(300),
            every: Some(1_800),
            can_open: true,
            shop,
            ..Data::nothing(State::Known)
        }
    }

    #[test]
    fn days_and_countdowns_read_as_players_read_them() {
        assert_eq!(day_text("2026-10-17"), "17/10/2026");
        assert_eq!(day_text("soon"), "soon");
        assert_eq!(countdown_text(30), "less than a minute");
        assert_eq!(countdown_text(60), "1 min");
        assert_eq!(countdown_text(61), "2 min");
        assert_eq!(countdown_text(3_600), "1 h");
        assert_eq!(countdown_text(5 * 3_600 + 12 * 60), "5 h 12 min");
    }

    #[test]
    fn an_offer_says_what_the_player_can_do() {
        let mut lava = offer("saber_lavalamp", "rare", 225);
        assert_eq!(can(&lava, 225), Can::Buy);
        assert_eq!(can(&lava, 200), Can::Short(25));
        lava.owned = true;
        assert_eq!(can(&lava, 9_999), Can::Owned);
        lava.bought = true;
        assert_eq!(can(&lava, 9_999), Can::Bought);
        assert_eq!(tier_of(&offer("saber_sun", "common", 1)), Rarity::Mythical);
        assert_eq!(
            tier_of(&offer("saber_future", "legendary", 1)),
            Rarity::Legendary
        );
    }

    /// S opens the shop and reads the hub's day, only while the hub lets the player open
    /// holocrons; Escape and S go back; the arrows choose; a purchase asks to confirm.
    #[test]
    fn the_shop_comes_and_goes_and_a_purchase_asks_twice() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = Data {
            can_open: false,
            ..known(None)
        };
        assert_eq!(panel.key(KeyCode::KeyS, false), PanelAction::None);
        assert!(!panel.showing_shop(), "closed while opening is for staff");
        panel.data = known(Some(sample_day(3_600)));
        assert_eq!(panel.key(KeyCode::KeyS, false), PanelAction::ReadShop);
        assert!(panel.showing_shop());
        let _ = panel.key(KeyCode::ArrowRight, false);
        let _ = panel.key(KeyCode::ArrowRight, false);
        assert_eq!(panel.shop_selected, 2);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        assert_eq!(
            panel.key(KeyCode::Enter, false),
            PanelAction::Buy("saber_lavalamp".to_owned())
        );
        // On its way: nothing more is asked until the hub answers.
        assert!(panel.buying());
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        // Too dear: nothing to confirm.
        panel.buying = None;
        let _ = panel.key(KeyCode::ArrowRight, false);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        assert_eq!(panel.confirm, None);
        // Past the offers, the holocrons for credits: the Uncommon one asks twice; the
        // Rare one is too dear for 300 credits.
        let _ = panel.key(KeyCode::ArrowRight, false);
        assert_eq!(panel.shop_selected, 4);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        assert_eq!(
            panel.key(KeyCode::Enter, false),
            PanelAction::BuyHolocron("uncommon".to_owned())
        );
        panel.buying = None;
        let _ = panel.key(KeyCode::ArrowRight, false);
        assert_eq!(panel.shop_selected, 5);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        assert_eq!(panel.key(KeyCode::Enter, false), PanelAction::None);
        let _ = panel.key(KeyCode::ArrowRight, false);
        assert_eq!(panel.shop_selected, 5, "the last choice");
        assert_eq!(panel.key(KeyCode::Escape, false), PanelAction::None);
        assert!(!panel.showing_shop(), "Escape goes back to the page");
    }

    /// The shop is read again once its day turns, once.
    #[test]
    fn a_turned_day_is_read_again_once() {
        let mut panel = Panel::new();
        panel.open(true);
        panel.data = known(Some(sample_day(0)));
        let now = Instant::now();
        assert!(!panel.shop_due(now), "not while the page shows");
        let _ = panel.show_shop(true);
        assert!(panel.shop_due(now));
        assert!(!panel.shop_due(now), "once");
        panel.data = known(Some(sample_day(3_600)));
        panel.data.shop.as_mut().unwrap().serial = 2;
        assert!(!panel.shop_due(now));
    }

    /// The shop fits the canvas and the window in every shape: read, not read, refused;
    /// the cards end above the message line and Back.
    #[test]
    fn the_shop_fits() {
        let mut refused = sample_day(0);
        refused.message = "Shards and the shop open with holocrons: soon.".to_owned();
        refused.day.offers.clear();
        for shop in [None, Some(sample_day(5 * 3_600)), Some(refused)] {
            for viewport in [
                [1920.0, 1080.0],
                [3840.0, 2160.0],
                [1440.0, 1080.0],
                [2560.0, 1080.0],
            ] {
                let mut panel = Panel::new();
                panel.open(true);
                panel.data = known(shop.clone());
                let _ = panel.show_shop(true);
                panel.build(viewport);
                assert!(!panel.overflowed(), "{viewport:?}");
                let back = panel.ui.rect_for(SHOP_BACK_TOKEN).expect("Back");
                assert!(back.bottom() <= viewport[1] && back.x >= 0.0);
                let frame = Frame::new(viewport);
                for index in 0..panel.offers().len() {
                    let card = panel
                        .ui
                        .rect_for(SHOP_CARD_BASE + index as u16)
                        .expect("a card");
                    assert!(card.x >= 0.0 && card.right() <= viewport[0], "{viewport:?}");
                    assert!(card.bottom() < frame.point(0.0, MESSAGE_Y)[1]);
                    let buy = panel.ui.rect_for(SHOP_BUY_BASE + index as u16).unwrap();
                    assert!(buy.bottom() <= card.bottom());
                }
                let back_top = back.y;
                for index in 0..2 {
                    if let Some(button) = panel.ui.rect_for(SHOP_HOLOCRON_BASE + index) {
                        assert!(button.x >= 0.0 && button.right() <= viewport[0]);
                        assert!(button.bottom() < back_top, "{viewport:?}");
                    }
                }
            }
        }
        const { assert!(CARDS_TOP + CARD_HEIGHT < MESSAGE_Y && MESSAGE_Y + 30.0 < HOLOCRONS_Y) };
        const { assert!(HOLOCRONS_Y + HOLOCRON_HEIGHT + 30.0 < BACK_Y) };
        const { assert!(4.0 * CARD_WIDTH + 3.0 * CARD_GAP <= RIGHT - LEFT) };
    }
}
