//! SJK's emblem on the main menus: the gold starburst with the JK blade.
//!
//! The classic main page shows it in the ring, where retail played its
//! `video/ja01` logo, and the modern main page above its title. It is bundled
//! (`assets/branding`, made by `scripts/sjk_branding.py`), so it shows with
//! or without the player's retail artwork.
//!
//! The emblem is three pictures drawn over each other at the same place
//! ([`EmblemLayer`]): the still emblem, then two additive glow layers, the
//! orange core and ring and the cyan blade lights, whose strengths follow the
//! menu clock ([`motion::emblem_core_glow`], [`motion::emblem_lights_glow`]).
//! The renderer blends the glow layers additively, as light, so the core
//! brightens towards yellow instead of only covering what is under it. The
//! pictures are in display values, like all 2D art (the UI colour model in
//! `docs/rendering.md`).
//!
//! They are decoded once per process on a worker thread ([`request`]), with
//! their mip chains, so the emblem stays smooth at 1080p, where the 1024
//! picture is drawn at under half size, and sharp at 4K.

use super::art::motion;
use crate::menu_widgets::MenuCanvas;
use image::RgbaImage;
use sjk_ui::{Color, DrawCommand, Rect, TextureId};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// First `TextureId` naming a layer: clear of the atlas cells, the classic
/// art's range (`0x4000_0000` on) and the reserved ids at the top.
const TEXTURE_BASE: u32 = 0x5000_0000;

/// Where the emblem sits on the classic main page's 640x480 canvas
/// (`[x, y, width, height]`). The centre window's opening in retail's
/// `main.menu` art spans x 231.5 to 409 and y 176 to 370 (centre 320.5, 273),
/// round at the top and bottom and cut straight by the side windows; the
/// emblem is centred on it and 176 units across, inside the reticle's outer
/// circle, so its spikes end at the straight sides and its blade short of the
/// rim. The canvas is scaled uniformly into the window, so this holds for
/// every aspect ratio and resolution.
pub(crate) const CLASSIC_RING: [f32; 4] = [232.5, 185.0, 176.0, 176.0];

/// One picture of the emblem, in draw order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EmblemLayer {
    /// The emblem itself, alpha blended.
    Base,
    /// The orange core and ring, added as light.
    Core,
    /// The cyan lights on the blade, added as light.
    Lights,
}

impl EmblemLayer {
    /// Every layer, in draw order.
    pub(crate) const ALL: [Self; 3] = [Self::Base, Self::Core, Self::Lights];

    /// The `TexturedQuad` texture that draws this layer.
    pub(crate) fn texture(self) -> TextureId {
        TextureId(TEXTURE_BASE + self as u32)
    }

    /// The layer a `TexturedQuad` texture names, if it names one.
    pub(crate) fn from_texture(texture: TextureId) -> Option<Self> {
        let index = texture.0.checked_sub(TEXTURE_BASE)?;
        Self::ALL.get(index as usize).copied()
    }

    /// Position in [`Self::ALL`].
    pub(crate) fn index(self) -> usize {
        self as usize
    }

    /// Whether the layer adds light (its black adds nothing) instead of
    /// covering what is under it.
    pub(crate) fn additive(self) -> bool {
        self != Self::Base
    }

    /// The bundled PNG.
    fn png(self) -> &'static [u8] {
        match self {
            Self::Base => include_bytes!("../../../../assets/branding/sjk-logo.png"),
            Self::Core => include_bytes!("../../../../assets/branding/emblem-core.png"),
            Self::Lights => include_bytes!("../../../../assets/branding/emblem-lights.png"),
        }
    }

    /// The layer's strength at menu time `seconds`: the emblem is steady,
    /// the glows move.
    pub(crate) fn strength(self, seconds: f64) -> f32 {
        match self {
            Self::Base => 1.0,
            Self::Core => motion::emblem_core_glow(seconds),
            Self::Lights => motion::emblem_lights_glow(seconds),
        }
    }
}

/// Draw the emblem over window rectangle `rect` as it looks at menu time
/// `seconds`. Nothing shows until the renderer has the pictures.
pub(crate) fn draw(canvas: &mut MenuCanvas, rect: Rect, seconds: f64) {
    let draw = canvas.draw_list_mut();
    for layer in EmblemLayer::ALL {
        let _ = draw.push(DrawCommand::TexturedQuad {
            rect,
            texture: layer.texture(),
            color: Color::new(1.0, 1.0, 1.0, layer.strength(seconds)),
        });
    }
}

/// One decoded layer: a square picture with its mip chain.
pub(crate) struct MipChain {
    /// Edge of level 0.
    pub(crate) size: u32,
    /// Tightly packed RGBA rows of each level, largest first.
    pub(crate) levels: Vec<Vec<u8>>,
}

