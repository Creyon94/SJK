//! The settings rows as the SJK UI draws them (`docs/sjk-ui.md`, Settings):
//! the categories down a lit rail on the left, the open category's rows in the
//! middle under their sub-headings, the focused setting explained in a column
//! on the right, the search at the top. The rows are a classic+ panel's
//! ([`super::ClassicRows`]), so search, lists, defaults and typed numbers work
//! as they do there; only the drawing and the frame differ.
//!
//! Positions are pixels of the SJK UI's 16:9 frame ([`Frame`]).

use super::classic_view::{
    CLASSIC_SCROLL_TOKEN, CLASSIC_SCROLLBAR_TOKEN, DROPDOWN_BASE, MAX_SEGMENTS, Number, RowControl,
    choice_label, group_icon, segment_choices, switch_text_on,
};
use super::help;
use super::*;
use crate::menu::classic::panel::{CHROME_BASE, SEARCH_TOKEN};
use crate::menu::sjk::{
    Frame, SEARCH_PILL, SearchPill, TextTarget, color, fade_across, key_hint, key_hint_width, kit,
    text, wrap,
};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The categories down the rail, as the menu lists them: each one's name and
/// settings icon (`settings_icons`), and the one on show (none while a search
/// shows its results).
pub(crate) struct Rail<'a> {
    pub(crate) categories: &'a [(&'a str, &'a str)],
    pub(crate) current: Option<usize>,
    /// Where the way back leads: "Main menu", or "Game menu" over a match.
    pub(crate) back: &'a str,
}

/// The back key's target, as the settings form's back cap.
const BACK_TOKEN: u16 = 900;

/// The search pill (`menu::sjk::top_bar`'s).
const SEARCH: [f32; 4] = SEARCH_PILL;
/// The rail's line, its first category's top and the step between them.
const RAIL_X: f32 = 96.0;
const RAIL_TOP: f32 = 210.0;
const CATEGORY_STEP: f32 = 60.0;
const CATEGORY_HEIGHT: f32 = 52.0;
const CATEGORY_WIDTH: f32 = 330.0;
/// The rows' column, the top of its first line, a line's height (a row or a
/// sub-heading) and how many lines show before it scrolls.
pub(crate) const ROWS_X: f32 = 470.0;
pub(crate) const ROWS_WIDTH: f32 = 800.0;
pub(crate) const ROWS_TOP: f32 = 176.0;
pub(crate) const LINE: f32 = 56.0;
pub(crate) const VISIBLE: usize = 14;
/// A row's name starts here; its control ends at [`CONTROL_RIGHT`] and starts
/// no further left than [`CONTROL_LEFT`]; the reset arrow is centred after it.
pub(crate) const LABEL_X: f32 = ROWS_X + 22.0;
pub(crate) const CONTROL_RIGHT: f32 = ROWS_X + ROWS_WIDTH - 44.0;
pub(super) const CONTROL_LEFT: f32 = CONTROL_RIGHT - 330.0;
const RESET_X: f32 = ROWS_X + ROWS_WIDTH - 22.0;
/// A field's width, a slider's track and its number.
const FIELD_WIDTH: f32 = 230.0;
const TRACK_WIDTH: f32 = 230.0;
const NUMBER_WIDTH: f32 = 46.0;
const TRACK_X: f32 = CONTROL_RIGHT - NUMBER_WIDTH - 18.0 - TRACK_WIDTH;
/// An open list's rows, and the lowest it reaches.
const LIST_ROW: f32 = 40.0;
const LIST_BOTTOM: f32 = KEYS_Y - 16.0;
/// The detail column.
pub(crate) const DETAIL_X: f32 = 1360.0;
pub(crate) const DETAIL_TOP: f32 = 190.0;
pub(crate) const DETAIL_WIDTH: f32 = 464.0;
/// The keys' line at the bottom.
pub(crate) const KEYS_Y: f32 = 992.0;

/// Body type's width per character, as a share of its size: Exo 2 sets a
/// setting's name at about 0.4 em a character.
pub(crate) const BODY_ADVANCE: f32 = 0.4;

