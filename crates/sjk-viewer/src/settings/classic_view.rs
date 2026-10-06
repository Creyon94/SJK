//! The settings rows as a classic option panel (`setup.menu` items): the
//! label set against the label column, the value after it; toggles read
//! Yes or No, numbers are retail sliders with their value beside them, and
//! the focused item sits on the `menu_blendbox` highlight.
//!
//! The panel is classic+ (`docs/classic-plus.md`): the detail box under the
//! rows says what the focused setting does ([`super::help`]), its value,
//! default and range, when a change applies and its console name. A row
//! changed from its default carries a mark, a setting that applies after a
//! restart or on the next map a `*`; Backspace or the right button returns
//! the focused setting to its default, and the description line names the
//! keys of the focused row. The renderer settings show this way too, as the
//! groups of the classic renderer page.

use super::help::{self, Timing};
use super::*;
use crate::menu::classic::layout::Span;
use crate::menu::classic::panel::{Detail, OPTION, PanelFrame, focus_text};
use std::fmt::Write as _;
use winit::keyboard::KeyCode;

/// The rows' wheel target over the panel and the scrollbar beside it, clear
/// of the row (0-499), tab (500), slider value (700), chrome (800) and back
/// (900) tokens.
pub(super) const CLASSIC_SCROLL_TOKEN: u16 = 911;
pub(super) const CLASSIC_SCROLLBAR_TOKEN: u16 = 912;
/// Choice `i` of an open dropdown answers to `DROPDOWN_BASE + i`.
pub(super) const DROPDOWN_BASE: u16 = 1_000;
/// Rows one wheel notch scrolls, as the settings form steps.
pub(super) const CLASSIC_WHEEL_ROWS: i32 = 1;

/// What the classic+ panel knows about a row beyond its value: its default
/// as the row would show it, and whether the value differs from it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RowDefault {
    pub(super) text: Option<String>,
    pub(super) changed: bool,
}

impl SettingsMenu {
    /// Show rows `span` of tab `tab` as a classic option panel drawn in
    /// `frame`'s geometry.
    pub(crate) fn open_classic(
        &mut self,
        console: &ViewerConsole,
        tab: usize,
        span: Span,
        frame: crate::menu::classic::panel::Frame,
    ) {
        self.open_tab(console, tab);
        let rows = span.within(0, self.rows().len());
        self.selected = rows.start;
        self.clear_search();
        self.classic = Some(ClassicRows {
            lines: ClassicRows::span(rows),
            slider_span: frame.slider_span(),
            first: 0,
            visible: frame.capacity(),
        });
    }

    /// Show every row of renderer tab `tab` (IMAGE, LIGHTING, SHADOWS) as a
    /// classic option panel; backing out leaves the screen, to the classic
    /// renderer page's owner.
    pub(crate) fn open_classic_renderer(
        &mut self,
        console: &ViewerConsole,
        tab: usize,
        frame: crate::menu::classic::panel::Frame,
    ) {
        self.open_renderer(console);
        self.select_tab(console, tab);
        self.clear_search();
        self.classic = Some(ClassicRows {
            lines: ClassicRows::span(0..self.rows().len()),
            slider_span: frame.slider_span(),
            first: 0,
            visible: frame.capacity(),
        });
    }

    /// Show classic Setup group `group`'s rows as a classic option panel.
    pub(crate) fn open_classic_group(
        &mut self,
        console: &ViewerConsole,
        group: Group,
        frame: crate::menu::classic::panel::Frame,
    ) {
        self.open_tab(console, 0);
        self.section = Section::Group(group);
        self.tab = 0;
        self.refresh(console);
        self.selected = 0;
        self.clear_search();
        self.classic = Some(ClassicRows {
            lines: ClassicRows::span(0..self.rows().len()),
            slider_span: frame.slider_span(),
            first: 0,
            visible: frame.capacity(),
        });
    }

