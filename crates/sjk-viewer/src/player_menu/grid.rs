//! The character page's model grid: one icon tile per listed catalogue
//! entry (the team's characters first, then the species; see
//! `team_filter`) under the Model row, scrolled by rows when more tiles are
//! listed than fit above the species rows.

use super::*;
use crate::menu_widgets::FormLayout;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign};

/// First tile token; the `i`th tile on screen answers to `TILE_BASE + i`
/// (counted from the first visible one, so a long list never runs into
/// other tokens).
pub(super) const TILE_BASE: u16 = 100;
/// Tile tokens there are room for, from [`TILE_BASE`].
pub(super) const MAX_VISIBLE_TILES: u16 = 200;
/// Token of the wheel target covering the grid.
pub(super) const GRID_SCROLL_TOKEN: u16 = 902;
/// Row index of the Model row the grid belongs to.
pub(super) const MODEL_ROW: usize = 3;
/// Row index of the first row under the grid (a species' head, else the hat).
pub(super) const PART_ROW: usize = 4;
/// Tiles per grid row.
pub(super) const COLUMNS: usize = 8;

/// Gap between tiles, in form-scale pixels.
const TILE_GAP: f32 = 6.0;
/// Room kept under the grid for the status line and footer.
const BOTTOM_RESERVE: f32 = 96.0;
/// Gap between the grid and the species part rows.
const PARTS_GAP: f32 = 12.0;
/// Gap between the two species part columns.
const PART_COLUMN_GAP: f32 = 24.0;

/// Resolved grid geometry for one frame.
pub(super) struct GridLayout {
    pub(super) rect: Rect,
    tile: f32,
    gap: f32,
    pub(super) visible_rows: usize,
    pub(super) total_rows: usize,
}

impl GridLayout {
    /// Grid for `tiles` entries between the Model row and the part rows
    /// (`part_rows` of them, two per line).
    pub(super) fn new(layout: &FormLayout, tiles: usize, part_rows: usize) -> Self {
        let s = layout.scale;
        let top = layout.row_rect(MODEL_ROW).bottom() + TILE_GAP * s;
        let part_lines = part_rows.div_ceil(2) as f32;
        let mut bottom = layout.viewport[1] - BOTTOM_RESERVE * s - part_lines * layout.row_height;
        if part_rows > 0 {
            bottom -= PARTS_GAP * s;
        }
        let gap = TILE_GAP * s;
        let tile = (layout.column_width - gap * (COLUMNS - 1) as f32) / COLUMNS as f32;
        let total_rows = tiles.div_ceil(COLUMNS);
        let fit = ((bottom - top + gap) / (tile + gap)).floor().max(1.0) as usize;
        let visible_rows = fit.min(total_rows).max(1);
        let height = visible_rows as f32 * (tile + gap) - gap;
        Self {
            rect: Rect::new(layout.margin, top, layout.column_width, height),
            tile,
            gap,
            visible_rows,
            total_rows,
        }
    }

    /// Rect of visible slot `slot` (row-major from the first visible row).
    fn tile_rect(&self, slot: usize) -> Rect {
        let step = self.tile + self.gap;
        Rect::new(
            self.rect.x + (slot % COLUMNS) as f32 * step,
            self.rect.y + (slot / COLUMNS) as f32 * step,
            self.tile,
            self.tile,
        )
    }

    /// Largest first visible row that still fills the grid.
    pub(super) fn max_scroll(&self) -> usize {
        self.total_rows.saturating_sub(self.visible_rows)
    }

    /// Cell of row `PART_ROW + axis` (the species parts, then the hat and
    /// cape): two columns under the grid.
    pub(super) fn part_cell(&self, layout: &FormLayout, axis: usize) -> Rect {
        let s = layout.scale;
        let gap = PART_COLUMN_GAP * s;
        let width = (layout.column_width - gap) * 0.5;
        Rect::new(
            self.rect.x + (axis % 2) as f32 * (width + gap),
            self.rect.bottom() + PARTS_GAP * s + (axis / 2) as f32 * layout.row_height,
            width,
            layout.row_height,
        )
    }
}

impl PlayerMenu {
    /// Scroll the grid by `rows` (negative = up) within its bounds.
    pub(super) fn scroll_grid(&mut self, rows: i32) {
        let target = (self.grid_scroll as i32 + rows).max(0) as usize;
        self.grid_scroll = target.min(self.grid_max_scroll);
    }

    /// Tiles per row of the grid on show.
    fn grid_columns(&self) -> usize {
        if self.classic_style {
            super::classic::GRID_COLUMNS
        } else {
            COLUMNS
        }
    }