impl SettingsMenu {
    /// Draw the open panel rows as the SJK UI's Settings screen, with `rail`'s
    /// categories, at `reveal` opacity.
    pub(crate) fn append_sjk(
        &mut self,
        target: TextTarget<'_>,
        viewport: [f32; 2],
        reveal: f32,
        rail: &Rail<'_>,
    ) {
        let frame = Frame::new(viewport);
        if self.wheel.is_open() {
            self.sjk_controls = None;
            self.append_wheel_sjk(target, viewport, reveal, rail);
            return;
        }
        self.ui.begin_transparent(viewport);
        self.ui.push_opacity(reveal);
        backdrop(&mut self.ui, viewport);
        self.top_bar(&frame, rail.back);
        draw_rail(&mut self.ui, &frame, rail);
        let slots = self.sjk_rows(&frame, VISIBLE, LIST_BOTTOM);
        self.sjk_detail(&frame, rail);
        self.sjk_keys(&frame);
        self.sjk_dropdown(&frame, &slots, VISIBLE, LIST_BOTTOM);
        self.ui.pop_opacity();
        self.ui.finish(self.selected as u16);
        target.append(&self.ui, viewport);
        self.sjk_controls = Some(frame.point(CONTROL_LEFT, 0.0)[0]);
    }

    /// The way back (to `back`), the screen's name and the search.
    fn top_bar(&mut self, frame: &Frame, back: &str) {
        let found = (!self.search.trim().is_empty()).then(|| {
            self.classic
                .as_ref()
                .map_or(0, |classic| classic.rows().count())
        });
        top_bar(
            &mut self.ui,
            frame,
            back,
            &self.search,
            self.searching,
            "Find a setting",
            found,
        );
    }

