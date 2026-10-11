//! The player card's outline: while a player's card is up
//! ([`crate::hud::player_card`]), their body is drawn once more as an inverted hull,
//! its back faces pushed out along the normals in the card's accent colour, so only a
//! rim around the silhouette shows past the body. It is depth tested like the body:
//! nothing of it shows through walls.
//!
//! A shader's push is fixed, so the bundled shader file holds [`LEVELS`] of them, a
//! factor of [`STEP`] apart; the level drawn is the one nearest a push proportional to
//! the player's distance, which keeps the rim about the same width on screen.

use sjk_vfs::{VfsError, VirtualFileSystem};

/// Pushes the shader file offers, in world units.
pub(crate) const LEVELS: usize = 10;
/// The smallest push.
const FIRST: f32 = 0.2;
/// The factor between two levels' pushes.
const STEP: f32 = 1.4;
/// Push per world unit of distance: about 2.5 pixels of a 1080-line view at the
/// default field of view.
const PUSH_PER_UNIT: f32 = 0.0028;

/// The levels' shader names, thinnest first.
pub(crate) const SHADERS: [&str; LEVELS] = [
    "sjk/cardOutline0",
    "sjk/cardOutline1",
    "sjk/cardOutline2",
    "sjk/cardOutline3",
    "sjk/cardOutline4",
    "sjk/cardOutline5",
    "sjk/cardOutline6",
    "sjk/cardOutline7",
    "sjk/cardOutline8",
    "sjk/cardOutline9",
];

/// One level: back faces only (`cull back`), pushed out by its amount (a wave with no
/// amplitude is a constant), blended by the instance's colour and alpha (the fade).
macro_rules! level {
    ($name:literal, $push:literal) => {
        concat!(
            $name,
            "\n{\n\tcull back\n\tdeformVertexes wave 100 sin ",
            $push,
            " 0 0 0\n\t{\n\t\tmap $whiteimage\n",
            "\t\tblendFunc GL_SRC_ALPHA GL_ONE_MINUS_SRC_ALPHA\n\t\tdepthWrite\n",
            "\t\trgbGen entity\n\t\talphaGen entity\n\t}\n}\n"
        )
    };
}

/// The pushes are `FIRST * STEP^level`, rounded.
const SHADER_FILE: &str = concat!(
    "// SJK: the player card's outline (card_outline.rs).\n",
    level!("sjk/cardOutline0", "0.2"),
    level!("sjk/cardOutline1", "0.28"),
    level!("sjk/cardOutline2", "0.39"),
    level!("sjk/cardOutline3", "0.55"),
    level!("sjk/cardOutline4", "0.77"),
    level!("sjk/cardOutline5", "1.08"),
    level!("sjk/cardOutline6", "1.51"),
    level!("sjk/cardOutline7", "2.11"),
    level!("sjk/cardOutline8", "2.95"),
    level!("sjk/cardOutline9", "4.13"),
);

/// Mount the shader file, below game data like SJK's other content.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), VfsError> {
    vfs.mount_memory(
        "SJK card outline",
        [("shaders/sjk_card_outline.shader", SHADER_FILE.as_bytes())],
    )?;
    Ok(())
}

/// The level for a player `distance` units from the camera.
pub(crate) fn level(distance: f32) -> usize {
    let push = (distance * PUSH_PER_UNIT).max(FIRST);
    ((push / FIRST).ln() / STEP.ln())
        .round()
        .clamp(0.0, (LEVELS - 1) as f32) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_defines_every_level_with_its_push() {
        let shaders =
            sjk_shader::parse_shader_script(SHADER_FILE.as_bytes(), "card_outline").unwrap();
        assert_eq!(shaders.len(), LEVELS);
        for (level, (name, shader)) in SHADERS.iter().zip(&shaders).enumerate() {
            assert_eq!(&shader.name, name);
            assert_eq!(shader.cull, sjk_shader::ShaderCull::Back, "{name}");
            let [sjk_shader::Deform::Wave { wave, .. }] = shader.deforms.as_slice() else {
                panic!("{name} has one wave deform");
            };
            let push = FIRST * STEP.powi(level as i32);
            assert!((wave.base - push).abs() < 0.006, "{name} pushes {push}");
            assert_eq!((wave.amplitude, wave.frequency), (0.0, 0.0));
            let [stage] = shader.stages.as_slice() else {
                panic!("{name} has one stage");
            };
            assert!(stage.depth_write, "{name}");
            assert_eq!(stage.rgb_generator.as_deref(), Some("entity"));
            assert_eq!(stage.alpha_generator.as_deref(), Some("entity"));
        }
    }

    #[test]
    fn the_push_grows_with_distance_and_stays_in_range() {
        assert_eq!(level(0.0), 0);
        assert_eq!(level(50.0), 0);
        let mut last = 0;
        for distance in (0..4_000).step_by(50) {
            let now = level(distance as f32);
            assert!(now >= last && now < LEVELS);
            last = now;
        }
        assert_eq!(level(100_000.0), LEVELS - 1);
        // About the push a distance asks for, within half a step.
        let push = FIRST * STEP.powi(level(500.0) as i32);
        assert!((push / (500.0 * PUSH_PER_UNIT)).ln().abs() <= STEP.ln() * 0.5 + 1e-4);
    }
}
