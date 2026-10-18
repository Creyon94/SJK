//! GPU memory across in-game map changes: builds the next map on the running
//! world's context, the way the world-install worker does (`session_transition.rs`),
//! adopts it and drops the old world, again and again. After each change it prints
//! the allocator's live bytes and the labels that grew, so a world that outlives its
//! map shows up by name. Players' 2026.1013-2026.1018 builds ran out of video memory
//! building a map after an hour or more of map changes. Off-screen, ignored like the
//! other world shots: it needs a GPU adapter and `JKA_GAME_DATA`.
//!
//! Run: `JKA_GAME_DATA=<GameData> cargo test -p sjk-viewer map_changes_vram --
//! --ignored --test-threads=1 --nocapture`.

use super::*;
use std::collections::BTreeMap;

const MAPS: [&str; 4] = [
    "maps/mp/ffa3.bsp",
    "maps/mp/ffa5.bsp",
    "maps/mp/siege_desert.bsp",
    "maps/mp/duel6.bsp",
];
const CHANGES: usize = 12;
const MIB: f64 = 1024.0 * 1024.0;

/// Live bytes per allocation label, and their total.
fn live(device: &wgpu::Device) -> (u64, BTreeMap<String, u64>) {
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let Some(report) = device.generate_allocator_report() else {
        return (0, BTreeMap::new());
    };
    eprintln!(
        "    reserved {:.0} MiB in {} blocks",
        report.total_reserved_bytes as f64 / MIB,
        report.blocks.len()
    );
    let mut by_label = BTreeMap::new();
    for allocation in report.allocations {
        *by_label.entry(allocation.name).or_insert(0) += allocation.size;
    }
    (report.total_allocated_bytes, by_label)
}

#[test]
#[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
fn map_changes_vram() {
    on_big_stack(|| {
        let size = [1920, 1080];
        let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", size, None, &[]) else {
            return;
        };
        let _ = frame(&mut gpu, 4);
        let (start, mut previous) = live(&gpu.device);
        eprintln!("vram: start {:.0} MiB", start as f64 / MIB);
        let mut after = Vec::new();
        for change in 0..CHANGES {
            let map = MAPS[change % MAPS.len()];
            let (mut input, _next) = world_input(map, None, &[]);
            input.console = None;
            let built = pollster::block_on(GpuState::new_with_context(
                Arc::clone(&gpu.context),
                size,
                input,
                None,
            ))
            .expect("the next map is built");
            let (peak, _) = live(&gpu.device);
            let mut adopted = gpu.adopt_world(built);
            attach_target(&mut adopted, size);
            gpu = adopted;
            let _ = frame(&mut gpu, 4);
            let (total, by_label) = live(&gpu.device);
            eprintln!(
                "vram: change {change:2} {map:28} peak {:6.0} MiB, after {:6.0} MiB",
                peak as f64 / MIB,
                total as f64 / MIB
            );
            for (label, bytes) in &by_label {
                let before = previous.get(label).copied().unwrap_or(0);
                if *bytes > before + (1 << 20) && change >= MAPS.len() {
                    eprintln!(
                        "    grew: {label} {:.1} -> {:.1} MiB",
                        before as f64 / MIB,
                        *bytes as f64 / MIB
                    );
                }
            }
            if change < MAPS.len() {
                let mut largest: Vec<_> = by_label.iter().collect();
                largest.sort_by(|a, b| b.1.cmp(a.1));
                for (label, bytes) in largest.into_iter().take(8) {
                    eprintln!("    largest: {label} {:.1} MiB", *bytes as f64 / MIB);
                }
            }
            previous = by_label;
            after.push(total);
        }
        // The same map, a whole cycle apart, must not hold more than it did then.
        let first = after[MAPS.len() - 1];
        let last = after[after.len() - 1];
        eprintln!(
            "vram: duel6 after cycle 1 {:.0} MiB, after the last {:.0} MiB",
            first as f64 / MIB,
            last as f64 / MIB
        );
        assert!(
            last <= first + first / 20,
            "GPU memory grew across map changes: {first} -> {last} bytes"
        );
    });
}
