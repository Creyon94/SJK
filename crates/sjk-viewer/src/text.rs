//! DPI-aware text layout and cached glyph atlases for the client UI.
//!
//! Inter is rasterized once when the graphics device is created.  The render
//! loop only performs glyph lookup and appends vertices into reused buffers;
//! it never rasterizes a glyph or grows the atlas.  The console font,
//! JetBrains Mono ([`console_font`]), and the vector replacements for the retail
//! game fonts ([`retail_font`]), which the classic HUD font and the optional game
//! fonts ([`crate::game_font`]) use, are rasterized the same way. The retail fonts'
//! `¬` logo, bundled as a vector glyph, replaces Inter's ([`logo_glyph`]).

mod bounded;
mod cell;
pub(crate) mod console_font;
pub(crate) mod logo_glyph;
pub(crate) mod retail_font;
pub(crate) mod sdf;
pub(crate) mod style;
pub(crate) use bounded::append_bounded;
pub(crate) use cell::append_cell;
pub(crate) use logo_glyph::LogoGlyph;
pub(crate) use style::TextStyle;

use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings, Metrics};
use image::{Rgba, RgbaImage};
use std::error::Error;

pub(crate) const MAX_TEXT_VERTICES: usize = 32_768;
const GLYPH_COUNT: usize = 256;
const ATLAS_WIDTH: u32 = 4_096;
/// Glyph gutter; must stay >= 2 px at the deepest mip level so minified
/// sampling never bleeds a neighbour's coverage into a glyph edge.
const ATLAS_PADDING: u32 = 2 << (ATLAS_MIP_LEVELS - 1);
/// Modern glyphs are rasterized at 3x and minified for small UI text.
const MODERN_RASTER_SCALE: f32 = 3.0;
/// Mip levels uploaded for the modern atlas (96 px raster down to 12 px).
pub(crate) const ATLAS_MIP_LEVELS: u32 = 4;
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");

/// Font weight available in the modern UI atlas.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TextFace {
    Regular,
    Semibold,
}

impl TextFace {
    fn index(self) -> usize {
        match self {
            Self::Regular => 0,
            Self::Semibold => 1,
        }
    }
}

/// A single cached glyph: layout metrics in font units (physical pixels at
/// scale 1) and its rectangle in normalized atlas coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FontGlyph {
    /// Ink width.
    pub(crate) width: f32,
    /// Ink height.
    pub(crate) height: f32,
    /// Pen advance.
    pub(crate) advance: f32,
    /// Ink left edge from the pen.
    pub(crate) offset_x: f32,
    /// Ink top edge below the line top.
    pub(crate) offset_y: f32,
    /// Atlas rectangle `[u0, v0, u1, v1]`, `v0` at the ink top.
    pub(crate) uv: [f32; 4],
}

/// Immutable glyph metrics used by all menu, console, HUD, and chat layout.
pub(crate) struct UiFont {
    glyphs: [[FontGlyph; GLYPH_COUNT]; 2],
    /// Baseline-to-baseline line height in physical framebuffer pixels.
    pub(crate) height: f32,
    modern: bool,
    /// Player size and spacing preference for menu text, see [`style`].
    style: TextStyle,
}

impl UiFont {
    /// Metrics and atlas rectangle of `byte` (Latin-1) in `face`, for layouts
    /// outside the 2D text path such as the ground HUD's world-space numbers.
    pub(crate) fn glyph(&self, face: TextFace, byte: u8) -> FontGlyph {
        self.glyphs[if self.modern { face.index() } else { 0 }][usize::from(byte)]
    }

    /// Whether this is the bundled vector font rather than the retail HUD font.
    pub(crate) const fn is_modern(&self) -> bool {
        self.modern
    }

    /// The player's menu text style (neutral until the cvars are synced).
    pub(crate) const fn style(&self) -> TextStyle {
        self.style
    }

    /// Apply the player's `ui_textScale` / `ui_letterSpacing` preference.
    pub(crate) fn set_style(&mut self, style: TextStyle) {
        self.style = style;
    }
}

/// CPU assets uploaded once to the GPU text texture.
pub(crate) struct FontAtlas {
    pub(crate) font: UiFont,
    pub(crate) image: RgbaImage,
    /// `image` is a signed distance field for the distance-field text pipeline.
    pub(crate) distance_field: bool,
}

