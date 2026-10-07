//! The player screen in the SJK UI (`docs/sjk-ui.md`, Character): the
//! player's name as the screen's title, the three pages as tabs under it,
//! their rows in a column on the left drawn with the SJK UI's kit, and the
//! model standing on the menu map's stage on the right, a caption beside it as
//! in a gallery. It is the modern screen's state with another view: the rows,
//! the grid's tiles, the tabs and the back key answer to the modern screen's
//! tokens, so its keys and pointer work unchanged. Each row registers its
//! control first and then the whole row, so a token's rectangle is its
//! control's ([`MenuCanvas::rect_for`]); a click on a row outside its control
//! only chooses the row (`pointer.rs`).
//!
//! Positions are pixels of the SJK UI's 16:9 frame ([`Frame`]).

use super::force::POWER_NAMES;
use super::force_icons::{power_texture, side_texture};
use super::grid::{COLUMNS, GRID_SCROLL_TOKEN, MAX_VISIBLE_TILES, PART_ROW, TILE_BASE};
use super::rows::{
    CharacterRow, FORCE_APPLY_ROW, FORCE_DISCARD_ROW, FORCE_POWER_ROW, FORCE_RESET_ROW,
    FORCE_SIDE_ROW, SaberRow, saber_color,
};
use super::saber::PALETTE;
use super::*;
use crate::menu::sjk::{
    Frame, TextTarget, color, fade, fade_across, key_hint, key_hint_width, kit, text,
};
use crate::menu_widgets::{BACK_TOKEN, TAB_BASE, TextFamily};
use sjk_client::{ForceSide, LegacyCatalogStatus};
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The pages' names on their tabs.
const TABS: [&str; 3] = ["Character", "Saber", "Force"];

/// The top bar's middle line and the tabs' top.
const BAR_Y: f32 = 87.0;
const TABS_Y: f32 = 146.0;
/// The form's column, its rows' top and height, where a row's name starts
/// and where its control ends, and a control's usual width.
const COLUMN_X: f32 = 96.0;
const COLUMN_WIDTH: f32 = 664.0;
const ROWS_TOP: f32 = 214.0;
const ROW: f32 = 52.0;
const LABEL_X: f32 = COLUMN_X + 22.0;
const CONTROL_RIGHT: f32 = COLUMN_X + COLUMN_WIDTH - 10.0;
const CONTROL_WIDTH: f32 = 330.0;
/// A control's height inside its row.
const CONTROL: f32 = 38.0;
/// The model grid's tiles.
const TILE_GAP: f32 = 8.0;
const TILE: f32 = (COLUMN_WIDTH - TILE_GAP * (COLUMNS - 1) as f32) / COLUMNS as f32;
/// The rows end above this; the keys' line below it.
const BOTTOM: f32 = 960.0;
const KEYS_Y: f32 = 992.0;
/// A slider's track and its number.
const TRACK_WIDTH: f32 = 230.0;
const NUMBER_WIDTH: f32 = 46.0;
/// The Force page: the side cards' height, a power cell's, the gap between
/// the two columns.
const SIDE_HEIGHT: f32 = 64.0;
const CELL: f32 = 44.0;
const GUTTER: f32 = 16.0;

/// A rectangle of the frame as the kit takes it.
type Area = [f32; 4];

impl PlayerMenu {
    /// Draw the screen in the SJK UI at `reveal` opacity.
    pub(crate) fn append_sjk(&mut self, target: TextTarget<'_>, viewport: [f32; 2], reveal: f32) {
        let frame = Frame::new(viewport);
        self.canvas.begin_transparent(viewport);
        self.canvas.push_opacity(reveal);
        scrims(&mut self.canvas, viewport, &frame);
        self.sjk_top(&frame);
        match self.page {
            ProfilePage::Character => self.sjk_character(&frame),
            ProfilePage::Saber => self.sjk_saber(&frame),
            ProfilePage::Force => self.sjk_force(&frame),
        }
        self.sjk_caption(&frame);
        self.sjk_keys(&frame);
        self.canvas.pop_opacity();
        self.canvas.finish(self.selected as u16);
        target.append(&self.canvas, viewport);
    }