    /// The rows on show, at most `visible` lines of them, and their controls,
    /// those under the open list (which ends above `list_bottom`) left out;
    /// returns each shown row's line top.
    pub(super) fn sjk_rows(
        &mut self,
        frame: &Frame,
        visible: usize,
        list_bottom: f32,
    ) -> Vec<(usize, f32)> {
        let s = frame.s;
        let lines: Vec<Line> = match &mut self.classic {
            Some(classic) => {
                classic.visible = visible;
                classic.first = classic.first.min(classic.max_first());
                // Where the track lies across a row, for the pointer.
                classic.slider_span = ((TRACK_X - ROWS_X) / ROWS_WIDTH, TRACK_WIDTH / ROWS_WIDTH);
                classic.lines.clone()
            }
            None => Vec::new(),
        };
        let first = self.classic.as_ref().map_or(0, |classic| classic.first);
        let shown = first..lines.len().min(first + visible);
        if lines.len() > visible {
            self.ui.scroll_region(
                CLASSIC_SCROLL_TOKEN,
                frame.rect(ROWS_X, ROWS_TOP, ROWS_WIDTH, visible as f32 * LINE),
            );
        }
        if lines.is_empty() && !self.search.trim().is_empty() {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("No setting matches \u{201c}{}\u{201d}.", self.search.trim()),
                frame.rect(LABEL_X, ROWS_TOP + 14.0, ROWS_WIDTH - 44.0, 28.0),
                19.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        // An open list hides the controls of the rows under it: text draws
        // over every shape, so the list could not.
        let covered = self.list_rect(&lines[shown.clone()], list_bottom);
        let mut slots = Vec::with_capacity(visible);
        for (index, line) in lines[shown.clone()].iter().enumerate() {
            let top = ROWS_TOP + index as f32 * LINE;
            let row = match *line {
                Line::Heading(heading) => {
                    kit::heading(&mut self.ui, frame, ROWS_X, top + 36.0, ROWS_WIDTH, heading);
                    continue;
                }
                Line::Row(row) => row,
            };
            slots.push((row, top));
            let open = self.dropdown.as_ref().is_some_and(|open| open.row == row);
            let under_list = !open
                && covered.is_some_and(|[_, y, _, height]| top + LINE > y && top < y + height);
            self.sjk_row(frame, row, top, under_list);
        }
        if lines.len() > visible {
            self.ui.scrollbar(
                CLASSIC_SCROLLBAR_TOKEN,
                frame.rect(
                    ROWS_X + ROWS_WIDTH + 14.0,
                    ROWS_TOP,
                    4.0,
                    visible as f32 * LINE,
                ),
                first,
                visible,
                lines.len(),
            );
        }
        slots
    }

    /// Row `row` on the line whose top is `top`: its band when focused, its
    /// name, the changed dot, its control (left out `under_list`) and the reset
    /// arrow.
    fn sjk_row(&mut self, frame: &Frame, row: usize, top: f32, under_list: bool) {
        let s = frame.s;
        let Some(setting) = self.rows().get(row).copied() else {
            return;
        };
        let focused = row == self.selected;
        let middle = top + LINE * 0.5;
        if focused {
            kit::band(&mut self.ui, frame, [ROWS_X, top, ROWS_WIDTH, LINE]);
        }
        let (label, _) = help::classic_label(setting.cvar, setting.label);
        let label_width = CONTROL_LEFT - LABEL_X - 30.0;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{label}"),
            frame.rect(LABEL_X, middle - 14.0, label_width, 28.0),
            19.0 * s,
            if focused {
                Color::new(1.0, 1.0, 1.0, 1.0)
            } else {
                color::alpha(color::TEXT, 0.88)
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        let changed = self
            .defaults
            .get(row)
            .is_some_and(|default| default.changed);
        if changed {
            let width = (label.chars().count() as f32 * 19.0 * BODY_ADVANCE).min(label_width);
            kit::changed_dot(&mut self.ui, frame, LABEL_X + width + 12.0, middle);
        }
        self.ui
            .hit_region(row as u16, frame.rect(ROWS_X, top, ROWS_WIDTH, LINE));
        if !under_list {
            self.sjk_control(frame, row, setting, middle, focused);
        }
        if focused && changed {
            kit::reset(
                &mut self.ui,
                frame,
                RESET_X,
                middle,
                RowControl::Reset(row).token(),
            );
        }
    }

    /// Row `row`'s control, centred on `middle`.
    fn sjk_control(
        &mut self,
        frame: &Frame,
        row: usize,
        setting: Setting,
        middle: f32,
        focused: bool,
    ) {
        let value = self.values.get(row).cloned().unwrap_or_default();
        let field = [
            CONTROL_RIGHT - FIELD_WIDTH,
            middle - kit::CONTROL_HEIGHT * 0.5,
            FIELD_WIDTH,
            kit::CONTROL_HEIGHT,
        ];
        match setting.kind {
            ValueKind::Bool => {
                kit::switch(
                    &mut self.ui,
                    frame,
                    CONTROL_RIGHT,
                    middle,
                    switch_text_on(&value),
                    focused,
                );
            }
            ValueKind::Integer { min, max, .. } => {
                let ratio = value
                    .parse::<f64>()
                    .ok()
                    .filter(|_| max > min)
                    .map_or(0.0, |value| {
                        ((value - min as f64) / (max - min) as f64) as f32
                    });
                self.sjk_slider(frame, row, middle, ratio, &value, focused);
            }
            ValueKind::Float { min, max, .. } => {
                let ratio = value
                    .parse::<f64>()
                    .ok()
                    .filter(|_| max > min)
                    .map_or(0.0, |value| ((value - min) / (max - min)) as f32);
                self.sjk_slider(frame, row, middle, ratio, &value, focused);
            }
            kind if segment_choices(kind).is_some() => {
                let values = segment_choices(kind).unwrap_or_default();
                let current = values
                    .iter()
                    .position(|candidate| candidate.eq_ignore_ascii_case(&value))
                    .unwrap_or(0);
                let labels: [&str; MAX_SEGMENTS] =
                    std::array::from_fn(|index| values.get(index).map_or("", |v| choice_label(v)));
                kit::segments(
                    &mut self.ui,
                    frame,
                    CONTROL_RIGHT,
                    middle,
                    &labels[..values.len()],
                    current,
                    focused,
                    RowControl::Segment { row, index: 0 }.token(),
                );
            }
            ValueKind::Choice(_)
            | ValueKind::Resolution
            | ValueKind::DisplayMode
            | ValueKind::HudPicker
            | ValueKind::WheelPages => {
                let open = self.dropdown.as_ref().is_some_and(|open| open.row == row);
                kit::field(
                    &mut self.ui,
                    frame,
                    field,
                    format_args!(
                        "{}",
                        crate::menu::classic::view::Sentence(&shown_value(&value))
                    ),
                    focused || open,
                    true,
                );
            }
            ValueKind::Text => match self
                .editing
                .as_ref()
                .filter(|draft| draft.row == row)
                .map(|draft| draft.text.as_str())
            {
                Some(buffer) => {
                    kit::field(
                        &mut self.ui,
                        frame,
                        field,
                        format_args!("{buffer}_"),
                        true,
                        false,
                    );
                }
                None => {
                    kit::field(
                        &mut self.ui,
                        frame,
                        field,
                        format_args!("{value}"),
                        focused,
                        false,
                    );
                }
            },
        }
    }

    /// A slider's track, knob and number (or the number being typed).
    fn sjk_slider(
        &mut self,
        frame: &Frame,
        row: usize,
        middle: f32,
        ratio: f32,
        value: &str,
        focused: bool,
    ) {
        let s = frame.s;
        kit::slider(
            &mut self.ui,
            frame,
            TRACK_X,
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
        self.ui.hit_region(
            crate::menu_widgets::numeric::VALUE_BASE + row as u16,
            number,
        );
        match self.numeric.as_ref().filter(|edit| edit.row == row) {
            Some(edit) => {
                self.ui.set_family(TextFamily::Display);
                edit.draw(&mut self.ui, number, 1.4 * s);
                self.ui.set_family(TextFamily::Body);
            }
            None => text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{}", shown_value(value)),
                number,
                21.0 * s,
                if focused {
                    color::TEXT
                } else {
                    color::alpha(color::TEXT, 0.9)
                },
                FontWeight::Regular,
                TextAlign::End,
            ),
        }
    }

    /// Where the open list lies (frame pixels), when its row is among the
    /// lines shown, `lines`: under the row's field, or over it when it would
    /// pass `bottom` first.
    fn list_rect(&self, lines: &[Line], bottom: f32) -> Option<[f32; 4]> {
        let dropdown = self.dropdown.as_ref()?;
        let index = lines
            .iter()
            .position(|line| *line == Line::Row(dropdown.row))?;
        let middle = ROWS_TOP + index as f32 * LINE + LINE * 0.5;
        let height = 16.0 + dropdown.picks.len() as f32 * LIST_ROW;
        let below = middle + kit::CONTROL_HEIGHT * 0.5 + 6.0;
        let y = if below + height <= bottom {
            below
        } else {
            (middle - kit::CONTROL_HEIGHT * 0.5 - 6.0 - height).max(ROWS_TOP)
        };
        Some([CONTROL_RIGHT - FIELD_WIDTH, y, FIELD_WIDTH, height])
    }

    /// The open list, last so it takes the pointer over the rows, above
    /// `bottom`; a list whose row scrolled away (`slots` are the rows shown of
    /// at most `visible` lines) closes.
    pub(super) fn sjk_dropdown(
        &mut self,
        frame: &Frame,
        slots: &[(usize, f32)],
        visible: usize,
        bottom: f32,
    ) {
        let Some(dropdown) = &self.dropdown else {
            return;
        };
        if !slots.iter().any(|(row, _)| *row == dropdown.row) {
            self.dropdown = None;
            return;
        }
        let Some(classic) = &self.classic else {
            return;
        };
        let shown = &classic.lines[classic.first..classic.lines.len().min(classic.first + visible)];
        let Some(rect) = self.list_rect(shown, bottom) else {
            return;
        };
        let labels: Vec<&str> = dropdown
            .picks
            .iter()
            .map(|pick| Self::pick_label(*pick))
            .collect();
        let (highlighted, current) = (dropdown.highlighted, dropdown.current);
        kit::list(
            &mut self.ui,
            frame,
            rect,
            LIST_ROW,
            &labels,
            highlighted,
            current,
            DROPDOWN_BASE,
        );
    }

    /// The focused setting explained: its group's icon and name, what it does,
    /// then its default, range or choices, when a change applies, where a
    /// search found it and its console name.
    fn sjk_detail(&mut self, frame: &Frame, rail: &Rail<'_>) {
        let s = frame.s;
        let Some(setting) = self
            .shows(self.selected)
            .then(|| self.rows().get(self.selected).copied())
            .flatten()
        else {
            return;
        };
        let found_in = (self.section == Section::Search)
            .then(|| search::group(self.selected))
            .flatten();
        let icon = match found_in {
            Some(group) => group_icon(group),
            None => rail
                .current
                .and_then(|index| rail.categories.get(index))
                .map(|(_, icon)| *icon),
        };
        let (title, timing) = help::timing(setting.label);
        let title_x = DETAIL_X + if icon.is_some() { 58.0 } else { 0.0 };
        let title_lines: Vec<&str> = wrap(title, 24).take(2).collect();
        let title_height = title_lines.len().max(1) as f32 * 38.0;
        if let Some(texture) = icon.and_then(crate::settings_icons::texture) {
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(
                    DETAIL_X,
                    DETAIL_TOP + (title_height - 44.0) * 0.5,
                    44.0,
                    44.0,
                ),
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
        }
        for (index, line) in title_lines.iter().enumerate() {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{line}"),
                frame.rect(
                    title_x,
                    DETAIL_TOP + index as f32 * 38.0,
                    DETAIL_X + DETAIL_WIDTH - title_x,
                    40.0,
                ),
                32.0 * s,
                color::TEXT,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let mut y = DETAIL_TOP + title_height + 22.0;
        if let Some(help) = help::help(setting.cvar) {
            for line in wrap(help, 44).take(9) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{line}"),
                    frame.rect(DETAIL_X, y, DETAIL_WIDTH, 30.0),
                    19.0 * s,
                    color::alpha(color::TEXT, 0.86),
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 30.0;
            }
            y += 22.0;
        }
        let default = self
            .defaults
            .get(self.selected)
            .and_then(|default| default.text.clone());
        if let Some(default) = default {
            self.fact(
                frame,
                &mut y,
                "Default",
                format_args!("{}", shown_value(&default)),
                color::TEXT,
            );
        }
        match setting.kind {
            ValueKind::Integer { min, max, .. } if min < 0 => {
                self.fact(
                    frame,
                    &mut y,
                    "Range",
                    format_args!("Auto, 0 to {max}"),
                    color::TEXT,
                );
            }
            ValueKind::Integer { min, max, .. } => {
                self.fact(
                    frame,
                    &mut y,
                    "Range",
                    format_args!("{min} to {max}"),
                    color::TEXT,
                );
            }
            ValueKind::Float { min, max, .. } => {
                self.fact(
                    frame,
                    &mut y,
                    "Range",
                    format_args!("{} to {}", Number(min), Number(max)),
                    color::TEXT,
                );
            }
            ValueKind::Choice(choices) => {
                self.fact(
                    frame,
                    &mut y,
                    "Choices",
                    format_args!("{}", Choices(choices)),
                    color::TEXT,
                );
            }
            _ => {}
        }
        if let Some(note) = timing.note() {
            let when = note.strip_prefix("applies ").unwrap_or(note);
            self.fact(
                frame,
                &mut y,
                "Applies",
                format_args!("{when}"),
                color::GOLD_BRIGHT,
            );
        }
        if let Some(group) = found_in {
            self.fact(frame, &mut y, "In", format_args!("{group}"), color::TEXT);
        }
        let source = match setting.kind {
            ValueKind::WheelPages => "File",
            _ => "Console",
        };
        self.fact(
            frame,
            &mut y,
            source,
            format_args!("{}", setting.cvar),
            color::HOLO,
        );
        if self.section == Section::Group(Group::Quick) {
            y += 18.0;
            for line in wrap(
                "Drop another client's .cfg file on the window to import its name, model, FOV and keys.",
                44,
            ) {
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{line}"),
                    frame.rect(DETAIL_X, y, DETAIL_WIDTH, 26.0),
                    16.0 * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                y += 26.0;
            }
        }
    }

    /// One fact of the detail column: `name` then `value`, at `y`, which moves
    /// on to the next line.
    fn fact(
        &mut self,
        frame: &Frame,
        y: &mut f32,
        name: &str,
        value: std::fmt::Arguments<'_>,
        colour: Color,
    ) {
        let s = frame.s;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{name}"),
            frame.rect(DETAIL_X, *y, 110.0, 28.0),
            17.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            value,
            frame.rect(DETAIL_X + 110.0, *y, DETAIL_WIDTH - 110.0, 28.0),
            17.0 * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        *y += 32.0;
    }

    /// The keys of what has the keyboard, right-aligned at the bottom.
    pub(super) fn sjk_keys(&mut self, frame: &Frame) {
        let s = frame.s;
        let mut keys: Vec<(&[&str], &str)> = Vec::with_capacity(5);
        if self.numeric.is_some() || self.editing.is_some() {
            keys.extend([(&["Enter"][..], "set"), (&["Esc"][..], "cancel")]);
        } else if self.dropdown.is_some() {
            keys.extend([
                (&["Up", "Down"][..], "choose"),
                (&["Enter"][..], "apply"),
                (&["Esc"][..], "keep"),
            ]);
        } else if self.searching {
            keys.extend([(&["Enter"][..], "results"), (&["Esc"][..], "clear")]);
        } else {
            if let Some(setting) = self
                .rows()
                .get(self.selected)
                .filter(|_| self.shows(self.selected))
            {
                keys.push(match setting.kind {
                    ValueKind::Bool => (&["Enter"][..], "switch"),
                    ValueKind::Choice(_) | ValueKind::DisplayMode
                        if segment_choices(setting.kind).is_none() =>
                    {
                        (&["Enter"][..], "choices")
                    }
                    ValueKind::Resolution => (&["Enter"][..], "sizes"),
                    ValueKind::HudPicker => (&["Enter"][..], "pictures"),
                    ValueKind::WheelPages => (&["Enter"][..], "edit"),
                    ValueKind::Text => (&["Enter"][..], "type"),
                    _ => (&["Left", "Right"][..], "change"),
                });
            }
            if self
                .defaults
                .get(self.selected)
                .is_some_and(|default| default.changed)
            {
                keys.push((&["Backspace"][..], "default"));
            }
            if self.popup() {
                keys.push((&["Tab"][..], "all settings"));
                keys.push((&["Esc"][..], "done"));
            } else if self.section == Section::Search {
                keys.push((&["Esc"][..], "clear the search"));
            } else {
                keys.push((&["Tab"][..], "next group"));
            }
        }
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * keys.len().saturating_sub(1) as f32;
        let [right, y] = frame.point(1824.0, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

/// The Settings screen's top bar, the key bindings' too: the way back to
/// `back` (the back token 900), the screen's name and the search pill showing
/// `search` (with its cursor while `searching`), `prompt` when empty and how
/// many were `found` while a search is typed.
pub(crate) fn top_bar(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    back: &str,
    search: &str,
    searching: bool,
    prompt: &str,
    found: Option<usize>,
) {
    crate::menu::sjk::top_bar(
        canvas,
        frame,
        back,
        BACK_TOKEN,
        "Settings",
        Some(SearchPill {
            query: search,
            active: searching,
            prompt,
            found,
            token: SEARCH_TOKEN,
        }),
    );
}

/// A value as the SJK UI shows it: AUTO (the frame cap's special value) in
/// sentence case like every other.
fn shown_value(value: &str) -> std::borrow::Cow<'_, str> {
    if value == "AUTO" {
        "Auto".into()
    } else {
        value.into()
    }
}

/// A list of choices, in sentence case, separated by commas.
struct Choices(&'static [&'static str]);

impl std::fmt::Display for Choices {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, choice) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            write!(
                formatter,
                "{}",
                crate::menu::classic::view::Sentence(choice_label(choice))
            )?;
        }
        Ok(())
    }
}

