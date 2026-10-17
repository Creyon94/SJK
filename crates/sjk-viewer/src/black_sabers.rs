//! JA+/jaPRO black blades: JoF EJK #50 (`4ebc7e7e`, `ClampSaberColor`).
//! On a plugin server a set plugin bit (or `cg_blackSabers 0`) turns black into
//! orange; elsewhere colour 11 keeps the client's ordinary fallback.
//! Raw colour numbers stay in this adapter rather than the runtime world.

use super::BladeColor;
use sjk_client::{CompatProfile, LegacyClientInfo};
use sjk_protocol::{GameState, LEGACY_MAX_CLIENTS};

// Lowercase for allocation-free registry reads in the frame path.
pub(crate) const CVAR: &str = "cg_blacksabers";
const DISABLE_BIT: i64 = 1 << 3;

/// Servers with the black-blade contract, told by the session's
/// [`CompatProfile`] (its documented serverinfo table) rather than a second one:
/// EJK #50 permits JA+ and its jaPRO lineage, and TaystJK inherits jaPRO's colour
/// enumeration.
fn supports_black(profile: &CompatProfile) -> bool {
    matches!(
        profile,
        CompatProfile::JaPlus { .. } | CompatProfile::TaystJk
    )
}

/// The serverinfo and clientinfo part of the policy, rebuilt only after
/// `CS_SERVERINFO` or a `CS_PLAYERS` slot changed ([`Self::invalidate`]).
#[derive(Clone, Copy, Default)]
pub(crate) struct Cache {
    valid: bool,
    supported: bool,
    black: [u32; 2],
}

impl Cache {
    /// Called by the configstring dispatch for slots 0 and `CS_PLAYERS`.
    pub(crate) fn invalidate(&mut self) {
        self.valid = false;
    }

    /// The frame's policy; the cvars and mutes are read each frame, the
    /// configstrings only when they changed.
    pub(crate) fn policy(
        &mut self,
        game: Option<&GameState>,
        enabled: bool,
        disabled: i64,
        muted: u32,
    ) -> Policy {
        let Some(game) = game else {
            return Policy::default();
        };
        if !self.valid {
            *self = Self::scan(game);
        }
        Policy {
            black: self.black,
            supported: self.supported,
            allowed: self.supported && enabled && disabled & DISABLE_BIT == 0,
            muted,
        }
    }

    fn scan(game: &GameState) -> Self {
        let mut cache = Self {
            valid: true,
            supported: supports_black(&CompatProfile::from_game_state(game)),
            black: [0; 2],
        };
        for client in 0..LEGACY_MAX_CLIENTS {
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
                    cache.black[hand] |= 1 << client;
                }
            }
        }
        cache
    }
}

/// One frame's black-blade decisions, reused by held and flying blades.
#[derive(Clone, Copy, Default)]
pub(crate) struct Policy {
    black: [u32; 2],
    supported: bool,
    allowed: bool,
    muted: u32,
}

impl Policy {
    /// Uncached, for tests and one-off callers.
    #[cfg(test)]
    pub(crate) fn new(game: Option<&GameState>, enabled: bool, disabled: i64, muted: u32) -> Self {
        Cache::default().policy(game, enabled, disabled, muted)
    }

    /// This policy with entity `entity`'s black blade dropped from each hand where
    /// `replaced` holds (a team colour forced over it, `team_sabers.rs`).
    pub(crate) fn without(mut self, entity: u64, replaced: [bool; 2]) -> Self {
        if let Some(client) = entity
            .checked_sub(1)
            .filter(|client| *client < LEGACY_MAX_CLIENTS as u64)
        {
            for (bits, replaced) in self.black.iter_mut().zip(replaced) {
                if replaced {
                    *bits &= !(1 << client);
                }
            }
        }
        self
    }

    pub(crate) fn color(self, entity: u64, hand: usize, fallback: BladeColor) -> BladeColor {
        let Some(client) = entity
            .checked_sub(1)
            .filter(|client| *client < LEGACY_MAX_CLIENTS as u64)
        else {
            return fallback;
        };
        if self.muted & (1 << client) != 0 || !self.supported {
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