    /// Carry on as the modern screen after the menu style changed under a
    /// classic panel: a classic group's row is shown on the modern tab that
    /// holds it.
    pub(crate) fn continue_modern(&mut self, console: &ViewerConsole) {
        if !matches!(self.section, Section::Group(_) | Section::Search) {
            return;
        }
        let cvar = self.rows().get(self.selected).map(|setting| setting.cvar);
        let found = (0..TABS.len()).find_map(|tab| {
            let row = settings(tab)
                .iter()
                .position(|setting| Some(setting.cvar) == cvar)?;
            Some((tab, row))
        });
        let (tab, row) = found.unwrap_or((0, 0));
        self.section = Section::General;
        self.tab = tab;
        self.selected = row;
        self.refresh(console);
    }

    /// Back to the full tabbed screen, keeping the tab.
    pub(crate) fn leave_classic(&mut self) {
        if self.classic.take().is_some() {
            self.editing = None;
            self.numeric = None;
            self.dropdown = None;
            self.searching = false;
        }
    }

    /// Search for `text` as if it had been typed, for the menu snapshots.
    #[cfg(test)]
    pub(crate) fn search_for_snapshot(&mut self, console: &ViewerConsole, text: &str) {
        self.set_search(console, text.to_owned());
    }

    /// Open the selected row's dropdown, for the menu snapshots.
    #[cfg(test)]
    pub(crate) fn dropdown_for_snapshot(&mut self, console: &ViewerConsole) {
        let row = self.selected;
        assert!(self.open_dropdown(console, row), "no dropdown on row {row}");
        if let Some(dropdown) = &mut self.dropdown {
            dropdown.highlighted = (dropdown.current + 1) % dropdown.picks.len();
        }
    }

    /// Forget the search: a group opened from the list shows its own rows.
    fn clear_search(&mut self) {
        self.search.clear();
        self.searching = false;
        self.search_return = None;
        self.dropdown = None;
    }

    /// A key while the search field has the keyboard: typing searches every
    /// option; Enter, Down or Tab go to the results; Escape clears the text,
    /// then leaves the field.
    pub(super) fn search_key(&mut self, key: KeyCode, text: Option<&str>, console: &ViewerConsole) {
        let mut search = self.search.clone();
        match key {
            KeyCode::Escape if search.is_empty() => {
                self.searching = false;
                return;
            }
            KeyCode::Escape => search.clear(),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::ArrowDown | KeyCode::Tab => {
                self.searching = false;
                return;
            }
            KeyCode::Backspace => {
                search.pop();
            }
            _ => match text {
                Some(text) => search.extend(text.chars().filter(|c| !c.is_control()).take(32)),
                None => return,
            },
        }
        self.set_search(console, search);
    }

    /// Search every option for `search`: the panel shows the results under
    /// their groups' headings; an empty search returns to the group the
    /// panel showed before.
    pub(super) fn set_search(&mut self, console: &ViewerConsole, search: String) {
        if search == self.search && !search.is_empty() {
            return;
        }
        self.search = search;
        self.dropdown = None;
        self.editing = None;
        self.numeric = None;
        let Some(classic) = &mut self.classic else {
            return;
        };
        if self.search.trim().is_empty() {
            if let Some(back) = self.search_return.take() {
                self.section = back.section;
                self.tab = back.tab;
                classic.lines = back.lines;
                classic.first = back.first;
                self.selected = back.selected;
                self.refresh(console);
            }
            return;
        }
        if self.search_return.is_none() {
            self.search_return = Some(SearchReturn {
                section: self.section,
                tab: self.tab,
                lines: std::mem::take(&mut classic.lines),
                selected: self.selected,
                first: classic.first,
            });
        }
        self.section = Section::Search;
        self.tab = 0;
        let Some(classic) = &mut self.classic else {
            return;
        };
        classic.lines = search::lines(&self.search);
        classic.first = 0;
        // The results from the top, the first one selected.
        self.selected = classic.rows().next().unwrap_or(0);
        self.refresh(console);
    }