    /// Whether a click at `point` on row token `token` lands outside the
    /// row's control, so it only chooses the row.
    pub(super) fn sjk_beside_control(&self, token: u16, point: sjk_ui::Vec2) -> bool {
        let Some(rect) = self.canvas.rect_for(token) else {
            return false;
        };
        !(point.x >= rect.x
            && point.x <= rect.right()
            && point.y >= rect.y
            && point.y <= rect.bottom())
    }

    /// The way back, the player's name as the title, the pages' tabs.
    fn sjk_top(&mut self, frame: &Frame) {
        let s = frame.s;
        let [x, y] = frame.point(COLUMN_X, BAR_Y - 12.0);
        let end = key_hint(&mut self.canvas, &["Esc"], "Main menu", x, y, s);
        self.canvas
            .hit_region(BACK_TOKEN, sjk_ui::Rect::new(x, y, end - x, 24.0 * s));
        let title_x = (end - frame.origin[0]) / s + 22.0;
        let name = if self.draft.name.trim().is_empty() {
            "Padawan"
        } else {
            self.draft.name.as_str()
        };
        text(
            &mut self.canvas,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(title_x, BAR_Y - 30.0, 1_000.0, 60.0),
            48.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut x = COLUMN_X;
        for (index, label) in TABS.iter().enumerate() {
            // Rajdhani SemiBold at 26 is about 12 pixels a character.
            let width = 12.0 * label.chars().count() as f32 + 4.0;
            let token = TAB_BASE + index as u16;
            let current = index == self.page.index();
            let hovered = self.canvas.token_hovered(token);
            text(
                &mut self.canvas,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(x, TABS_Y, width + 20.0, 34.0),
                26.0 * s,
                match (current, hovered) {
                    (true, _) => color::GOLD_BRIGHT,
                    (false, true) => color::TEXT,
                    (false, false) => color::MUTED,
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            if current {
                let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(x, TABS_Y + 38.0, width, 3.0),
                    radius: 1.5 * s,
                    color: color::GOLD_BRIGHT,
                });
            }
            self.canvas
                .hit_region(token, frame.rect(x - 8.0, TABS_Y - 4.0, width + 16.0, 46.0));
            x += width + 40.0;
        }
    }

    /// A row's band when it is the focused one, and its name; returns whether
    /// it is focused.
    fn sjk_row(&mut self, frame: &Frame, index: usize, area: Area, label: &str) -> bool {
        let focused = index == self.selected;
        if focused {
            kit::band(&mut self.canvas, frame, area);
        }
        let [x, y, _, height] = area;
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!("{label}"),
            frame.rect(x + 22.0, y + (height - 26.0) * 0.5, 220.0, 26.0),
            19.0 * frame.s,
            if focused {
                Color::new(1.0, 1.0, 1.0, 1.0)
            } else {
                color::alpha(color::TEXT, 0.88)
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        focused
    }

    /// Register row `index`'s control `control`, then the whole row `area`.
    fn sjk_targets(&mut self, frame: &Frame, index: usize, control: Area, area: Area) {
        let token = index as u16;
        self.canvas.hit_region(
            token,
            frame.rect(control[0], control[1], control[2], control[3]),
        );
        self.canvas
            .hit_region(token, frame.rect(area[0], area[1], area[2], area[3]));
    }

    /// The character page: name, team, search and model, the model grid, then
    /// a species' parts and the hat and cape in two columns.
    fn sjk_character(&mut self, frame: &Frame) {
        let rows = self.character_rows();
        let catalog = catalog_of(&self.loader);
        let species = match self.choice {
            Some(Choice::Species(index)) => catalog.and_then(|catalog| catalog.species.get(index)),
            _ => None,
        };
        let model_label = match (self.choice, catalog) {
            (Some(Choice::Character(index)), Some(catalog)) => catalog
                .characters
                .get(index)
                .map_or(self.draft.model.clone(), |entry| entry.cvar_value.clone()),
            _ => species.map_or(self.draft.model.clone(), |species| species.model.clone()),
        };
        let part_labels: [String; 3] = std::array::from_fn(|axis| {
            species
                .and_then(|species| {
                    let parts = match axis {
                        0 => &species.heads,
                        1 => &species.torsos,
                        _ => &species.legs,
                    };
                    parts.get(self.variants[axis]).cloned()
                })
                .unwrap_or_else(|| "-".to_owned())
        });
        let skins = species.map_or(0, |species| species.colors.len());
        for (index, row) in rows.iter().enumerate().take(PART_ROW) {
            let top = ROWS_TOP + index as f32 * ROW;
            let area = [COLUMN_X, top, COLUMN_WIDTH, ROW];
            let focused = self.sjk_row(frame, index, area, row.label());
            let control = [
                CONTROL_RIGHT - CONTROL_WIDTH,
                top + (ROW - CONTROL) * 0.5,
                CONTROL_WIDTH,
                CONTROL,
            ];
            match row {
                CharacterRow::Name => {
                    let caret = if self.name_editing { "_" } else { "" };
                    kit::field(
                        &mut self.canvas,
                        frame,
                        control,
                        format_args!("{}{caret}", self.draft.name),
                        focused || self.name_editing,
                        false,
                    );
                }
                CharacterRow::Search => {
                    if self.search.is_empty() && !self.search_editing {
                        kit::field(
                            &mut self.canvas,
                            frame,
                            control,
                            format_args!("Type a name to filter"),
                            focused,
                            false,
                        );
                    } else {
                        let caret = if self.search_editing { "_" } else { "" };
                        kit::field(
                            &mut self.canvas,
                            frame,
                            control,
                            format_args!("{}{caret}   {} found", self.search, self.tiles.len()),
                            focused || self.search_editing,
                            false,
                        );
                    }
                }
                CharacterRow::Team => {
                    kit::cycler(
                        &mut self.canvas,
                        frame,
                        control,
                        format_args!(
                            "{}",
                            crate::menu::classic::view::Sentence(
                                &self.team.label().to_ascii_lowercase()
                            )
                        ),
                        None,
                        focused,
                    );
                }
                _ => {
                    kit::cycler(
                        &mut self.canvas,
                        frame,
                        control,
                        format_args!("{model_label}"),
                        None,
                        focused,
                    );
                }
            }
            self.sjk_targets(frame, index, control, area);
        }
        let part_rows = rows.len().saturating_sub(PART_ROW);
        let part_lines = part_rows.div_ceil(2) as f32;
        let grid_top = ROWS_TOP + PART_ROW as f32 * ROW + 8.0;
        let grid_bottom = self.sjk_grid(frame, grid_top, BOTTOM - part_lines * ROW - 12.0);
        let half = (COLUMN_WIDTH - GUTTER) * 0.5;
        for (offset, row) in rows.iter().enumerate().skip(PART_ROW) {
            let local = offset - PART_ROW;
            let x = COLUMN_X + (local % 2) as f32 * (half + GUTTER);
            let top = grid_bottom + 12.0 + (local / 2) as f32 * ROW;
            let area = [x, top, half, ROW];
            let focused = self.sjk_row(frame, offset, area, row.label());
            let control = [
                x + half - 10.0 - 190.0,
                top + (ROW - CONTROL) * 0.5,
                190.0,
                CONTROL,
            ];
            match row {
                CharacterRow::Hat | CharacterRow::Cape => {
                    let slot = row.cosmetic().unwrap_or(sjk_client::CosmeticSlot::Hat);
                    let worn = self.cosmetics.worn_label(slot);
                    kit::cycler(
                        &mut self.canvas,
                        frame,
                        control,
                        format_args!("{}", worn.as_deref().unwrap_or("None")),
                        None,
                        focused,
                    );
                }
                CharacterRow::Skin => {
                    let swatch = Some(rgb_color(self.draft.rgb));
                    if skins == 0 {
                        kit::cycler(
                            &mut self.canvas,
                            frame,
                            control,
                            format_args!("-"),
                            swatch,
                            focused,
                        );
                    } else {
                        kit::cycler(
                            &mut self.canvas,
                            frame,
                            control,
                            format_args!("{} of {skins}", self.variants[3] + 1),
                            swatch,
                            focused,
                        );
                    }
                }
                part => {
                    let label = &part_labels[part.axis().unwrap_or(0).min(2)];
                    kit::cycler(
                        &mut self.canvas,
                        frame,
                        control,
                        format_args!("{label}"),
                        None,
                        focused,
                    );
                }
            }
            self.sjk_targets(frame, offset, control, area);
        }
        if let Some(line) = self.catalogue_line().map(str::to_owned) {
            text(
                &mut self.canvas,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(LABEL_X, grid_top + 8.0, COLUMN_WIDTH - 44.0, 24.0),
                17.0 * frame.s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
    }

    /// What the catalogue's state says while it is not ready.
    fn catalogue_line(&self) -> Option<&str> {
        match self.status() {
            LegacyCatalogStatus::Ready => None,
            LegacyCatalogStatus::Idle | LegacyCatalogStatus::Loading => {
                Some("Reading the character catalogue...")
            }
            LegacyCatalogStatus::Failed => Some(
                self.loader
                    .as_ref()
                    .and_then(LegacyAssetCatalogLoader::error)
                    .unwrap_or("The character catalogue is unavailable."),
            ),
        }
    }

    /// The model grid from `top`, as many rows as fit above `bottom`; returns
    /// the grid's bottom.
    fn sjk_grid(&mut self, frame: &Frame, top: f32, bottom: f32) -> f32 {
        let s = frame.s;
        let step = TILE + TILE_GAP;
        let tiles = self.tiles.len();
        let total_rows = tiles.div_ceil(COLUMNS);
        let fit = ((bottom - top + TILE_GAP) / step).floor().max(1.0) as usize;
        let visible_rows = fit.min(total_rows).max(1);
        let current = self.tile_position();
        self.grid_max_scroll = total_rows.saturating_sub(visible_rows);
        if self.grid_follow {
            self.grid_follow = false;
            let row = current.unwrap_or(0) / COLUMNS;
            if row < self.grid_scroll {
                self.grid_scroll = row;
            } else if row >= self.grid_scroll + visible_rows {
                self.grid_scroll = row + 1 - visible_rows;
            }
        }
        self.grid_scroll = self.grid_scroll.min(self.grid_max_scroll);
        let height = visible_rows as f32 * step - TILE_GAP;
        self.canvas.scroll_region(
            GRID_SCROLL_TOKEN,
            frame.rect(COLUMN_X, top, COLUMN_WIDTH, height),
        );
        self.hovered_entry = None;
        let first = self.grid_scroll * COLUMNS;
        let last = (first + visible_rows * COLUMNS)
            .min(tiles)
            .min(first + usize::from(MAX_VISIBLE_TILES));
        for slot in first..last {
            let local = slot - first;
            let x = COLUMN_X + (local % COLUMNS) as f32 * step;
            let y = top + (local / COLUMNS) as f32 * step;
            let absolute = self.tiles[slot];
            let token = TILE_BASE + local as u16;
            let hovered = self.canvas.token_hovered(token);
            if hovered {
                self.hovered_entry = Some(absolute);
            }
            let chosen = current == Some(slot);
            let rect = frame.rect(x, y, TILE, TILE);
            let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 8.0 * s,
                color: color::alpha(color::SPACE, 0.6),
            });
            match self.icons.icon(absolute) {
                Some(texture) => {
                    let _ = self.canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                        rect,
                        texture,
                        color: Color::new(1.0, 1.0, 1.0, if chosen || hovered { 1.0 } else { 0.8 }),
                    });
                }
                None if self.icons.failed(absolute) => {
                    self.tile_name(absolute, rect, 11.0 * s, color::TEXT, 0.2 * s);
                }
                None => {}
            }
            if chosen || hovered {
                let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * s,
                    width: if chosen { 2.5 * s } else { 1.5 * s },
                    color: if chosen {
                        color::GOLD_BRIGHT
                    } else {
                        Color::new(1.0, 1.0, 1.0, 0.6)
                    },
                });
            }
            self.canvas.hit_region(token, rect);
        }
        if total_rows > visible_rows {
            self.canvas.scrollbar(
                GRID_SCROLL_TOKEN,
                frame.rect(COLUMN_X + COLUMN_WIDTH + 12.0, top, 4.0, height),
                self.grid_scroll,
                visible_rows,
                total_rows,
            );
        }
        top + height
    }

    /// The saber page: style, hilt, blade colour and its channels, twice for
    /// Dual.
    fn sjk_saber(&mut self, frame: &Frame) {
        let rows = self.saber_rows();
        let catalog = catalog_of(&self.loader);
        let hilts = [false, true].map(|second| self.saber.hilt_label(catalog, second).to_owned());
        for (index, row) in rows.iter().enumerate() {
            let top = ROWS_TOP + index as f32 * ROW;
            let area = [COLUMN_X, top, COLUMN_WIDTH, ROW];
            let focused = self.sjk_row(frame, index, area, row.label());
            let middle = top + ROW * 0.5;
            let control = [
                CONTROL_RIGHT - CONTROL_WIDTH,
                middle - CONTROL * 0.5,
                CONTROL_WIDTH,
                CONTROL,
            ];
            match row {
                SaberRow::Style => {
                    let label = self.saber.style().label();
                    kit::cycler(
                        &mut self.canvas,
                        frame,
                        control,
                        format_args!(
                            "{}",
                            crate::menu::classic::view::Sentence(&label.to_ascii_lowercase())
                        ),
                        None,
                        focused,
                    );
                    self.sjk_targets(frame, index, control, area);
                }
                SaberRow::Hilt | SaberRow::SecondHilt => {
                    let label = &hilts[usize::from(row.second())];
                    kit::cycler(
                        &mut self.canvas,
                        frame,
                        control,
                        format_args!("{label}"),
                        None,
                        focused,
                    );
                    self.sjk_targets(frame, index, control, area);
                }
                SaberRow::Blade | SaberRow::SecondBlade => {
                    let custom = self.saber.custom_rgb(row.second());
                    let chips = PALETTE.map(|index| saber_color(index, custom));
                    let active = usize::from(self.saber.color(row.second()));
                    kit::chips(&mut self.canvas, frame, control, &chips, active, focused);
                    self.sjk_targets(frame, index, control, area);
                }
                _ => {
                    let Some(channel) = row.channel() else {
                        continue;
                    };
                    let value = self.saber.channel(row.second(), channel);
                    let ratio = f32::from(value) / 255.0;
                    let track_x = CONTROL_RIGHT - NUMBER_WIDTH - 18.0 - TRACK_WIDTH;
                    kit::slider(
                        &mut self.canvas,
                        frame,
                        track_x,
                        middle,
                        TRACK_WIDTH,
                        ratio,
                        focused,
                    );
                    let number = frame.rect(
                        CONTROL_RIGHT - NUMBER_WIDTH - 6.0,
                        middle - 15.0,
                        NUMBER_WIDTH + 6.0,
                        30.0,
                    );
                    match self.numeric.as_ref().filter(|edit| edit.row == index) {
                        Some(edit) => {
                            self.canvas.set_family(TextFamily::Display);
                            edit.draw(&mut self.canvas, number, 1.4 * frame.s);
                            self.canvas.set_family(TextFamily::Body);
                        }
                        None => text(
                            &mut self.canvas,
                            TextFamily::Display,
                            format_args!("{value}"),
                            number,
                            21.0 * frame.s,
                            if focused {
                                color::TEXT
                            } else {
                                color::alpha(color::TEXT, 0.9)
                            },
                            FontWeight::Regular,
                            TextAlign::End,
                        ),
                    }
                    // The track takes the pointer a little beyond its ends.
                    let track = [track_x - 10.0, middle - 16.0, TRACK_WIDTH + 20.0, 32.0];
                    self.sjk_targets(frame, index, track, area);
                    // After the row: the region registered last takes the
                    // pointer where they overlap.
                    self.canvas.hit_region(
                        crate::menu_widgets::numeric::VALUE_BASE + index as u16,
                        number,
                    );
                }
            }
        }
    }

    /// The Force page: rank and points, the two sides, the eighteen powers in
    /// two columns, then Start over, Discard and Apply.
    fn sjk_force(&mut self, frame: &Frame) {
        let s = frame.s;
        let remaining = self.force.remaining_points();
        let rank = self.force.allocation().rank;
        let dirty = self.force.is_dirty();
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!("Rank {rank}, {remaining} points left"),
            frame.rect(LABEL_X, ROWS_TOP, 400.0, 24.0),
            18.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        if dirty {
            text(
                &mut self.canvas,
                TextFamily::Body,
                format_args!("Not applied yet"),
                frame.rect(CONTROL_RIGHT - 300.0, ROWS_TOP, 300.0, 24.0),
                18.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Regular,
                TextAlign::End,
            );
        }
        // The two sides.
        let side_top = ROWS_TOP + 36.0;
        let picker = [COLUMN_X, side_top, COLUMN_WIDTH, SIDE_HEIGHT];
        if self.selected == FORCE_SIDE_ROW {
            kit::band(
                &mut self.canvas,
                frame,
                [
                    COLUMN_X - 6.0,
                    side_top - 6.0,
                    COLUMN_WIDTH + 12.0,
                    SIDE_HEIGHT + 12.0,
                ],
            );
        }
        let chosen = self.force.allocation().side;
        let half = (COLUMN_WIDTH - GUTTER) * 0.5;
        for (index, (side, label, tint)) in [
            (ForceSide::Light, "Light side", color::HOLO),
            (ForceSide::Dark, "Dark side", color::EMBER),
        ]
        .into_iter()
        .enumerate()
        {
            let x = COLUMN_X + index as f32 * (half + GUTTER);
            let card = frame.rect(x, side_top, half, SIDE_HEIGHT);
            let picked = side == chosen;
            let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: card,
                radius: 12.0 * s,
                color: if picked {
                    color::alpha(tint, 0.16)
                } else {
                    color::alpha(color::SPACE, 0.5)
                },
            });
            let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
                rect: card,
                radius: 12.0 * s,
                width: if picked { 2.0 * s } else { 1.5 * s },
                color: color::alpha(tint, if picked { 0.85 } else { 0.25 }),
            });
            let emblem = side_texture(side);
            let mut label_x = x + 22.0;
            if self.force_icons.is_texture_ready(emblem) {
                let _ = self.canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                    rect: frame.rect(
                        x + 12.0,
                        side_top + 10.0,
                        SIDE_HEIGHT - 20.0,
                        SIDE_HEIGHT - 20.0,
                    ),
                    texture: emblem,
                    color: Color::new(1.0, 1.0, 1.0, if picked { 1.0 } else { 0.4 }),
                });
                label_x = x + SIDE_HEIGHT + 4.0;
            }
            text(
                &mut self.canvas,
                TextFamily::Display,
                format_args!("{label}"),
                frame.rect(
                    label_x,
                    side_top + (SIDE_HEIGHT - 32.0) * 0.5,
                    half - (label_x - x) - 12.0,
                    32.0,
                ),
                26.0 * s,
                if picked { color::TEXT } else { color::MUTED },
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        self.canvas.hit_region(
            FORCE_SIDE_ROW as u16,
            frame.rect(picker[0], picker[1], picker[2], picker[3]),
        );
        // The powers.
        let powers_top = side_top + SIDE_HEIGHT + 18.0;
        for (index, name) in POWER_NAMES.iter().enumerate() {
            let row = FORCE_POWER_ROW + index;
            let x = COLUMN_X + (index % 2) as f32 * (half + GUTTER);
            let top = powers_top + (index / 2) as f32 * CELL;
            let area = [x, top, half, CELL];
            let focused = self.selected == row;
            let available = self.force.is_available(index);
            if focused {
                kit::band(&mut self.canvas, frame, area);
            }
            let icon = power_texture(index);
            let mut name_x = x + 14.0;
            if self.force_icons.is_texture_ready(icon) {
                let _ = self.canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                    rect: frame.rect(x + 10.0, top + 6.0, CELL - 12.0, CELL - 12.0),
                    texture: icon,
                    color: Color::new(1.0, 1.0, 1.0, if available { 0.95 } else { 0.25 }),
                });
                name_x = x + CELL + 4.0;
            }
            text(
                &mut self.canvas,
                TextFamily::Body,
                format_args!("{}", SentenceCase(name)),
                frame.rect(name_x, top + (CELL - 24.0) * 0.5, 160.0, 24.0),
                17.0 * s,
                match (available, focused) {
                    (false, _) => color::QUIET,
                    (true, true) => Color::new(1.0, 1.0, 1.0, 1.0),
                    (true, false) => color::alpha(color::TEXT, 0.88),
                },
                FontWeight::Regular,
                TextAlign::Start,
            );
            let level = self.force.allocation().levels[index];
            let control = [x + half - 124.0, top + 6.0, 116.0, CELL - 12.0];
            let tint = if focused && available {
                color::TEXT
            } else {
                color::MUTED
            };
            let middle = top + CELL * 0.5;
            kit::caret_mark(
                &mut self.canvas,
                frame,
                control[0] + 12.0,
                middle,
                true,
                tint,
            );
            kit::caret_mark(
                &mut self.canvas,
                frame,
                control[0] + control[2] - 12.0,
                middle,
                false,
                tint,
            );
            kit::pips(
                &mut self.canvas,
                frame,
                control[0] + 30.0,
                middle,
                level,
                3,
                available,
            );
            self.sjk_targets(frame, row, control, area);
        }
        // The actions.
        let actions_top = powers_top + 9.0 * CELL + 20.0;
        let width = (COLUMN_WIDTH - 2.0 * 12.0) / 3.0;
        for (slot, (row, label, enabled, primary)) in [
            (FORCE_RESET_ROW, "Start over", true, false),
            (FORCE_DISCARD_ROW, "Discard", dirty, false),
            (
                FORCE_APPLY_ROW,
                if dirty { "Apply" } else { "Applied" },
                dirty,
                true,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            kit::button(
                &mut self.canvas,
                frame,
                [
                    COLUMN_X + slot as f32 * (width + 12.0),
                    actions_top,
                    width,
                    48.0,
                ],
                label,
                primary,
                enabled,
                self.selected == row,
                row as u16,
            );
        }
    }

    /// Beside the model, as in a gallery: what the page shows of it.
    fn sjk_caption(&mut self, frame: &Frame) {
        let s = frame.s;
        let (title, detail) = match self.page {
            ProfilePage::Character => {
                let (model, skin) = self
                    .draft
                    .model
                    .split_once('/')
                    .unwrap_or((&self.draft.model, "default"));
                (
                    crate::menu::classic::view::Sentence(model).to_string(),
                    format!("{} skin", crate::menu::classic::view::Sentence(skin)),
                )
            }
            ProfilePage::Saber => (
                crate::menu::classic::view::Sentence(
                    &self.saber.style().label().to_ascii_lowercase(),
                )
                .to_string(),
                format!(
                    "{}, {}",
                    self.saber.hilt_label(catalog_of(&self.loader), false),
                    blade_word(self.saber.color(false))
                ),
            ),
            ProfilePage::Force => (
                match self.force.allocation().side {
                    ForceSide::Light => "Light side".to_owned(),
                    ForceSide::Dark => "Dark side".to_owned(),
                },
                format!("Rank {}", self.force.allocation().rank),
            ),
        };
        text(
            &mut self.canvas,
            TextFamily::Display,
            format_args!("{title}"),
            frame.rect(1_224.0, 860.0, 600.0, 44.0),
            36.0 * s,
            color::TEXT,
            FontWeight::Regular,
            TextAlign::End,
        );
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!("{detail}"),
            frame.rect(1_224.0, 904.0, 600.0, 26.0),
            18.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::End,
        );
        let _ = self.canvas.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(1_824.0 - 120.0, 850.0, 120.0, 1.5),
            color: color::alpha(color::GOLD, 0.7),
        });
    }

    /// The keys of what has the keyboard, right-aligned at the bottom.
    fn sjk_keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let mut keys: Vec<(&[&str], &str)> = Vec::with_capacity(4);
        if self.name_editing || self.search_editing || self.numeric.is_some() {
            keys.extend([(&["Enter"][..], "done"), (&["Esc"][..], "cancel")]);
        } else {
            let row = self.sjk_row_kind();
            keys.push(match row {
                RowKind::Type => (&["Enter"][..], "type"),
                RowKind::Act => (&["Enter"][..], "do it"),
                RowKind::Step => (&["Left", "Right"][..], "change"),
            });
            let next = TABS[(self.page.index() + 1) % TABS.len()];
            keys.push((&["Tab"][..], next));
        }
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * keys.len().saturating_sub(1) as f32;
        let [right, y] = frame.point(1_824.0, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.canvas, caps, action, x, y, s) + gap;
        }
    }

    /// What kind of row has the keyboard.
    fn sjk_row_kind(&self) -> RowKind {
        match self.page {
            ProfilePage::Character => match self.character_rows().get(self.selected) {
                Some(CharacterRow::Name | CharacterRow::Search) => RowKind::Type,
                _ => RowKind::Step,
            },
            ProfilePage::Saber => RowKind::Step,
            ProfilePage::Force if self.selected >= FORCE_RESET_ROW => RowKind::Act,
            ProfilePage::Force => RowKind::Step,
        }
    }

    /// Where a slider row's track lies (window pixels), for the pointer:
    /// its rectangle is the track's ([`Self::sjk_targets`]).
    pub(super) fn sjk_slider_ratio(rect: sjk_ui::Rect, x: f32) -> f32 {
        // The registered rectangle runs 10 frame pixels past each end.
        let inset = rect.width * 10.0 / (TRACK_WIDTH + 20.0);
        ((x - rect.x - inset) / (rect.width - 2.0 * inset)).clamp(0.0, 1.0)
    }
}