/// Vertex consumed by `text.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct TextVertex {
    position: [f32; 2],
    texture_coordinates: [f32; 2],
    color: [f32; 4],
}

impl TextVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

    /// WGPU layout shared by all text draw calls.
    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct RasterizedGlyph {
    face: usize,
    byte: usize,
    metrics: Metrics,
    pixels: Vec<u8>,
}

/// Rasterize Inter Regular and SemiBold at the current monitor DPI.
///
/// Latin-1 is deliberately pre-cached: protocol-26 UI strings are byte based,
/// so this covers the complete wire character range without atlas mutation in
/// the frame loop. With the retail `logo` from the game data, byte 0xAC draws
/// it in both faces instead of Inter's `¬`, as Jedi Academy's fonts do.
pub(crate) fn load_modern(
    dpi_scale: f64,
    logo: Option<&LogoGlyph>,
) -> Result<FontAtlas, Box<dyn Error>> {
    let fonts = [
        Font::from_bytes(INTER_REGULAR, FontSettings::default())?,
        Font::from_bytes(INTER_SEMIBOLD, FontSettings::default())?,
    ];
    // The largest menu face is 2.7x body size.  A 3x source atlas means every
    // production text size is sampled at native resolution or downsampled,
    // never magnified from a small bitmap as the retail font was.
    let pixel_size = (32.0 * dpi_scale.clamp(1.0, 3.0) as f32 * MODERN_RASTER_SCALE).round();
    let line_metrics = fonts[0]
        .horizontal_line_metrics(pixel_size)
        .ok_or("Inter has no horizontal line metrics")?;
    let mut rasterized = Vec::with_capacity(GLYPH_COUNT * fonts.len());
    for (face, font) in fonts.iter().enumerate() {
        for byte in 0..GLYPH_COUNT {
            // Slot `byte` holds the Windows-1252 character of that byte, as JKA's
            // own fonts do, so 0x80 is `€` rather than an invisible C1 control.
            let character = slot_character(font, byte as u8);
            let (metrics, pixels) = font.rasterize(character, pixel_size);
            rasterized.push(RasterizedGlyph {
                face,
                byte,
                metrics,
                pixels,
            });
        }
    }
    if let Some(logo) = logo {
        for face in 0..fonts.len() {
            let cap_height = rasterized[face * GLYPH_COUNT + usize::from(b'H')]
                .metrics
                .height as f32;
            let glyph = &mut rasterized[face * GLYPH_COUNT + usize::from(logo_glyph::BYTE)];
            let scaled = logo.rasterize(cap_height);
            glyph.metrics = Metrics {
                xmin: scaled.xmin,
                ymin: scaled.ymin,
                width: scaled.width,
                height: scaled.height,
                advance_width: scaled.advance,
                ..glyph.metrics
            };
            glyph.pixels = scaled.pixels;
        }
    }

    let (image, rectangles) = paint_atlas(&rasterized);
    let mut glyphs = [[FontGlyph::default(); GLYPH_COUNT]; 2];
    for (glyph, uv) in rasterized.iter().zip(rectangles) {
        let top = (line_metrics.ascent - (glyph.metrics.ymin as f32 + glyph.metrics.height as f32))
            / MODERN_RASTER_SCALE;
        glyphs[glyph.face][glyph.byte] = FontGlyph {
            width: glyph.metrics.width as f32 / MODERN_RASTER_SCALE,
            height: glyph.metrics.height as f32 / MODERN_RASTER_SCALE,
            advance: glyph.metrics.advance_width / MODERN_RASTER_SCALE,
            offset_x: glyph.metrics.xmin as f32 / MODERN_RASTER_SCALE,
            offset_y: top,
            uv,
        };
    }
    Ok(FontAtlas {
        font: UiFont {
            glyphs,
            height: line_metrics.new_line_size / MODERN_RASTER_SCALE,
            modern: true,
            style: TextStyle::NEUTRAL,
        },
        image,
        distance_field: false,
    })
}

