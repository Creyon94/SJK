//! One character in a fixed cell, without a shadow: the classic console's
//! monospaced grid, as `SCR_DrawSmallChar` draws it.

use super::{TextFace, TextVertex, UiFont, push_quad_within};

/// Append `byte` drawn into the cell `[x, y, width, height]` (pixels) in
/// `color`, into a batch of at most `limit` vertices. The glyph is sized to the
/// cell's height and its advance centred in the cell's width, so the console
/// font ([`super::console_font`]), whose advance is the cell, keeps its own
/// side bearings. Nothing is drawn for a glyph without ink, such as a space.
pub(crate) fn append_cell(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    byte: u8,
    cell: [f32; 4],
    color: [f32; 4],
    viewport: [f32; 2],
    limit: usize,
) {
    let glyph = font.glyph(TextFace::Regular, byte);
    if glyph.width <= 0.0 || glyph.height <= 0.0 {
        return;
    }
    let [x, y, width, height] = cell;
    let scale = height / font.height.max(1.0);
    let left = x + (width - glyph.advance * scale) * 0.5 + glyph.offset_x * scale;
    push_quad_within(
        vertices,
        [
            left,
            y + glyph.offset_y * scale,
            glyph.width * scale,
            glyph.height * scale,
        ],
        glyph.uv,
        color,
        viewport,
        limit,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_glyphs_draw_in_their_cell_without_a_shadow() {
        let font = super::super::console_font::load().unwrap().font;
        let mut vertices = Vec::new();
        append_cell(
            &mut vertices,
            &font,
            b'A',
            [16.0, 32.0, 16.0, 32.0],
            [1.0; 4],
            [640.0, 480.0],
            usize::MAX,
        );
        // One quad: no shadow quad under it.
        assert_eq!(vertices.len(), 6);
        append_cell(
            &mut vertices,
            &font,
            b' ',
            [32.0, 32.0, 16.0, 32.0],
            [1.0; 4],
            [640.0, 480.0],
            usize::MAX,
        );
        assert_eq!(vertices.len(), 6);
        // The limit is respected.
        append_cell(
            &mut vertices,
            &font,
            b'B',
            [32.0, 32.0, 16.0, 32.0],
            [1.0; 4],
            [640.0, 480.0],
            8,
        );
        assert_eq!(vertices.len(), 6);
    }
}