    /// Open the dropdown of row `row` when it is a choice (a switch, a choice
    /// row or the display mode); false for other rows.
    pub(super) fn open_dropdown(&mut self, console: &ViewerConsole, row: usize) -> bool {
        let Some(setting) = self.rows().get(row).copied() else {
            return false;
        };
        let picks: Vec<Pick> = match setting.kind {
            ValueKind::Bool => vec![Pick::Switch(true), Pick::Switch(false)],
            ValueKind::Choice(values) => values.iter().map(|value| Pick::Value(value)).collect(),
            ValueKind::DisplayMode => {
                let exclusive = self.exclusive_available();
                DisplayMode::ALL
                    .into_iter()
                    .filter(|mode| exclusive || *mode != DisplayMode::Exclusive)
                    .map(Pick::Mode)
                    .collect()
            }
            _ => return false,
        };
        let current = match setting.kind {
            ValueKind::Bool => usize::from(!console.cvar(setting.cvar).is_some_and(switch_on)),
            ValueKind::Choice(values) => {
                let value = console.cvar(setting.cvar).map(CvarValue::as_text);
                values
                    .iter()
                    .position(|candidate| Some(*candidate) == value.as_deref())
                    .unwrap_or(0)
            }
            _ => {
                let mode = DisplayMode::requested(console).effective(self.exclusive_available());
                picks
                    .iter()
                    .position(|pick| *pick == Pick::Mode(mode))
                    .unwrap_or(0)
            }
        };
        self.selected = row;
        self.dropdown = Some(Dropdown {
            row,
            picks,
            highlighted: current,
            current,
        });
        true
    }

