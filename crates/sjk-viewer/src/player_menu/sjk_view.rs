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

use super::force::{NextLevel, POWER_NAMES, POWER_NOTES};
use super::force_icons::{power_texture, side_texture};
use super::grid::{COLUMNS, GRID_SCROLL_TOKEN, MAX_VISIBLE_TILES, PART_ROW, TILE_BASE};
use super::rows::{
    CharacterRow, FORCE_APPLY_ROW, FORCE_DISCARD_ROW, FORCE_POWER_ROW, FORCE_RESET_ROW,
    FORCE_SIDE_ROW, SaberRow, saber_color,
};
use super::saber::{PALETTE, SaberStyle};
use super::*;
use crate::menu::sjk::{
    Frame, TextTarget, color, fade, fade_across, key_hint, key_hint_width, kit, text, wrap,
};
use crate::menu_widgets::{BACK_TOKEN, TAB_BASE, TextFamily};
use sjk_client::{ForcePower, ForceSide, LegacyCatalogStatus};
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
/// The points meter's line, a group's sub-heading and the gap after a group.
const METER_Y: f32 = 250.0;
const GROUP_HEADING: f32 = 34.0;
const GROUP_GAP: f32 = 14.0;
/// A level's priced cell and the gap between two.
const LEVEL_CELL: [f32; 2] = [40.0, 30.0];
const LEVEL_GAP: f32 = 6.0;
/// The box at the bottom right showing the power under the pointer.
const POWER_BOX: Area = [1_384.0, 640.0, 440.0, 300.0];
/// Level `l` (1 to 3) of power `p` answers to `LEVEL_BASE + p * 3 + l - 1`.
const LEVEL_BASE: u16 = 1_000;
/// The saber styles as the Style row's buttons offer them.
const STYLES: [SaberStyle; 3] = [SaberStyle::Single, SaberStyle::Staff, SaberStyle::Dual];
/// Style button `i` answers to `STYLE_BASE + i`.
const STYLE_BASE: u16 = 1_080;
/// The hilt lists' wheel areas: the first saber's, then the second's.
const HILT_SCROLL: u16 = 1_090;
/// Hilt `i` of the first saber's list answers to `HILT_BASE + i`, of the
/// second's to `HILT_BASE + HILT_STRIDE + i`.
const HILT_BASE: u16 = 1_100;
const HILT_STRIDE: u16 = 300;
/// A hilt list's sub-heading, its rows' height and how many show.
const HILT_HEAD: f32 = 32.0;
const HILT_ROW: f32 = 34.0;
const HILT_ROWS: usize = 6;

/// The pointer token of hilt `index` in the first saber's list, or the
/// second's.
fn hilt_token(second: bool, index: usize) -> u16 {
    HILT_BASE
        + if second { HILT_STRIDE } else { 0 }
        + index.min(usize::from(HILT_STRIDE) - 1) as u16
}

/// Which list (the second saber's or not) and which hilt a hilt token names.
fn hilt_of(token: u16) -> Option<(bool, usize)> {
    let offset = token.checked_sub(HILT_BASE)?;
    match offset / HILT_STRIDE {
        0 => Some((false, usize::from(offset))),
        1 => Some((true, usize::from(offset - HILT_STRIDE))),
        _ => None,
    }
}

/// What a hilt list keeps between frames: its first line on show, and the
/// hilt it last showed chosen, so a newly chosen one is brought into view.
#[derive(Debug, Default)]
pub(super) struct HiltList {
    first: usize,
    chosen: String,
    /// The columns it was laid out in (two for Single and Staff, one each
    /// for Dual's): a change lays it out afresh.
    columns: usize,
}

/// The pointer token of level `level` (1 to 3) of power `power`.
fn level_token(power: usize, level: u8) -> u16 {
    LEVEL_BASE + (power * 3) as u16 + u16::from(level.saturating_sub(1))
}

/// The power and level a level token names.
pub(super) fn level_of(token: u16) -> Option<(usize, u8)> {
    let offset = token.checked_sub(LEVEL_BASE)?;
    let power = usize::from(offset / 3);
    (power < POWER_NAMES.len()).then_some((power, (offset % 3) as u8 + 1))
}

/// One of the Force page's groups: its sub-heading, its powers in classic+'s
/// order, and where it sits (column 0 or 1, the first or second block).
struct ForceGroup {
    name: &'static str,
    powers: &'static [u8],
    column: usize,
    line: usize,
}

/// The Force page's groups as classic+ groups them: Neutral, the side's own
/// five (the other side's are left out, as there), Lightsaber.
fn force_groups(side: ForceSide) -> [ForceGroup; 3] {
    use super::classic::layout::{DARK_POWERS, LIGHT_POWERS, NEUTRAL_POWERS, SABER_POWERS};
    let (name, powers): (&str, &'static [u8]) = match side {
        ForceSide::Light => ("Light side", &LIGHT_POWERS),
        ForceSide::Dark => ("Dark side", &DARK_POWERS),
    };
    [
        ForceGroup {
            name: "Neutral",
            powers: &NEUTRAL_POWERS,
            column: 0,
            line: 0,
        },
        ForceGroup {
            name,
            powers,
            column: 1,
            line: 0,
        },
        ForceGroup {
            name: "Lightsaber",
            powers: &SABER_POWERS,
            column: 0,
            line: 1,
        },
    ]
}

