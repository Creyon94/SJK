//! The Collection's Shaders tab: the saber's looks as a rack, a row to each, or as a grid
//! of cards ([`SHADER_VIEW_CVAR`], switched by V or the two marks by the kinds): the
//! stock blade first in the player's colour, then every blade skin
//! (`docs/unlockables.md`), the rarest tier first, each framed in its tier's colour,
//! alive when owned, grey and still under a padlock when not. The one chosen is the
//! blade the player's model holds beside the page ([`Backstage`]), a locked one too, as
//! a preview only this screen draws; the words beside the model say what it is, since
//! when it is the player's or how to get it, and Equip or Unequip. Body shaders, for the
//! whole character, are to come: their kind is shown, not yet offered.

use super::view::{LEAD_Y, LEFT_X, RIGHT_X, SABER_AREA, left_lines};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use crate::rarity_fx;
use crate::symbols;
use crate::unlockables::Unlockable;
use sjk_ui::{DrawCommand, FontWeight, TextAlign};

/// The rack: where its first row starts, a row's room and a swatch's size.
const RACK_TOP: f32 = 324.0;
const ROW_STEP: f32 = 106.0;
const ROW_HEIGHT: f32 = 96.0;
const SWATCH: [f32; 2] = [380.0, 84.0];
const RACK_WIDTH: f32 = 860.0;

/// The chroma mark's side beside a rack row's name and the detail's kind line, and the
/// room it keeps from the words after it (frame pixels).
const MARK: f32 = 24.0;
const MARK_GAP: f32 = 10.0;
/// The grid's cards: a card's size and the room between them (frame pixels).
const CARD: [f32; 2] = [164.0, 196.0];
const CARD_GAP: [f32; 2] = [10.0, 14.0];
/// How far the chosen card lifts (frame pixels), and its outline's colour: white, so it
/// never reads as a tier (Mythical's frame is gold).
const CHOSEN_LIFT: f32 = 5.0;
const CHOSEN_OUTLINE: sjk_ui::Color = sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0);
/// The view switches' side and where the first stands, left of the kinds.
const SWITCH: f32 = 36.0;
const SWITCH_X: f32 = LEFT_X + 466.0;

/// The detail's state pill: its height, its least width and its gap from the name.
const TAG_HEIGHT: f32 = 26.0;
const TAG_MIN_WIDTH: f32 = 80.0;
const TAG_GAP: f32 = 18.0;
/// The chosen shader's card, under the lone saber (frame pixels), the room inside its
/// edge and its corners' radius.
const SHADER_CARD: [f32; 4] = [1_000.0, 512.0, RIGHT_X - 1_000.0, 478.0];
const CARD_PAD: f32 = 28.0;
const CARD_RADIUS: f32 = 16.0;
/// The card's facts: a chip's height, its icon's side and the room between chips.
const FACT_HEIGHT: f32 = 34.0;
const FACT_ICON: f32 = 20.0;
const FACT_GAP: f32 = 10.0;

/// What a fact's chip shows before its words.
#[derive(Clone, Copy)]
enum Fact {
    /// The chroma's colour wheel.
    Chroma,
    /// One of SJK Symbols' icons.
    Icon(char),
}

/// Where `skin` comes from, as facts' icons and words: the medal that brings it, or a
/// holocron (it drops) and the card that also brings it, or the SJK team.
fn sources(skin: &unlockables::Unlockable) -> Vec<(char, String)> {
    if let Some(medal) = crate::medals::Medal::ALL
        .iter()
        .find(|medal| medal.blade() == Some(skin.id))
    {
        return vec![(symbols::MEDAL, format!("{} medal", medal.name()))];
    }
    let mut sources = Vec::new();
    if skin.droppable {
        sources.push((symbols::HOLOCRON, "Holocron drop".to_owned()));
    }
    if let Some(card) = unlockables::card_bringing(skin.id) {
        let name = crate::cards::faces::name_of(card);
        sources.push((symbols::FINISH[0], format!("{name} card")));
    }
    if sources.is_empty() {
        sources.push((symbols::GIFT, "SJK team gift".to_owned()));
    }
    sources
}

/// Where a rack row's words go (frame pixels, `[x, y, width, height]`), the row's top
/// at `y`: the chroma mark (a chroma only) and the name on the first line, the name
/// ending short of the row's right edge, and the state under them.
fn rack_words(y: f32, chroma: bool) -> ([f32; 4], [f32; 4], [f32; 4]) {
    let x = LEFT_X + SWATCH[0] + 34.0;
    let right = LEFT_X - 12.0 + RACK_WIDTH - 16.0;
    let mark = [x, y + 23.0, MARK, MARK];
    let name_x = if chroma { x + MARK + MARK_GAP } else { x };
    let name = [name_x, y + 18.0, right - name_x, 34.0];
    let status = [x, y + 56.0, right - x, 22.0];
    (mark, name, status)
}

/// The detail's name and state pill on the line at `y` of the column from `x`, `width`
/// wide: the pill at the column's right end, the name ellipsised short of it.
fn title_rects(x: f32, y: f32, width: f32, tag_width: f32) -> ([f32; 4], [f32; 4]) {
    let tag_width = tag_width.max(TAG_MIN_WIDTH);
    let tag = [x + width - tag_width, y + 18.0, tag_width, TAG_HEIGHT];
    let name = [x, y, (tag[0] - TAG_GAP - x).max(0.0), 62.0];
    (name, tag)
}

/// The first row the rack shows, from the one it showed first: moved just enough that
/// the chosen row is in view, never past either end.
fn rack_window(first: usize, chosen: usize, rows: usize) -> usize {
    window(first, chosen, rows, RACK_SHOWN)
}

/// The first of `rows` a window of `shown` shows, from the one it showed first: moved
/// just enough that `chosen` is in view, never past either end.
pub(super) fn window(first: usize, chosen: usize, rows: usize, shown: usize) -> usize {
    let first = if chosen < first {
        chosen
    } else if chosen >= first + shown {
        chosen + 1 - shown
    } else {
        first
    };
    first.min(rows.saturating_sub(shown))
}

/// Card `index`'s rectangle in the grid (frame pixels) when its first line shown is
/// `first_line`.
fn card_rect(index: usize, first_line: usize) -> [f32; 4] {
    let line = (index / GRID_COLUMNS) as f32 - first_line as f32;
    let column = (index % GRID_COLUMNS) as f32;
    [
        LEFT_X - 12.0 + column * (CARD[0] + CARD_GAP[0]),
        RACK_TOP + line * (CARD[1] + CARD_GAP[1]),
        CARD[0],
        CARD[1],
    ]
}

/// A shader's name on a card: without the " blade" every one ends in.
fn card_name(name: &str) -> &str {
    name.strip_suffix(" blade").unwrap_or(name)
}

/// The owned line of a shader the player holds: since when, and from the team or the
/// medal or card that brings it (a medal the client does not know reads as the team, a
/// card without a face in the packs by its id).
fn owned_line(grant: &sjk_identity::Unlock) -> String {
    if crate::debug_collectibles::added(grant) {
        return crate::debug_collectibles::NOTE.to_owned();
    }
    let date = crate::medals::date_text(grant.granted);
    let medal = grant
        .medal
        .as_deref()
        .and_then(crate::medals::Medal::from_id)
        .map(|medal| format!("with your {} medal", medal.name()));
    let card = || {
        grant
            .card
            .as_deref()
            .map(|card| format!("with your {} card", crate::cards::faces::name_of(card)))
    };
    let from = medal
        .or_else(card)
        .unwrap_or_else(|| "from the SJK team".to_owned());
    let held = if grant.count > 1 {
        format!(", {} held", grant.count)
    } else {
        String::new()
    };
    if date.is_empty() {
        format!("Yours, {from}{held}")
    } else {
        format!("Yours since {date}, {from}{held}")
    }
}

/// The filter's chips and search field: where they stand and how tall they are.
const FILTER_Y: f32 = 292.0;
const FILTER_HEIGHT: f32 = 28.0;

/// The chip of tier `index` of [`FILTER_TIERS`]: its token, name, colour, whether it is
/// the one on and its width.
fn chip_of(
    index: usize,
    filter: &ShaderFilter,
    width: f32,
) -> (u16, String, sjk_ui::Color, bool, f32) {
    let tier = FILTER_TIERS[index];
    (
        FILTER_TIER_BASE + 1 + index as u16,
        tier.label().to_owned(),
        tier.colour(),
        filter.tier == Some(tier),
        width,
    )
}