    /// A key while a dropdown is open: Up and Down choose, Enter or Space
    /// apply, Escape (or Backspace) closes it with nothing changed.
    pub(super) fn dropdown_key(&mut self, key: KeyCode, console: &mut ViewerConsole) {
        let Some(dropdown) = &mut self.dropdown else {
            return;
        };
        let count = dropdown.picks.len();
        match key {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                dropdown.highlighted = (dropdown.highlighted + count - 1) % count;
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                dropdown.highlighted = (dropdown.highlighted + 1) % count;
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                let pick = dropdown.highlighted;
                self.apply_pick(console, pick);
            }
            KeyCode::Escape | KeyCode::Backspace => self.dropdown = None,
            _ => {}
        }
    }

    /// Apply choice `index` of the open dropdown and close it.
    pub(super) fn apply_pick(&mut self, console: &mut ViewerConsole, index: usize) {
        let Some(dropdown) = self.dropdown.take() else {
            return;
        };
        let (Some(pick), Some(setting)) = (
            dropdown.picks.get(index).copied(),
            self.rows().get(dropdown.row).copied(),
        ) else {
            return;
        };
        match pick {
            Pick::Switch(on) => {
                let value = match console.cvar(setting.cvar) {
                    Some(CvarValue::Bool(_)) => on.to_string(),
                    _ => u8::from(on).to_string(),
                };
                console.set_cvar(setting.cvar, &value);
            }
            Pick::Value(value) => {
                console.set_cvar(setting.cvar, value);
            }
            Pick::Mode(mode) => mode.store(console),
        }
        self.refresh(console);
    }

    /// The label of dropdown choice `pick` as the row would show it.
    fn pick_label(pick: Pick) -> &'static str {
        match pick {
            Pick::Switch(true) => "Yes",
            Pick::Switch(false) => "No",
            Pick::Value(value) => value,
            Pick::Mode(mode) => mode.label(),
        }
    }

    /// Return row `row` to its default value. Rows whose value is not one
    /// cvar's (resolution, display mode) are left alone.
    pub(super) fn reset_to_default(&mut self, console: &mut ViewerConsole, row: usize) {
        let Some(setting) = self.rows().get(row) else {
            return;
        };
        if matches!(setting.kind, ValueKind::Resolution | ValueKind::DisplayMode) {
            return;
        }
        if let Some(default) = console.cvar_default(setting.cvar).map(CvarValue::as_text) {
            console.set_cvar(setting.cvar, &default);
        }
        self.editing = None;
        self.numeric = None;
        self.refresh(console);
    }

    /// Draw the classic panel screen around the rows of the open span.
    pub(crate) fn append_classic(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
        reveal: f32,
        frame: &PanelFrame,
    ) {
        if self.hud.is_open() {
            self.append_hud_picker(vertices, font, viewport, reveal, Some(frame.art));
            return;
        }
        let place = frame.begin(&mut self.ui, viewport, reveal);
        let found = (!self.search.is_empty()).then(|| {
            self.classic
                .as_ref()
                .map_or(0, |classic| classic.rows().count())
        });
        place.search_field(
            &mut self.ui,
            &self.search,
            self.searching,
            "type to find",
            found,
        );
        let visible = place.capacity();
        let lines: Vec<Line> = match &mut self.classic {
            Some(classic) => {
                classic.visible = visible;
                classic.first = classic.first.min(classic.max_first());
                classic.lines.clone()
            }
            None => Vec::new(),
        };
        let first = self.classic.as_ref().map_or(0, |classic| classic.first);
        let shown_lines = first..lines.len().min(first + visible);
        // A list longer than the panel scrolls with the wheel anywhere over
        // its rows and shows a scrollbar, as the classic key bindings do.
        if lines.len() > visible {
            let top = place.row(0);
            self.ui.scroll_region(
                CLASSIC_SCROLL_TOKEN,
                Rect::new(top.x, top.y, top.width, visible as f32 * top.height),
            );
        }
        if lines.is_empty() && found.is_some() {
            place.value_plain(&mut self.ui, 0, "No option matches the search.", OPTION);
        }
        if let Some(slot) = lines[shown_lines.clone()]
            .iter()
            .position(|line| *line == Line::Row(self.selected))
        {
            place.highlight(&mut self.ui, slot);
        }
        // Slot of each shown row, for the thumbs and the dropdown.
        let mut slots: Vec<(usize, usize)> = Vec::with_capacity(visible);
        // Rows under an open dropdown leave their values out (text draws over
        // every shape, so the list could not hide them).
        let dropdown_slot = self.dropdown.as_ref().and_then(|dropdown| {
            let slot = lines[shown_lines.clone()]
                .iter()
                .position(|line| *line == Line::Row(dropdown.row))?;
            Some((slot, dropdown.picks.len()))
        });
        let covered = |slot: usize| {
            dropdown_slot.is_some_and(|(open, count)| place.dropdown_covers(open, count, slot))
        };
        for (slot, line) in lines[shown_lines.clone()].iter().enumerate() {
            let row = match *line {
                Line::Heading(text) => {
                    place.heading(&mut self.ui, slot, text);
                    continue;
                }
                Line::Row(row) => row,
            };
            slots.push((row, slot));
            let Some(setting) = self.rows().get(row) else {
                continue;
            };
            let focused = row == self.selected;
            let color = if focused { focus_text() } else { OPTION };
            let (label, timing) = help::classic_label(setting.cvar, setting.label);
            place.label_marked(&mut self.ui, slot, label, color, timing != Timing::Now);
            if self
                .defaults
                .get(row)
                .is_some_and(|default| default.changed)
            {
                place.changed_mark(&mut self.ui, slot);
            }
            self.ui.hit_region(row as u16, place.row(slot));
            if covered(slot) {
                continue;
            }
            let value = self.values.get(row).map_or("?", String::as_str);
            match setting.kind {
                ValueKind::Bool => {
                    place.value(&mut self.ui, slot, yes_no(value), color);
                }
                ValueKind::Integer { .. } | ValueKind::Float { .. } => {
                    place.draw_slider_bar(&mut self.ui, slot, color);
                    let editing = self.numeric.as_ref().filter(|edit| edit.row == row);
                    let target = place.slider_value_rect(slot);
                    self.ui.hit_region(
                        crate::menu_widgets::numeric::VALUE_BASE + row as u16,
                        target,
                    );
                    match editing {
                        Some(edit) => edit.draw(&mut self.ui, target, place.scale()),
                        None => place.slider_value(&mut self.ui, slot, value, color),
                    }
                }
                ValueKind::Choice(_)
                | ValueKind::Resolution
                | ValueKind::DisplayMode
                | ValueKind::HudPicker => place.value(&mut self.ui, slot, value, color),
                ValueKind::Text => match self
                    .editing
                    .as_ref()
                    .filter(|draft| draft.row == row)
                    .map(|draft| draft.text.as_str())
                {
                    Some(buffer) => {
                        place.value_fmt(&mut self.ui, slot, format_args!("{buffer}_"), color)
                    }
                    None => place.value_plain(&mut self.ui, slot, value, color),
                },
            }
        }
        // Thumbs after every bar, so the slider art binds once each.
        for &(row, slot) in &slots {
            let Some(setting) = self.rows().get(row) else {
                continue;
            };
            let value = self
                .values
                .get(row)
                .and_then(|value| value.parse::<f64>().ok());
            let ratio = match (setting.kind, value) {
                (ValueKind::Integer { min, max, .. }, Some(value)) if max > min => {
                    ((value - min as f64) / (max - min) as f64) as f32
                }
                (ValueKind::Float { min, max, .. }, Some(value)) if max > min => {
                    ((value - min) / (max - min)) as f32
                }
                (ValueKind::Integer { .. } | ValueKind::Float { .. }, _) => 0.0,
                _ => continue,
            };
            place.draw_slider_thumb(&mut self.ui, slot, ratio);
        }
        if lines.len() > visible {
            self.ui.scrollbar(
                CLASSIC_SCROLLBAR_TOKEN,
                place.scrollbar_track(visible),
                first,
                visible,
                lines.len(),
            );
        }
        self.write_detail_facts();
        let detail = detail_of(
            self.rows(),
            self.shows(self.selected).then_some(self.selected),
            &self.values,
            &self.detail_facts,
        );
        place.detail(&mut self.ui, &detail);
        // The dropdown last, over the rows and the detail box, so it takes the
        // pointer.
        if let Some(dropdown) = &self.dropdown {
            match slots.iter().find(|(row, _)| *row == dropdown.row) {
                Some(&(_, slot)) => {
                    let labels: Vec<&str> = dropdown
                        .picks
                        .iter()
                        .map(|pick| Self::pick_label(*pick))
                        .collect();
                    place.dropdown(
                        &mut self.ui,
                        slot,
                        &labels,
                        dropdown.highlighted,
                        dropdown.current,
                        DROPDOWN_BASE,
                    );
                }
                None => self.dropdown = None,
            }
        }
        self.write_key_hint();
        place.finish(&mut self.ui, Some(&self.key_hint));
        self.ui.finish(self.selected as u16);
        self.ui.append_text(vertices, font, viewport);
    }

    /// What the detail box says about the selected row.
    #[cfg(test)]
    fn detail(&self) -> Detail<'_> {
        detail_of(
            self.rows(),
            self.shows(self.selected).then_some(self.selected),
            &self.values,
            &self.detail_facts,
        )
    }

    /// The detail box's facts line for the selected row: its default, its
    /// range or choices, and when a change applies.
    fn write_detail_facts(&mut self) {
        let facts = &mut self.detail_facts;
        facts.clear();
        let Some(setting) = section_settings(self.section, self.tab).get(self.selected) else {
            return;
        };
        let part = |facts: &mut String, args: std::fmt::Arguments<'_>| {
            if !facts.is_empty() {
                facts.push_str("   \u{b7}   ");
            }
            let _ = facts.write_fmt(args);
        };
        if let Some(default) = self
            .defaults
            .get(self.selected)
            .and_then(|default| default.text.as_deref())
        {
            part(facts, format_args!("Default {default}"));
        }
        match setting.kind {
            ValueKind::Integer { min, max, .. } if min < 0 => {
                part(facts, format_args!("AUTO, 0 to {max}"));
            }
            ValueKind::Integer { min, max, .. } => part(facts, format_args!("{min} to {max}")),
            ValueKind::Float { min, max, .. } => {
                part(facts, format_args!("{} to {}", Number(min), Number(max)));
            }
            ValueKind::Choice(choices) => {
                part(facts, format_args!("{} choices", choices.len()));
            }
            _ => {}
        }
        if let Some(note) = help::timing(setting.label).1.note() {
            part(facts, format_args!("{note}"));
        }
        if self.section == Section::Search
            && let Some(group) = search::group(self.selected)
        {
            part(facts, format_args!("In {group}"));
        }
    }

    /// The description line's keys for the selected row.
    fn write_key_hint(&mut self) {
        self.key_hint.clear();
        let hint = &mut self.key_hint;
        if self.numeric.is_some() {
            hint.push_str("Type a value, then ENTER to set it or ESC to cancel.");
            return;
        }
        if self.editing.is_some() {
            hint.push_str("Type the new text, then ENTER to set it or ESC to cancel.");
            return;
        }
        if self.dropdown.is_some() {
            hint.push_str("UP or DOWN to choose, ENTER to apply, ESC to keep it as it is");
            return;
        }
        if self.searching {
            hint.push_str("Type to find any option   \u{b7}   ENTER to the results, ESC to clear");
            return;
        }
        let Some(setting) = section_settings(self.section, self.tab).get(self.selected) else {
            return;
        };
        hint.push_str(match setting.kind {
            ValueKind::Bool | ValueKind::Choice(_) | ValueKind::DisplayMode => {
                "ENTER or a click for the choices, LEFT or RIGHT to step"
            }
            ValueKind::Integer { .. } | ValueKind::Float { .. } => {
                "LEFT or RIGHT to change it, or type a number"
            }
            ValueKind::Text => "ENTER to type a new value",
            ValueKind::Resolution => "LEFT or RIGHT to step, ENTER for the list of sizes",
            ValueKind::HudPicker => "LEFT or RIGHT to step, ENTER to pick from pictures",
        });
        if self
            .defaults
            .get(self.selected)
            .is_some_and(|default| default.changed)
        {
            hint.push_str("   \u{b7}   BACKSPACE for the default");
        }
    }
}

