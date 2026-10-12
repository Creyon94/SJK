//! World shots of opening holocrons and of the Collection's Cards tab
//! ([`crate::console::holocrons_panel::opening`], `collection_cards.rs`) over the live duel6
//! in the SJK UI: the Holocrons tab with its Open buttons and the credits; the ceremony at
//! each moment of a card's entrance (the burst, the card turning in, the credits counting,
//! the words) in each finish; the wait for the hub and a refusal; then the Cards tab with
//! every kind of sealed card, a Solar one chosen, and with none; and the loot table over
//! the Holocrons tab. Made-up holdings; they
//! are off-screen renders (no game window), ignored like the other world shots: they need
//! a GPU adapter and the installed game data named by `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer cards_shots -- --ignored
//! --test-threads=1 --nocapture`; the pictures are in `target/world-shots`.

use super::*;
use crate::cards::{FINISHES, RARITIES};
use crate::console::holocrons_panel::Data;
use crate::console::holocrons_panel::opening::Shown;
use crate::profile_hub::Tab;

/// The Holocrons tab's made-up holdings with holocrons to open and credits.
fn holdings() -> Data {
    Data {
        sealed: [vec![11, 12, 15], vec![13], Vec::new(), vec![14]],
        credits: Some(1_340),
        ..super::holocrons_tab::holdings()
    }
}

/// One holocron of tier `tier` opened: a card of `rarity` in `finish`.
fn shown(tier: usize, rarity: usize, finish: usize, credits: u64) -> Shown {
    Shown {
        tier,
        rarity: &RARITIES[rarity],
        finish: &FINISHES[finish],
        edition: "Alpha Edition".to_owned(),
        credits,
    }
}

