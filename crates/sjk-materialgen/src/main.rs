//! `sjk-materialgen` command: see the library documentation and `--help`.

use sjk_materialgen::cli::{Command, USAGE, parse};
use sjk_materialgen::package::NOTICE;
use sjk_materialgen::run::{Summary, describe_candidate, run};
use std::collections::BTreeMap;
use std::process::ExitCode;

fn main() -> ExitCode {
    let options = match parse(std::env::args().skip(1)) {
        Ok(Command::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Command::Run(options)) => options,
        Err(error) => {
            eprintln!("sjk-materialgen: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("game data: {}", options.game_data.display());
    let summary = match run(&options) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("sjk-materialgen: {error}");
            return ExitCode::FAILURE;
        }
    };
    report(&options, &summary);
    if summary.failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn report(options: &sjk_materialgen::run::Options, summary: &Summary) {
    let selection = &summary.selection;
    for archive in &summary.excluded {
        println!("left out earlier output: {}", archive.display());
    }
    println!(
        "maps: {} ({})",
        selection.maps.len(),
        abbreviated(&selection.maps)
    );
    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for skipped in &selection.skipped {
        *reasons.entry(skipped.reason.describe()).or_default() += 1;
    }
    println!("skipped shaders/textures: {}", selection.skipped.len());
    for (reason, count) in &reasons {
        println!("  {count:>5}  {reason}");
    }
    if options.dry_run {
        println!(
            "would generate maps for {} textures:",
            selection.candidates.len()
        );
        for candidate in &selection.candidates {
            println!("  {}", describe_candidate(candidate));
        }
        for skipped in &selection.skipped {
            println!("  skip {:<44} {}", skipped.reason.describe(), skipped.name);
        }
        println!(
            "dry run: nothing written ({:.1} s)",
            summary.elapsed.as_secs_f64()
        );
        return;
    }
    if !summary.flat.is_empty() {
        println!("no surface detail, no maps: {}", abbreviated(&summary.flat));
    }
    println!(
        "emission maps: {} written for {} textures with signs of light (see the manifest)",
        summary.emission_written, summary.emission_considered
    );
    for (image, error) in &summary.failed {
        eprintln!("failed: {image}: {error}");
    }
    println!(
        "generated {} images for {} textures in {:.1} s: {} ({:.1} MiB)",
        summary.images,
        summary.generated,
        summary.elapsed.as_secs_f64(),
        options.out.display(),
        summary.bytes as f64 / (1024.0 * 1024.0)
    );
    let directory = options
        .out
        .parent()
        .map_or_else(|| ".".into(), |p| p.display().to_string());
    println!();
    println!("To try the maps without touching the game folder, start SJK with");
    println!("  SJK_CONTENT={directory}");
    println!(
        "(cmd: set \"SJK_CONTENT={directory}\"; PowerShell: $env:SJK_CONTENT=\"{directory}\")."
    );
    println!(
        "Or copy the pk3 into {}/base yourself; its zzz_ name loads after the retail pk3s.",
        options.game_data.display()
    );
    println!("Then: seta r_normalMapping 1; seta r_specularMapping 1; restart.");
    println!("Emission maps show with r_emissiveMaps 1 (the default).");
    println!();
    println!("{NOTICE}");
}

fn abbreviated(names: &[String]) -> String {
    if names.len() <= 8 {
        names.join(", ")
    } else {
        format!("{}, ... {} more", names[..8].join(", "), names.len() - 8)
    }
}
