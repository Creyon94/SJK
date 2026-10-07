//! The console font: JetBrains Mono, bundled and rasterized once like Inter.
//!
//! It replaces the retail console character set (`gfx/2d/charsgrid_med`), whose
//! 8x16-texel bitmap glyphs looked heavy and blocky once magnified for 1440p and
//! 4K (SJK draws no bitmap fonts; docs/sjk.md "Fonts"). The console and its
//! notify lines, and what the cgame drew with `CG_DrawBigString`,
//! `CG_DrawSmallString` or `CG_DrawStringExt`, draw with it.
//!
//! The font keeps the character set's layout contract, so those surfaces lay out
//! exactly as before: a line is [`HEIGHT`] units tall and every character advances
//! [`WIDTH`] units, half the height (`SMALLCHAR_WIDTH` 8 by `SMALLCHAR_HEIGHT` 16).
//! The em is sized so JetBrains Mono's advance fills the cell, and the baseline
//! centres the font's ascent and descent in it. Slots hold the Windows-1252
//! character of their byte, as the character set did; bytes the font lacks draw `.`.

use super::{
    ATLAS_PADDING, ATLAS_WIDTH, FontAtlas, FontGlyph, GLYPH_COUNT, RasterizedGlyph, TextStyle,
    UiFont, pack_glyphs, slot_character,
};
use fontdue::{Font, FontSettings};
use image::{Rgba, RgbaImage};
use std::error::Error;

/// Name used in the log.
pub(crate) const NAME: &str = "JetBrains Mono";
const JETBRAINS_MONO: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
/// Line height in layout units, as `SMALLCHAR_HEIGHT`.
pub(crate) const HEIGHT: f32 = 16.0;
/// Character advance in layout units, as `SMALLCHAR_WIDTH`.
pub(crate) const WIDTH: f32 = HEIGHT * 0.5;
/// Raster em size in pixels: a 4K console cell at `con_scale 3` (an em of about
/// 80 pixels) is still drawn from the full-size atlas, and the mip chain covers
/// 1080p cells down to about 12 pixels per em.
const RASTER_EM: f32 = 96.0;

/// Rasterize the console font into a coverage atlas with mipmappable padding.
pub(crate) fn load() -> Result<FontAtlas, Box<dyn Error>> {
    let font = Font::from_bytes(JETBRAINS_MONO, FontSettings::default())?;
    let line = font
        .horizontal_line_metrics(RASTER_EM)
        .ok_or("JetBrains Mono has no horizontal line metrics")?;
    let advance = font.metrics('0', RASTER_EM).advance_width;
    if advance <= 0.0 {
        return Err("JetBrains Mono has no advance for '0'".into());
    }
    // Layout units per raster pixel: the advance fills one cell's width.
    let unit = WIDTH / advance;
    // The baseline that centres the ascent and descent in the cell.
    let baseline = (HEIGHT + (line.ascent + line.descent) * unit) * 0.5;
    let rasterized: Vec<RasterizedGlyph> = (0..GLYPH_COUNT)
        .map(|byte| {
            let (metrics, pixels) = font.rasterize(slot_character(&font, byte as u8), RASTER_EM);
            RasterizedGlyph {
                face: 0,
                byte,
                metrics,
                pixels,
            }
        })
        .collect();
    let placements = pack_glyphs(&rasterized);
    let height = placements
        .iter()
        .zip(&rasterized)
        .map(|([_, y], glyph)| y + glyph.metrics.height as u32 + ATLAS_PADDING)
        .max()
        .unwrap_or(1)
        .next_power_of_two();
    // White RGB under transparent texels, as the Inter atlas (`load_modern`).
    let mut image = RgbaImage::from_pixel(ATLAS_WIDTH, height, Rgba([255, 255, 255, 0]));
    let mut glyphs = [[FontGlyph::default(); GLYPH_COUNT]; 2];
    for (glyph, [x, y]) in rasterized.iter().zip(placements) {
        let metrics = glyph.metrics;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let alpha = glyph.pixels[row * metrics.width + column];
                image.put_pixel(
                    x + column as u32,
                    y + row as u32,
                    Rgba([255, 255, 255, alpha]),
                );
            }
        }
        let ink_top = metrics.ymin as f32 + metrics.height as f32;
        let entry = FontGlyph {
            width: metrics.width as f32 * unit,
            height: metrics.height as f32 * unit,
            advance: WIDTH,
            offset_x: metrics.xmin as f32 * unit,
            offset_y: baseline - ink_top * unit,
            uv: [
                x as f32 / ATLAS_WIDTH as f32,
                y as f32 / height as f32,
                (x + metrics.width as u32) as f32 / ATLAS_WIDTH as f32,
                (y + metrics.height as u32) as f32 / height as f32,
            ],
        };
        glyphs[0][glyph.byte] = entry;
        glyphs[1][glyph.byte] = entry;
    }
    Ok(FontAtlas {
        font: UiFont {
            glyphs,
            height: HEIGHT,
            modern: false,
            style: TextStyle::NEUTRAL,
        },
        image,
        distance_field: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TextFace;

    #[test]
    fn every_character_advances_one_cell_and_ink_stays_in_its_column() {
        let atlas = load().unwrap();
        let font = &atlas.font;
        assert_eq!(font.height, HEIGHT);
        assert!(!font.is_modern());
        for byte in 0..=255_u8 {
            let glyph = font.glyph(TextFace::Regular, byte);
            assert_eq!(glyph.advance, WIDTH, "{byte}");
            if glyph.width > 0.0 {
                assert!(glyph.offset_x >= -0.5, "{byte} {glyph:?}");
                assert!(
                    glyph.offset_x + glyph.width <= WIDTH + 0.5,
                    "{byte} {glyph:?}"
                );
            }
        }
        let width = crate::text::visible_text_width(font, "^1ab c", 2.0);
        assert_eq!(width, 4.0 * WIDTH * 2.0);
    }

    #[test]
    fn capitals_and_descenders_fit_the_line() {
        let font = load().unwrap().font;
        let cap = font.glyph(TextFace::Regular, b'H');
        let descender = font.glyph(TextFace::Regular, b'g');
        assert!(cap.offset_y > 0.0 && descender.offset_y + descender.height <= HEIGHT);
        // Caps sit on the same baseline as the bottom of `x`.
        let x = font.glyph(TextFace::Regular, b'x');
        assert!((cap.offset_y + cap.height - (x.offset_y + x.height)).abs() < 0.1);
    }

    #[test]
    fn a_space_draws_nothing_and_the_atlas_keeps_its_coverage() {
        let atlas = load().unwrap();
        let space = atlas.font.glyph(TextFace::Regular, b' ');
        assert_eq!([space.width, space.height], [0.0, 0.0]);
        // Wide enough that `sdf::for_atlas` leaves it as coverage.
        assert!(atlas.image.width() >= crate::text::sdf::HD_LONG_SIDE);
        assert!(!atlas.distance_field);
    }
}