/// Darkness over the map, deepest behind the rail and rows, so every line
/// reads over any part of it.
pub(crate) fn backdrop(canvas: &mut crate::menu_widgets::MenuCanvas, viewport: [f32; 2]) {
    let [width, height] = viewport;
    let space = |alpha| color::alpha(color::SPACE, alpha);
    let split = width * 0.6;
    fade_across(
        canvas,
        sjk_ui::Rect::new(0.0, 0.0, split, height),
        space(0.95),
        space(0.9),
    );
    fade_across(
        canvas,
        sjk_ui::Rect::new(split, 0.0, width - split, height),
        space(0.9),
        space(0.8),
    );
}

/// The categories down the rail, the one on show lit; category `i` answers to
/// the panel's chrome token `CHROME_BASE + i`.
pub(crate) fn draw_rail(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    rail: &Rail<'_>,
) {
    let s = frame.s;
    let count = rail.categories.len() as f32;
    let bottom = RAIL_TOP + count * CATEGORY_STEP - (CATEGORY_STEP - CATEGORY_HEIGHT) + 20.0;
    let lit = rail
        .current
        .map(|index| RAIL_TOP + index as f32 * CATEGORY_STEP + CATEGORY_HEIGHT * 0.5);
    kit::rail(canvas, frame, RAIL_X, RAIL_TOP - 20.0, bottom, lit);
    for (index, (label, icon)) in rail.categories.iter().enumerate() {
        let top = RAIL_TOP + index as f32 * CATEGORY_STEP;
        let token = CHROME_BASE + index as u16;
        let current = rail.current == Some(index);
        let hovered = canvas.token_hovered(token);
        if let Some(texture) = crate::settings_icons::texture(icon) {
            let _ = canvas.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(
                    RAIL_X + 34.0,
                    top + (CATEGORY_HEIGHT - 38.0) * 0.5,
                    38.0,
                    38.0,
                ),
                texture,
                color: Color::new(1.0, 1.0, 1.0, if current || hovered { 1.0 } else { 0.75 }),
            });
        }
        text(
            canvas,
            TextFamily::Display,
            format_args!("{label}"),
            frame.rect(RAIL_X + 88.0, top + 8.0, CATEGORY_WIDTH - 88.0, 36.0),
            26.0 * s,
            match (current, hovered) {
                (true, _) => color::GOLD_BRIGHT,
                (false, true) => color::TEXT,
                (false, false) => color::MUTED,
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        canvas.hit_region(
            token,
            frame.rect(RAIL_X, top, CATEGORY_WIDTH, CATEGORY_HEIGHT),
        );
    }
}

