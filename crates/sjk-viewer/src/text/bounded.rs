//! Retained text stays inside its layout rectangle, including glyph shadows.
use super::*;
use sjk_ui::{Rect, TextOverflow};

/// Submit bounded text without letting the legacy viewport wrapping escape a row.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_bounded(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    value: &str,
    origin: [f32; 2],
    rect: Rect,
    clip: Rect,
    scale: f32,
    viewport: [f32; 2],
    face: TextFace,
    base_color: [f32; 4],
    spacing: f32,
    overflow: TextOverflow,
    palette: CodePalette,
) {
    let mut cursor = origin;
    let mut color = base_color;
    let bytes = value.as_bytes();
    let mut index = 0;
    let ellipsis_width = 3.0 * (font.glyph(face, b'.').advance * scale + spacing);
    let truncate = overflow == TextOverflow::Ellipsis
        && visible_text_width_style(font, value, scale, face, spacing) > rect.width;
    let right = rect.right() - if truncate { ellipsis_width } else { 0.0 };
    while index < bytes.len() && cursor[1] < clip.bottom() {
        if bytes[index] == b'^' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit() {
            color = palette.colour(bytes[index + 1] - b'0');
            color[3] *= base_color[3];
            index += 2;
            continue;
        }
        let (key, step) = super::glyph_at(value, index);
        index += step;
        let glyph = font.glyph_of(face, key);
        let advance = glyph.advance * scale + spacing;
        let newline = key == GlyphKey::Slot(b'\n');
        if newline || cursor[0] + advance > right {
            if overflow == TextOverflow::Wrap {
                cursor = [rect.x, cursor[1] + font.height * scale * 1.18];
                if newline {
                    continue;
                }
            } else if newline || truncate {
                if truncate {
                    for _ in 0..3 {
                        glyph_quad(
                            vertices,
                            font.glyph(face, b'.'),
                            cursor,
                            scale,
                            color,
                            palette.halo(color),
                            clip,
                            viewport,
                        );
                        cursor[0] += ellipsis_width / 3.0;
                    }
                }
                break;
            }
        }
        let halo = palette.halo(color);
        glyph_quad(vertices, glyph, cursor, scale, color, halo, clip, viewport);
        cursor[0] += advance;
        if overflow == TextOverflow::Clip && cursor[0] >= clip.right() {
            break;
        }
    }
}

/// The opacity of each of a dark glyph's eight pale copies in [`CodePalette::Haloed`].
const HALO_ALPHA: f32 = 0.18;

#[allow(clippy::too_many_arguments)]
fn glyph_quad(
    vertices: &mut Vec<TextVertex>,
    glyph: FontGlyph,
    cursor: [f32; 2],
    scale: f32,
    color: [f32; 4],
    halo: bool,
    clip: Rect,
    viewport: [f32; 2],
) {
    let rect = Rect::new(
        cursor[0] + glyph.offset_x * scale,
        cursor[1] + glyph.offset_y * scale,
        glyph.width * scale,
        glyph.height * scale,
    );
    let step = scale.max(1.0);
    if halo {
        // A dark glyph: the same glyph in a pale tint a step out all round, the
        // diagonals too so the ring stays even ([`CodePalette::Haloed`]), which edges
        // it on a dark ground. Faint (Sol, 29/10/2026: 0.38 drew too strong an
        // outline): the copies overlap at the glyph's edge into a soft rim.
        let tint = [0.86, 0.89, 0.95, HALO_ALPHA * color[3]];
        let d = step * std::f32::consts::FRAC_1_SQRT_2;
        for [dx, dy] in [
            [-step, 0.0],
            [step, 0.0],
            [0.0, -step],
            [0.0, step],
            [-d, -d],
            [d, -d],
            [-d, d],
            [d, d],
        ] {
            let ring = Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height);
            clipped_quad(vertices, ring, glyph.uv, tint, clip, viewport);
        }
    } else {
        let shadow = Rect::new(rect.x + step, rect.y + step, rect.width, rect.height);
        clipped_quad(
            vertices,
            shadow,
            glyph.uv,
            [0.0, 0.0, 0.0, 0.55 * color[3]],
            clip,
            viewport,
        );
    }
    clipped_quad(vertices, rect, glyph.uv, color, clip, viewport);
}

fn clipped_quad(
    vertices: &mut Vec<TextVertex>,
    rect: Rect,
    uv: [f32; 4],
    color: [f32; 4],
    clip: Rect,
    viewport: [f32; 2],
) {
    let left = rect.x.max(clip.x);
    let top = rect.y.max(clip.y);
    let right = rect.right().min(clip.right());
    let bottom = rect.bottom().min(clip.bottom());
    if right <= left || bottom <= top {
        return;
    }
    let u = |x| uv[0] + (uv[2] - uv[0]) * (x - rect.x) / rect.width;
    let v = |y| uv[1] + (uv[3] - uv[1]) * (y - rect.y) / rect.height;
    push_quad(
        vertices,
        [left, top, right - left, bottom - top],
        [u(left), v(top), u(right), v(bottom)],
        color,
        viewport,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dark_glyph_gets_a_halo_in_the_haloed_palette_only() {
        let font = test_font();
        let rect = Rect::new(0.0, 0.0, 200.0, 20.0);
        let draw = |palette| {
            let mut vertices = Vec::new();
            append_bounded(
                &mut vertices,
                &font,
                "^0a^7b",
                [0.0, 0.0],
                rect,
                rect,
                1.0,
                [640.0, 480.0],
                TextFace::Regular,
                [1.0; 4],
                0.0,
                TextOverflow::Clip,
                palette,
            );
            vertices
        };
        // Game colours: a shadow and a glyph each.
        assert_eq!(draw(CodePalette::Game).len(), 2 * 2 * 6);
        // Haloed: the black `a` edged by eight pale copies instead of its shadow, the
        // white `b` as before.
        let haloed = draw(CodePalette::Haloed);
        assert_eq!(haloed.len(), (9 + 2) * 6);
        assert!(haloed[..8 * 6].iter().all(|vertex| vertex.color[0] > 0.8));
        assert_eq!(haloed[8 * 6].color, [0.0, 0.0, 0.0, 1.0]);
        assert!(!CodePalette::Haloed.halo(quake_color(7)));
        assert!(!CodePalette::Game.halo(quake_color(0)));
    }
}