/// What a row shows: the stock blade, or a blade skin.
#[derive(Clone, Copy)]
struct Row {
    skin: Option<&'static Unlockable>,
    owned: bool,
    worn: bool,
    /// The second saber wears it (`cg_saberSkin2`) and not the first.
    second: bool,
    /// How many the player holds (holocrons may drop the same shader again): 1 for the
    /// stock blade and a shader held once, 0 when locked.
    count: u32,
    /// Its blade number shows: only while `debug_collectibles` is on (Sol, 21/10/2026).
    numbered: bool,
}

/// `word` with how many are held after it when there are several: "Yours x2".
fn held_words(word: &str, count: u32) -> String {
    if count > 1 {
        format!("{word} x{count}")
    } else {
        word.to_owned()
    }
}

impl Row {
    fn name(self) -> &'static str {
        self.skin.map_or("Stock blade", |skin| skin.name)
    }

    /// Its blade number after its tier ("  ·  042"), nothing for the stock blade or
    /// when numbers are hidden ([`Self::numbered`]).
    fn number_words(self) -> String {
        self.skin
            .filter(|_| self.numbered)
            .map(|skin| format!("  ·  {}", skin.number_text()))
            .unwrap_or_default()
    }

    /// Its tier, `None` for the stock blade.
    fn tier(self) -> Option<unlockables::Rarity> {
        self.skin.map(|skin| skin.tier)
    }

    /// Its frame's colour: its tier's, quieter when locked; the stock blade's is the
    /// page's own.
    fn frame_colour(self) -> sjk_ui::Color {
        match self.tier() {
            Some(tier) => color::alpha(tier.colour(), if self.owned { 0.9 } else { 0.45 }),
            None => color::alpha(color::HOLO, 0.3),
        }
    }

    /// Its state, as the rack and the grid say it, after its icon (a lightsaber worn, a
    /// tick held, a padlock locked) and with how many are held.
    fn status(self) -> String {
        match (self.skin, self.worn, self.owned) {
            (None, true, _) => format!("{}  Worn, in your colour", symbols::SABER),
            (None, false, _) if self.second => {
                format!("{}  2nd blade, in your colour", symbols::SABER)
            }
            (None, false, _) => "In your colour".to_owned(),
            (Some(_), false, true) if self.second => {
                format!(
                    "{}  {}",
                    symbols::SABER,
                    held_words("2nd blade", self.count)
                )
            }
            (Some(_), true, _) => format!("{}  {}", symbols::SABER, held_words("Worn", self.count)),
            (Some(_), false, true) => {
                format!("{}  {}", symbols::TICK, held_words("Yours", self.count))
            }
            (Some(_), false, false) => format!("{}  Locked", symbols::LOCK),
        }
    }

    /// Its state on its card's pill (the copies held are a fact of their own there):
    /// `None` for the stock blade.
    fn state(self) -> Option<String> {
        match (self.skin, self.worn, self.owned) {
            (None, _, _) => None,
            (Some(_), true, _) => Some(format!("{}  Worn", symbols::SABER)),
            (Some(_), false, true) if self.second => Some(format!("{}  2nd blade", symbols::SABER)),
            (Some(_), false, true) => Some(format!("{}  Yours", symbols::TICK)),
            (Some(_), false, false) => Some(format!("{}  Locked", symbols::LOCK)),
        }
    }
}