/// What a row does with the keyboard, for the keys' line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RowKind {
    Type,
    Step,
    Act,
}

/// A power's name in sentence case ("Mind trick").
struct SentenceCase<'a>(&'a str);

impl std::fmt::Display for SentenceCase<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut words = self.0.split(' ');
        if let Some(first) = words.next() {
            formatter.write_str(first)?;
        }
        for word in words {
            write!(formatter, " {}", word.to_lowercase())?;
        }
        Ok(())
    }
}

/// A `color1` value's blade as words ("blue blade").
fn blade_word(index: u8) -> &'static str {
    match sjk_client::SaberColor::from_index(index) {
        Ok(sjk_client::SaberColor::Red) => "red blade",
        Ok(sjk_client::SaberColor::Orange) => "orange blade",
        Ok(sjk_client::SaberColor::Yellow) => "yellow blade",
        Ok(sjk_client::SaberColor::Green) => "green blade",
        Ok(sjk_client::SaberColor::Purple) => "purple blade",
        Ok(sjk_client::SaberColor::Rgb) => "custom blade",
        _ => "blue blade",
    }
}

fn rgb_color(rgb: [u8; 3]) -> Color {
    Color::new(
        f32::from(rgb[0]) / 255.0,
        f32::from(rgb[1]) / 255.0,
        f32::from(rgb[2]) / 255.0,
        1.0,
    )
}

