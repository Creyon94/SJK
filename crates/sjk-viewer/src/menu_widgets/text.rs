//! Text storage and vector-font submission for [`MenuCanvas`].

use super::MenuCanvas;
use crate::game_font::{GameFonts, RetailFont, SjkFonts};
use crate::text::{TextStyle, TextVertex, UiFont};
use crate::ui_renderer;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign, TextId, TextOverflow};
use std::fmt::{Arguments, Write as _};

/// The family a text run is drawn in on screens of the SJK UI: its display
/// type (Rajdhani) for navigation and titles, its body type (Exo 2) for the
/// rest. Other screens draw every run in one font whatever its family.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TextFamily {
    #[default]
    Body,
    Display,
}

impl MenuCanvas {
    /// Store the text runs added from now on as `family` (until the next call or
    /// the next frame, which starts on [`TextFamily::Body`]).
    pub(crate) fn set_family(&mut self, family: TextFamily) {
        self.family = family;
    }

    /// Append retained text in the SJK UI's families: display runs to
    /// `fonts.display`, body runs to `fonts.body`, in the player's menu text
    /// `style`.
    pub(crate) fn append_text_families(
        &self,
        fonts: SjkFonts<'_>,
        viewport: [f32; 2],
        style: TextStyle,
    ) {
        let SjkFonts { display, body } = fonts;
        for ((vertices, font), family) in [(display, TextFamily::Display), (body, TextFamily::Body)]
        {
            ui_renderer::append_text_commands_where(
                &self.draw,
                |id| self.resolve(id),
                |id, _| {
                    self.families
                        .get(id.0 as usize)
                        .copied()
                        .unwrap_or_default()
                        == family
                },
                vertices,
                font,
                viewport,
                style,
            );
        }
    }
    /// Add a non-interactive text run.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text(
        &mut self,
        value: &str,
        rect: Rect,
        size: f32,
        color: Color,
        weight: FontWeight,
        letter_spacing: f32,
    ) {
        self.text_aligned(
            value,
            rect,
            size,
            color,
            weight,
            letter_spacing,
            TextAlign::Start,
        );
    }

    /// Add aligned, non-interactive text without allocating on the frame path.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_aligned(
        &mut self,
        value: &str,
        rect: Rect,
        size: f32,
        color: Color,
        weight: FontWeight,
        letter_spacing: f32,
        align: TextAlign,
    ) {
        let Some(id) = self.store_text(value) else {
            return;
        };
        let color = self.legible_text(color);
        let _ = self.draw.push(DrawCommand::Text {
            rect,
            text: id,
            size,
            color,
            align,
            overflow: TextOverflow::Ellipsis,
            weight,
            letter_spacing,
        });
    }

    /// Format directly into retained scratch storage and append aligned text.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_fmt_aligned(
        &mut self,
        value: Arguments<'_>,
        rect: Rect,
        size: f32,
        color: Color,
        weight: FontWeight,
        letter_spacing: f32,
        align: TextAlign,
    ) {
        let Some(id) = self.store_format(value) else {
            return;
        };
        let color = self.legible_text(color);
        let _ = self.draw.push(DrawCommand::Text {
            rect,
            text: id,
            size,
            color,
            align,
            overflow: TextOverflow::Ellipsis,
            weight,
            letter_spacing,
        });
    }

    /// Append retained text commands to the existing cached vector-font path,
    /// in the player's menu text style ([`UiFont::style`]).
    pub(crate) fn append_text(
        &self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        self.append_text_styled(vertices, font, viewport, font.style());
    }

    /// Append retained text commands in an explicit style, for surfaces that
    /// are not menus (chat, scoreboard) or size their text themselves (console).
    pub(crate) fn append_text_styled(
        &self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
        style: TextStyle,
    ) {
        ui_renderer::append_text_commands(
            &self.draw,
            |id| self.resolve(id),
            vertices,
            font,
            viewport,
            style,
        );
    }

    /// Append retained text, each command in the game font `font_of` names for
    /// it when that font is on (see [`GameFonts::append_routed`]), the rest to
    /// `vertices` with `font`. Routed surfaces (chat, scoreboard) are not menus,
    /// so they draw as laid out ([`TextStyle::NEUTRAL`]).
    pub(crate) fn append_text_routed(
        &self,
        fonts: &mut GameFonts,
        font_of: impl Fn(TextId, &str) -> Option<RetailFont> + Copy,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        fonts.append_routed(
            &self.draw,
            |id| self.resolve(id),
            font_of,
            (vertices, font),
            viewport,
            TextStyle::NEUTRAL,
        );
    }

    /// Id the next stored text run will get, to mark where a group of runs
    /// starts and ends.
    pub(crate) fn next_text_id(&self) -> u32 {
        self.text_len as u32
    }

    fn store_text(&mut self, value: &str) -> Option<TextId> {
        let Some(slot) = self.text.get_mut(self.text_len) else {
            self.dropped += 1;
            return None;
        };
        slot.clear();
        slot.push_str(value);
        Some(self.stored())
    }

    fn store_format(&mut self, value: Arguments<'_>) -> Option<TextId> {
        let Some(slot) = self.text.get_mut(self.text_len) else {
            self.dropped += 1;
            return None;
        };
        slot.clear();
        let _ = slot.write_fmt(value);
        Some(self.stored())
    }

    /// Close the run just written into slot `text_len`: record its family and
    /// return its id.
    fn stored(&mut self) -> TextId {
        if let Some(family) = self.families.get_mut(self.text_len) {
            *family = self.family;
        }
        let id = TextId(self.text_len as u32);
        self.text_len += 1;
        id
    }

    fn resolve(&self, id: TextId) -> &str {
        self.text
            .get(id.0 as usize)
            .map(String::as_str)
            .unwrap_or("")
    }
}
