//! Retained main-menu presentation: a full-bleed hero over the live map with
//! a left column of box-free entries. Everything scales with viewport height
//! so 1080p, ultrawide and 4K keep the same proportions. SJK's emblem
//! ([`super::emblem`]) crowns the title, aligned with the column.

use super::art::motion;
use super::{ClientMenu, MAIN_ITEMS, emblem};
use crate::menu_widgets::{HeroColumn, MenuCanvas, Scrim};
use crate::ui_renderer::{BANNER_SIZE, BANNER_TEXTURE};
use crate::{TextVertex, UiFont};
use sjk_ui::{DrawCommand, FontWeight, Rect, TextAlign};

/// Build version ([`crate::build_info::VERSION`]), bottom right; the only footer
/// text the main menu carries. The classic menus and console show it too.
pub(crate) const VERSION_LINE: &str = concat!("SJK ", env!("SJK_BUILD_VERSION"));

/// Top of the entry list and the height of one entry.
fn list_metrics(viewport: [f32; 2], scale: f32) -> (f32, f32) {
    (viewport[1] * 0.47, 72.0 * scale)
}

/// Side of the emblem above the title at a scale of 1, and its gaps to the
/// line under it and to the top of the window.
const EMBLEM_SIDE: f32 = 128.0;
const EMBLEM_GAP: f32 = 16.0;

/// Where the emblem sits: over the "JEDI ACADEMY / MULTIPLAYER" line whose
/// top is `eyebrow_top`, its left edge on the column's. It shrinks to fit
/// short windows and is left out when under a quarter of its size would fit.
fn emblem_rect(column: &HeroColumn, eyebrow_top: f32) -> Option<Rect> {
    let s = column.scale;
    let bottom = eyebrow_top - EMBLEM_GAP * s;
    let side = (EMBLEM_SIDE * s).min(bottom - EMBLEM_GAP * s);
    (side >= EMBLEM_SIDE * s * 0.25).then(|| Rect::new(column.margin, bottom - side, side, side))
}

/// Build the main menu into `canvas` at `reveal` opacity; shared by the live
/// client and evidence.
pub(crate) fn build(canvas: &mut MenuCanvas, viewport: [f32; 2], selection: usize, reveal: f32) {
    let column = HeroColumn::new(viewport);
    let (s, x, width) = (column.scale, column.margin, column.column_width);
    let height = viewport[1];
    let (list_top, row_height) = list_metrics(viewport, s);
    canvas.begin_hero(viewport, reveal, Scrim::Full);
    let theme = canvas.theme();
    let wordmark_y = height * 0.19;
    if let Some(rect) = emblem_rect(&column, wordmark_y - 30.0 * s) {
        emblem::draw(canvas, rect, motion::seconds());
    }
    canvas.text(
        "JEDI ACADEMY   /   MULTIPLAYER",
        Rect::new(x, wordmark_y - 30.0 * s, width, 18.0 * s),
        14.0 * s,
        theme.accent,
        FontWeight::Semibold,
        3.2 * s,
    );
    canvas.text(
        "SJK",
        Rect::new(x - 6.0 * s, wordmark_y, width, 140.0 * s),
        132.0 * s,
        theme.foreground,
        FontWeight::Semibold,
        -2.0 * s,
    );
    // The saber emblem, in the player's accent, fills the gap to the items.
    let emblem_height = 96.0 * s;
    let emblem_width = emblem_height * BANNER_SIZE[0] as f32 / BANNER_SIZE[1] as f32;
    let _ = canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
        rect: Rect::new(x, wordmark_y + 150.0 * s, emblem_width, emblem_height),
        texture: BANNER_TEXTURE,
        color: theme.accent,
    });
    for (row, item) in MAIN_ITEMS.iter().enumerate() {
        canvas.hero_item(
            row as u16,
            item.label,
            item.hint,
            column.row_rect(list_top, row, row_height),
            row == selection,
            s,
        );
    }
    let footer_y = height - 64.0 * s;
    canvas.text_aligned(
        VERSION_LINE,
        Rect::new(
            viewport[0] - x - 320.0 * s,
            footer_y + 3.0 * s,
            320.0 * s,
            16.0 * s,
        ),
        12.0 * s,
        theme.muted,
        FontWeight::Regular,
        1.2 * s,
        TextAlign::End,
    );
    canvas.end_hero();
    canvas.finish(selection as u16);
}

impl ClientMenu {
    pub(super) fn append_main(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let reveal = self.screen_reveal();
        if self.menu_style == super::MenuStyle::Classic {
            super::classic::view::build(&mut self.ui, viewport, &self.classic, reveal, self.art);
        } else {
            build(&mut self.ui, viewport, self.main_selection, reveal);
        }
        self.ui.append_text(vertices, font, viewport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emblem_crowns_the_title_clear_of_the_entries() {
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [2_560.0, 1_080.0],
            [1_440.0, 1_080.0],
            [1_280.0, 720.0],
            [1_024.0, 768.0],
        ] {
            let column = HeroColumn::new(viewport);
            let s = column.scale;
            let eyebrow_top = viewport[1] * 0.19 - 30.0 * s;
            let rect = emblem_rect(&column, eyebrow_top).expect("room above the title");
            assert_eq!(rect.width, rect.height, "{viewport:?}");
            assert_eq!(rect.x, column.margin, "{viewport:?}");
            assert!(rect.y >= EMBLEM_GAP * s - 1e-3, "{viewport:?}: {rect:?}");
            assert!(rect.bottom() <= eyebrow_top - EMBLEM_GAP * s + 1e-3);
            // Above the entry list, by a wide margin.
            let (list_top, _) = list_metrics(viewport, s);
            assert!(rect.bottom() < list_top);
        }
        // Full size at 1080 lines and above; 256 pixels at 4K.
        let at_4k = emblem_rect(&HeroColumn::new([3_840.0, 2_160.0]), 2_160.0 * 0.19 - 60.0);
        assert_eq!(at_4k.map(|rect| rect.width), Some(256.0));
        // A very short window shrinks it, then leaves it out.
        let short = HeroColumn::new([800.0, 560.0]);
        let rect = emblem_rect(&short, 560.0 * 0.19 - 30.0 * short.scale).expect("shrunk");
        assert!(rect.width < EMBLEM_SIDE * short.scale && rect.y >= 0.0);
        let tiny = HeroColumn::new([480.0, 240.0]);
        assert!(emblem_rect(&tiny, 240.0 * 0.19 - 30.0 * tiny.scale).is_none());
    }
}
