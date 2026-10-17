//! JA+/jaPRO black blades: JoF EJK #50 (`4ebc7e7e`, `ClampSaberColor`).
//! A set plugin bit disables black; unsupported/disabled blades become orange.
//! Raw colour numbers stay in this adapter rather than the runtime world.

use super::BladeColor;
use sjk_client::LegacyClientInfo;
use sjk_protocol::GameState;

// Lowercase for allocation-free registry reads in the frame path.
pub(crate) const CVAR: &str = "cg_blacksabers";
const DISABLE_BIT: i64 = 1 << 3;

/// Server identifiers advertising the black-blade contract, also recognized by
/// `sjk_client::CompatProfile`. EJK #50 permits JA+ and its jaPRO lineage;
/// TaystJK inherits jaPRO's colour enumeration. Generic code reads these rows.
const CONTRACTS: &[(&str, &str)] = &[
    ("gamename", "ja+"),
    ("gamename", "japlus"),
    ("gamename", "japro"),
    ("gamename", "taystjk"),
    ("fs_game", "japlus"),
    ("game", "japlus"),
    ("fs_game", "japro"),
    ("game", "japro"),
    ("fs_game", "taystjk"),
    ("game", "taystjk"),
    ("version", "ja+"),
    ("version", "japlus"),
    ("version", "taystjk"),
];

/// One bounded scan per presented frame, reused by held and flying blades.
#[derive(Clone, Copy, Default)]
pub(crate) struct Policy {
    black: [u64; 2],
    allowed: bool,
    muted: u32,
}

impl Policy {
    pub(crate) fn new(game: Option<&GameState>, enabled: bool, disabled: i64, muted: u32) -> Self {
        let Some(game) = game else {
            return Self::default();
        };
        let supported = game.config_string(0).is_some_and(|raw| {
            let info = LegacyClientInfo::new(raw);
            CONTRACTS.iter().any(|(key, token)| {
                info.bytes(key).is_some_and(|value| {
                    value
                        .windows(token.len())
                        .any(|part| part.eq_ignore_ascii_case(token.as_bytes()))
                })
            })
        });
        let mut policy = Self {
            allowed: supported && enabled && disabled & DISABLE_BIT == 0,
            muted,
            ..Self::default()
        };
        for client in 0..64 {
            let Some(raw) = game.config_string(1131 + client) else {
                continue;
            };
            let info = LegacyClientInfo::new(raw);
            for (hand, key) in ["c1", "c2"].into_iter().enumerate() {
                let color = info
                    .text(key)
                    .map(sjk_client::split_color_value)
                    .map(|value| value.0);
                if color.is_some_and(|color| color >= 0 && color % 12 == 11) {
                    policy.black[hand] |= 1 << client;
                }
            }
        }
        policy
    }

    pub(crate) fn color(self, entity: u64, hand: usize, fallback: BladeColor) -> BladeColor {
        let Some(client) = entity.checked_sub(1).filter(|client| *client < 64) else {
            return fallback;
        };
        if client < 32 && self.muted & (1 << client) != 0 {
            return fallback;
        }
        if self
            .black
            .get(hand)
            .is_none_or(|bits| bits & (1 << client) == 0)
        {
            return fallback;
        }
        if self.allowed {
            BladeColor::Black
        } else {
            BladeColor::Retail(crate::saber::Color::Orange)
        }
    }
}

#[cfg(test)]
#[path = "black_saber_tests.rs"]
mod tests;

/// Prefer installed community textures; no community asset is bundled or downloaded.
/// Fallback coverage uses the engine's neutral profiles, with black RGB and alpha.
pub(crate) fn image(
    vfs: &sjk_vfs::VirtualFileSystem,
    shaders: &sjk_shader::ShaderCatalog,
    kind: &str,
) -> image::RgbaImage {
    let name = format!("gfx/effects/sabers/black{kind}");
    if let Ok(image) = crate::shader_image::load_shader_image(vfs, shaders, &name) {
        return image;
    }
    let (size, mut pixels) = if kind == "glow" {
        (super::glow_size(), super::neutral_glow())
    } else {
        (super::core_size(), super::neutral_core())
    };
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = pixel[0];
        pixel.copy_from_slice(&[0, 0, 0, alpha]);
    }
    image::RgbaImage::from_raw(size[0], size[1], pixels).expect("generated RGBA profile")
}