/// The detail box for row `row` of `rows` (none when no row of the panel is
/// selected), with the rows' `values` and the `facts` line.
fn detail_of<'a>(
    rows: &'static [Setting],
    row: Option<usize>,
    values: &'a [String],
    facts: &'a str,
) -> Detail<'a> {
    let Some((row, setting)) = row.and_then(|row| Some((row, rows.get(row)?))) else {
        return Detail::default();
    };
    let value = values.get(row).map_or("", String::as_str);
    Detail {
        title: help::timing(setting.label).0,
        value: match setting.kind {
            ValueKind::Bool => yes_no(value),
            _ => value,
        },
        lines: help::lines(help::help(setting.cvar).unwrap_or_default()),
        facts,
        name: setting.cvar,
        icon: None,
    }
}

/// A toggle's value as a classic row shows it.
fn yes_no(value: &str) -> &'static str {
    if value.eq_ignore_ascii_case("on") {
        "Yes"
    } else {
        "No"
    }
}

/// A range end without trailing zeros: `0.5`, `70`, `0.005`.
struct Number(f64);

impl std::fmt::Display for Number {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = format!("{:.3}", self.0);
        let text = text.trim_end_matches('0').trim_end_matches('.');
        formatter.write_str(text)
    }
}

/// What row `setting` shows for its default, and whether `console`'s value
/// differs from it. Rows whose value is not one cvar's have neither.
pub(super) fn row_default(console: &ViewerConsole, setting: &Setting) -> RowDefault {
    // The HUD row names a HUD that two cvars select together.
    if matches!(
        setting.kind,
        ValueKind::Resolution | ValueKind::DisplayMode | ValueKind::HudPicker
    ) {
        return RowDefault::default();
    }
    let (Some(default), Some(value)) = (
        console.cvar_default(setting.cvar),
        console.cvar(setting.cvar),
    ) else {
        return RowDefault::default();
    };
    let text = match (setting.kind, default) {
        (ValueKind::Bool, value) => if switch_on(value) { "Yes" } else { "No" }.to_owned(),
        (ValueKind::Integer { min, .. }, CvarValue::Integer(value)) if min < 0 && *value < 0 => {
            "AUTO".to_owned()
        }
        (ValueKind::Float { .. }, CvarValue::Float(value)) => Number(*value).to_string(),
        (_, value) => value.as_text(),
    };
    let changed = match (setting.kind, default, value) {
        (ValueKind::Bool, default, value) => switch_on(default) != switch_on(value),
        (_, CvarValue::Float(default), CvarValue::Float(value)) => (default - value).abs() > 1e-6,
        (_, default, value) => !default
            .as_text()
            .trim()
            .eq_ignore_ascii_case(value.as_text().trim()),
    };
    RowDefault {
        text: Some(text),
        changed,
    }
}

