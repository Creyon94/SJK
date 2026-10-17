//! The holocron economy on this client (`docs/cards.md`, "Shards and the shop"): kyber
//! shards beside credits, what shattering a duplicate gives and crafting costs, the shop's
//! prices (in shards since 17/10/2026, Sol's decision), what a holocron costs in credits,
//! and the requests that trade. The hub decides everything; this keeps its tables
//! ([`current`]) and says what a screen may offer.
//!
//! Only shaders of the *trade* take part: the ones holocrons drop, less any a medal brings
//! ([`in_trade`], read from the catalogue and the medals, as the hub's
//! `economy::in_trade`). Cards are luck only, hats and capes free for everyone: neither is
//! ever sold, crafted or shattered.
//!
//! The Shaders tab's hooks (its shader cards' Shatter and Craft): [`in_trade`],
//! [`choices`], [`shatter_yield`], [`craft_cost`], [`balance`], [`can_trade`],
//! [`shatter`], [`craft`], [`trade_outcome`] and [`outcome_words`]. The shop is the
//! Holocrons tab's ([`crate::holocrons_panel`]).

use crate::unlockables::{Rarity, Unlockable};
use std::sync::Mutex;

/// The shader tiers, Common to Mythical.
pub(crate) const TIERS: usize = 5;

/// The economy's tables by shader tier, Common to Mythical.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Tables {
    /// The kyber shards shattering one duplicate gives.
    pub(crate) shatter: [u64; TIERS],
    /// The kyber shards crafting one costs.
    pub(crate) craft: [u64; TIERS],
    /// What one costs in the shop, in shards ([`Tables::shop_in_shards`]) or credits.
    pub(crate) shop: [u64; TIERS],
    /// Whether the shop sells for kyber shards (hubs since 17/10/2026) or credits (before).
    pub(crate) shop_in_shards: bool,
    /// What an Uncommon and a Rare holocron cost in credits; 0 when not for sale.
    pub(crate) holocrons: [u64; 2],
}

/// This client's copy of the hub's tables (`src/economy.rs` there, provisional
/// 17/10/2026), shown until the hub sends its own with the progress.
pub(crate) const BUILT_IN: Tables = Tables {
    shatter: [5, 10, 25, 60, 150],
    craft: [40, 100, 300, 800, 2_000],
    shop: [30, 75, 225, 600, 1_500],
    shop_in_shards: true,
    holocrons: [150, 400],
};

/// The holocron tiers credits may buy, by the hub's ids, in [`Tables::holocrons`]' order.
pub(crate) const HOLOCRONS_FOR_SALE: [&str; 2] = ["uncommon", "rare"];

static HUB: Mutex<Option<Tables>> = Mutex::new(None);

/// The tables to show: the hub's last that fit, else [`BUILT_IN`].
pub(crate) fn current() -> Tables {
    HUB.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .unwrap_or(BUILT_IN)
}

/// Take the tables the hub sent last (`Snapshot::economy`); nothing changes when it sent
/// none or tables this client cannot show (not five tiers, or a zero).
pub(crate) fn follow(economy: Option<&sjk_identity::EconomyTables>) {
    if let Some(table) = economy.and_then(of_hub) {
        *HUB.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(table);
    }
}

/// The hub's tables as this client keeps them, if they fit.
fn of_hub(economy: &sjk_identity::EconomyTables) -> Option<Tables> {
    let row = |values: &[u64]| -> Option<[u64; TIERS]> {
        let row: [u64; TIERS] = values.try_into().ok()?;
        row.iter().all(|value| *value > 0).then_some(row)
    };
    Some(Tables {
        shatter: row(&economy.shatter)?,
        craft: row(&economy.craft)?,
        shop: row(&economy.shop)?,
        shop_in_shards: economy.shop_currency == "shards",
        holocrons: HOLOCRONS_FOR_SALE.map(|tier| economy.holocrons.get(tier).copied().unwrap_or(0)),
    })
}

/// Whether the shop sells for kyber shards (else credits, as hubs before 17/10/2026).
pub(crate) fn shop_in_shards() -> bool {
    current().shop_in_shards
}

/// The holocrons credits may buy now: each tier's id and its price in credits.
pub(crate) fn holocrons_for_sale() -> Vec<(&'static str, u64)> {
    HOLOCRONS_FOR_SALE
        .into_iter()
        .zip(current().holocrons)
        .filter(|(_, price)| *price > 0)
        .collect()
}

/// Whether `skin` takes part in the trade: holocrons drop it and no medal brings it.
pub(crate) fn in_trade(skin: &Unlockable) -> bool {
    skin.droppable
        && !crate::medals::Medal::ALL
            .iter()
            .any(|medal| medal.blade() == Some(skin.id))
}