/// Every kind of sealed card: each rarity in each finish.
fn every_kind() -> Vec<sjk_identity::CardStack> {
    let mut list = Vec::new();
    for finish in &FINISHES {
        for rarity in &RARITIES {
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

fn sjk_cvars() -> [(&'static str, &'static str); 3] {
    [
        ("ui_menuStyle", "sjk"),
        (crate::settings::quick::HIDE_CVAR, "1"),
        ("name", "^1Sol^7Vulpes"),
    ]
}

/// The main menu over duel6 in `size`, the Collection open on its Holocrons tab (the
/// loot table shown by [`duel6_sjk_loot_table`]).
fn open_holocrons(size: [u32; 2]) -> Option<GpuState> {
    let menu = menu::ClientMenu::new(true, String::new());
    let (mut gpu, profile) = open("maps/mp/duel6.bsp", size, Some(menu), &sjk_cvars())?;
    // The window's profile folder lives as long as the test.
    std::mem::forget(profile);
    let _ = frame(&mut gpu, 10);
    gpu.holocron_stage_seconds = Some(2.4);
    if let Some(console) = gpu.console.as_mut() {
        console.preview_holocrons(holdings(), 0);
    }
    gpu.open_profile_hub(Tab::Holocrons);
    assert_eq!(gpu.profile_hub_tab(), Some(Tab::Holocrons));
    let _ = frame(&mut gpu, 150);
    Some(gpu)
}

fn overflowed(gpu: &GpuState) -> bool {
    gpu.console.as_ref().is_some_and(|console| {
        console.holocrons_page_overflowed() || console.profile_pages_overflowed()
    })
}

/// Play `cards`' ceremony drawn `elapsed` seconds into the first card's entrance.
fn ceremony(gpu: &mut GpuState, cards: Vec<Shown>, elapsed: f32) {
    if let Some(console) = gpu.console.as_mut() {
        console.preview_opening(Some(cards), elapsed, 3.1);
    }
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_opening() {
    on_big_stack(|| {
        let Some(mut gpu) = open_holocrons([1920, 1080]) else {
            return;
        };
        println!("{}", shoot(&mut gpu, 2, "duel6-opening-page").display());
        assert!(!overflowed(&gpu));
        // A player while the hub keeps opening to staff: a line in place of the buttons.
        if let Some(console) = gpu.console.as_mut() {
            console.preview_holocrons(
                Data {
                    can_open: false,
                    ..holdings()
                },
                0,
            );
        }
        let _ = frame(&mut gpu, 4);
        println!("{}", shoot(&mut gpu, 2, "duel6-opening-closed").display());
        assert!(!overflowed(&gpu));
        if let Some(console) = gpu.console.as_mut() {
            console.preview_holocrons(holdings(), 0);
        }
        // A Holo card's entrance, moment by moment.
        let mut moments = Vec::new();
        for (elapsed, name) in [
            (0.3, "burst"),
            (0.8, "turning"),
            (1.5, "counting"),
            (3.0, "words"),
        ] {
            ceremony(
                &mut gpu,
                vec![shown(1, 2, 1, 42), shown(0, 0, 0, 14)],
                elapsed,
            );
            let path = shoot(&mut gpu, 2, &format!("duel6-opening-holo-{name}"));
            assert!(!overflowed(&gpu), "{name}");
            println!("{}", path.display());
            moments.push(image::open(path).expect("the shot").to_rgba8());
        }
        println!(
            "{}",
            sheet(&moments, 2, 960, "duel6-opening-moments").display()
        );
        // Each finish, standing still.
        let mut finishes = Vec::new();
        for (tier, rarity, finish, credits) in [(0, 0, 0, 14), (2, 3, 1, 96), (3, 1, 2, 311)] {
            ceremony(&mut gpu, vec![shown(tier, rarity, finish, credits)], 3.0);
            let path = shoot(
                &mut gpu,
                2,
                &format!("duel6-opening-{}", FINISHES[finish].id),
            );
            assert!(!overflowed(&gpu));
            println!("{}", path.display());
            finishes.push(image::open(path).expect("the shot").to_rgba8());
        }
        println!(
            "{}",
            sheet(&finishes, 3, 640, "duel6-opening-finishes").display()
        );
        // The wait for the hub, then a refusal.
        if let Some(console) = gpu.console.as_mut() {
            console.preview_opening(None, 0.0, 0.0);
            console.preview_opening_state(None);
        }
        println!("{}", shoot(&mut gpu, 2, "duel6-opening-waiting").display());
        if let Some(console) = gpu.console.as_mut() {
            console.preview_opening_state(Some("That holocron was already opened."));
        }
        println!("{}", shoot(&mut gpu, 2, "duel6-opening-refused").display());
        assert!(!overflowed(&gpu));
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_cards_tab() {
    on_big_stack(|| {
        for (size, name) in [([1920, 1080], "1080p"), ([1440, 1080], "4x3")] {
            let menu = menu::ClientMenu::new(true, String::new());
            let Some((mut gpu, _profile)) =
                open("maps/mp/duel6.bsp", size, Some(menu), &sjk_cvars())
            else {
                return;
            };
            let _ = frame(&mut gpu, 10);
            if let Some(console) = gpu.console.as_mut() {
                console.preview_cards(every_kind(), 1_340);
                console.preview_collection(Some(Vec::new()), 3.1);
            }
            gpu.open_profile_hub(Tab::Cards);
            assert_eq!(gpu.profile_hub_tab(), Some(Tab::Cards));
            let _ = frame(&mut gpu, 120);
            // The Solar Rare, which the list puts near the start.
            if let Some(console) = gpu.console.as_mut() {
                console.collection_card_for_shot(1);
            }
            println!(
                "{}",
                shoot(&mut gpu, 4, &format!("duel6-cards-{name}")).display()
            );
            assert!(!overflowed(&gpu), "{name}");
        }
        let menu = menu::ClientMenu::new(true, String::new());
        let Some((mut gpu, _profile)) =
            open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &sjk_cvars())
        else {
            return;
        };
        let _ = frame(&mut gpu, 10);
        if let Some(console) = gpu.console.as_mut() {
            console.preview_cards(Vec::new(), 0);
        }
        gpu.open_profile_hub(Tab::Cards);
        let _ = frame(&mut gpu, 120);
        println!("{}", shoot(&mut gpu, 2, "duel6-cards-none").display());
    });
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn duel6_sjk_loot_table() {
    on_big_stack(|| {
        for (size, name) in [([1920, 1080], "1080p"), ([1440, 1080], "4x3")] {
            let Some(mut gpu) = open_holocrons(size) else {
                return;
            };
            if size[0] == 1920 {
                // The tab with its Loot table button.
                println!("{}", shoot(&mut gpu, 2, "duel6-loot-button").display());
            }
            if let Some(console) = gpu.console.as_mut() {
                console.preview_loot(true);
            }
            println!(
                "{}",
                shoot(&mut gpu, 2, &format!("duel6-loot-table-{name}")).display()
            );
            assert!(!overflowed(&gpu), "{name}");
        }
    });
}
