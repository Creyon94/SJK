//! Blade Lab signature shaders: blade skins drawn by their own shader from Creyon's Blade
//! Lab (`docs/rendering.md`, "Blade Lab signature shaders").
//!
//! The lab (a three.js page) draws each blade with SJK's own skin shading ported to GLSL,
//! and lets a blade add a *signature*: `sig_glow`, `sig_core` and `sig_solid`, which take
//! what the skin's file drew and return the blade's own light, core and solid layer. The
//! signatures' GLSL lives in `blade_signatures/` beside its WGSL translation
//! (`scripts/blade_signatures.py`). A skin this table names is drawn by `saber.wgsl` as its
//! file says, each pass then handing its result to the signature (the `LAB_SIGNATURE`
//! design number); its pipelines are built from `saber.wgsl` with the stand-in signature
//! replaced by its own ([`module_source`]), once a blade wears it, off the render thread
//! (`saber_gpu.rs`). Until they are built the blade is drawn from its file alone.
//!
//! A signature skin whose file is in no pack is drawn from the lab's file built in here
//! ([`Signature::default_file`]), so the skins show (in the Collection's preview, and on
//! whoever is given one) before the hub's pack carries them.

use crate::blade_skin_file::BladeSkinDef;
use crate::blade_skin_file::extras::SkinOption;

/// One signature shader: the skin it draws and its translated WGSL.
#[derive(Debug)]
pub(crate) struct Signature {
    pub(crate) id: &'static str,
    wgsl: &'static str,
    file: &'static str,
}

macro_rules! signature {
    ($name:literal) => {
        Signature {
            id: concat!("saber_", $name),
            wgsl: include_str!(concat!("blade_signatures/", $name, ".wgsl")),
            file: include_str!(concat!("blade_signatures/", $name, ".bladeskin")),
        }
    };
}

/// The signatures, mythicals first (Creyon's order, 19/10/2026). The last fourteen are his
/// Blade Lab designs reworked in full (19/10/2026: the Star Wars rework; the Bleeding Kyber in
/// round 9), which replace their hand-ported designs (`blade_lab_designs.rs`).
pub(crate) const SIGNATURES: [Signature; 82] = [
    signature!("abyssal_eye"),
    signature!("aurora_wyrm"),
    signature!("balance"),
    signature!("between_worlds"),
    signature!("big_bang"),
    signature!("carbonite"),
    signature!("dyson_sphere"),
    signature!("eclipse"),
    signature!("event_horizon"),
    signature!("exogorth"),
    signature!("hyperspace"),
    signature!("kyber_heart"),
    signature!("nebula_song"),
    signature!("quasar"),
    signature!("singing_kyber"),
    signature!("sith_holocron"),
    signature!("star_dragon"),
    signature!("starforge"),
    signature!("superlaser"),
    signature!("uneti_tree"),
    signature!("witchfire"),
    signature!("world_shift"),
    signature!("wraithlight"),
    signature!("monarch"),
    signature!("obsidian"),
    signature!("quantum"),
    signature!("quicksilver"),
    signature!("reactor"),
    signature!("serpent"),
    signature!("peppermint"),
    signature!("bubble"),
    signature!("lavalamp"),
    signature!("sketch"),
    signature!("hourglass"),
    signature!("fiber"),
    signature!("phoenix"),
    signature!("bleeding"),
    // Round 11: the Blade Lab's legendaries, ported Sol's way and reworked to be Star Wars.
    signature!("hoth_avalanche"),
    signature!("blood_moon"),
    signature!("force_echo"),
    signature!("hypermatter"),
    signature!("kessel_run"),
    signature!("unknown_regions"),
    signature!("krayt_pearl"),
    signature!("starkiller_frost"),
    signature!("royal_naboo"),
    signature!("bespin_storm"),
    signature!("scarif_tide"),
    signature!("assassin_droid"),
    signature!("aurodium"),
    signature!("kashyyyk_monsoon"),
    signature!("malachor_forge"),
    signature!("yavin_vines"),
    signature!("exegol_rift"),
    signature!("shattered_kyber"),
    signature!("twin_suns"),
    signature!("jedi_spirits"),
    signature!("ord_mantell_storm"),
    signature!("mustafar_eruption"),
    signature!("hyperlane"),
    // Round 12: the Blade Lab's rares, a tier below the legendaries.
    signature!("ahch_to_glow"),
    signature!("death_star_plans"),
    signature!("jedi_texts"),
    signature!("star_map"),
    signature!("ilum_frost"),
    signature!("christophsis_crystal"),
    signature!("deflector_shield"),
    signature!("healed_kyber"),
    signature!("temple_mosaic"),
    signature!("dagobah_wisps"),
    signature!("coruscant_night"),
    signature!("navigation_beacon"),
    signature!("puzzle_holocron"),
    signature!("mon_cala_reef"),
    signature!("jakku_sandstorm"),
    signature!("moisture_farm"),
    signature!("tipoca_stormglass"),
    signature!("temple_sunbeam"),
    signature!("ion_coil"),
    signature!("sith_thorn"),
    signature!("holo_recording"),
    signature!("data_disc"),
];