/// Pack `rasterized` into a coverage atlas [`ATLAS_WIDTH`] wide with mipmappable
/// padding, and return it with each glyph's rectangle `[u0, v0, u1, v1]`.
fn paint_atlas(rasterized: &[RasterizedGlyph]) -> (RgbaImage, Vec<[f32; 4]>) {
    let placements = pack_glyphs(rasterized);
    let height = placements
        .iter()
        .zip(rasterized)
        .map(|([_, y], glyph)| y + glyph.metrics.height as u32 + ATLAS_PADDING)
        .max()
        .unwrap_or(1)
        .next_power_of_two();
    // Transparent texels retain white RGB so bilinear sampling at a glyph
    // boundary does not interpolate toward black before straight-alpha blend.
    let mut image = RgbaImage::from_pixel(ATLAS_WIDTH, height, Rgba([255, 255, 255, 0]));
    let rectangles = rasterized
        .iter()
        .zip(placements)
        .map(|(glyph, [x, y])| {
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
            [
                x as f32 / ATLAS_WIDTH as f32,
                y as f32 / height as f32,
                (x + metrics.width as u32) as f32 / ATLAS_WIDTH as f32,
                (y + metrics.height as u32) as f32 / height as f32,
            ]
        })
        .collect();
    (image, rectangles)
}

fn pack_glyphs(glyphs: &[RasterizedGlyph]) -> Vec<[u32; 2]> {
    let mut positions = Vec::with_capacity(glyphs.len());
    let mut x = ATLAS_PADDING;
    let mut y = ATLAS_PADDING;
    let mut row_height = 0;
    for glyph in glyphs {
        let width = glyph.metrics.width as u32;
        let height = glyph.metrics.height as u32;
        if x + width + ATLAS_PADDING > ATLAS_WIDTH {
            x = ATLAS_PADDING;
            y += row_height + ATLAS_PADDING;
            row_height = 0;
        }
        positions.push([x, y]);
        x += width + ATLAS_PADDING;
        row_height = row_height.max(height);
    }
    positions
}

/// The classic status HUD's font: SJK HUD, the bundled vector replacement for
/// retail's `arialnb` ([`retail_font::HUD`]).
pub(crate) fn load_classic() -> Result<FontAtlas, Box<dyn Error>> {
    retail_font::load(&retail_font::HUD)
}

/// Colour of the `^<digit>` code `index` (0-9), as a display value.
///
/// OpenJK's `g_color_table` (`shared/qcommon/q_color.c`) at full strength: the
/// 2D layer draws display values ([`crate::ui_target`]), so `^1` is the same
/// pure red retail showed. The table has ten entries and `ColorIndex` masks with
/// `Q_COLOR_BITS` (0xF), so `^8` is orange and `^9` mid grey rather than
/// retail's `& 7` wrap to black and red.
pub(crate) fn quake_color(index: u8) -> [f32; 4] {
    match index {
        0 => [0.0, 0.0, 0.0, 1.0],
        1 => [1.0, 0.0, 0.0, 1.0],
        2 => [0.0, 1.0, 0.0, 1.0],
        3 => [1.0, 1.0, 0.0, 1.0],
        4 => [0.0, 0.0, 1.0, 1.0],
        5 => [0.0, 1.0, 1.0, 1.0],
        6 => [1.0, 0.0, 1.0, 1.0],
        8 => [1.0, 0.5, 0.0, 1.0],
        9 => [0.5, 0.5, 0.5, 1.0],
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}

fn push_quad(
    vertices: &mut Vec<TextVertex>,
    rectangle: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
    viewport: [f32; 2],
) {
    push_quad_within(vertices, rectangle, uv, color, viewport, MAX_TEXT_VERTICES);
}

/// [`push_quad`] into a batch of at most `limit` vertices.
fn push_quad_within(
    vertices: &mut Vec<TextVertex>,
    rectangle: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
    viewport: [f32; 2],
    limit: usize,
) {
    if vertices.len() + 6 > limit {
        return;
    }
    let [x, y, width, height] = rectangle;
    let ndc = |position: [f32; 2]| {
        [
            position[0] / viewport[0] * 2.0 - 1.0,
            1.0 - position[1] / viewport[1] * 2.0,
        ]
    };
    let points = [
        ([x, y], [uv[0], uv[1]]),
        ([x + width, y], [uv[2], uv[1]]),
        ([x + width, y + height], [uv[2], uv[3]]),
        ([x, y], [uv[0], uv[1]]),
        ([x + width, y + height], [uv[2], uv[3]]),
        ([x, y + height], [uv[0], uv[3]]),
    ];
    vertices.extend(points.map(|(position, texture_coordinates)| TextVertex {
        position: ndc(position),
        texture_coordinates,
        color,
    }));
}

/// Append anti-aliased glyph quads without allocating.
pub(crate) fn append_text(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    text: &str,
    origin: [f32; 2],
    scale: f32,
    viewport: [f32; 2],
) -> usize {
    append_text_face(
        vertices,
        font,
        text,
        origin,
        scale,
        viewport,
        TextFace::Regular,
    )
}

/// Append text using an explicit Inter weight.
pub(crate) fn append_text_face(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    text: &str,
    origin: [f32; 2],
    scale: f32,
    viewport: [f32; 2],
    face: TextFace,
) -> usize {
    append_text_style(
        vertices,
        font,
        text,
        origin,
        scale,
        viewport,
        face,
        [0.964, 0.978, 0.991, 1.0],
        0.0,
    )
}

/// Append text with an explicit weight, base colour, and tracking.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_text_style(
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    text: &str,
    origin: [f32; 2],
    scale: f32,
    viewport: [f32; 2],
    face: TextFace,
    base_color: [f32; 4],
    letter_spacing: f32,
) -> usize {
    let mut cursor = origin;
    let mut color = base_color;
    let mut lines = 1;
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'^' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit() {
            color = quake_color(bytes[index + 1] - b'0');
            index += 2;
            continue;
        }
        let (character, step) = glyph_byte_at(text, index);
        index += step;
        if character == b'\n' || cursor[0] + font.height * scale > viewport[0] {
            cursor[0] = origin[0];
            cursor[1] += font.height * scale * 1.18;
            lines += 1;
            if character == b'\n' {
                continue;
            }
        }
        let glyph = font.glyph(face, character);
        let rectangle = [
            cursor[0] + glyph.offset_x * scale,
            cursor[1] + glyph.offset_y * scale,
            glyph.width * scale,
            glyph.height * scale,
        ];
        let mut shadow = rectangle;
        shadow[0] += scale.max(1.0);
        shadow[1] += scale.max(1.0);
        // The shadow fades with the glyph so opacity groups hide text fully.
        push_quad(
            vertices,
            shadow,
            glyph.uv,
            [0.0, 0.0, 0.0, 0.55 * color[3]],
            viewport,
        );
        push_quad(vertices, rectangle, glyph.uv, color, viewport);
        cursor[0] += glyph.advance * scale + letter_spacing;
    }
    lines
}