/// The kyber shards shattering one duplicate of tier `tier` gives.
pub(crate) fn shatter_yield(tier: Rarity) -> u64 {
    current().shatter[tier as usize]
}

/// The kyber shards crafting a shader of tier `tier` costs.
pub(crate) fn craft_cost(tier: Rarity) -> u64 {
    current().craft[tier as usize]
}

/// What a shader of tier `tier` costs in the shop (the shop itself shows the prices the
/// hub sends with its offers; this is the table's).
#[cfg(test)]
pub(crate) fn shop_price(tier: Rarity) -> u64 {
    current().shop[tier as usize]
}

/// A count of shards as players read it: "1 250".
pub(crate) fn shards_text(shards: u64) -> String {
    crate::cards::grouped(shards)
}

/// What a shader card may offer for `skin`, held `held` times by grant or drops (a
/// medal's copy is not counted), the player holding `shards`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Choices {
    /// Shatter one duplicate for this many shards: only of the trade, with two or more.
    pub(crate) shatter: Option<u64>,
    /// Craft it for this many shards: only of the trade, held none.
    pub(crate) craft: Option<u64>,
    /// Whether the player's shards pay for the craft.
    pub(crate) affordable: bool,
}

/// What a shader card may offer for `skin` (see [`Choices`]).
pub(crate) fn choices(skin: &Unlockable, held: u32, shards: u64) -> Choices {
    if !in_trade(skin) {
        return Choices {
            shatter: None,
            craft: None,
            affordable: false,
        };
    }
    let cost = craft_cost(skin.tier);
    Choices {
        shatter: (held > 1).then(|| shatter_yield(skin.tier)),
        craft: (held == 0).then_some(cost),
        affordable: shards >= cost,
    }
}

/// The player's credits and kyber shards, once the hub sent their profile.
pub(crate) fn balance() -> Option<(u64, u64)> {
    crate::player_identity::with_snapshot(|snapshot| {
        snapshot.me.as_ref().map(|me| (me.credits, me.shards))
    })
    .flatten()
}

/// Whether the hub lets the player trade now: as for opening holocrons, only staff while
/// opening is tested (false until the hub said).
pub(crate) fn can_trade() -> bool {
    crate::player_identity::with_snapshot(|snapshot| {
        snapshot.holocrons.is_some_and(|progress| progress.can_open)
    })
    .unwrap_or(false)
}

/// Shatter one duplicate of shader `id`; false when the identity has not started. What
/// came of it arrives in [`trade_outcome`].
pub(crate) fn shatter(id: &str) -> bool {
    crate::player_identity::with_service(|service| service.shatter(id)).is_some()
}

/// Craft shader `id` with kyber shards; false when the identity has not started. What
/// came of it arrives in [`trade_outcome`].
pub(crate) fn craft(id: &str) -> bool {
    crate::player_identity::with_service(|service| service.craft(id)).is_some()
}

/// Buy a holocron of tier `tier` with credits; false when the identity has not started.
/// What came of it arrives in [`trade_outcome`], the holocron in the player's list.
pub(crate) fn buy_holocron(tier: &str) -> bool {
    crate::player_identity::with_service(|service| service.buy_holocron(tier)).is_some()
}

/// What became of the last shatter, craft or purchase, once the identity has started.
pub(crate) fn trade_outcome() -> Option<sjk_identity::TradeOutcome> {
    crate::player_identity::with_snapshot(|snapshot| snapshot.trade.clone()).flatten()
}