/// What the second saber (or a staff's other half) wears by its own choice
/// (`cg_saberSkin2`): `None` when it has none and wears the first's, `Some(None)` for the
/// stock blade, `Some(Some(id))` for a skin the player holds (one not held is no choice,
/// as `looks::SecondSkin::own` reads it).
fn second_own(inputs: &Inputs<'_>) -> Option<Option<&'static str>> {
    match inputs.second.trim() {
        "" => None,
        crate::looks::STOCK_SECOND => Some(None),
        id => unlockables::blade_skin(id)
            .filter(|skin| inputs.holdings.unlock(skin.id).is_some())
            .map(|skin| Some(skin.id)),
    }
}

impl Panel {
    /// The rows, and the lead's words.
    fn rows(inputs: &Inputs<'_>, filter: &ShaderFilter) -> Vec<Row> {
        let worn = inputs
            .holdings
            .worn_blade_skin(inputs.setting)
            .map(|skin| skin.id);
        // "2nd blade" only while there is a second saber or a staff's other half to wear it,
        // on a row the first does not wear.
        let pair = inputs.pair != crate::saber_skins::Pair::None;
        let second = second_own(inputs);
        std::iter::once(Row {
            skin: None,
            owned: true,
            worn: worn.is_none(),
            second: pair && second == Some(None) && worn.is_some(),
            count: 1,
            numbered: false,
        })
        .chain(
            unlockables::blade_skins_by_tier()
                .iter()
                .filter(|skin| filter.passes(skin, inputs.holdings.unlock(skin.id).is_some()))
                .map(|&skin| Row {
                    skin: Some(skin),
                    owned: inputs.holdings.unlock(skin.id).is_some(),
                    worn: worn == Some(skin.id),
                    second: pair && second == Some(Some(skin.id)) && worn != Some(skin.id),
                    count: inputs
                        .holdings
                        .unlock(skin.id)
                        .map_or(0, |unlock| unlock.count),
                    numbered: inputs.holdings.debug(),
                }),
        )
        .collect()
    }

    /// The Shaders tab, swatches alive at `seconds`.
    pub(super) fn shaders(&mut self, frame: &Frame, inputs: &Inputs<'_>, seconds: f32) {
        let s = frame.s;
        self.grid = inputs.grid;
        self.pair = inputs.pair;
        if self.pair == crate::saber_skins::Pair::None {
            self.second_target = false;
        }
        let rows = Self::rows(inputs, &self.shader_filter);
        self.shader_list = rows.iter().filter_map(|row| row.skin).collect();
        self.shader_count = rows.len();
        self.shader_swatches.clear();
        // A blade card's button asked for its shader.
        if let Some(id) = self.shader_focus.take()
            && let Some(row) = rows
                .iter()
                .position(|row| row.skin.is_some_and(|skin| skin.id == id))
        {
            self.shader = row;
        }
        if self.shader >= rows.len() {
            self.shader = rows.iter().position(|row| row.worn).unwrap_or(0);
        }
        let owned = unlockables::blade_skins()
            .filter(|skin| inputs.holdings.unlock(skin.id).is_some())
            .count();
        let headline = if inputs.holdings.reason().is_none() {
            format!("{owned} of {} owned", unlockables::ALL.len())
        } else {
            "Saber shaders".to_owned()
        };
        let shown_words;
        let line = match inputs.holdings {
            unlockables::Holdings::Known(_) if self.shader_filter.active() => {
                shown_words = format!(
                    "{} of {} shaders shown: T tier, O owned, / search.",
                    rows.len() - 1,
                    unlockables::ALL.len()
                );
                shown_words.as_str()
            }
            unlockables::Holdings::IdentityOff => {
                "Shaders are kept on the SJK hub: switch the identity on in Settings, Network."
            }
            unlockables::Holdings::NoHub => {
                "Shaders are kept on the SJK hub: no hub is set (cl_hubUrl)."
            }
            unlockables::Holdings::Waiting => "Waiting for the SJK hub to say which are yours.",
            holdings @ unlockables::Holdings::Known(_) if holdings.debug() => {
                "Debug collectibles is on: every shader is shown as yours, on this PC only."
            }
            unlockables::Holdings::Known(_) => {
                "How your blade looks and sounds, for every SJK player on your server."
            }
        };
        self.lead(frame, &headline, line);
        self.filter_bar(frame);
        self.kinds(frame, owned);
        self.view_switches(frame);
        let unlocks = matches!(inputs.holdings, unlockables::Holdings::Known(_));
        // A side just chosen: show the row it wears (the second's own, else the first's).
        if std::mem::take(&mut self.jump_to_worn) {
            let own = second_own(inputs);
            let worn_by_target = rows
                .iter()
                .position(|row| {
                    self.second_target && own.is_some_and(|own| own == row.skin.map(|s| s.id))
                })
                .or_else(|| rows.iter().position(|row| row.worn));
            if let Some(index) = worn_by_target {
                self.shader = index;
            }
        }
        // The rack (or the grid) scrolls just enough to keep the chosen one in view.
        self.shader_first = rack_window(self.shader_first, self.shader, rows.len());
        self.grid_first = window(
            self.grid_first,
            self.shader / GRID_COLUMNS,
            rows.len().div_ceil(GRID_COLUMNS),
            GRID_LINES_SHOWN,
        );
        let first = self.shader_first;
        // While the second has no choice of its own it wears the first's.
        let second = second_own(inputs);
        for (index, row) in rows.iter().enumerate() {
            // What a second saber wears now: its own choice, else the first's.
            let row_id = row.skin.map(|skin| skin.id);
            let second_id = row_id.unwrap_or(crate::looks::STOCK_SECOND);
            let second_worn = second.map_or(row.worn, |own| own == row_id);
            self.shader_rows[index] = ShaderRow {
                wear: match row.skin {
                    // The stock blade, worn by taking the skin off.
                    None => (!row.worn).then_some(""),
                    Some(skin) if row.owned && unlocks => Some(if row.worn { "" } else { skin.id }),
                    Some(_) => None,
                },
                // Offered while the player holds two sabers or a staff: Equip puts it on the
                // second, Unequip leaves the second the stock blade (the stock row has none).
                second: if inputs.pair == crate::saber_skins::Pair::None
                    || !(row.skin.is_none() || (row.owned && unlocks))
                {
                    None
                } else if second_worn {
                    row.skin.map(|_| crate::looks::STOCK_SECOND)
                } else {
                    Some(second_id)
                },
                second_worn,
                // Both sides at once ("Both" box): they wear it, the second following the
                // first; Unequip leaves both the stock blade.
                both: if inputs.pair == crate::saber_skins::Pair::None
                    || !(row.skin.is_none() || (row.owned && unlocks))
                    || (row.skin.is_none() && row.worn && second_worn)
                {
                    None
                } else if row.worn && second_worn {
                    Some(("", ""))
                } else {
                    Some((row_id.unwrap_or(""), ""))
                },
                both_worn: row.worn && second_worn,
            };
        }
        if self.grid {
            let first_line = self.grid_first;
            let shown = first_line * GRID_COLUMNS..(first_line + GRID_LINES_SHOWN) * GRID_COLUMNS;
            for (index, row) in rows.iter().enumerate() {
                if shown.contains(&index) {
                    let rect = card_rect(index, first_line);
                    self.grid_card(frame, *row, index, rect, inputs, seconds);
                }
            }
            self.grid_scroll_bar(frame, first_line, rows.len().div_ceil(GRID_COLUMNS));
        } else {
            for (index, row) in rows.iter().enumerate() {
                if (first..first + RACK_SHOWN).contains(&index) {
                    let y = RACK_TOP + (index - first) as f32 * ROW_STEP;
                    self.rack_row(frame, *row, index, y, inputs, seconds);
                }
            }
            self.rack_scroll_bar(frame, first, rows.len());
        }
        if let Some(row) = rows.get(self.shader).copied() {
            self.shader_card(frame, row, inputs, seconds);
        }
        let _ = s;
    }

    /// The filter under the headline: a chip for All and one for each tier (in its tier's
    /// colour), the owned-only chip and the search field.
    fn filter_bar(&mut self, frame: &Frame) {
        let s = frame.s;
        let y = FILTER_Y;
        let mut left = LEFT_X;
        let chips: [(u16, String, sjk_ui::Color, bool, f32); 7] = [
            (
                FILTER_TIER_BASE,
                "All".to_owned(),
                color::HOLO,
                self.shader_filter.tier.is_none(),
                56.0,
            ),
            chip_of(0, &self.shader_filter, 90.0),
            chip_of(1, &self.shader_filter, 104.0),
            chip_of(2, &self.shader_filter, 70.0),
            chip_of(3, &self.shader_filter, 100.0),
            chip_of(4, &self.shader_filter, 96.0),
            (
                FILTER_OWNED_TOKEN,
                "Owned".to_owned(),
                color::GOLD,
                self.shader_filter.owned,
                84.0,
            ),
        ];
        for (token, label, colour, lit, width) in chips {
            let rect = frame.rect(left, y, width, FILTER_HEIGHT);
            let hovered = self.ui.token_hovered(token);
            if lit || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: FILTER_HEIGHT * 0.5 * s,
                    color: color::alpha(colour, if lit { 0.2 } else { 0.09 }),
                });
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: FILTER_HEIGHT * 0.5 * s,
                width: 1.2 * s,
                color: color::alpha(colour, if lit { 1.0 } else { 0.4 }),
            });
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{label}"),
                frame.rect(left, y + 4.0, width, FILTER_HEIGHT - 8.0),
                14.0 * s,
                if lit { color::TEXT } else { color::MUTED },
                FontWeight::Semibold,
                TextAlign::Center,
            );
            self.ui.hit_region(token, rect);
            left += width + 8.0;
        }
        // The search field takes what is left of the rack's width.
        let width = (LEFT_X - 12.0 + RACK_WIDTH - left).max(120.0);
        let rect = frame.rect(left, y, width, FILTER_HEIGHT);
        let focused = self.search_focus;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: 8.0 * s,
            color: color::alpha(color::SPACE, if focused { 0.8 } else { 0.5 }),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: 8.0 * s,
            width: 1.2 * s,
            color: if focused {
                color::GOLD
            } else {
                color::alpha(color::HOLO, 0.3)
            },
        });
        let caret = (self.epoch.elapsed().as_millis() / 500).is_multiple_of(2);
        let shown = if focused {
            format!(
                "{}{}",
                self.shader_filter.query,
                if caret { "|" } else { "" }
            )
        } else if self.shader_filter.query.is_empty() {
            "Search  /".to_owned()
        } else {
            self.shader_filter.query.clone()
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{shown}"),
            frame.rect(left + 12.0, y + 4.0, width - 24.0, FILTER_HEIGHT - 8.0),
            14.0 * s,
            if focused || !self.shader_filter.query.is_empty() {
                color::TEXT
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        self.ui.hit_region(SEARCH_TOKEN, rect);
    }

    /// The rack's scroll bar at its right, when it holds more rows than it shows.
    fn rack_scroll_bar(&mut self, frame: &Frame, first: usize, rows: usize) {
        if rows <= RACK_SHOWN {
            return;
        }
        let s = frame.s;
        let x = LEFT_X - 12.0 + RACK_WIDTH + 8.0;
        let height = RACK_SHOWN as f32 * ROW_STEP - (ROW_STEP - ROW_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, RACK_TOP, 4.0, height),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.14),
        });
        let thumb = height * RACK_SHOWN as f32 / rows as f32;
        let top = RACK_TOP + (height - thumb) * first as f32 / (rows - RACK_SHOWN) as f32;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, top, 4.0, thumb),
            radius: 2.0 * s,
            color: color::GOLD,
        });
    }

    /// The grid's scroll bar at its right, when it holds more lines than it shows.
    fn grid_scroll_bar(&mut self, frame: &Frame, first: usize, lines: usize) {
        if lines <= GRID_LINES_SHOWN {
            return;
        }
        let s = frame.s;
        let x = LEFT_X - 12.0 + RACK_WIDTH + 8.0;
        let height = GRID_LINES_SHOWN as f32 * (CARD[1] + CARD_GAP[1]) - CARD_GAP[1];
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, RACK_TOP, 4.0, height),
            radius: 2.0 * s,
            color: color::alpha(color::HOLO, 0.14),
        });
        let thumb = height * GRID_LINES_SHOWN as f32 / lines as f32;
        let top = RACK_TOP + (height - thumb) * first as f32 / (lines - GRID_LINES_SHOWN) as f32;
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x, top, 4.0, thumb),
            radius: 2.0 * s,
            color: color::GOLD,
        });
    }

    /// The two view switches left of the kinds: the rack (three bars) and the grid
    /// (four squares), the one on show lit.
    fn view_switches(&mut self, frame: &Frame) {
        let s = frame.s;
        let y = LEAD_Y - 2.0;
        for (index, (token, grid)) in [(LIST_VIEW_TOKEN, false), (GRID_VIEW_TOKEN, true)]
            .into_iter()
            .enumerate()
        {
            let x = SWITCH_X + index as f32 * (SWITCH + 8.0);
            let lit = self.grid == grid;
            let hovered = self.ui.token_hovered(token);
            let rect = frame.rect(x, y, SWITCH, SWITCH);
            if lit || hovered {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 9.0 * s,
                    color: color::alpha(color::GOLD, if lit { 0.16 } else { 0.08 }),
                });
            }
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 9.0 * s,
                width: 1.2 * s,
                color: if lit {
                    color::GOLD
                } else {
                    color::alpha(color::HOLO, 0.3)
                },
            });
            let ink = if lit {
                color::GOLD_BRIGHT
            } else {
                color::QUIET
            };
            let marks: &[[f32; 4]] = if grid {
                &[
                    [10.0, 10.0, 7.0, 7.0],
                    [19.0, 10.0, 7.0, 7.0],
                    [10.0, 19.0, 7.0, 7.0],
                    [19.0, 19.0, 7.0, 7.0],
                ]
            } else {
                &[
                    [10.0, 10.0, 16.0, 3.0],
                    [10.0, 16.5, 16.0, 3.0],
                    [10.0, 23.0, 16.0, 3.0],
                ]
            };
            for [mx, my, width, height] in marks {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(x + mx, y + my, *width, *height),
                    radius: 1.5 * s,
                    color: ink,
                });
            }
            self.ui.hit_region(token, rect);
        }
    }

    /// Card `index` of the grid in `rect` (frame pixels): its tier's frame and a band
    /// of its colour along the top, the blade alive (or grey and locked), the chroma
    /// mark, its name, its tier and its state.
    fn grid_card(
        &mut self,
        frame: &Frame,
        row: Row,
        index: usize,
        rect: [f32; 4],
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        let s = frame.s;
        let [x, y, width, height] = rect;
        let chosen = index == self.shader;
        // The chosen card lifts off the grid, a shadow under it and a white outline
        // round it: its own mark, apart from any tier's colour (Mythical's is gold).
        let y = if chosen { y - CHOSEN_LIFT } else { y };
        let token = SHADER_BASE + index as u16;
        let hovered = self.ui.token_hovered(token);
        let area = frame.rect(x, y, width, height);
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 2.0, y + CHOSEN_LIFT + 4.0, width - 4.0, height),
                radius: 12.0 * s,
                color: color::alpha(color::SPACE, 0.55),
            });
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 12.0 * s,
            color: color::alpha(color::SPACE, if chosen || hovered { 0.78 } else { 0.6 }),
        });
        if let Some(tier) = row.tier() {
            // The tier's band along the top, and its glow under it.
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 12.0, y, width - 24.0, 4.0),
                radius: 2.0 * s,
                color: color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.5 }),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 4.0, y + 4.0, width - 8.0, 30.0),
                radius: 10.0 * s,
                color: color::alpha(tier.colour(), if row.owned { 0.1 } else { 0.05 }),
            });
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 12.0 * s,
            width: if chosen { 1.8 } else { 1.2 } * s,
            color: row.frame_colour(),
        });
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(x - 3.0, y - 3.0, width + 6.0, height + 6.0),
                radius: 15.0 * s,
                width: 2.0 * s,
                color: color::alpha(CHOSEN_OUTLINE, 0.95),
            });
        }
        let swatch_rect = [x + 8.0, y + 22.0, width - 16.0, 52.0];
        self.shader_swatches.push((
            index,
            frame.rect(
                swatch_rect[0],
                swatch_rect[1],
                swatch_rect[2],
                swatch_rect[3],
            ),
        ));
        match row.skin {
            None => swatch::small_stock(self.ui.draw_list_mut(), frame, swatch_rect, inputs.stock),
            Some(skin) => {
                let look = self.skins.get(skin.id);
                let _ = swatch::small_swatch(
                    self.ui.draw_list_mut(),
                    frame,
                    swatch_rect,
                    look,
                    row.owned,
                    seconds,
                    swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                );
            }
        }
        let chroma = row.skin.is_some_and(|skin| skin.chroma);
        if chroma {
            swatch::chroma_mark(
                self.ui.draw_list_mut(),
                frame,
                x + width - 30.0,
                y + 89.0,
                20.0,
                if row.owned { 1.0 } else { 0.55 },
            );
        }
        let name_width = width - 20.0 - if chroma { 26.0 } else { 0.0 };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", card_name(row.name())),
            frame.rect(x + 10.0, y + 86.0, name_width, 26.0),
            21.0 * s,
            match (chosen, row.owned) {
                (true, _) => CHOSEN_OUTLINE,
                (false, true) => color::TEXT,
                (false, false) => color::MUTED,
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let (tier_text, tier_colour) = match row.tier() {
            Some(tier) => (
                format!("{}  {}{}", tier.symbol(), tier.label(), row.number_words()),
                color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.6 }),
            ),
            None => ("Your colour".to_owned(), color::QUIET),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{tier_text}"),
            frame.rect(x + 10.0, y + 122.0, width - 20.0, 20.0),
            14.0 * s,
            tier_colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let state = match row.skin {
            None if !row.worn => String::new(),
            None => format!("{}  Worn", symbols::SABER),
            Some(_) => row.status(),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{state}"),
            frame.rect(x + 10.0, y + 152.0, width - 20.0, 20.0),
            14.0 * s,
            if row.worn || (row.owned && row.skin.is_some()) {
                color::GOLD
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        if let Some(tier) = row.tier() {
            rarity_fx::draw(
                self.ui.draw_list_mut(),
                rarity_fx::Item::of(tier, row.owned, rarity_fx::Size::Full),
                area,
                12.0 * s,
                s,
                seconds,
            );
        }
        self.ui.hit_region(token, area);
    }

    /// The kinds of shader along the lead: Saber, and Body to come.
    fn kinds(&mut self, frame: &Frame, owned: usize) {
        let s = frame.s;
        let x = LEFT_X + 560.0;
        let y = LEAD_Y - 2.0;
        let pills: [(&str, String, bool); 2] = [
            ("Saber", format!("{owned}/{}", unlockables::ALL.len()), true),
            ("Body", "soon".to_owned(), false),
        ];
        let mut left = x;
        for (name, count, lit) in pills {
            let width = 132.0;
            let rect = frame.rect(left, y, width, 36.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 18.0 * s,
                width: 1.2 * s,
                color: if lit {
                    color::GOLD
                } else {
                    color::alpha(color::HOLO, 0.25)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(left + 18.0, y + 5.0, 70.0, 26.0),
                19.0 * s,
                if lit {
                    color::GOLD_BRIGHT
                } else {
                    color::QUIET
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{count}"),
                frame.rect(left + 70.0, y + 7.0, width - 84.0, 22.0),
                14.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Regular,
                TextAlign::End,
            );
            left += width + 12.0;
        }
    }

    /// Row `index` of the rack at `y`.
    fn rack_row(
        &mut self,
        frame: &Frame,
        row: Row,
        index: usize,
        y: f32,
        inputs: &Inputs<'_>,
        seconds: f32,
    ) {
        let s = frame.s;
        let chosen = index == self.shader;
        let token = SHADER_BASE + index as u16;
        let hovered = self.ui.token_hovered(token);
        if chosen || hovered {
            kit::band(
                &mut self.ui,
                frame,
                [LEFT_X - 12.0, y, RACK_WIDTH, ROW_HEIGHT],
            );
        }
        if chosen {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(LEFT_X - 12.0, y + 14.0, 3.0, ROW_HEIGHT - 28.0),
                radius: 1.5 * s,
                color: color::GOLD_BRIGHT,
            });
        }
        let rect = [
            LEFT_X + 6.0,
            y + (ROW_HEIGHT - SWATCH[1]) * 0.5,
            SWATCH[0],
            SWATCH[1],
        ];
        self.shader_swatches
            .push((index, frame.rect(rect[0], rect[1], rect[2], rect[3])));
        match row.skin {
            None => swatch::small_stock(self.ui.draw_list_mut(), frame, rect, inputs.stock),
            Some(skin) => {
                let look = self.skins.get(skin.id);
                let _ = swatch::small_swatch(
                    self.ui.draw_list_mut(),
                    frame,
                    rect,
                    look,
                    row.owned,
                    seconds,
                    swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                );
            }
        }
        // The swatch framed in its tier's colour.
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(rect[0], rect[1], rect[2], rect[3]),
            radius: 10.0 * s,
            width: 1.4 * s,
            color: row.frame_colour(),
        });
        if let Some(tier) = row.tier() {
            rarity_fx::draw(
                self.ui.draw_list_mut(),
                rarity_fx::Item::of(tier, row.owned, rarity_fx::Size::Small),
                frame.rect(rect[0], rect[1], rect[2], rect[3]),
                10.0 * s,
                s,
                seconds,
            );
        }
        let chroma = row.skin.is_some_and(|skin| skin.chroma);
        let (mark, name, status) = rack_words(y, chroma);
        if let Some(tier) = row.tier() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{} {}{}", tier.symbol(), tier.label(), row.number_words()),
                frame.rect(status[0], status[1], status[2], status[3]),
                15.0 * s,
                color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.6 }),
                FontWeight::Semibold,
                TextAlign::End,
            );
        }
        if chroma {
            swatch::chroma_mark(
                self.ui.draw_list_mut(),
                frame,
                mark[0],
                mark[1],
                mark[2],
                if row.owned { 1.0 } else { 0.55 },
            );
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", row.name()),
            frame.rect(name[0], name[1], name[2], name[3]),
            28.0 * s,
            match (chosen, row.owned) {
                (true, _) => color::GOLD_BRIGHT,
                (false, true) => color::TEXT,
                (false, false) => color::MUTED,
            },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{}", row.status()),
            frame.rect(status[0], status[1], status[2], status[3]),
            15.0 * s,
            if row.worn || (row.owned && row.skin.is_some()) {
                color::GOLD
            } else {
                color::QUIET
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        self.ui
            .hit_region(token, frame.rect(LEFT_X - 12.0, y, RACK_WIDTH, ROW_HEIGHT));
    }

    /// The chosen shader over the right of the screen: the saber on its own above (the
    /// stage draws it in [`SABER_AREA`], turned by a drag; with no stage, its swatch,
    /// larger), and its card under it, framed in its tier's colour and alive as its tier
    /// is: the tier's mark and name and the edition, the name and its state, its facts as
    /// icons (a chroma, how many are held, where it comes from, its smith), what it is,
    /// since when it is the player's (with the team's note) or how to get it, its parts
    /// and Equip or Unequip.
    fn shader_card(&mut self, frame: &Frame, row: Row, inputs: &Inputs<'_>, seconds: f32) {
        let s = frame.s;
        self.scale = s;
        self.spin.advance(seconds);
        self.saber_area(frame, row, inputs, seconds);
        let [card_x, card_y, card_width, card_height] = SHADER_CARD;
        let area = frame.rect(card_x, card_y, card_width, card_height);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: CARD_RADIUS * s,
            color: color::alpha(color::SPACE, 0.82),
        });
        if let Some(tier) = row.tier() {
            // The tier's band along the top, and its glow under it, as on the grid's cards.
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(card_x + 24.0, card_y, card_width - 48.0, 4.0),
                radius: 2.0 * s,
                color: color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.5 }),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(card_x + 6.0, card_y + 6.0, card_width - 12.0, 56.0),
                radius: (CARD_RADIUS - 4.0) * s,
                color: color::alpha(tier.colour(), if row.owned { 0.09 } else { 0.04 }),
            });
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: CARD_RADIUS * s,
            width: 1.4 * s,
            color: row.frame_colour(),
        });
        if let Some(tier) = row.tier() {
            rarity_fx::draw(
                self.ui.draw_list_mut(),
                rarity_fx::Item::of(tier, row.owned, rarity_fx::Size::Full),
                area,
                CARD_RADIUS * s,
                s,
                seconds,
            );
        }
        let x = card_x + CARD_PAD;
        let width = card_width - CARD_PAD * 2.0;
        let mut y = card_y + 20.0;
        // Its tier's mark and name, or what the stock blade is; its edition at the right.
        let (kind, kind_colour) = match row.tier() {
            Some(tier) => (
                format!("{}  {}{}", tier.symbol(), tier.label(), row.number_words()),
                color::alpha(tier.colour(), if row.owned { 1.0 } else { 0.7 }),
            ),
            None => (format!("{}  Your saber", symbols::SABER), color::HOLO),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{kind}"),
            frame.rect(x, y, width, 28.0),
            21.0 * s,
            kind_colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let Some(skin) = row.skin {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "{}  {}",
                    symbols::EDITION,
                    crate::editions::name(skin.edition)
                ),
                frame.rect(x, y + 3.0, width, 22.0),
                15.0 * s,
                color::MUTED,
                FontWeight::Semibold,
                TextAlign::End,
            );
        }
        y += 36.0;
        let lit = row.owned && row.skin.is_some();
        // The name never runs under the state: it ends short of the pill, ellipsised,
        // whatever face and size the text is drawn in.
        let tag = row.state();
        let (name_rect, tag_rect) = match &tag {
            Some(tag) => {
                let (name, pill) =
                    title_rects(x, y, width, crate::text::display_width(tag, 15.0) + 40.0);
                (name, Some(pill))
            }
            None => ([x, y, width, 62.0], None),
        };
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", row.name()),
            frame.rect(name_rect[0], name_rect[1], name_rect[2], name_rect[3]),
            52.0 * s,
            if lit { color::GOLD_BRIGHT } else { color::TEXT },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let (Some(tag), Some([tag_x, tag_y, tag_width, tag_height])) = (&tag, tag_rect) {
            let rect = frame.rect(tag_x, tag_y, tag_width, tag_height);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 13.0 * s,
                width: 1.2 * s,
                color: if lit { color::GOLD } else { color::QUIET },
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{tag}"),
                frame.rect(tag_x, tag_y + 2.0, tag_width, 22.0),
                15.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
        y += 72.0;
        // Its facts, a chip each: a chroma, the copies held, where it comes from, its
        // smith.
        let mut left = x;
        if row.skin.is_some_and(|skin| skin.chroma) {
            left = self.fact(
                frame,
                [left, y],
                Fact::Chroma,
                "Takes your colour",
                color::HOLO,
            );
        }
        if row.count > 1 {
            let copies = format!("{} copies", row.count);
            left = self.fact(
                frame,
                [left, y],
                Fact::Icon(symbols::COPIES),
                &copies,
                color::HOLO,
            );
        }
        if let Some(skin) = row.skin {
            for (icon, from) in sources(skin) {
                left = self.fact(frame, [left, y], Fact::Icon(icon), &from, color::HOLO);
            }
            let smith = format!("Smith: {}", skin.smith);
            let _ = self.fact(
                frame,
                [left, y],
                Fact::Icon(symbols::SMITH),
                &smith,
                color::HOLO,
            );
        } else {
            let _ = self.fact(
                frame,
                [left, y],
                Fact::Icon(symbols::GIFT),
                "Every player's",
                color::HOLO,
            );
        }
        y += FACT_HEIGHT + 16.0;
        let description = row.skin.map_or(
            "Your blade in the colour you pick on Profile, Saber.",
            |skin| skin.description,
        );
        y = left_lines(
            &mut self.ui,
            frame,
            description,
            [x, y, width],
            (78, 3),
            18.0,
            color::TEXT,
            TextAlign::Start,
        );
        y += 6.0;
        let grant = row.skin.and_then(|skin| inputs.holdings.unlock(skin.id));
        let (line, colour) = match (row.skin, grant) {
            (None, _) if row.worn => (
                format!("{}  What you hold now: no shader on.", symbols::SABER),
                color::MUTED,
            ),
            (None, _) => ("Wearing it takes your shader off.".to_owned(), color::MUTED),
            (Some(_), Some(grant)) => (
                format!("{}  {}", symbols::TICK, owned_line(grant)),
                color::GOLD,
            ),
            (Some(skin), None) => (
                format!("{}  How to get it: {}", symbols::LOCK, skin.how_to_get),
                color::MUTED,
            ),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(x, y, width, 24.0),
            16.0 * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 26.0;
        if let Some(note) = grant
            .map(|grant| grant.note.as_str())
            .filter(|note| !note.is_empty() && *note != crate::debug_collectibles::NOTE)
        {
            let quoted = format!("\u{201c}{note}\u{201d}");
            y = left_lines(
                &mut self.ui,
                frame,
                &quoted,
                [x, y, width],
                (80, 1),
                16.0,
                color::MUTED,
                TextAlign::Start,
            );
        }
        // A locked shader a holocron may drop says which, and how likely.
        if let Some(skin) = row.skin.filter(|skin| !row.owned && skin.droppable) {
            self.drops(frame, skin, [x, y + 12.0]);
        }
        // Equip or Unequip at the foot of the card, then the second saber's, its parts
        // between.
        let button_y = card_y + card_height - CARD_PAD - 44.0;
        let shown_row = self.shader_rows.get(self.shader).copied();
        self.shader_options.clear();
        if let Some(skin) = row.skin.filter(|_| row.owned) {
            self.shader_parts(
                frame,
                skin.id,
                [x, y + 10.0, width, button_y - 12.0],
                inputs,
            );
        }
        let mut left = x;
        // Equip follows the target the switch over the saber chose (the gold pill says
        // which): the first saber, or the second (the staff's other half).
        let label = shown_row.and_then(|shown| {
            if self.both_sides() {
                shown.both.map(|_| {
                    if row.skin.is_none() {
                        ("Stock on both", true)
                    } else if shown.both_worn {
                        ("Unequip both", false)
                    } else {
                        ("Equip both", true)
                    }
                })
            } else if self.second_target {
                shown.second.map(|_| {
                    (
                        if shown.second_worn {
                            "Unequip"
                        } else {
                            "Equip"
                        },
                        !shown.second_worn,
                    )
                })
            } else {
                shown.wear.map(|wear| {
                    let label = if wear.is_empty() && row.skin.is_some() {
                        "Unequip"
                    } else if row.skin.is_none() {
                        "Wear the stock blade"
                    } else {
                        "Equip"
                    };
                    (label, !(wear.is_empty() && row.skin.is_some()))
                })
            }
        });
        if let Some((label, primary)) = label {
            let button_width = (crate::text::display_width(label, 18.0) + 56.0).max(180.0);
            kit::button(
                &mut self.ui,
                frame,
                [x, button_y, button_width, 44.0],
                label,
                primary,
                true,
                false,
                WEAR_TOKEN,
            );
            left += button_width + 12.0;
        }
        self.trade_buttons(frame, row, inputs, [left, button_y], x + width);
    }

    /// The way to the Forge at `[x, y]` on the card's foot, while the hub's economy is
    /// known (`docs/cards.md`, "The Forge"): "Craft in the Forge" for a shader of the
    /// trade not held, "Break down in the Forge" for one of the player's own copies (a
    /// medal's or a card's is not theirs to trade), with what it costs or gives. A click
    /// shows the Forge on that section with the shader chosen.
    fn trade_buttons(
        &mut self,
        frame: &Frame,
        row: Row,
        inputs: &Inputs<'_>,
        [x, y]: [f32; 2],
        right: f32,
    ) {
        self.forge_link = None;
        let Some(skin) = row
            .skin
            .filter(|skin| self.trade_known && crate::economy::in_trade(skin))
        else {
            return;
        };
        let s = frame.s;
        let unlock = inputs.holdings.unlock(skin.id);
        let own = unlock
            .filter(|unlock| unlock.medal.is_none() && unlock.card.is_none())
            .map_or(0, |unlock| unlock.count);
        let (section, label, line) = match (unlock, own) {
            (None, _) => (
                forge::Section::Craft,
                "Craft in the Forge".to_owned(),
                craft_words(skin, self.held_cards(inputs)),
            ),
            (Some(_), 0) => return,
            (Some(_), _) => (
                forge::Section::Break,
                "Break down in the Forge".to_owned(),
                format!(
                    "{}  +{} kyber shards a copy",
                    symbols::SHARD,
                    crate::economy::shards_text(crate::economy::shatter_yield(skin.tier))
                ),
            ),
        };
        let width = (crate::text::display_width(&label, 20.0) + 56.0).max(200.0);
        kit::button(
            &mut self.ui,
            frame,
            [x, y, width, 44.0],
            &label,
            false,
            true,
            false,
            FORGE_LINK_TOKEN,
        );
        self.forge_link = Some((section, skin.id));
        let left = x + width + 12.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{line}"),
            frame.rect(left, y + 11.0, (right - left).max(0.0), 22.0),
            15.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::End,
        );
    }

    /// The room above the card: the stage's lone saber, which a drag turns (a hint says
    /// so, and a locked one says it is a preview); with no stage, the swatch, larger.
    fn saber_area(&mut self, frame: &Frame, row: Row, inputs: &Inputs<'_>, seconds: f32) {
        let s = frame.s;
        let [x, y, width, height] = SABER_AREA;
        // The sabers take a drag (and a click) anywhere in their room, but the switch over
        // them is registered after, so a click on a pill reaches the pill: the later of two
        // overlapping regions is the one hit.
        if self.backstage != Backstage::None {
            self.ui
                .hit_region(SABER_TOKEN, frame.rect(x, y, width, height));
        }
        let switch_bottom = self.target_switch(frame, inputs, [x, y]);
        if self.backstage == Backstage::None {
            let rect = [x + 40.0, y + height * 0.5 - 59.0, width - 80.0, 118.0];
            match row.skin {
                None => swatch::small_stock(self.ui.draw_list_mut(), frame, rect, inputs.stock),
                Some(skin) => {
                    let look = self.skins.get(skin.id);
                    let _ = swatch::small_swatch(
                        self.ui.draw_list_mut(),
                        frame,
                        rect,
                        look,
                        row.owned,
                        seconds,
                        swatch::chroma_turn(look, swatch::rgb_bytes(inputs.stock)),
                    );
                }
            }
            return;
        }
        let hint_colour = if self.spin.held() {
            color::HOLO
        } else {
            color::QUIET
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}  Drag to turn{}",
                symbols::TURN,
                match self.pair {
                    crate::saber_skins::Pair::TwoSabers => {
                        ",  pick a saber above (or click it), choose a shader, Equip"
                    }
                    crate::saber_skins::Pair::Staff => {
                        ",  pick a half above (or click the staff), choose a shader, Equip"
                    }
                    crate::saber_skins::Pair::None => "",
                }
            ),
            frame.rect(x, y + height - 24.0, width, 22.0),
            15.0 * s,
            hint_colour,
            FontWeight::Semibold,
            TextAlign::End,
        );
        if !row.owned {
            let words = format!("{}  Preview: not yours yet", symbols::LOCK);
            let pill = crate::text::display_width(&words, 16.0) + 40.0;
            let rect = frame.rect(x, switch_bottom, pill, 34.0);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 17.0 * s,
                color: color::alpha(color::SPACE, 0.6),
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 17.0 * s,
                width: 1.2 * s,
                color: color::alpha(color::HOLO, 0.6),
            });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{words}"),
                frame.rect(x, switch_bottom + 6.0, pill, 22.0),
                16.0 * s,
                color::HOLO,
                FontWeight::Semibold,
                TextAlign::Center,
            );
        }
    }

    /// The target switch at `[x, y]`, over the lone saber, while the player holds two
    /// sabers or a staff: the first saber (or half) and the second, each with what it
    /// wears, the one Equip dresses lit in gold. Answers where what follows it may start
    /// (`y` without a switch).
    fn target_switch(&mut self, frame: &Frame, inputs: &Inputs<'_>, [x, y]: [f32; 2]) -> f32 {
        let s = frame.s;
        let (first_word, second_word) = match self.pair {
            // One saber of one blade: a second skin chosen before waits, and the page says
            // how to show it, in place of the switch.
            crate::saber_skins::Pair::None if !inputs.second.trim().is_empty() => {
                let words = format!(
                    "{}  Your 2nd blade shows with two sabers or a staff: Profile, Saber",
                    symbols::SABER
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{words}"),
                    frame.rect(x, y + 4.0, 700.0, 22.0),
                    15.0 * s,
                    color::QUIET,
                    FontWeight::Semibold,
                    TextAlign::Start,
                );
                return y + 34.0;
            }
            crate::saber_skins::Pair::None => return y,
            crate::saber_skins::Pair::TwoSabers => ("1st saber", "2nd saber"),
            crate::saber_skins::Pair::Staff => ("1st half", "Other half"),
        };
        let first = inputs.holdings.worn_blade_skin(inputs.setting);
        let first_name = first.map_or("Stock blade", |skin| skin.name);
        let second_name = match inputs.second.trim() {
            "" => "Same as the 1st",
            crate::looks::STOCK_SECOND => "Stock blade",
            id => unlockables::blade_skin(id)
                .filter(|skin| inputs.holdings.unlock(skin.id).is_some())
                .map_or("Same as the 1st", |skin| skin.name),
        };
        let (width, height) = (236.0, 52.0);
        for (index, (word, name, token)) in [
            (first_word, first_name, TARGET_FIRST_TOKEN),
            (second_word, second_name, TARGET_SECOND_TOKEN),
        ]
        .into_iter()
        .enumerate()
        {
            let left = x + index as f32 * (width + 10.0);
            let lit = self.both_sides() || self.second_target == (index == 1);
            let hovered = self.ui.token_hovered(token);
            let rect = frame.rect(left, y, width, height);
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 14.0 * s,
                color: if lit {
                    color::alpha(color::GOLD, 0.14)
                } else {
                    color::alpha(color::SPACE, 0.62)
                },
            });
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect,
                radius: 14.0 * s,
                width: if lit { 1.6 * s } else { 1.2 * s },
                color: if lit {
                    color::GOLD
                } else if hovered {
                    color::alpha(color::HOLO, 0.6)
                } else {
                    color::alpha(color::HOLO, 0.25)
                },
            });
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{}  {word}", symbols::SABER),
                frame.rect(left + 16.0, y + 6.0, width - 32.0, 18.0),
                13.0 * s,
                if lit { color::GOLD } else { color::QUIET },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{name}"),
                frame.rect(left + 16.0, y + 24.0, width - 32.0, 24.0),
                18.0 * s,
                if lit {
                    color::GOLD_BRIGHT
                } else {
                    color::MUTED
                },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            self.ui.hit_region(token, rect);
        }
        // The "Both" box, beside the pills: Equip dresses both sides at once.
        let left = x + 2.0 * (width + 10.0) + 8.0;
        let middle = y + height * 0.5;
        let both_word = match self.pair {
            crate::saber_skins::Pair::Staff => "Both halves",
            _ => "Both sabers",
        };
        // Drawn in the face it is measured in, with room to spare (Creyon, 21/10/2026: in
        // the body face it was wider than measured and cut to "Both sab...").
        let label_width = crate::text::display_width(both_word, 16.0) + 16.0;
        let hovered = self.ui.token_hovered(BOTH_TOKEN);
        let after = kit::tick(&mut self.ui, frame, left, middle, self.both, hovered);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{both_word}"),
            frame.rect(after + 10.0, middle - 11.0, label_width, 22.0),
            16.0 * s,
            if self.both { color::GOLD } else { color::MUTED },
            FontWeight::Semibold,
            TextAlign::Start,
        );
        self.ui.hit_region(
            BOTH_TOKEN,
            frame.rect(left - 6.0, y, after - left + label_width + 22.0, height),
        );
        y + height + 10.0
    }

    /// Where a locked `skin` drops, from `[x, y]`: a chip for each holocron tier that may
    /// bring it, in the tier's colour, with the chance an opening of one does (the hub's
    /// odds, [`crate::loot_odds`], split evenly among the droppable shaders of its tier).
    fn drops(&mut self, frame: &Frame, skin: &unlockables::Unlockable, [x, y]: [f32; 2]) {
        let s = frame.s;
        let rarity = skin.tier as usize;
        let pool = unlockables::blade_skins()
            .filter(|other| other.droppable && other.tier == skin.tier)
            .count();
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}  Where it drops, a chance per opening",
                symbols::HOLOCRON
            ),
            frame.rect(x, y, SHADER_CARD[2] - CARD_PAD * 2.0, 22.0),
            16.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut left = x;
        for tier in &crate::holocrons::TIERS {
            let chance = crate::cards::loot::shader_in(tier.index, rarity, pool);
            if chance <= 0.0 {
                continue;
            }
            let words = format!("{} {}", tier.label, crate::cards::loot::percent(chance));
            left = self.fact(
                frame,
                [left, y + 30.0],
                Fact::Icon(symbols::RARITY[tier.index + 1]),
                &words,
                tier.colour,
            );
        }
    }

    /// One of the card's facts at `[x, y]`: a chip with its icon (or the chroma's colour
    /// wheel) in `tint` and its words. Returns where the next one starts.
    fn fact(
        &mut self,
        frame: &Frame,
        [x, y]: [f32; 2],
        fact: Fact,
        words: &str,
        tint: sjk_ui::Color,
    ) -> f32 {
        let s = frame.s;
        let width = 14.0 + FACT_ICON + 8.0 + crate::text::display_width(words, 16.0) + 16.0;
        let rect = frame.rect(x, y, width, FACT_HEIGHT);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect,
            radius: FACT_HEIGHT * 0.5 * s,
            color: color::alpha(color::SPACE, 0.55),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect,
            radius: FACT_HEIGHT * 0.5 * s,
            width: 1.0 * s,
            color: color::alpha(color::HOLO, 0.35),
        });
        let icon_x = x + 14.0;
        match fact {
            Fact::Chroma => swatch::chroma_mark(
                self.ui.draw_list_mut(),
                frame,
                icon_x,
                y + (FACT_HEIGHT - FACT_ICON) * 0.5,
                FACT_ICON,
                1.0,
            ),
            Fact::Icon(icon) => text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{icon}"),
                frame.rect(icon_x - 2.0, y + 6.0, FACT_ICON + 4.0, 22.0),
                17.0 * s,
                tint,
                FontWeight::Semibold,
                TextAlign::Center,
            ),
        }
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{words}"),
            frame.rect(icon_x + FACT_ICON + 8.0, y + 6.0, width, 22.0),
            16.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        x + width + FACT_GAP
    }

    /// The chosen shader's parts its wearer may switch off, under its words from `y`:
    /// a tick each in two columns (three past four), named as its file names them,
    /// ticked while on; a click or its number key switches it, on the model at once and,
    /// worn, for every SJK player.
    fn shader_parts(
        &mut self,
        frame: &Frame,
        skin: &'static str,
        [x, y, width, bottom]: [f32; 4],
        inputs: &Inputs<'_>,
    ) {
        let Some(look) = self.skins.get(skin) else {
            return;
        };
        let options = look.options();
        if options.is_empty() {
            return;
        }
        let s = frame.s;
        let columns = if options.len() > 4 { 3 } else { 2 };
        let column = width / columns as f32;
        // Never under `bottom` (the card's button): the rows close up when room is short.
        let rows = options.len().div_ceil(columns);
        let step = ((bottom - (y + 28.0)) / rows as f32).clamp(26.0, 36.0);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Parts"),
            frame.rect(x, y, width, 22.0),
            16.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let shown: Vec<(&'static str, String, bool, String)> = options
            .iter()
            .map(|option| {
                let on = crate::saber_skin_options::is_on(inputs.options, skin, &option.id);
                (skin, option.id.clone(), on, option.name.clone())
            })
            .collect();
        for (index, (_, _, on, name)) in shown.iter().enumerate() {
            let token = OPTION_BASE + index as u16;
            let left = x + (index % columns) as f32 * column;
            let middle = y + 28.0 + (index / columns) as f32 * step + step * 0.5;
            let hovered = self.ui.token_hovered(token);
            let after = kit::tick(&mut self.ui, frame, left, middle, *on, hovered);
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{name}"),
                frame.rect(
                    after + 10.0,
                    middle - 11.0,
                    column - kit::TICK_BOX - 18.0,
                    22.0,
                ),
                16.0 * s,
                if *on { color::TEXT } else { color::MUTED },
                FontWeight::Semibold,
                TextAlign::Start,
            );
            self.ui.hit_region(
                token,
                frame.rect(left - 4.0, middle - step * 0.5, column - 8.0, step),
            );
        }
        self.shader_options = shown
            .into_iter()
            .map(|(skin, option, on, _)| (skin, option, on))
            .collect();
    }
}