impl MipChain {
    /// Build the chain of `image` down to one texel. Each level averages
    /// 2x2 texels weighted by their alpha, so the transparent (black)
    /// surroundings of the emblem do not darken its edges as it shrinks.
    pub(crate) fn new(image: RgbaImage) -> Self {
        let size = image.width();
        let mut levels = vec![image.into_raw()];
        let mut edge = size;
        while edge > 1 {
            let next = (edge / 2).max(1);
            let level = halve(levels.last().map_or(&[][..], Vec::as_slice), edge, next);
            levels.push(level);
            edge = next;
        }
        Self { size, levels }
    }
}

/// The `next`-square level below the `edge`-square RGBA `pixels`.
fn halve(pixels: &[u8], edge: u32, next: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((next * next * 4) as usize);
    let texel = |x: u32, y: u32| {
        let at = ((y.min(edge - 1) * edge + x.min(edge - 1)) * 4) as usize;
        &pixels[at..at + 4]
    };
    for y in 0..next {
        for x in 0..next {
            let mut colour = [0_u32; 3];
            let mut alpha = 0_u32;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let source = texel(x * 2 + dx, y * 2 + dy);
                let weight = u32::from(source[3]);
                for (sum, channel) in colour.iter_mut().zip(source) {
                    *sum += u32::from(*channel) * weight;
                }
                alpha += weight;
            }
            // Fully transparent texels stay black.
            for sum in colour {
                out.push((sum + alpha / 2).checked_div(alpha).unwrap_or(0) as u8);
            }
            out.push(((alpha + 2) / 4) as u8);
        }
    }
    out
}

/// The decoded layers, by [`EmblemLayer::index`].
pub(crate) struct Decoded {
    layers: [Option<MipChain>; 3],
}

impl Decoded {
    /// The picture of `layer`, if it decoded.
    pub(crate) fn layer(&self, layer: EmblemLayer) -> Option<&MipChain> {
        self.layers[layer.index()].as_ref()
    }
}

static DECODED: OnceLock<Decoded> = OnceLock::new();
static REQUESTED: AtomicBool = AtomicBool::new(false);

/// Start decoding the emblem on a worker thread, once per process; later
/// calls do nothing.
pub(crate) fn request() {
    if REQUESTED.swap(true, Ordering::AcqRel) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("sjk-menu-emblem".into())
        .spawn(|| {
            let _ = DECODED.set(decode_all());
        });
    if let Err(error) = spawned {
        crate::log::progress(format_args!(
            "warning: menu emblem not loaded, drawing without it: {error}"
        ));
    }
}

/// The decoded layers, once the worker has finished.
pub(crate) fn decoded() -> Option<&'static Decoded> {
    DECODED.get()
}

fn decode_all() -> Decoded {
    Decoded {
        layers: EmblemLayer::ALL.map(|layer| match decode(layer) {
            Ok(chain) => Some(chain),
            Err(error) => {
                crate::log::progress(format_args!("warning: menu emblem {layer:?}: {error}"));
                None
            }
        }),
    }
}