#[cfg(test)]
mod tests {
    use super::super::ClassicRows;
    use super::*;

    fn classic(rows: std::ops::Range<usize>, visible: usize) -> ClassicRows {
        ClassicRows {
            lines: ClassicRows::span(rows),
            slider_span: (0.5, 0.25),
            first: 0,
            visible,
        }
    }

    #[test]
    fn keyboard_selection_stays_in_view() {
        let mut panel = classic(10..40, 15);
        panel.reveal(30);
        assert_eq!(panel.first, 6, "row 30 is the last of lines 6..21");
        panel.reveal(12);
        assert_eq!(panel.first, 2);
        panel.reveal(14);
        assert_eq!(panel.first, 2, "already shown");
        // A row the panel does not show (the key-bindings row) leaves it alone.
        panel.reveal(3);
        assert_eq!(panel.first, 2);
    }

    #[test]
    fn wheel_and_bar_stop_at_the_ends() {
        let mut panel = classic(0..20, 15);
        assert_eq!(panel.max_first(), 5);
        panel.scroll_by(-3);
        assert_eq!(panel.first, 0);
        panel.scroll_by(9);
        assert_eq!(panel.first, 5);
        panel.scroll_to_ratio(0.4);
        assert_eq!(panel.first, 2);
        panel.scroll_to_ratio(7.0);
        assert_eq!(panel.first, 5);
        // A span that fits never scrolls.
        let mut short = classic(0..9, 15);
        short.scroll_by(4);
        assert_eq!(short.first, 0);
    }

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    fn row_of(menu: &SettingsMenu, cvar: &str) -> usize {
        menu.rows()
            .iter()
            .position(|setting| setting.cvar == cvar)
            .unwrap()
    }