/// The character the modern atlas draws in slot `byte`: the Windows-1252 character of
/// that byte, or `.` where Inter has no glyph for it (a control byte such as the 0x0B
/// some players put in names, or one of Windows-1252's five unassigned bytes).
/// OpenJK's `RE_Font_DrawString` draws `.` for every glyph its font lacks, so retail
/// and EternalJK show those names with dots; line feed, carriage return and space keep
/// their own (empty) slots.
fn slot_character(font: &Font, byte: u8) -> char {
    let character = sjk_protocol::windows_1252_char(byte);
    if matches!(byte, b'\n' | b'\r' | b' ') || font.lookup_glyph_index(character) != 0 {
        character
    } else {
        '.'
    }
}

/// Take the atlas index that draws the character at `index`, and its UTF-8 length.
///
/// The atlas holds 256 glyphs indexed by Windows-1252 byte, matching how JKA's own fonts are
/// laid out. Text reaching the renderer is a Rust `str`, so any character above ASCII occupies
/// several UTF-8 bytes, and indexing the atlas with those bytes drew one character as two
/// glyphs: `ñ` became `Ã±`, and U+FFFD became `ï¿½` — the trailing `½` players kept seeing in
/// names. A character is drawn with the glyph of its Windows-1252 byte, so `€` typed here and
/// byte 0x80 from another client (decoded as U+0080) both draw slot 0x80. Characters with no
/// Windows-1252 byte have no glyph in a 256-entry atlas and fall back to `?`.
///
/// `index` must be a character boundary, which holds because every caller advances by whole
/// characters or by an ASCII colour escape.
pub(crate) fn glyph_byte_at(text: &str, index: usize) -> (u8, usize) {
    match text[index..].chars().next() {
        Some(character) => (
            sjk_protocol::windows_1252_byte(character).unwrap_or(b'?'),
            character.len_utf8(),
        ),
        None => (b'?', 1),
    }
}

/// Measure visible text while ignoring Quake color escapes.
pub(crate) fn visible_text_width(font: &UiFont, text: &str, scale: f32) -> f32 {
    visible_text_width_face(font, text, scale, TextFace::Regular)
}