/// Dark behind the form on the left, fading out before the model; light
/// fades at the top behind the title and at the bottom behind the keys.
fn scrims(canvas: &mut crate::menu_widgets::MenuCanvas, viewport: [f32; 2], frame: &Frame) {
    let [width, height] = viewport;
    let space = |alpha| color::alpha(color::SPACE, alpha);
    let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
    fade_across(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, x(820.0), height),
        space(0.9),
        space(0.78),
    );
    fade_across(
        canvas,
        sjk_ui::Rect::new(x(820.0), 0.0, x(1_120.0) - x(820.0), height),
        space(0.78),
        space(0.0),
    );
    let y = |frame_y: f32| frame.point(0.0, frame_y)[1];
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, width, y(170.0)),
        space(0.55),
        space(0.0),
    );
    fade(
        canvas,
        sjk_ui::Rect::new(0.0, y(840.0), width, height - y(840.0)),
        space(0.0),
        space(0.7),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // The grid's eight tiles fill the column.
    const _: () = assert!(TILE > 70.0 && TILE < 80.0);
    // The longest page (Dual sabers, eleven rows; Force's actions) ends above the keys.
    const _: () = assert!(ROWS_TOP + 11.0 * ROW < BOTTOM);
    const _: () = assert!(ROWS_TOP + 36.0 + SIDE_HEIGHT + 18.0 + 9.0 * CELL + 20.0 + 48.0 < BOTTOM);

    #[test]
    fn power_names_read_in_sentence_case() {
        assert_eq!(SentenceCase("Mind Trick").to_string(), "Mind trick");
        assert_eq!(SentenceCase("Heal").to_string(), "Heal");
        assert_eq!(SentenceCase("Team Energize").to_string(), "Team energize");
    }

    #[test]
    fn a_sliders_pointer_maps_its_track_ends_to_the_range_ends() {
        let track = sjk_ui::Rect::new(100.0, 0.0, TRACK_WIDTH + 20.0, 32.0);
        assert_eq!(PlayerMenu::sjk_slider_ratio(track, 110.0), 0.0);
        assert_eq!(
            PlayerMenu::sjk_slider_ratio(track, 110.0 + TRACK_WIDTH),
            1.0
        );
        assert!(
            (PlayerMenu::sjk_slider_ratio(track, 110.0 + TRACK_WIDTH * 0.5) - 0.5).abs() < 1e-4
        );
        assert_eq!(PlayerMenu::sjk_slider_ratio(track, 0.0), 0.0);
    }
}