/// What crafting `skin` costs, said beside its "Craft in the Forge": lowered, and the
/// blade card named, while `cards` (the player's) hold a copy of its blade card
/// (21/10/2026).
fn craft_words(
    skin: &'static crate::unlockables::Unlockable,
    cards: &[sjk_identity::CardStack],
) -> String {
    let card = crate::economy::blade_card(skin, cards, &crate::cards::faces::current());
    let (cost, off) = crate::economy::craft_price(skin, card);
    let cost = crate::economy::shards_text(cost);
    match off {
        Some((finish, off)) => format!(
            "{}  {cost} kyber shards to craft, {} card -{off} %",
            symbols::SHARD,
            finish.name
        ),
        None => format!("{}  {cost} kyber shards to craft", symbols::SHARD),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::sjk::Frame;

    #[test]
    fn the_owned_line_names_the_medal_that_brings_a_shader() {
        let grant = |medal: Option<&str>| sjk_identity::Unlock {
            id: "saber_glitch".to_owned(),
            granted: 0,
            note: String::new(),
            count: 1,
            medal: medal.map(str::to_owned),
            card: None,
        };
        assert_eq!(owned_line(&grant(None)), "Yours, from the SJK team");
        assert_eq!(
            owned_line(&grant(Some("bug_hunter"))),
            "Yours, with your Bug Hunter medal"
        );
        assert_eq!(
            owned_line(&grant(Some("from_the_future"))),
            "Yours, from the SJK team"
        );
        let dated = sjk_identity::Unlock {
            granted: 1_791_000_000,
            ..grant(Some("early_tester"))
        };
        assert!(owned_line(&dated).starts_with("Yours since "));
        let twice = sjk_identity::Unlock {
            count: 3,
            ..grant(None)
        };
        assert_eq!(owned_line(&twice), "Yours, from the SJK team, 3 held");
        assert_eq!(held_words("Yours", 1), "Yours");
        assert_eq!(held_words("Worn", 2), "Worn x2");
        assert!(owned_line(&dated).ends_with(", with your Early Tester medal"));
        let card = sjk_identity::Unlock {
            id: "saber_sun".to_owned(),
            card: Some("sjk_unknown_999".to_owned()),
            ..grant(None)
        };
        assert_eq!(owned_line(&card), "Yours, with your sjk_unknown_999 card");
    }

    /// Whether two window rectangles share any area.
    fn overlap(a: sjk_ui::Rect, b: sjk_ui::Rect) -> bool {
        a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
    }

    #[test]
    fn names_never_run_under_the_state_or_the_chroma_mark() {
        let longest = unlockables::blade_skins()
            .map(|skin| skin.name)
            .max_by(|a, b| {
                crate::text::display_width(a, 56.0).total_cmp(&crate::text::display_width(b, 56.0))
            })
            .unwrap();
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
        ] {
            let frame = Frame::new(viewport);
            let window = |r: [f32; 4]| frame.rect(r[0], r[1], r[2], r[3]);
            for chroma in [false, true] {
                let (mark, name, status) = rack_words(RACK_TOP, chroma);
                let (mark, name, status) = (window(mark), window(name), window(status));
                assert!(!overlap(name, status), "{viewport:?}");
                if chroma {
                    assert!(
                        !overlap(mark, name) && !overlap(mark, status),
                        "{viewport:?}"
                    );
                }
                // Inside the rack, with room for the longest name at its size.
                let rack_right = frame.rect(LEFT_X - 12.0 + RACK_WIDTH, 0.0, 0.0, 0.0).x;
                assert!(name.x + name.width <= rack_right);
                assert!(name.width >= crate::text::display_width(longest, 28.0) * frame.s);
            }
            let inner_x = SHADER_CARD[0] + CARD_PAD;
            let inner_width = SHADER_CARD[2] - CARD_PAD * 2.0;
            for tag in ["Worn", "Yours", "Locked"] {
                let (name, pill) = title_rects(
                    inner_x,
                    SHADER_CARD[1] + 56.0,
                    inner_width,
                    crate::text::display_width(tag, 15.0) + 40.0,
                );
                let (name, pill) = (window(name), window(pill));
                assert!(!overlap(name, pill), "{viewport:?} {tag}");
                assert!(
                    pill.x + pill.width <= window([inner_x + inner_width, 0.0, 0.0, 0.0]).x + 0.01
                );
                assert!(
                    name.width > 200.0 * frame.s,
                    "{viewport:?}: room for a name"
                );
            }
        }
    }

    /// The lone saber's room and the card under it stand clear of each other, of the
    /// rack and the grid on the left and of the keys' line.
    #[test]
    fn the_saber_and_its_card_keep_to_the_right() {
        let [sx, sy, sw, sh] = SABER_AREA;
        let [cx, cy, cw, ch] = SHADER_CARD;
        assert!(sy + sh <= cy, "the saber above the card");
        assert!(cy + ch < view::KEYS_Y, "the card above the keys");
        let list_right =
            (LEFT_X - 12.0 + RACK_WIDTH).max(card_rect(GRID_COLUMNS - 1, 0)[0] + CARD[0]);
        assert!(sx > list_right + 30.0 && cx > list_right + 30.0);
        assert!(sx + sw <= RIGHT_X && cx + cw <= RIGHT_X);
        // A shader's facts fit along the card: the widest a chroma held twice from a
        // medal could ask.
        let facts = [
            "Takes your colour",
            "2 copies",
            "Early Contributor medal",
            "Smith: Creyon",
        ]
        .iter()
        .map(|words| 14.0 + FACT_ICON + 8.0 + crate::text::display_width(words, 16.0) + 16.0)
        .sum::<f32>()
            + FACT_GAP * 3.0;
        assert!(facts <= cw - CARD_PAD * 2.0, "{facts}");
    }

    #[test]
    fn the_rack_scrolls_only_to_keep_the_chosen_row_in_view() {
        let rows = SHADER_ROWS;
        assert!(rows > RACK_SHOWN, "more shaders than the rack shows");
        // Within the window the rack holds still (a pointer moving over the rows
        // chooses them without scrolling them away).
        assert_eq!(rack_window(0, RACK_SHOWN - 1, rows), 0);
        assert_eq!(rack_window(3, 4, rows), 3);
        // Past either end it moves just enough.
        assert_eq!(rack_window(0, RACK_SHOWN, rows), 1);
        assert_eq!(rack_window(5, 2, rows), 2);
        assert_eq!(rack_window(0, rows - 1, rows), rows - RACK_SHOWN);
        assert_eq!(rack_window(40, 0, rows), 0);
    }

    #[test]
    fn the_wheel_scrolls_the_rack_and_keeps_the_chosen_row_shown() {
        let mut panel = Panel::new();
        panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
        // As the first frame leaves it, the stock blade worn.
        panel.shader = 0;
        let wheel = |panel: &mut Panel, y: f32| {
            let _ = panel.handle_pointer(
                InputEvent::PointerWheel {
                    position: sjk_ui::Vec2::new(500.0, 500.0),
                    delta: sjk_ui::Vec2::new(0.0, y),
                },
                false,
            );
        };
        for _ in 0..3 {
            wheel(&mut panel, -1.0);
        }
        assert_eq!(panel.shader_first, 3);
        assert_eq!(panel.shader, 3, "the chosen row follows into view");
        for _ in 0..SHADER_ROWS {
            wheel(&mut panel, -1.0);
        }
        assert_eq!(panel.shader_first, SHADER_ROWS - RACK_SHOWN);
        wheel(&mut panel, 1.0);
        assert_eq!(panel.shader_first, SHADER_ROWS - RACK_SHOWN - 1);
    }

    /// Every row and the words beside the model fit the canvas over the keys, for an
    /// owned and worn skin, a locked one and the stock blade, with and without a model,
    /// at 1080 lines, 4K, 4:3 and 21:9.
    #[test]
    fn the_rack_and_its_words_fit_over_the_keys() {
        let sun = [sjk_identity::Unlock {
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: "a long note from the team ".repeat(8),
            count: 1,
            medal: None,
            card: None,
        }];
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            for backstage in [Backstage::None, Backstage::World { head: None }] {
                for row in 0..SHADER_ROWS {
                    let inputs = super::super::tests::inputs(
                        unlockables::Holdings::Known(&sun),
                        "saber_sun",
                    );
                    let mut panel = Panel::new();
                    panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
                    panel.set_backstage(backstage);
                    panel.shader = row;
                    panel.build(&inputs, &font, viewport);
                    assert!(!panel.ui.overflowed(), "{viewport:?}");
                    let frame = Frame::new(viewport);
                    let keys = frame.point(0.0, super::super::view::KEYS_Y)[1];
                    for command in panel.ui.draw_list().commands() {
                        if let DrawCommand::Text { rect, .. } = command {
                            assert!(rect.bottom() <= keys || rect.y >= keys, "{row} {rect:?}");
                            assert!(rect.right() <= viewport[0] + 1.0, "{rect:?}");
                        }
                    }
                    // The rows in view answer the pointer, above the keys; the rest
                    // are scrolled off.
                    let first = panel.shader_first;
                    assert!((first..first + RACK_SHOWN).contains(&row), "{row}");
                    for index in 0..SHADER_ROWS {
                        let area = panel.ui.rect_for(SHADER_BASE + index as u16);
                        let shown = (first..first + RACK_SHOWN).contains(&index);
                        assert_eq!(area.is_some(), shown, "{index}");
                        if let Some(area) = area {
                            assert!(area.bottom() <= keys, "{index} {viewport:?}");
                        }
                    }
                }
            }
        }
    }
}