/// Measure one face without producing vertices.
pub(crate) fn visible_text_width_face(
    font: &UiFont,
    text: &str,
    scale: f32,
    face: TextFace,
) -> f32 {
    visible_text_width_style(font, text, scale, face, 0.0)
}

/// Measure one face with additional tracking between glyph advances.
pub(crate) fn visible_text_width_style(
    font: &UiFont,
    text: &str,
    scale: f32,
    face: TextFace,
    letter_spacing: f32,
) -> f32 {
    let bytes = text.as_bytes();
    let mut width = 0.0;
    let mut maximum = 0.0_f32;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'^' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit() {
            index += 2;
        } else if bytes[index] == b'\n' {
            maximum = maximum.max(width);
            width = 0.0;
            index += 1;
        } else {
            let (character, step) = glyph_byte_at(text, index);
            width += font.glyph(face, character).advance * scale + letter_spacing;
            index += step;
        }
    }
    maximum.max(width)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Glyph bytes of `text`, walking it as the renderer does.
    fn glyph_bytes(text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut index = 0;
        while index < text.len() {
            let (byte, step) = glyph_byte_at(text, index);
            bytes.push(byte);
            index += step;
        }
        bytes
    }

    #[test]
    fn typed_and_received_symbols_draw_their_windows_1252_glyph() {
        let symbols = "×¥’¡²³‘€½¼©ñæ§®™£°µ·«»ÄäÖöÜüßÆØøÑ…";
        let expected = sjk_protocol::encode_legacy_text(symbols);
        // Typed here: Unicode characters.
        assert_eq!(glyph_bytes(symbols), *expected);
        // Received from a legacy client: the same bytes, decoded as Latin-1.
        let received: String = expected.iter().map(|&byte| char::from(byte)).collect();
        assert_eq!(glyph_bytes(&received), *expected);
        assert_eq!(glyph_bytes("♥"), [b'?']);
    }

    #[test]
    fn slots_inter_lacks_draw_a_dot_as_retail_does() {
        let font = Font::from_bytes(INTER_REGULAR, FontSettings::default()).unwrap();
        assert_eq!(slot_character(&font, 0x0b), '.');
        assert_eq!(slot_character(&font, 0x81), '.');
        assert_eq!(slot_character(&font, b' '), ' ');
        assert_eq!(slot_character(&font, b'\n'), '\n');
        for (byte, character) in [(0x80, '€'), (0x85, '…'), (0x92, '’'), (0xd7, '×')] {
            assert_eq!(slot_character(&font, byte), character);
        }
    }

    #[test]
    fn colour_codes_eight_and_nine_are_orange_and_grey() {
        assert_eq!(quake_color(8), [1.0, 0.5, 0.0, 1.0]);
        assert_eq!(quake_color(9), [0.5, 0.5, 0.5, 1.0]);
        // No longer the retail `& 7` wrap onto black and red.
        assert_ne!(quake_color(8), quake_color(0));
        assert_ne!(quake_color(9), quake_color(1));
    }

    #[test]
    fn colour_codes_zero_to_seven_match_the_retail_table() {
        // OpenJK `g_color_table`, shared/qcommon/q_color.c.
        let retail = [
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [0.0, 1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ];
        for (index, colour) in retail.iter().enumerate() {
            assert_eq!(quake_color(index as u8), *colour, "^{index}");
        }
    }

    fn test_font() -> UiFont {
        let mut glyphs = [[FontGlyph::default(); GLYPH_COUNT]; 2];
        for face in &mut glyphs {
            for glyph in face.iter_mut() {
                *glyph = FontGlyph {
                    width: 8.0,
                    height: 10.0,
                    advance: 8.0,
                    ..FontGlyph::default()
                };
            }
        }
        UiFont {
            glyphs,
            height: 12.0,
            modern: true,
            style: TextStyle::NEUTRAL,
        }
    }

    #[test]
    fn drawn_glyphs_take_the_extended_code_colours() {
        let font = test_font();
        let mut vertices = Vec::new();
        append_text(
            &mut vertices,
            &font,
            "^8a^9b",
            [0.0, 0.0],
            1.0,
            [640.0, 480.0],
        );
        // Each glyph emits a shadow quad then the glyph quad, six vertices each.
        assert_eq!(vertices.len(), 24);
        assert_eq!(vertices[6].color, [1.0, 0.5, 0.0, 1.0]);
        assert_eq!(vertices[18].color, [0.5, 0.5, 0.5, 1.0]);
    }
}