/// The Force page's rows in the order the keys go through them: the sides,
/// the groups' powers (Neutral, the side's, Lightsaber), then the actions.
pub(super) fn force_key_order(side: ForceSide) -> Vec<usize> {
    let mut order = vec![FORCE_SIDE_ROW];
    for group in force_groups(side) {
        order.extend(
            group
                .powers
                .iter()
                .map(|power| FORCE_POWER_ROW + usize::from(*power)),
        );
    }
    order.extend([FORCE_RESET_ROW, FORCE_DISCARD_ROW, FORCE_APPLY_ROW]);
    order
}

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

    /// The SJK UI's own pointer targets, before the modern screen's:
    /// - a level cell chooses its power's row when hovered and, clicked, sets
    ///   the power to that level (its own level steps it down one, as
    ///   classic+'s stars do);
    /// - a style button chooses the style;
    /// - a hilt chooses that hilt, the wheel over a list scrolls it, and a
    ///   click on a list beside its hilts only chooses its row.
    ///
    /// `None` leaves the event to the modern screen.
    pub(super) fn sjk_pointer(
        &mut self,
        kind: sjk_ui::UiEventKind,
        token: u16,
        wheel: f32,
        console: &mut ViewerConsole,
    ) -> Option<PlayerMenuResult> {
        use sjk_ui::UiEventKind;
        let hover = matches!(kind, UiEventKind::HoverEnter | UiEventKind::Hover);
        let activate = kind == UiEventKind::Activate;
        match self.page {
            ProfilePage::Force => {
                let (power, level) = level_of(token)?;
                let row = FORCE_POWER_ROW + power;
                if hover {
                    self.selected = row;
                } else if activate {
                    self.selected = row;
                    let current = self.force.allocation().levels[power];
                    let wanted = if level == current { level - 1 } else { level };
                    self.force.set_level(power, wanted);
                } else {
                    return None;
                }
            }
            ProfilePage::Saber => {
                let hilt_row = |second: bool| {
                    self.saber_rows().iter().position(|row| {
                        *row == if second {
                            SaberRow::SecondHilt
                        } else {
                            SaberRow::Hilt
                        }
                    })
                };
                if let Some(style) = token
                    .checked_sub(STYLE_BASE)
                    .and_then(|index| STYLES.get(usize::from(index)))
                {
                    if hover {
                        self.selected = 0;
                    } else if activate {
                        self.selected = 0;
                        self.saber.set_style(*style, catalog_of(&self.loader));
                        self.saber.apply(console);
                    } else {
                        return None;
                    }
                } else if let Some(list) = token
                    .checked_sub(HILT_SCROLL)
                    .filter(|list| *list < 2)
                    .map(usize::from)
                    .or_else(|| hilt_of(token).map(|(second, _)| usize::from(second)))
                    .filter(|_| kind == UiEventKind::Wheel)
                {
                    let state = &mut self.sjk_hilts[list];
                    state.first = if wheel > 0.0 {
                        state.first.saturating_sub(1)
                    } else {
                        state.first + 1
                    };
                } else if let Some((second, index)) = hilt_of(token) {
                    let row = hilt_row(second)?;
                    if hover {
                        self.selected = row;
                    } else if activate {
                        self.selected = row;
                        if let Some(catalog) = catalog_of(&self.loader) {
                            self.saber.select(catalog, index, second);
                            self.saber.apply(console);
                        }
                    } else {
                        return None;
                    }
                } else if activate
                    && [hilt_row(false), hilt_row(true)].contains(&Some(usize::from(token)))
                {
                    self.selected = usize::from(token);
                } else {
                    return None;
                }
            }
            ProfilePage::Character => return None,
        }
        Some(PlayerMenuResult::None)
    }

    /// The rows of the Saber and Force pages in the order they show them
    /// (`None` on the Character page, whose rows go in their own order): the
    /// Saber page's hilt lists before its blades (Dual's two side by side),
    /// the Force page's groups ([`force_key_order`]).
    fn sjk_key_order(&self) -> Option<Vec<usize>> {
        match self.page {
            ProfilePage::Character => None,
            ProfilePage::Force => Some(force_key_order(self.force.allocation().side)),
            ProfilePage::Saber => {
                let rows = self.saber_rows();
                let first = |row: &SaberRow| {
                    !matches!(row, SaberRow::Style | SaberRow::Hilt | SaberRow::SecondHilt)
                };
                Some(
                    rows.iter()
                        .enumerate()
                        .filter(|(_, row)| !first(row))
                        .chain(rows.iter().enumerate().filter(|(_, row)| first(row)))
                        .map(|(index, _)| index)
                        .collect(),
                )
            }
        }
    }

    /// Up (`direction` -1) or Down on the Saber or Force page: the next row
    /// in the order the page shows them; false on the Character page.
    pub(super) fn sjk_step(&mut self, direction: isize) -> bool {
        let Some(order) = self.sjk_key_order() else {
            return false;
        };
        let next = match order.iter().position(|row| *row == self.selected) {
            Some(at) => super::controller::wrap(at, direction, order.len()),
            None if direction < 0 => order.len() - 1,
            None => 0,
        };
        self.selected = order[next];
        true
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

    /// The saber page: the style as three buttons, the hilts as a list (two
    /// side by side for Dual: the right hand's and the left's), then each
    /// blade's colour and its channels.
    fn sjk_saber(&mut self, frame: &Frame) {
        let rows = self.saber_rows();
        let style = self.saber.style();
        // The style: the row, then its three buttons over it.
        let style_area = [COLUMN_X, ROWS_TOP, COLUMN_WIDTH, ROW];
        let focused = self.sjk_row(frame, 0, style_area, SaberRow::Style.label());
        let middle = ROWS_TOP + ROW * 0.5;
        let control = [
            CONTROL_RIGHT - CONTROL_WIDTH,
            middle - CONTROL * 0.5,
            CONTROL_WIDTH,
            CONTROL,
        ];
        self.sjk_targets(frame, 0, control, style_area);
        let current = STYLES.iter().position(|each| *each == style).unwrap_or(0);
        kit::segments(
            &mut self.canvas,
            frame,
            CONTROL_RIGHT,
            middle,
            &STYLES.map(SaberStyle::label),
            current,
            focused,
            STYLE_BASE,
        );
        // The hilts.
        let lists_top = ROWS_TOP + ROW + 8.0;
        let half = (COLUMN_WIDTH - GUTTER) * 0.5;
        let height = HILT_HEAD + HILT_ROWS as f32 * HILT_ROW + 12.0;
        if style == SaberStyle::Dual {
            for (second, x, heading) in [
                (false, COLUMN_X, "Right hand"),
                (true, COLUMN_X + half + GUTTER, "Left hand"),
            ] {
                self.sjk_hilt_list(frame, second, [x, lists_top, half, height], 1, heading);
            }
        } else {
            let heading = if style == SaberStyle::Staff {
                "Staff"
            } else {
                "Hilt"
            };
            self.sjk_hilt_list(
                frame,
                false,
                [COLUMN_X, lists_top, COLUMN_WIDTH, height],
                2,
                heading,
            );
        }
        // The blades.
        let mut top = lists_top + height + 10.0;
        for (index, row) in rows.iter().enumerate() {
            if matches!(row, SaberRow::Style | SaberRow::Hilt | SaberRow::SecondHilt) {
                continue;
            }
            let area = [COLUMN_X, top, COLUMN_WIDTH, ROW];
            let focused = self.sjk_row(frame, index, area, row.label());
            let middle = top + ROW * 0.5;
            let control = [
                CONTROL_RIGHT - CONTROL_WIDTH,
                middle - CONTROL * 0.5,
                CONTROL_WIDTH,
                CONTROL,
            ];
            top += ROW;
            if row.is_blade() {
                let custom = self.saber.custom_rgb(row.second());
                let chips = PALETTE.map(|index| saber_color(index, custom));
                let active = usize::from(self.saber.color(row.second()));
                kit::chips(&mut self.canvas, frame, control, &chips, active, focused);
                self.sjk_targets(frame, index, control, area);
                continue;
            }
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
            // After the row: the region registered last takes the pointer
            // where they overlap.
            self.canvas.hit_region(
                crate::menu_widgets::numeric::VALUE_BASE + index as u16,
                number,
            );
        }
    }

    /// The hilts the first saber's list (`second` false) or the second's
    /// offers, in the catalogue's order (the game's load order).
    fn sjk_hilt_names(&self, second: bool) -> Vec<(String, String)> {
        let style = if second {
            SaberStyle::Single
        } else {
            self.saber.style()
        };
        catalog_of(&self.loader).map_or_else(Vec::new, |catalog| {
            catalog
                .saber_hilts
                .iter()
                .filter(|hilt| super::saber::allowed(hilt, style))
                .map(|hilt| (hilt.name.clone(), hilt.display_name.clone()))
                .collect()
        })
    }

    /// One hilt list on `area` under its `heading`: the hilts in `columns`
    /// columns, row after row, the chosen one gold with a dot, scrolled to
    /// keep a newly chosen one in view; the wheel scrolls it, a click
    /// chooses.
    fn sjk_hilt_list(
        &mut self,
        frame: &Frame,
        second: bool,
        area: Area,
        columns: usize,
        heading: &str,
    ) {
        let s = frame.s;
        let row = self
            .saber_rows()
            .iter()
            .position(|each| {
                *each
                    == if second {
                        SaberRow::SecondHilt
                    } else {
                        SaberRow::Hilt
                    }
            })
            .unwrap_or(1);
        let [x, y, width, height] = area;
        if self.selected == row {
            kit::band(&mut self.canvas, frame, area);
        }
        // The whole list first: the hilts, registered after, take the pointer.
        self.canvas
            .hit_region(row as u16, frame.rect(x, y, width, height));
        let list = usize::from(second);
        self.canvas
            .scroll_region(HILT_SCROLL + list as u16, frame.rect(x, y, width, height));
        kit::heading(
            &mut self.canvas,
            frame,
            x + 22.0,
            y + HILT_HEAD * 0.5,
            width - 34.0,
            heading,
        );
        let names = self.sjk_hilt_names(second);
        let chosen = names
            .iter()
            .position(|(name, _)| name.eq_ignore_ascii_case(self.saber.hilt(second)));
        let lines = names.len().div_ceil(columns);
        let state = &mut self.sjk_hilts[list];
        // A hilt chosen since the last frame (by the keys, say) comes into
        // view; the first time the list shows, in its middle.
        if state.columns != columns || !state.chosen.eq_ignore_ascii_case(self.saber.hilt(second)) {
            let first_show = state.chosen.is_empty() || state.columns != columns;
            state.columns = columns;
            state.chosen.clear();
            state.chosen.push_str(self.saber.hilt(second));
            if let Some(line) = chosen.map(|index| index / columns) {
                if first_show {
                    state.first = line.saturating_sub(HILT_ROWS / 2);
                } else if line < state.first {
                    state.first = line;
                } else if line >= state.first + HILT_ROWS {
                    state.first = line + 1 - HILT_ROWS;
                }
            }
        }
        state.first = state.first.min(lines.saturating_sub(HILT_ROWS));
        let first = state.first;
        if names.is_empty() {
            text(
                &mut self.canvas,
                TextFamily::Body,
                format_args!("No hilts found"),
                frame.rect(x + 22.0, y + HILT_HEAD + 6.0, width - 44.0, HILT_ROW),
                17.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let cell_width = (width - 24.0) / columns as f32;
        for line in first..(first + HILT_ROWS).min(lines) {
            for column in 0..columns {
                let index = line * columns + column;
                let Some((_, label)) = names.get(index) else {
                    break;
                };
                let cell = [
                    x + 12.0 + column as f32 * cell_width,
                    y + HILT_HEAD + 6.0 + (line - first) as f32 * HILT_ROW,
                    cell_width - 6.0,
                    HILT_ROW - 2.0,
                ];
                let rect = frame.rect(cell[0], cell[1], cell[2], cell[3]);
                let token = hilt_token(second, index);
                let picked = chosen == Some(index);
                let hovered = self.canvas.token_hovered(token);
                if picked || hovered {
                    let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect,
                        radius: 8.0 * s,
                        color: color::alpha(
                            if picked { color::GOLD } else { color::HOLO },
                            if picked { 0.16 } else { 0.1 },
                        ),
                    });
                }
                if picked {
                    kit::changed_dot(
                        &mut self.canvas,
                        frame,
                        cell[0] + cell[2] - 16.0,
                        cell[1] + cell[3] * 0.5,
                    );
                }
                text(
                    &mut self.canvas,
                    TextFamily::Body,
                    format_args!("{label}"),
                    frame.rect(cell[0] + 12.0, cell[1], cell[2] - 40.0, cell[3]),
                    17.0 * s,
                    match (picked, hovered) {
                        (true, _) => color::GOLD_BRIGHT,
                        (false, true) => color::TEXT,
                        (false, false) => color::alpha(color::TEXT, 0.82),
                    },
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                self.canvas.hit_region(token, rect);
            }
        }
        if lines > HILT_ROWS {
            self.canvas.scrollbar(
                HILT_SCROLL + list as u16,
                frame.rect(
                    x + width - 8.0,
                    y + HILT_HEAD + 6.0,
                    3.0,
                    HILT_ROWS as f32 * HILT_ROW,
                ),
                first,
                HILT_ROWS,
                lines,
            );
        }
    }

    /// The Force page: mastery and points over their meter, the two sides,
    /// the powers grouped as classic+ groups them (Neutral and Lightsaber on
    /// the left, the side's five on the right), each with its three levels
    /// priced, then Start over, Discard and Apply.
    fn sjk_force(&mut self, frame: &Frame) {
        let s = frame.s;
        let remaining = self.force.remaining_points();
        let total = self.force.total_points();
        let rank = self.force.allocation().rank;
        let dirty = self.force.is_dirty();
        let hovered = self.sjk_hovered_level();
        text(
            &mut self.canvas,
            TextFamily::Display,
            format_args!("{}", super::classic::force_page::mastery(rank)),
            frame.rect(COLUMN_X, ROWS_TOP - 6.0, 400.0, 32.0),
            26.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!(
                "{remaining} of {total} points left{}",
                if dirty { ", not applied yet" } else { "" }
            ),
            frame.rect(CONTROL_RIGHT - 420.0, ROWS_TOP - 4.0, 420.0, 28.0),
            17.0 * s,
            if dirty {
                color::GOLD_BRIGHT
            } else {
                color::MUTED
            },
            FontWeight::Regular,
            TextAlign::End,
        );
        self.sjk_points_meter(frame, remaining, total, hovered);
        // The two sides.
        let side_top = METER_Y + 22.0;
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
        // The powers, in classic+'s groups.
        let groups_top = side_top + SIDE_HEIGHT + 22.0;
        for group in force_groups(chosen) {
            let x = COLUMN_X + group.column as f32 * (half + GUTTER);
            let top = groups_top + group.line as f32 * (GROUP_HEADING + 5.0 * CELL + GROUP_GAP);
            kit::heading(
                &mut self.canvas,
                frame,
                x,
                top + GROUP_HEADING * 0.5,
                half,
                group.name,
            );
            for (slot, power) in group.powers.iter().enumerate() {
                let row_top = top + GROUP_HEADING + slot as f32 * CELL;
                self.sjk_power(
                    frame,
                    usize::from(*power),
                    [x, row_top, half, CELL],
                    hovered,
                );
            }
        }
        // The actions.
        // Under the Lightsaber group, the left column's second.
        let actions_top = groups_top
            + (GROUP_HEADING + 5.0 * CELL + GROUP_GAP)
            + GROUP_HEADING
            + 3.0 * CELL
            + 24.0;
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

    /// The points left as a gold bar under the page's first line; a hovered
    /// level shows what it would take from it (white), or the whole bar turns
    /// ember when there are not enough.
    fn sjk_points_meter(
        &mut self,
        frame: &Frame,
        remaining: u16,
        total: u16,
        hovered: Option<(usize, u8)>,
    ) {
        let s = frame.s;
        let total = total.max(1);
        let share = |points: u16| COLUMN_WIDTH * f32::from(points.min(total)) / f32::from(total);
        let bar = |canvas: &mut crate::menu_widgets::MenuCanvas, x: f32, width: f32, colour| {
            if width > 0.0 {
                let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(x, METER_Y, width, 5.0),
                    radius: 2.5 * s,
                    color: colour,
                });
            }
        };
        bar(
            &mut self.canvas,
            COLUMN_X,
            COLUMN_WIDTH,
            color::alpha(color::HOLO, 0.16),
        );
        let left = share(remaining);
        let cost = hovered.map_or(0, |(power, level)| self.force.cost_to(power, level));
        if cost > remaining {
            bar(
                &mut self.canvas,
                COLUMN_X,
                left,
                color::alpha(color::EMBER, 0.9),
            );
        } else {
            bar(&mut self.canvas, COLUMN_X, left, color::GOLD);
            let taken = share(cost);
            bar(
                &mut self.canvas,
                COLUMN_X + left - taken,
                taken,
                Color::new(1.0, 1.0, 1.0, 0.85),
            );
        }
    }

    /// The level cell under the pointer: its power and level.
    fn sjk_hovered_level(&self) -> Option<(usize, u8)> {
        (0..POWER_NAMES.len())
            .flat_map(|power| (1..=3).map(move |level| (power, level)))
            .find(|(power, level)| self.canvas.token_hovered(level_token(*power, *level)))
    }

    /// Power `index` on `area`: its holocron and name, then its three levels
    /// as cells priced as classic+ prices them, the bought ones gold; a
    /// hovered level previews what buying up to it would take.
    fn sjk_power(&mut self, frame: &Frame, index: usize, area: Area, hovered: Option<(usize, u8)>) {
        let s = frame.s;
        let Some(power) = ForcePower::ALL.get(index).copied() else {
            return;
        };
        let [x, top, width, height] = area;
        let row = FORCE_POWER_ROW + index;
        let focused = self.selected == row;
        let level = self.force.allocation().levels[index];
        let usable = level > 0
            || !matches!(
                self.force.next_level(index),
                NextLevel::OtherSide | NextLevel::TeamOnly | NextLevel::NeedsOffense
            );
        if focused {
            kit::band(&mut self.canvas, frame, area);
        }
        // The whole row first: the cells, registered after, take the pointer.
        self.canvas
            .hit_region(row as u16, frame.rect(x, top, width, height));
        let icon = power_texture(index);
        let mut name_x = x + 14.0;
        if self.force_icons.is_texture_ready(icon) {
            let _ = self.canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(x + 10.0, top + 6.0, height - 12.0, height - 12.0),
                texture: icon,
                color: Color::new(1.0, 1.0, 1.0, if usable { 0.95 } else { 0.3 }),
            });
            name_x = x + height + 4.0;
        }
        let cells_x = x + width - 8.0 - 3.0 * LEVEL_CELL[0] - 2.0 * LEVEL_GAP;
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!("{}", SentenceCase(POWER_NAMES[index])),
            frame.rect(name_x, top, cells_x - name_x - 6.0, height),
            17.0 * s,
            match (usable, focused) {
                (false, _) => color::QUIET,
                (true, true) => Color::new(1.0, 1.0, 1.0, 1.0),
                (true, false) => color::alpha(color::TEXT, 0.88),
            },
            if level > 0 {
                FontWeight::Semibold
            } else {
                FontWeight::Regular
            },
            TextAlign::Start,
        );
        let target = hovered
            .filter(|(hovered_power, _)| *hovered_power == index)
            .map(|(_, level)| level);
        let affordable = target.is_some_and(|target| {
            self.force.cost_to(index, target) <= self.force.remaining_points()
        });
        let free_saber = self.force.free_saber();
        let middle = top + height * 0.5;
        for cell_level in 1..=3_u8 {
            let cell = [
                cells_x + f32::from(cell_level - 1) * (LEVEL_CELL[0] + LEVEL_GAP),
                middle - LEVEL_CELL[1] * 0.5,
                LEVEL_CELL[0],
                LEVEL_CELL[1],
            ];
            let rect = frame.rect(cell[0], cell[1], cell[2], cell[3]);
            let bought = cell_level <= level;
            let preview = !bought && target.is_some_and(|target| cell_level <= target);
            let (fill, edge, ink) = match (usable, bought, preview) {
                (false, true, _) => (
                    Some(color::alpha(color::GOLD, 0.35)),
                    color::alpha(color::GOLD, 0.35),
                    color::SPACE,
                ),
                (false, false, _) => (
                    None,
                    color::alpha(color::HOLO, 0.14),
                    color::alpha(color::QUIET, 0.6),
                ),
                (true, true, _) => (Some(color::GOLD), color::GOLD, color::SPACE),
                (true, false, true) if affordable => (
                    Some(color::alpha(color::GOLD, 0.3)),
                    color::GOLD_BRIGHT,
                    color::TEXT,
                ),
                (true, false, true) => (None, color::EMBER, color::EMBER),
                (true, false, false) => (
                    None,
                    color::alpha(color::HOLO, if focused { 0.5 } else { 0.3 }),
                    color::MUTED,
                ),
            };
            if let Some(fill) = fill {
                let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 7.0 * s,
                    color: fill,
                });
            }
            let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 7.0 * s,
                width: 1.5 * s,
                color: edge,
            });
            if power.level_is_free(cell_level, free_saber) {
                text(
                    &mut self.canvas,
                    TextFamily::Body,
                    format_args!("free"),
                    rect,
                    13.0 * s,
                    ink,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            } else {
                text(
                    &mut self.canvas,
                    TextFamily::Display,
                    format_args!("{}", power.level_cost(cell_level)),
                    rect,
                    18.0 * s,
                    ink,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
            self.canvas.hit_region(level_token(index, cell_level), rect);
        }
    }

    /// The power the box at the bottom right shows: the one whose row has the
    /// keyboard (the pointer gives it a row it hovers).
    fn sjk_box_power(&self) -> Option<usize> {
        (self.page == ProfilePage::Force)
            .then(|| self.selected.checked_sub(FORCE_POWER_ROW))
            .flatten()
            .filter(|index| *index < POWER_NAMES.len())
    }

    /// The box at the bottom right on the Force page: the power under the
    /// pointer (or the keyboard) in big, its holocron, group, level, what it
    /// does, its levels' prices and what the next one takes.
    fn sjk_force_box(&mut self, frame: &Frame, index: usize) {
        let s = frame.s;
        let Some(power) = ForcePower::ALL.get(index).copied() else {
            return;
        };
        let [x, y, width, height] = POWER_BOX;
        let rect = frame.rect(x, y, width, height);
        let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x + 4.0, y + 8.0, width, height),
            radius: 18.0 * s,
            color: color::alpha(color::SPACE, 0.5),
        });
        let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 18.0 * s,
            color: Color::new(0.04, 0.06, 0.12, 0.92),
        });
        let tint = match power.side() {
            Some(ForceSide::Light) => color::HOLO,
            Some(ForceSide::Dark) => color::EMBER,
            None => color::GOLD,
        };
        let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 18.0 * s,
            width: 1.5 * s,
            color: color::alpha(tint, 0.5),
        });
        let icon = power_texture(index);
        let level = self.force.allocation().levels[index];
        let text_x = if self.force_icons.is_texture_ready(icon) {
            let _ = self.canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(x + 22.0, y + 22.0, 112.0, 112.0),
                texture: icon,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
            x + 152.0
        } else {
            x + 24.0
        };
        let line = width - (text_x - x) - 20.0;
        text(
            &mut self.canvas,
            TextFamily::Display,
            format_args!("{}", SentenceCase(POWER_NAMES[index])),
            frame.rect(text_x, y + 22.0, line, 46.0),
            40.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let group = match power.side() {
            Some(ForceSide::Light) => "Light side",
            Some(ForceSide::Dark) => "Dark side",
            None if index >= 15 => "Lightsaber",
            None => "Neutral",
        };
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!("{group}"),
            frame.rect(text_x, y + 70.0, line, 24.0),
            16.0 * s,
            tint,
            FontWeight::Regular,
            TextAlign::Start,
        );
        kit::pips(
            &mut self.canvas,
            frame,
            text_x + 6.0,
            y + 116.0,
            level,
            3,
            true,
        );
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!("Level {level} of 3"),
            frame.rect(text_x + 62.0, y + 104.0, line - 62.0, 24.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        for (number, part) in wrap(POWER_NOTES[index], 46).take(3).enumerate() {
            text(
                &mut self.canvas,
                TextFamily::Body,
                format_args!("{part}"),
                frame.rect(
                    x + 24.0,
                    y + 150.0 + number as f32 * 26.0,
                    width - 48.0,
                    26.0,
                ),
                17.0 * s,
                color::alpha(color::TEXT, 0.9),
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let (status, status_colour) = match self.force.next_level(index) {
            NextLevel::Mastered => ("Mastered".to_owned(), color::GOLD_BRIGHT),
            NextLevel::Free => ("Next level: free".to_owned(), color::GOLD_BRIGHT),
            NextLevel::Costs(cost) => (format!("Next level: {cost} points"), color::GOLD_BRIGHT),
            NextLevel::Short(cost) => (
                format!("Next level: {cost} points, too few left"),
                color::EMBER,
            ),
            NextLevel::OtherSide => ("The other side's power".to_owned(), color::QUIET),
            NextLevel::TeamOnly => ("Team games only".to_owned(), color::QUIET),
            NextLevel::NeedsOffense => ("Needs Saber offense 1".to_owned(), color::EMBER),
        };
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!("{status}"),
            frame.rect(x + 24.0, y + height - 44.0, width - 48.0, 26.0),
            17.0 * s,
            status_colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let free_saber = self.force.free_saber();
        let price = |level: u8| {
            if power.level_is_free(level, free_saber) {
                "free".to_owned()
            } else {
                power.level_cost(level).to_string()
            }
        };
        text(
            &mut self.canvas,
            TextFamily::Body,
            format_args!("Levels {} / {} / {}", price(1), price(2), price(3)),
            frame.rect(x + 24.0, y + height - 44.0, width - 48.0, 26.0),
            16.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::End,
        );
    }

    /// Beside the model, as in a gallery: what the page shows of it.
    fn sjk_caption(&mut self, frame: &Frame) {
        // The Force page shows the power under the pointer instead.
        if let Some(index) = self.sjk_box_power() {
            self.sjk_force_box(frame, index);
            return;
        }
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
    // Dual sabers (the style, the hilt lists, eight blade rows) end above
    // the keys.
    const _: () = assert!(
        ROWS_TOP + ROW + 8.0 + HILT_HEAD + HILT_ROWS as f32 * HILT_ROW + 12.0 + 10.0 + 8.0 * ROW
            < BOTTOM
    );
    // Force: the groups under the sides, the actions under Lightsaber, and the
    // power box above the keys.
    const _: () = assert!(
        METER_Y
            + 22.0
            + SIDE_HEIGHT
            + 22.0
            + 2.0 * GROUP_HEADING
            + 8.0 * CELL
            + GROUP_GAP
            + 24.0
            + 48.0
            < BOTTOM
    );
    const _: () = assert!(POWER_BOX[1] + POWER_BOX[3] < KEYS_Y - 20.0);
    // A power's name keeps room beside its three level cells.
    const _: () = assert!(
        (COLUMN_WIDTH - GUTTER) * 0.5 - CELL - 8.0 - 3.0 * LEVEL_CELL[0] - 2.0 * LEVEL_GAP > 120.0
    );
    // The token ranges stay apart: the levels, the style buttons, the lists'
    // wheel areas, then the hilts.
    const _: () = assert!(LEVEL_BASE + 18 * 3 <= STYLE_BASE && STYLE_BASE + 3 <= HILT_SCROLL);
    const _: () = assert!(HILT_SCROLL + 2 <= HILT_BASE);

    /// The screen in the SJK UI on `page`, Dual sabers when `dual`, drawn at
    /// 1080 lines with a rank 7 profile that has bought nothing.
    fn drawn(page: ProfilePage, dual: bool) -> PlayerMenu {
        let mut menu = PlayerMenu::new();
        menu.set_sjk(true);
        menu.force.load_template("fresh", "7-1-000000000000000000");
        if dual {
            menu.saber.set_style(SaberStyle::Dual, None);
        }
        menu.set_page(page);
        draw(&mut menu);
        menu
    }

    fn draw(menu: &mut PlayerMenu) {
        let font = crate::text::load_modern(1.0, None).unwrap().font;
        let mut vertices = Vec::new();
        menu.append_sjk(
            TextTarget::Inter(&mut vertices, &font),
            [1920.0, 1080.0],
            1.0,
        );
    }

    #[test]
    fn level_and_hilt_tokens_round_trip() {
        for power in 0..POWER_NAMES.len() {
            for level in 1..=3 {
                assert_eq!(level_of(level_token(power, level)), Some((power, level)));
            }
        }
        assert_eq!(level_of(LEVEL_BASE - 1), None);
        assert_eq!(level_of(level_token(17, 3) + 1), None);
        for second in [false, true] {
            for index in [0, 1, 58, 255] {
                assert_eq!(hilt_of(hilt_token(second, index)), Some((second, index)));
            }
        }
        assert_eq!(hilt_of(HILT_BASE - 1), None);
    }

    #[test]
    fn the_force_page_groups_powers_as_classic_plus_and_skips_the_other_side() {
        use super::super::classic::layout::{DARK_POWERS, LIGHT_POWERS};
        let light = force_key_order(ForceSide::Light);
        // The sides, thirteen powers, the three actions.
        assert_eq!(light.len(), 1 + 13 + 3);
        assert_eq!(light[0], FORCE_SIDE_ROW);
        assert_eq!(
            &light[light.len() - 3..],
            [FORCE_RESET_ROW, FORCE_DISCARD_ROW, FORCE_APPLY_ROW]
        );
        // Neutral first (Jump), then the side's (Absorb), then Lightsaber.
        assert_eq!(light[1], FORCE_POWER_ROW + 1);
        assert_eq!(light[6], FORCE_POWER_ROW + usize::from(LIGHT_POWERS[0]));
        assert_eq!(light[13], FORCE_POWER_ROW + 17);
        for power in DARK_POWERS {
            assert!(!light.contains(&(FORCE_POWER_ROW + usize::from(power))));
        }
        let menu = drawn(ProfilePage::Force, false);
        for (power, name) in POWER_NAMES.iter().enumerate() {
            let shown = light.contains(&(FORCE_POWER_ROW + power));
            for level in 1..=3 {
                assert_eq!(
                    menu.canvas.rect_for(level_token(power, level)).is_some(),
                    shown,
                    "{name} level {level}",
                );
            }
        }
        assert!(!menu.canvas.overflowed());
    }

    #[test]
    fn a_level_cell_buys_up_to_it_and_its_own_level_steps_down() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut menu = drawn(ProfilePage::Force, false);
        let push = 3;
        let left = menu.force.remaining_points();
        let click = |menu: &mut PlayerMenu, console: &mut ViewerConsole, level| {
            menu.sjk_pointer(
                sjk_ui::UiEventKind::Activate,
                level_token(push, level),
                0.0,
                console,
            )
        };
        assert!(click(&mut menu, &mut console, 2).is_some());
        assert_eq!(menu.force.allocation().levels[push], 2);
        assert_eq!(menu.selected, FORCE_POWER_ROW + push);
        assert_eq!(menu.force.remaining_points(), left - 1 - 3);
        click(&mut menu, &mut console, 2);
        assert_eq!(menu.force.allocation().levels[push], 1);
        // The box at the bottom right now shows Push.
        assert_eq!(menu.sjk_box_power(), Some(push));
        // A draft only: nothing is written before Apply.
        assert!(menu.force.is_dirty());
    }

    #[test]
    fn the_style_buttons_choose_the_style() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut menu = drawn(ProfilePage::Saber, false);
        for (index, style) in STYLES.into_iter().enumerate() {
            assert!(menu.canvas.rect_for(STYLE_BASE + index as u16).is_some());
            menu.sjk_pointer(
                sjk_ui::UiEventKind::Activate,
                STYLE_BASE + index as u16,
                0.0,
                &mut console,
            );
            assert_eq!(menu.saber.style(), style);
        }
        draw(&mut menu);
        assert!(!menu.canvas.overflowed());
    }

    #[test]
    fn dual_sabers_take_both_lists_before_the_blades() {
        let menu = drawn(ProfilePage::Saber, true);
        let rows = menu.saber_rows();
        let order: Vec<SaberRow> = menu
            .sjk_key_order()
            .unwrap()
            .into_iter()
            .map(|index| rows[index])
            .collect();
        assert_eq!(
            order[..4],
            [
                SaberRow::Style,
                SaberRow::Hilt,
                SaberRow::SecondHilt,
                SaberRow::Blade
            ]
        );
        assert_eq!(order.len(), rows.len());
        // Both lists take the pointer, and the page fits its canvas.
        for list in 0..2 {
            assert!(menu.canvas.rect_for(HILT_SCROLL + list).is_some());
        }
        assert!(!menu.canvas.overflowed());
        assert!(
            drawn(ProfilePage::Character, false)
                .sjk_key_order()
                .is_none()
        );
    }

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