    /// Slot in [`Self::tiles`] of the `local`th tile on screen.
    pub(super) fn visible_slot(&self, local: usize) -> usize {
        self.grid_scroll * self.grid_columns() + local
    }

    /// Draw the tiles, the scrollbar and their pointer targets.
    pub(super) fn append_grid(&mut self, layout: &FormLayout, grid: &GridLayout) {
        let s = layout.scale;
        let tiles = self.tiles.len();
        let current = self.tile_position();
        self.grid_max_scroll = grid.max_scroll();
        if self.grid_follow {
            self.grid_follow = false;
            let row = current.unwrap_or(0) / COLUMNS;
            if row < self.grid_scroll {
                self.grid_scroll = row;
            } else if row >= self.grid_scroll + grid.visible_rows {
                self.grid_scroll = row + 1 - grid.visible_rows;
            }
        }
        self.grid_scroll = self.grid_scroll.min(self.grid_max_scroll);
        self.canvas.scroll_region(GRID_SCROLL_TOKEN, grid.rect);
        self.hovered_entry = None;
        let first = self.grid_scroll * COLUMNS;
        let last = (first + grid.visible_rows * COLUMNS)
            .min(tiles)
            .min(first + usize::from(MAX_VISIBLE_TILES));
        for slot in first..last {
            let rect = grid.tile_rect(slot - first);
            let absolute = self.tiles[slot];
            self.append_tile(rect, slot - first, absolute, current == Some(slot), s);
        }
        if grid.total_rows > grid.visible_rows {
            let track = Rect::new(
                grid.rect.right() + 8.0 * s,
                grid.rect.y,
                3.0 * s,
                grid.rect.height,
            );
            self.canvas.scrollbar(
                GRID_SCROLL_TOKEN,
                track,
                self.grid_scroll,
                grid.visible_rows,
                grid.total_rows,
            );
        }
    }

    /// One tile: the `local`th on screen answers to the pointer, catalogue
    /// entry `absolute` names its icon.
    fn append_tile(&mut self, rect: Rect, local: usize, absolute: usize, current: bool, s: f32) {
        let token = TILE_BASE + local as u16;
        let hovered = self.canvas.token_hovered(token);
        if hovered {
            self.hovered_entry = Some(absolute);
        }
        let radius = 4.0 * s;
        let theme = self.canvas.theme();
        let icon = self.icons.icon(absolute);
        let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius,
            color: Color::new(0.0, 0.0, 0.0, 0.35),
        });
        if let Some(texture) = icon {
            let _ = self.canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect,
                texture,
                color: Color::new(1.0, 1.0, 1.0, if current || hovered { 1.0 } else { 0.82 }),
            });
        } else if self.icons.failed(absolute) {
            let color = Color::new(0.916, 0.945, 0.973, 0.8);
            self.tile_name(absolute, rect, 11.0 * s, color, 0.2 * s);
        }
        if current || hovered {
            let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius,
                width: if current { 2.0 * s } else { 1.0 * s },
                color: if current {
                    theme.accent
                } else {
                    Color::new(1.0, 1.0, 1.0, 0.45)
                },
            });
        }
        self.canvas.hit_region(token, rect);
    }

    /// The model's name in two lines (model, then skin) across a tile whose
    /// icon cannot be shown.
    pub(super) fn tile_name(
        &mut self,
        absolute: usize,
        rect: Rect,
        size: f32,
        color: Color,
        tracking: f32,
    ) {
        let Some(name) = catalog_of(&self.loader).and_then(|catalog| entry_name(catalog, absolute))
        else {
            return;
        };
        let (model, skin) = name.split_once('/').unwrap_or((name, ""));
        let line = size * 1.25;
        let top = rect.y + (rect.height - line * 2.0) * 0.5;
        for (index, text) in [model, skin].into_iter().enumerate() {
            self.canvas.text_aligned(
                text,
                Rect::new(rect.x, top + index as f32 * line, rect.width, line),
                size,
                color,
                FontWeight::Regular,
                tracking,
                TextAlign::Center,
            );
        }
    }
}

/// What the grid calls catalogue entry `absolute`: a character's `model/skin`
/// or a species' model.
pub(super) fn entry_name(
    catalog: &sjk_client::LegacyAssetCatalog,
    absolute: usize,
) -> Option<&str> {
    match catalog.characters.get(absolute) {
        Some(character) => Some(&character.cvar_value),
        None => catalog
            .species
            .get(absolute - catalog.characters.len())
            .map(|species| species.model.as_str()),
    }
}