// Names, controls and the reset arrow in the rows' column, in order.
const _: () = assert!(LABEL_X < CONTROL_LEFT && TRACK_X > CONTROL_LEFT);
const _: () = assert!(CONTROL_RIGHT < RESET_X - 16.0 && RESET_X + 16.0 <= ROWS_X + ROWS_WIDTH);
// The rail, the rows' column with its scrollbar and the detail column side by
// side, inside the frame's margins.
const _: () = assert!(RAIL_X + CATEGORY_WIDTH < ROWS_X);
const _: () = assert!(ROWS_X + ROWS_WIDTH + 18.0 < DETAIL_X);
const _: () = assert!(DETAIL_X + DETAIL_WIDTH <= 1824.0);
// Every line shown ends above the keys; the search clears the rows.
const _: () = assert!(ROWS_TOP + VISIBLE as f32 * LINE < KEYS_Y - 20.0);
const _: () = assert!(SEARCH[1] + SEARCH[3] < ROWS_TOP);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_read_in_sentence_case() {
        assert_eq!(shown_value("AUTO"), "Auto");
        assert_eq!(shown_value("1920x1080"), "1920x1080");
        assert_eq!(
            Choices(&["off", "sjk", "strong"]).to_string(),
            "Off, SJK, Strong"
        );
    }
}