/// What a trade's outcome says, in a line: "Tidal blade shattered: +5 kyber shards.",
/// or why it failed. `name` is the shader's name (a holocron's tier name for one bought).
pub(crate) fn outcome_words(outcome: &sjk_identity::TradeOutcome, name: &str) -> String {
    use sjk_identity::Trade;
    if !outcome.done() {
        return outcome.message.clone();
    }
    let amount = shards_text(outcome.amount);
    match outcome.trade {
        Trade::Shatter => format!("{name} shattered: +{amount} kyber shards."),
        Trade::Craft => format!("{name} crafted for {amount} kyber shards. It is yours."),
        Trade::Buy if shop_in_shards() => {
            format!("{name} bought for {amount} kyber shards. It is yours.")
        }
        Trade::Buy => format!("{name} bought for {amount} credits. It is yours."),
        Trade::Holocron => format!(
            "{name} bought for {} credits. It waits in your holocrons.",
            crate::cards::credits_text(outcome.amount)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unlockables;

    /// The copy matches the hub's (`src/economy.rs`): the same numbers there, pinned by
    /// both sides' tests.
    #[test]
    fn the_built_in_tables_are_the_hubs() {
        assert_eq!(BUILT_IN.shatter, [5, 10, 25, 60, 150]);
        assert_eq!(BUILT_IN.craft, [40, 100, 300, 800, 2_000]);
        assert_eq!(BUILT_IN.shop, [30, 75, 225, 600, 1_500]);
        const { assert!(BUILT_IN.shop_in_shards) };
        assert_eq!(BUILT_IN.holocrons, [150, 400]);
        for table in [BUILT_IN.shatter, BUILT_IN.craft, BUILT_IN.shop] {
            assert!(table.windows(2).all(|pair| pair[0] < pair[1]));
        }
        for tier in HOLOCRONS_FOR_SALE {
            assert!(crate::holocrons::TIERS.iter().any(|known| known.id == tier));
        }
    }

    #[test]
    fn the_hubs_tables_replace_the_copy_only_when_they_fit() {
        let wire = |shatter: Vec<u64>| sjk_identity::EconomyTables {
            shatter,
            craft: vec![1, 2, 3, 4, 5],
            shop: vec![10, 20, 30, 40, 50],
            ..Default::default()
        };
        assert_eq!(
            of_hub(&wire(vec![9, 8, 7, 6, 5])).map(|t| t.shatter),
            Some([9, 8, 7, 6, 5])
        );
        // A hub before the shop sold for shards: credits, no holocrons for sale.
        let older = of_hub(&wire(vec![1, 2, 3, 4, 5])).unwrap();
        assert_eq!((older.shop_in_shards, older.holocrons), (false, [0, 0]));
        let newer = of_hub(&sjk_identity::EconomyTables {
            shop_currency: "shards".to_owned(),
            holocrons: [("rare".to_owned(), 400)].into_iter().collect(),
            ..wire(vec![1, 2, 3, 4, 5])
        })
        .unwrap();
        assert_eq!((newer.shop_in_shards, newer.holocrons), (true, [0, 400]));
        assert_eq!(of_hub(&wire(vec![1, 2, 3, 4])), None, "four tiers");
        assert_eq!(of_hub(&wire(vec![0, 2, 3, 4, 5])), None, "a zero");
    }

    /// Medal blades never take part; every shader holocrons drop does (since 17/10/2026
    /// every one but the medals' drops, so the team's Sun too).
    #[test]
    fn only_what_holocrons_drop_and_no_medal_brings_is_traded() {
        for medal in crate::medals::Medal::ALL {
            if let Some(id) = medal.blade() {
                assert!(!in_trade(unlockables::find(id).unwrap()), "{id}");
            }
        }
        assert!(in_trade(unlockables::find("saber_sun").unwrap()));
        assert!(in_trade(unlockables::find("saber_tidal").unwrap()));
        assert!(
            unlockables::blade_skins()
                .filter(|skin| in_trade(skin))
                .all(|skin| skin.droppable)
        );
    }

    #[test]
    fn a_card_offers_shatter_for_duplicates_and_craft_for_what_is_missing() {
        let tidal = unlockables::find("saber_tidal").unwrap();
        let none = choices(tidal, 0, 0);
        assert_eq!(
            (none.shatter, none.craft, none.affordable),
            (None, Some(40), false)
        );
        assert!(choices(tidal, 0, 40).affordable);
        let one = choices(tidal, 1, 500);
        assert_eq!(
            (one.shatter, one.craft),
            (None, None),
            "the last copy stays"
        );
        assert_eq!(choices(tidal, 3, 0).shatter, Some(5));
        let glitch = unlockables::find("saber_glitch").unwrap();
        let medal = choices(glitch, 4, 10_000);
        assert_eq!((medal.shatter, medal.craft), (None, None));
        assert_eq!(shatter_yield(Rarity::Mythical), 150);
        assert_eq!(craft_cost(Rarity::Rare), 300);
        assert_eq!(shop_price(Rarity::Legendary), 600);
    }

    #[test]
    fn an_outcome_reads_as_a_line() {
        use sjk_identity::{Trade, TradeOutcome};
        let done = |trade, amount| TradeOutcome {
            serial: 1,
            trade,
            id: "saber_tidal".to_owned(),
            amount,
            message: String::new(),
        };
        assert_eq!(
            outcome_words(&done(Trade::Shatter, 5), "Tidal blade"),
            "Tidal blade shattered: +5 kyber shards."
        );
        assert_eq!(
            outcome_words(&done(Trade::Buy, 1_500), "Lava Lamp blade"),
            "Lava Lamp blade bought for 1 500 kyber shards. It is yours."
        );
        assert_eq!(
            outcome_words(&done(Trade::Holocron, 400), "Rare Holocron"),
            "Rare Holocron bought for 400 credits. It waits in your holocrons."
        );
        let refused = TradeOutcome {
            message: "Not enough kyber shards.".to_owned(),
            ..done(Trade::Craft, 0)
        };
        assert_eq!(outcome_words(&refused, "x"), "Not enough kyber shards.");
        assert_eq!(shards_text(12_345), "12 345");
    }
}