/// The lab's helpers every signature may call (`lab.glsl`, translated).
const LAB: &str = include_str!("blade_signatures/lab.wgsl");
/// The stand-in signature in `saber.wgsl` that a signature replaces.
const BEGIN: &str = "// @signature begin\n";
const END: &str = "// @signature end\n";
/// The sections a signature's own parts ride on, part 0 to 5 (`SIG_PART_BITS` in
/// `saber.wgsl`): sections a signature never draws, so their bits are free.
const PART_SECTIONS: [&str; 6] = ["sputter", "glitch", "scan", "bands", "segments", "film"];

/// An option's id from its name: lower case letters, digits and `_`, at most 24.
fn slug(name: &str) -> String {
    let mut id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    id.truncate(24);
    id
}

/// The design number `saber.wgsl` draws a signature skin under (`LAB_SIGNATURE`).
pub(crate) const NUMBER: u32 = 100;

/// The signature of skin `id`, if it has one.
pub(crate) fn of(id: &str) -> Option<&'static Signature> {
    SIGNATURES.iter().find(|signature| signature.id == id)
}

impl Signature {
    /// A `// @key value` line of its header (`@room`, `@solid`, `@palette`).
    fn meta(&self, key: &str) -> Option<&'static str> {
        self.wgsl
            .lines()
            .take_while(|line| line.starts_with("//"))
            .find_map(|line| {
                line.strip_prefix("// @")?
                    .strip_prefix(key)?
                    .strip_prefix(' ')
                    .map(str::trim)
            })
    }

    /// How far (units) its quad reaches past the glow for what it draws there (`@room`).
    pub(crate) fn room(&self) -> f32 {
        self.meta("room")
            .and_then(|room| room.parse().ok())
            .unwrap_or(0.0)
    }

    /// Whether it draws a solid layer (`@solid 1`).
    pub(crate) fn solid(&self) -> bool {
        self.meta("solid") == Some("1")
    }

    /// Its `design` lane in `saber.wgsl`'s `Skin`: the signature's number, no reach of its
    /// own, its room (in the designs' metres) and whether it has a solid layer.
    pub(crate) fn lane(&self) -> [f32; 4] {
        [
            NUMBER as f32,
            0.0,
            self.room() * crate::blade_lab_designs::METRE,
            if self.solid() { 1.0 } else { 0.0 },
        ]
    }

    /// `def` as the signature draws it: none of the file's sections but its afterimages,
    /// heat haze, wisps and embers, which the renderer draws round any blade as round Sol's
    /// (Creyon's round 9: the Starforge lacked the Sun's heat), and the options every
    /// signature has, which
    /// `saber.wgsl` reads from section bits a signature skin has no other use for
    /// (`sig_take`): `effects` (the arcs bit) keeps only the blade itself, `bursts` (the
    /// motes bit) stops swings and clashes setting anything off, `ignition` (the hue bit)
    /// skips the show when it is lit.
    pub(crate) fn shown(&self, def: &BladeSkinDef) -> BladeSkinDef {
        let mut shown = crate::blade_lab_designs::bare(def);
        // Sol's own pieces a signature wears as his blades do (round 11: his lightning and
        // prominences are arcs, his souls and sparks motes).
        shown.arcs = def.arcs.clone();
        shown.motes = def.motes.clone();
        shown.hue = def.hue;
        shown.ghosts = def.ghosts;
        shown.haze = def.haze;
        shown.wisps = def.wisps;
        shown.embers = def.embers;
        shown.echo = def.echo;
        shown.star = def.star;
        shown.orbit = def.orbit;
        shown.veins = def.veins;
        shown.options = self.options(&shown);
        shown
    }

    /// Its parts, each switched on its own in the Collection (Creyon's round 11): the
    /// signature's own (`// @part <name>` lines, in order, on the bits `SIG_PART_BITS` reads
    /// in `saber.wgsl`), then Sol's sections its file has (named by `// @<section> <name>`
    /// lines, or plainly), then its swing and clash bursts, at most
    /// [`MAX_OPTIONS`](crate::blade_skin_file::extras::MAX_OPTIONS).
    fn options(&self, def: &BladeSkinDef) -> Vec<SkinOption> {
        use crate::blade_skin_file::extras::{MAX_OPTIONS, Section};
        let option = |name: &str, section: &str| SkinOption {
            id: slug(name),
            name: name.to_owned(),
            sections: vec![section.to_owned()],
        };
        let mut options: Vec<SkinOption> = self
            .parts()
            .zip(PART_SECTIONS)
            .map(|(name, section)| option(name, section))
            .collect();
        let sections = [
            (Section::Arcs, "Arcs"),
            (Section::Motes, "Motes"),
            (Section::Ghosts, "Afterimages"),
            (Section::Wisps, "Smoke"),
            (Section::Echo, "Echo"),
            (Section::Star, "Glints"),
            (Section::Embers, "Embers"),
            (Section::Veins, "Veins"),
            (Section::Haze, "Heat haze"),
            (Section::Hue, "Colour turn"),
        ];
        for (section, plain) in sections {
            if def.has(section) {
                let name = self.meta(section.name()).unwrap_or(plain);
                options.push(option(name, section.name()));
            }
        }
        options.push(option("Swing and clash bursts", "pulse"));
        options.truncate(MAX_OPTIONS);
        // Each on a bit of its own in the wearer's choices (`option_bit` hashes the id):
        // a name that lands on a taken bit gets a number.
        let mut taken = 0u64;
        for option in &mut options {
            let base = option.id.clone();
            let mut n = 2;
            while taken & crate::blade_skin_file::extras::option_bit(&option.id) != 0 {
                option.id = format!("{}_{n}", &base[..base.len().min(21)]);
                n += 1;
            }
            taken |= crate::blade_skin_file::extras::option_bit(&option.id);
        }
        options
    }

    /// The names of its own parts, from its `// @part` lines in order.
    fn parts(&self) -> impl Iterator<Item = &'static str> {
        self.wgsl
            .lines()
            .take_while(|line| line.starts_with("//"))
            .filter_map(|line| line.strip_prefix("// @part "))
            .map(str::trim)
    }

    /// Give `def` (a pack's file for this skin, or its own) the built-in file's sounds,
    /// heat haze, wisps, embers, echo, glints, veins and afterimages: the signature's own, which an older pack file of the
    /// skin (from before it was a signature) must not change.
    pub(crate) fn adopt(&self, def: &mut BladeSkinDef) {
        if let Ok(own) = crate::blade_skin_file::parse(self.id, self.file) {
            def.sounds = own.sounds;
            def.haze = own.haze;
            def.wisps = own.wisps;
            def.embers = own.embers;
            def.echo = own.echo;
            def.star = own.star;
            def.orbit = own.orbit;
            def.veins = own.veins;
            def.ghosts = own.ghosts;
            def.arcs = own.arcs;
            def.motes = own.motes;
            def.hue = own.hue;
        }
    }

    /// The lab's blade-skin file for it, for when no pack carries one.
    pub(crate) fn default_file(&self) -> &'static str {
        self.file
    }

    /// The shader its pipelines are built from: `saber.wgsl` with its own signature.
    pub(crate) fn module_source(&self) -> String {
        module_source(include_str!("saber.wgsl"), self.wgsl)
    }
}