fn decode(layer: EmblemLayer) -> Result<MipChain, String> {
    let image = image::load_from_memory_with_format(layer.png(), image::ImageFormat::Png)
        .map_err(|error| error.to_string())?
        .into_rgba8();
    let (width, height) = image.dimensions();
    if width != height || width == 0 {
        return Err(format!("{width}x{height} is not square"));
    }
    Ok(MipChain::new(image))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::art::ArtPiece;
    use crate::menu::classic::layout::{CANVAS, Page, Placement};

    #[test]
    fn textures_round_trip_and_stay_clear_of_other_ids() {
        for layer in EmblemLayer::ALL {
            assert_eq!(EmblemLayer::from_texture(layer.texture()), Some(layer));
            assert_eq!(ArtPiece::from_texture(layer.texture()), None);
            assert_eq!(EmblemLayer::ALL[layer.index()], layer);
        }
        for piece in ArtPiece::ALL {
            assert_eq!(EmblemLayer::from_texture(piece.texture()), None);
        }
        for id in [0, 1, u32::MAX, u32::MAX - 1, TEXTURE_BASE + 3] {
            assert_eq!(EmblemLayer::from_texture(TextureId(id)), None);
        }
        assert!(!EmblemLayer::Base.additive());
        assert!(EmblemLayer::Core.additive() && EmblemLayer::Lights.additive());
    }

    #[test]
    fn bundled_layers_decode_with_full_mip_chains() {
        let decoded = decode_all();
        for (layer, size) in EmblemLayer::ALL.into_iter().zip([1_024, 512, 512]) {
            let chain = decoded.layer(layer).expect("bundled layer decodes");
            assert_eq!(chain.size, size, "{layer:?}");
            assert_eq!(chain.levels.len() as u32, size.ilog2() + 1, "{layer:?}");
            for (level, pixels) in chain.levels.iter().enumerate() {
                let edge = (size >> level).max(1);
                assert_eq!(
                    pixels.len(),
                    (edge * edge * 4) as usize,
                    "{layer:?} {level}"
                );
            }
        }
        // The emblem is cut out: transparent corners, an opaque centre.
        let base = decoded.layer(EmblemLayer::Base).expect("emblem");
        let texel = |x: usize, y: usize| &base.levels[0][(y * 1_024 + x) * 4..][..4];
        assert_eq!(texel(0, 0)[3], 0);
        assert_eq!(texel(512, 512)[3], 255);
        // The glows add light: black outside the emblem, bright somewhere.
        for layer in [EmblemLayer::Core, EmblemLayer::Lights] {
            let pixels = &decoded.layer(layer).expect("glow").levels[0];
            assert_eq!(&pixels[..3], &[0, 0, 0], "{layer:?}");
            assert!(
                pixels
                    .chunks(4)
                    .any(|rgba| rgba[..3].iter().any(|c| *c > 200))
            );
        }
    }

    #[test]
    fn mips_keep_edge_colour_next_to_transparency() {
        // One opaque orange texel beside three transparent black ones.
        let mut image = RgbaImage::new(2, 2);
        image.put_pixel(0, 0, image::Rgba([240, 120, 20, 255]));
        let chain = MipChain::new(image);
        assert_eq!(chain.levels.len(), 2);
        assert_eq!(chain.levels[1], vec![240, 120, 20, 64]);
        // All transparent stays transparent black.
        let chain = MipChain::new(RgbaImage::new(4, 4));
        assert!(chain.levels.iter().flatten().all(|byte| *byte == 0));
        assert_eq!(chain.levels.len(), 3);
    }

    #[test]
    fn glow_strengths_follow_the_menu_clock() {
        for seconds in [0.0, 0.7, 2.1, 100.3] {
            assert_eq!(EmblemLayer::Base.strength(seconds), 1.0);
            assert_eq!(
                EmblemLayer::Core.strength(seconds),
                motion::emblem_core_glow(seconds)
            );
            assert_eq!(
                EmblemLayer::Lights.strength(seconds),
                motion::emblem_lights_glow(seconds)
            );
        }
        let mut canvas = MenuCanvas::new();
        canvas.begin_transparent([1920.0, 1080.0]);
        draw(&mut canvas, Rect::new(10.0, 20.0, 300.0, 300.0), 1.0);
        let quads: Vec<(TextureId, f32)> = canvas
            .draw_list()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::TexturedQuad { texture, color, .. } => Some((*texture, color.a)),
                _ => None,
            })
            .collect();
        assert_eq!(quads.len(), 3);
        assert_eq!(quads[0], (EmblemLayer::Base.texture(), 1.0));
        assert_eq!(quads[1].0, EmblemLayer::Core.texture());
        assert_eq!(quads[2].0, EmblemLayer::Lights.texture());
    }

    #[test]
    fn classic_emblem_sits_in_the_ring_clear_of_the_entries() {
        let [x, y, width, height] = CLASSIC_RING;
        assert_eq!(width, height, "the emblem is square");
        // Centred on the centre window's opening, inside retail's ring.
        assert!((x + width * 0.5 - 320.5).abs() < 0.01);
        assert!((y + height * 0.5 - 273.0).abs() < 0.01);
        let ring = [193.0, 145.0, 256.0, 256.0];
        assert!(x >= ring[0] && y >= ring[1]);
        assert!(x + width <= ring[0] + ring[2] && y + height <= ring[1] + ring[3]);
        // Within the opening's straight sides and its round top and bottom.
        assert!(x >= 231.5 && x + width <= 409.0);
        assert!(y >= 176.0 && y + height <= 370.0);
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [2_560.0, 1_080.0],
            [1_440.0, 1_080.0],
            [1_280.0, 1_024.0],
            [800.0, 600.0],
        ] {
            let place = Placement::new(viewport);
            let rect = place.rect(CLASSIC_RING);
            assert!((rect.width - rect.height).abs() < 1e-3, "{viewport:?}");
            assert!(rect.x >= 0.0 && rect.right() <= viewport[0], "{viewport:?}");
            assert!(
                rect.y >= 0.0 && rect.bottom() <= viewport[1],
                "{viewport:?}"
            );
            // Centred horizontally in the window, as the ring is.
            let centre = rect.x + rect.width * 0.5;
            let canvas_centre = place.rect([0.0, 0.0, CANVAS[0], CANVAS[1]]);
            let expected = canvas_centre.x + 320.5 * place.scale;
            assert!((centre - expected).abs() < 1e-2, "{viewport:?}");
            for slot in Page::Main.slots() {
                let target = place.rect(slot.target());
                let overlaps = rect.x < target.right()
                    && target.x < rect.right()
                    && rect.y < target.bottom()
                    && target.y < rect.bottom();
                assert!(!overlaps, "{viewport:?}: {:?}", slot.entry);
            }
        }
        // At 4K the 1024 picture is drawn near its own size, not magnified
        // far past it.
        let at_4k = Placement::new([3_840.0, 2_160.0]).rect(CLASSIC_RING);
        assert!(at_4k.width > 700.0 && at_4k.width < 1_024.0 * 1.1);
    }
}