    #[test]
    fn the_detail_box_describes_the_focused_setting() {
        let (_directory, mut console) = console();
        let mut menu = SettingsMenu::new();
        menu.open_classic_renderer(&console, 0, crate::menu::classic::panel::Frame::Main);
        assert_eq!(menu.rows().len(), RENDER_IMAGE.len());
        assert!(menu.classic.is_some() && menu.renderer_open());
        menu.selected = row_of(&menu, "r_sceneHdr");
        menu.write_detail_facts();
        let detail = menu.detail();
        assert_eq!(detail.title, "HDR scene");
        assert_eq!(detail.name, "r_sceneHdr");
        assert!(detail.lines[0].starts_with("Renders the scene"));
        assert!(
            menu.detail_facts.contains("applies after a restart"),
            "{}",
            menu.detail_facts
        );
        // A slider: its default and range; changed, then back to the default.
        menu.selected = row_of(&menu, "r_hdrExposure");
        menu.write_detail_facts();
        assert!(
            menu.detail_facts.contains("0.25 to 4"),
            "{}",
            menu.detail_facts
        );
        assert!(!menu.defaults[menu.selected].changed);
        menu.adjust(&mut console, 1);
        assert!(menu.defaults[menu.selected].changed);
        menu.write_key_hint();
        assert!(menu.key_hint.contains("BACKSPACE"), "{}", menu.key_hint);
        menu.reset_to_default(&mut console, menu.selected);
        assert!(!menu.defaults[menu.selected].changed);
        assert_eq!(
            console.cvar("r_hdrExposure"),
            console.cvar_default("r_hdrExposure")
        );
    }

    #[test]
    fn toggles_show_their_default_as_yes_or_no() {
        let (_directory, mut console) = console();
        let mut menu = SettingsMenu::new();
        menu.open_tab(&console, SettingsMenu::tab_index("VIDEO").unwrap());
        let row = row_of(&menu, "cg_marks");
        assert_eq!(menu.defaults[row].text.as_deref(), Some("Yes"));
        menu.selected = row;
        menu.adjust(&mut console, 1);
        assert!(menu.defaults[row].changed);
        // AUTO stands for the frame cap's special value.
        let cap = row_of(&menu, "com_maxfps");
        assert_eq!(menu.defaults[cap].text.as_deref(), Some("AUTO"));
        // Resolution and display mode are not one cvar's value.
        assert_eq!(menu.defaults[0], RowDefault::default());
    }

    #[test]
    fn range_ends_drop_trailing_zeros() {
        assert_eq!(Number(0.5).to_string(), "0.5");
        assert_eq!(Number(70.0).to_string(), "70");
        assert_eq!(Number(0.005).to_string(), "0.005");
        assert_eq!(Number(-2.0).to_string(), "-2");
    }
}