/// `saber` with its stand-in signature replaced by the lab's helpers and `signature`.
fn module_source(saber: &str, signature: &str) -> String {
    let begin = saber.find(BEGIN).expect("saber.wgsl marks its signature");
    let end = saber[begin..]
        .find(END)
        .expect("saber.wgsl ends its signature")
        + begin;
    let mut source = String::with_capacity(saber.len() + LAB.len() + signature.len() + 2);
    source.push_str(&saber[..begin + BEGIN.len()]);
    source.push_str(LAB);
    source.push('\n');
    source.push_str(signature);
    // A signature without one of the three keeps the stand-in's.
    let stand_in = &saber[begin + BEGIN.len()..end];
    for function in stand_in.split("\n\n") {
        let name = function
            .trim_start()
            .strip_prefix("fn ")
            .and_then(|rest| rest.split('(').next());
        if let Some(name) = name
            && !signature.contains(&format!("fn {name}("))
        {
            source.push('\n');
            source.push_str(function.trim_end());
            source.push('\n');
        }
    }
    source.push_str(&saber[end..]);
    source
}

/// Every signature's module, validated once (for the tests and the log).
#[cfg(test)]
fn modules() -> &'static Vec<(&'static str, String)> {
    use std::sync::OnceLock;
    static MODULES: OnceLock<Vec<(&'static str, String)>> = OnceLock::new();
    MODULES.get_or_init(|| {
        SIGNATURES
            .iter()
            .map(|signature| (signature.id, signature.module_source()))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::naga;

    #[test]
    fn every_signature_is_a_blade_skin_of_the_catalogue_once() {
        for (index, signature) in SIGNATURES.iter().enumerate() {
            assert!(
                crate::unlockables::blade_skin(signature.id).is_some(),
                "{} is not a blade skin",
                signature.id
            );
            assert!(
                SIGNATURES[..index]
                    .iter()
                    .all(|other| other.id != signature.id),
                "{} twice",
                signature.id
            );
        }
    }

    #[test]
    fn every_signature_module_validates() {
        for (id, source) in modules() {
            let module = naga::front::wgsl::parse_str(source)
                .unwrap_or_else(|error| panic!("{id}: {}", error.emit_to_string(source)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::default(),
            )
            .validate(&module)
            .unwrap_or_else(|error| panic!("{id}: {}", error.emit_to_string(source)));
        }
    }

    #[test]
    fn every_signature_has_its_room_and_its_own_three_passes() {
        for signature in &SIGNATURES {
            let room = signature.room();
            assert!(
                (0.0..=10.0).contains(&room),
                "{}: room {room}",
                signature.id
            );
            assert!(signature.wgsl.contains("fn sig_glow("), "{}", signature.id);
            assert_eq!(
                signature.solid(),
                signature.wgsl.contains("fn sig_solid(")
                    || signature.wgsl.contains("fn sig_vol_solid(")
                    || signature.wgsl.contains("fn sig_orbit_solid("),
                "{}: @solid and sig_solid disagree",
                signature.id
            );
        }
        let star_dragon = of("saber_star_dragon").unwrap();
        assert_eq!(star_dragon.room(), 10.0);
        assert_eq!(star_dragon.lane()[0], NUMBER as f32);
        assert!(
            include_str!("saber.wgsl").contains(&format!("const LAB_SIGNATURE: u32 = {NUMBER}u;"))
        );
    }

    #[test]
    fn a_module_keeps_the_stand_ins_the_signature_has_not() {
        let saber = include_str!("saber.wgsl");
        let source = module_source(
            saber,
            "fn sig_glow(g: vec3<f32>, b: vec2<f32>, r: f32, px: f32) -> vec3<f32> {\n    return g * 2.0;\n}\n",
        );
        assert_eq!(source.matches("fn sig_glow(").count(), 1);
        assert_eq!(source.matches("fn sig_core(").count(), 1);
        assert_eq!(source.matches("fn sig_solid(").count(), 1);
        assert!(source.contains("return g * 2.0;"));
        assert!(source.contains("fn h11("));
    }

    #[test]
    fn every_default_file_reads_as_a_blade_skin() {
        for signature in &SIGNATURES {
            crate::blade_skin_file::parse(signature.id, signature.default_file())
                .unwrap_or_else(|why| panic!("{}: {why}", signature.id));
        }
    }

    #[test]
    fn every_signature_offers_its_options_on_their_own_bits() {
        use crate::blade_skin_file::extras::{Section, option_bit};
        let signature = of("saber_wraithlight").unwrap();
        let def = crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
        let shown = signature.shown(&def);
        let ids: Vec<&str> = shown
            .options
            .iter()
            .map(|option| option.id.as_str())
            .collect();
        // Its own part first, then Sol's sections its file has, then the bursts.
        assert_eq!(
            ids,
            [
                "mist_and_echoes",
                "souls",
                "afterimages",
                "spirit_mist",
                "echo",
                "swing_and_clash_bursts"
            ]
        );
        let bits: Vec<u64> = ids.iter().map(|id| option_bit(id)).collect();
        for (index, bit) in bits.iter().enumerate() {
            assert!(!bits[..index].contains(bit), "{} shares a bit", ids[index]);
        }
        // All off: the sections the shader reads them from.
        let all = bits.iter().fold(0, |all, bit| all | bit);
        let mask = shown.off_mask(all);
        for section in [
            Section::Sputter,
            Section::Motes,
            Section::Ghosts,
            Section::Wisps,
            Section::Echo,
            Section::Pulse,
        ] {
            assert_ne!(mask & section.bit(), 0, "{section:?}");
        }
        // Its own part 0 rides on the first part bit (`SIG_PART_BITS[0]` in saber.wgsl).
        assert_eq!(
            shown.off_mask(option_bit("mist_and_echoes")),
            Section::Sputter.bit()
        );
        // Every signature's options sit on bits of their own.
        for signature in &SIGNATURES {
            let def =
                crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
            let bits: Vec<u64> = signature
                .shown(&def)
                .options
                .iter()
                .map(|option| option_bit(&option.id))
                .collect();
            for (index, bit) in bits.iter().enumerate() {
                assert!(
                    !bits[..index].contains(bit),
                    "{}: options share a bit",
                    signature.id
                );
            }
        }
    }

    /// The WGSL is the script's: none edited by hand, none left behind.
    #[test]
    fn every_signature_has_its_lab_source() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/blade_signatures");
        for signature in &SIGNATURES {
            let name = signature.id.trim_start_matches("saber_");
            let glsl = std::fs::read_to_string(folder.join(format!("{name}.glsl")))
                .unwrap_or_else(|why| panic!("{name}.glsl: {why}"));
            assert!(glsl.contains("sig_glow("), "{name}.glsl");
            assert!(signature.wgsl.starts_with(&format!(
                "// Generated by scripts/blade_signatures.py from {name}.glsl"
            )));
        }
        let wgsl = std::fs::read_dir(&folder)
            .unwrap()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| name.ends_with(".wgsl") && name != "lab.wgsl")
            .count();
        assert_eq!(wgsl, SIGNATURES.len(), "a signature not in the table");
    }
}

#[cfg(test)]
#[path = "blade_signatures_gpu_test.rs"]
mod gpu_test;

#[cfg(test)]
#[path = "blade_views_gpu_test.rs"]
mod views_test;
