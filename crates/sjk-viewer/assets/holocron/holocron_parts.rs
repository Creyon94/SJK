// The loot-box holocrons' parts and how they move (SJK; see README.md), included by
// crates/sjk-viewer/src/holocrons/stage.rs. Made by scripts/holocron_assets.py from
// holocron_forms.py's PARTS: edit that, not this.

/// Each tier's parts, in `TIERS` order, its body first.
#[rustfmt::skip]
pub(crate) const PARTS: [&[Part]; COUNT] = [
    &[
        Part { model: "models/sjk/holocron_uncommon.md3", locked: "models/sjk/holocron_uncommon_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
    ],
    &[
        Part { model: "models/sjk/holocron_rare.md3", locked: "models/sjk/holocron_rare_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_rare_halo.md3", locked: "models/sjk/holocron_rare_halo_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.25, 1.3], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.3], tumble: [0.55, 0.0], precess: [0.0, 0.25] },
    ],
    &[
        Part { model: "models/sjk/holocron_legendary.md3", locked: "models/sjk/holocron_legendary_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_legendary_cap.md3", locked: "models/sjk/holocron_legendary_cap_locked.md3", at: [0.0, 0.0, 3.0], bob: [0.35, 1.5], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.35], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_legendary_eye.md3", locked: "models/sjk/holocron_legendary_eye_locked.md3", at: [0.0, 0.0, 0.55], bob: [0.1, 1.5], orbit: [0.0, 0.0, 0.0], spin: [0.0, -1.2], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
    ],
    &[
        Part { model: "models/sjk/holocron_mythical.md3", locked: "models/sjk/holocron_mythical_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_crystal.md3", locked: "models/sjk/holocron_mythical_crystal_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.5], tumble: [0.0, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_ring_a.md3", locked: "models/sjk/holocron_mythical_ring_a_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [std::f32::consts::FRAC_PI_2, 0.3], precess: [0.0, 0.25] },
        Part { model: "models/sjk/holocron_mythical_ring_b.md3", locked: "models/sjk/holocron_mythical_ring_b_locked.md3", at: [0.0, 0.0, 0.0], bob: [0.0, 0.0], orbit: [0.0, 0.0, 0.0], spin: [0.0, 0.0], tumble: [1.0, -0.22], precess: [0.0, -0.15] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -3.5], bob: [0.0, 0.0], orbit: [3.5, 0.6, 0.0], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -0.91], bob: [0.0, 0.0], orbit: [3.837, 0.6, 0.63], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, 1.68], bob: [0.0, 0.0], orbit: [3.864, 0.6, 1.26], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -2.73], bob: [0.0, 0.0], orbit: [3.556, 0.6, 1.89], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -0.14], bob: [0.0, 0.0], orbit: [3.197, 0.6, 2.52], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, 2.45], bob: [0.0, 0.0], orbit: [3.116, 0.6, 3.15], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -1.96], bob: [0.0, 0.0], orbit: [3.388, 0.6, 3.78], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, 0.63], bob: [0.0, 0.0], orbit: [3.763, 0.6, 4.41], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, 3.22], bob: [0.0, 0.0], orbit: [3.896, 0.6, 5.04], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
        Part { model: "models/sjk/holocron_mythical_mote.md3", locked: "models/sjk/holocron_mythical_mote_locked.md3", at: [0.0, 0.0, -1.19], bob: [0.0, 0.0], orbit: [3.665, 0.6, 5.67], spin: [0.0, 1.6], tumble: [0.6, 0.0], precess: [0.0, 0.0] },
    ],
];
