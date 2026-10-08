//! Retained tabbed settings UI backed directly by archived shell cvars.

use crate::console::ViewerConsole;
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextVertex, UiFont};
use sjk_shell::CvarValue;
use sjk_ui::{DrawList, Rect};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

mod catalog;
mod classic_view;
mod display;
mod groups;
mod help;
mod hud_picker;
mod numeric;
mod pointer;
pub(crate) mod quick;
mod resolution;
mod resolution_list;
mod scroll;
mod search;
mod sjk_popup;
pub(crate) mod sjk_view;
mod view;

use catalog::*;
pub(crate) use catalog::{FIRST_SETUP_CAPTION, RESOLUTIONS};
pub(crate) use display::{
    DisplayMode, EXCLUSIVE_CVAR, MonitorModes, exclusive_supported, exclusive_video_mode,
};
pub(crate) use groups::Group;
use resolution::{PickResult, ResolutionChoice, ResolutionPicker};
pub(crate) use sjk_view::Rail;
pub(crate) enum SettingsResult {
    None,
    Back,
    OpenKeybinds,
    /// A button of the classic panel screen around the options: index into
    /// its page's slots.
    Classic(usize),
    /// Move the classic panel to the next (1) or previous (-1) group.
    ClassicCycle(i32),
    /// Leave First setup's pop-up for the whole Settings screen.
    AllSettings,
}

/// What First setup's pop-up does with `key` before its rows see it: Tab
/// leaves for the whole Settings screen; the search's and the groups' keys do
/// nothing, as it has neither.
fn popup_key(key: KeyCode) -> Option<SettingsResult> {
    match key {
        KeyCode::Tab => Some(SettingsResult::AllSettings),
        KeyCode::Slash | KeyCode::NumpadDivide | KeyCode::BracketLeft | KeyCode::BracketRight => {
            Some(SettingsResult::None)
        }
        _ => None,
    }
}

/// One line of a classic option panel: a heading (a group of search
/// results) or a setting row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Line {
    Heading(&'static str),
    Row(usize),
}

/// The lines a classic option panel shows: a span of the tab, or a search's
/// results under their groups; where its slider bars sit across a row, and
/// which part is scrolled into the panel when it has more lines than fit.
struct ClassicRows {
    lines: Vec<Line>,
    slider_span: (f32, f32),
    /// First shown line.
    first: usize,
    /// Lines the panel showed last frame.
    visible: usize,
    /// A row shown apart from the lines, after them in the keyboard's order:
    /// First setup's pop-up keeps "Don't show at start" at its foot, always
    /// in view. Only that pop-up pins one.
    pinned: Option<usize>,
}

impl ClassicRows {
    /// The lines of rows `rows`, without headings.
    fn span(rows: std::ops::Range<usize>) -> Vec<Line> {
        rows.map(Line::Row).collect()
    }

    /// Furthest the lines scroll.
    fn max_first(&self) -> usize {
        self.lines.len().saturating_sub(self.visible)
    }

    /// Line of setting row `row`, if the panel shows it.
    fn position(&self, row: usize) -> Option<usize> {
        self.lines.iter().position(|line| *line == Line::Row(row))
    }

    /// The setting rows, in order, the pinned one last.
    fn rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.lines
            .iter()
            .filter_map(|line| match line {
                Line::Row(row) => Some(*row),
                Line::Heading(_) => None,
            })
            .chain(self.pinned)
    }

    /// Whether the panel shows row `row`, in its lines or pinned.
    fn shows(&self, row: usize) -> bool {
        self.pinned == Some(row) || self.position(row).is_some()
    }

    /// The row `direction` rows away from `selected`, wrapping; the first
    /// row when `selected` is not shown.
    fn step(&self, selected: usize, direction: i32) -> Option<usize> {
        let rows: Vec<usize> = self.rows().collect();
        let first = *rows.first()?;
        let Some(index) = rows.iter().position(|row| *row == selected) else {
            return Some(first);
        };
        Some(rows[(index as i32 + direction).rem_euclid(rows.len() as i32) as usize])
    }

    /// Scroll by `rows` (negative = up) without moving the selection.
    fn scroll_by(&mut self, rows: i32) {
        self.first = (self.first as i32 + rows).clamp(0, self.max_first() as i32) as usize;
    }

    /// Scroll so the panel shows `ratio` (0 = top, 1 = bottom) of the span.
    fn scroll_to_ratio(&mut self, ratio: f32) {
        self.first = (ratio.clamp(0.0, 1.0) * self.max_first() as f32).round() as usize;
    }

    /// Scroll just enough to show row `selected`, and its heading when it
    /// is the first row under one.
    fn reveal(&mut self, selected: usize) {
        let Some(offset) = self.position(selected) else {
            return;
        };
        let visible = self.visible.max(1);
        let heading = offset
            .checked_sub(1)
            .filter(|above| matches!(self.lines[*above], Line::Heading(_)));
        for line in heading.into_iter().chain([offset]) {
            self.first = self.first.min(line).max((line + 1).saturating_sub(visible));
        }
    }
}

/// Where the panel was before a search, to return to when it is cleared.
struct SearchReturn {
    section: Section,
    tab: usize,
    lines: Vec<Line>,
    selected: usize,
    first: usize,
}

/// One choice of a classic dropdown.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Pick {
    /// One of a choice row's values.
    Value(&'static str),
    /// A display mode.
    Mode(DisplayMode),
}

/// A classic+ dropdown open on row `row`: its choices, the highlighted one
/// and the one in use. Nothing changes until a choice is applied.
struct Dropdown {
    row: usize,
    picks: Vec<Pick>,
    highlighted: usize,
    current: usize,
}

/// A Text setting being typed. Its row is fixed when typing starts, so Enter
/// writes the setting that was opened even if the pointer moved meanwhile.
struct TextDraft {
    row: usize,
    text: String,
}

/// Which set of tabs the form shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Section {
    /// The general settings ([`TABS`]).
    General,
    /// JKR's renderer settings ([`RENDERER_TABS`]).
    Renderer,
    /// A classic Setup group gathering rows of several tabs ([`Group`]).
    Group(Group),
    /// Every OPTIONS setting, for the classic panel's search ([`search`]).
    Search,
}

/// A row after a tab's settings that opens another screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Keybinds,
    Renderer,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::Keybinds => "Key bindings",
            Self::Renderer => "Renderer",
        }
    }
}

pub(crate) struct SettingsMenu {
    section: Section,
    /// The renderer section was opened on its own (the classic Setup page's
    /// RENDERER entry), so backing out leaves the screen instead of returning
    /// to the Video tab.
    renderer_direct: bool,
    tab: usize,
    selected: usize,
    /// Which rows show; keeps the selection on screen.
    scroll: scroll::RowScroll,
    values: Vec<String>,
    /// Each row's default and whether its value differs (classic+ panels).
    defaults: Vec<classic_view::RowDefault>,
    /// The classic+ detail box's facts line and the description line's keys,
    /// rewritten each frame.
    detail_facts: String,
    key_hint: String,
    editing: Option<TextDraft>,
    /// What the window's monitor offers; asked for each time the screen opens.
    monitor: Option<MonitorModes>,
    /// The screen opened and wants fresh [`MonitorModes`].
    wants_monitor: bool,
    /// Scratch list of the resolutions on offer.
    choices: Vec<ResolutionChoice>,
    /// The resolution list, open over the form.
    picker: ResolutionPicker,
    /// The HUD picker, open over the form.
    hud: hud_picker::HudPicker,
    numeric: Option<crate::menu_widgets::numeric::NumericEdit>,
    /// Set while the screen is a classic option panel.
    classic: Option<ClassicRows>,
    /// The classic panel's search: what is typed, whether the field has the
    /// keyboard, and where the panel was before the search.
    search: String,
    searching: bool,
    search_return: Option<SearchReturn>,
    /// Key bindings matching the search typed here, which the panel offers
    /// through the KEY BINDINGS tab (0 when none or no search).
    elsewhere: usize,
    /// The classic panel's dropdown, while one is open.
    dropdown: Option<Dropdown>,
    /// Where the rows' controls begin (window x) when the SJK UI drew the
    /// screen last frame: a click left of it, on a row's name, only chooses
    /// the row. `None` under the other views.
    sjk_controls: Option<f32>,
    ui: MenuCanvas,
}

impl SettingsMenu {
    /// The renderer section is the one on show (or the SJK UI's Graphics,
    /// which gathers its tabs).
    pub(crate) fn renderer_open(&self) -> bool {
        matches!(
            self.section,
            Section::Renderer | Section::Group(Group::Graphics)
        )
    }

    /// The resolution list or the HUD picker covers the screen; both draw
    /// themselves whatever the menu style.
    pub(crate) fn picker_open(&self) -> bool {
        self.picker.is_open() || self.hud.is_open()
    }

    /// The rows are a classic+ panel's (a group, a span of a tab, a search),
    /// which the classic+ and SJK UI views draw.
    pub(crate) fn has_panel_rows(&self) -> bool {
        self.classic.is_some()
    }

    /// The panel shows a search's results instead of its group.
    pub(crate) fn searching_results(&self) -> bool {
        self.section == Section::Search
    }

    pub(crate) fn new() -> Self {
        Self {
            section: Section::General,
            renderer_direct: false,
            tab: 0,
            selected: 0,
            scroll: scroll::RowScroll::new(),
            values: Vec::with_capacity(12),
            defaults: Vec::with_capacity(20),
            detail_facts: String::with_capacity(96),
            key_hint: String::with_capacity(96),
            editing: None,
            monitor: None,
            wants_monitor: false,
            choices: Vec::with_capacity(48),
            picker: ResolutionPicker::new(),
            hud: hud_picker::HudPicker::default(),
            numeric: None,
            classic: None,
            search: String::new(),
            searching: false,
            search_return: None,
            elsewhere: 0,
            dropdown: None,
            sjk_controls: None,
            ui: MenuCanvas::new(),
        }
    }

    /// Index of the first-start FIRST SETUP tab.
    pub(crate) fn quick_tab() -> usize {
        QUICK_TAB
    }

    /// Whether First setup's rows are on show: its classic group (the SJK UI's
    /// category or pop-up), or the modern FIRST SETUP tab.
    pub(crate) fn on_first_setup(&self) -> bool {
        match self.section {
            Section::Group(group) => group == Group::Quick,
            Section::General => self.tab == QUICK_TAB,
            _ => false,
        }
    }

    /// Index of the tab that carries the "Key bindings" row.
    pub(crate) fn keybinds_tab() -> usize {
        KEYBINDS_TAB
    }

    /// Index of the tab captioned `caption` (`"AUDIO"`), if there is one.
    pub(crate) fn tab_index(caption: &str) -> Option<usize> {
        TABS.iter().position(|tab| *tab == caption)
    }

    pub(crate) fn open(&mut self, console: &ViewerConsole) {
        self.open_tab(console, 0);
    }

    /// Open on tab `tab` (clamped to the catalogue).
    pub(crate) fn open_tab(&mut self, console: &ViewerConsole, tab: usize) {
        self.section = Section::General;
        self.renderer_direct = false;
        self.tab = tab.min(TABS.len() - 1);
        self.selected = 0;
        self.editing = None;
        self.picker.close();
        self.hud.mark_stale();
        self.wants_monitor = true;
        self.numeric = None;
        self.classic = None;
        self.refresh(console);
    }

    fn enter_renderer(&mut self, console: &ViewerConsole) {
        self.section = Section::Renderer;
        self.tab = 0;
        self.selected = 0;
        self.editing = None;
        self.numeric = None;
        self.classic = None;
        self.refresh(console);
    }

    /// Back out of the form: the renderer section returns to the Video tab's
    /// Renderer row, as JoF EJK's advanced renderer page returns to Video,
    /// unless it was opened on its own.
    fn back(&mut self, console: &ViewerConsole) -> SettingsResult {
        if self.section == Section::Renderer && !self.renderer_direct {
            self.section = Section::General;
            self.tab = RENDERER_TAB;
            self.selected = self.rows().len();
            self.editing = None;
            self.numeric = None;
            self.refresh(console);
            return SettingsResult::None;
        }
        SettingsResult::Back
    }

    /// Tab names of the current section.
    fn tabs(&self) -> &'static [&'static str] {
        match self.section {
            Section::General => &TABS,
            Section::Renderer => &RENDERER_TABS,
            Section::Group(group) => group.tabs(),
            Section::Search => &["SEARCH"],
        }
    }

    /// Settings of the current tab.
    fn rows(&self) -> &'static [Setting] {
        section_settings(self.section, self.tab)
    }

    /// The row after the settings that opens another screen, if this tab has
    /// one. Classic panels show spans of a tab without it.
    fn action(&self) -> Option<Action> {
        if self.classic.is_some() {
            return None;
        }
        match (self.section, self.tab) {
            (Section::General, KEYBINDS_TAB | QUICK_TAB) => Some(Action::Keybinds),
            (Section::General, RENDERER_TAB) => Some(Action::Renderer),
            _ => None,
        }
    }

    /// Selectable rows: the settings plus the action row.
    fn row_count(&self) -> usize {
        self.rows().len() + usize::from(self.action().is_some())
    }

    /// Activate the action row.
    fn activate_action(&mut self, console: &ViewerConsole) -> SettingsResult {
        match self.action() {
            Some(Action::Keybinds) => SettingsResult::OpenKeybinds,
            Some(Action::Renderer) => {
                self.enter_renderer(console);
                SettingsResult::None
            }
            None => SettingsResult::None,
        }
    }

    /// Switch to tab `tab` of the current section.
    fn select_tab(&mut self, console: &ViewerConsole, tab: usize) {
        self.tab = tab % self.tabs().len();
        self.selected = 0;
        self.editing = None;
        self.refresh(console);
    }

    /// Open the renderer settings on their own; backing out leaves the screen.
    pub(crate) fn open_renderer(&mut self, console: &ViewerConsole) {
        self.open_tab(console, RENDERER_TAB);
        self.enter_renderer(console);
        self.renderer_direct = true;
    }

    /// Whether the screen wants [`Self::set_monitor_modes`] (it just opened).
    pub(crate) fn wants_monitor_modes(&self) -> bool {
        self.wants_monitor
    }

    /// Take the window's monitor facts, which shape the resolution and
    /// display-mode choices.
    pub(crate) fn set_monitor_modes(&mut self, modes: MonitorModes, console: &ViewerConsole) {
        self.wants_monitor = false;
        self.monitor = Some(modes);
        if self.picker.is_open() {
            self.build_choices(console);
            self.picker.update_choices(&self.choices);
        }
        self.refresh(console);
    }

    /// Whether keyboard focus has rows to move through: the classic panel's
    /// lines, or the whole tab (with its key-bindings row).
    fn has_rows(&self) -> bool {
        match &self.classic {
            Some(classic) => classic.rows().next().is_some(),
            None => self.row_count() > 0,
        }
    }

    /// Whether row `row` is on show: in the classic panel's lines, or a row
    /// of the tab.
    fn shows(&self, row: usize) -> bool {
        match &self.classic {
            Some(classic) => classic.shows(row),
            None => row < self.row_count(),
        }
    }

    /// Move the selection `direction` rows: through the classic panel's rows
    /// (over its headings), or the tab's, wrapping.
    fn step_selection(&mut self, direction: i32) {
        if let Some(classic) = &self.classic {
            if let Some(row) = classic.step(self.selected, direction) {
                self.selected = row;
            }
        } else {
            let count = self.row_count();
            if count > 0 {
                self.selected =
                    (self.selected as i32 + direction).rem_euclid(count as i32) as usize;
            }
        }
        self.reveal_selected();
    }
    /// Keep the selected row inside a scrolled classic panel.
    fn reveal_selected(&mut self) {
        let selected = self.selected;
        if let Some(classic) = &mut self.classic {
            classic.reveal(selected);
        }
    }

    /// Select the row of setting `cvar`, scrolled into a panel (menu snapshots).
    #[cfg(test)]
    pub(crate) fn select_cvar(&mut self, cvar: &str) {
        if let Some(row) = self.rows().iter().position(|setting| setting.cvar == cvar) {
            self.selected = row;
            self.reveal_selected();
        }
    }

    pub(crate) fn visual_selection(&self) -> (usize, bool) {
        (self.selected, false)
    }
    pub(crate) fn draw_list(&self) -> &DrawList {
        self.ui.draw_list()
    }

    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        console: &mut ViewerConsole,
    ) -> SettingsResult {
        if event.state != ElementState::Pressed {
            return SettingsResult::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return SettingsResult::None;
        };
        if self.hud.is_open() {
            self.hud_picker_key(key, event.repeat, console);
            return SettingsResult::None;
        }
        if self.picker.is_open() {
            self.resolution_key(key, event.repeat, console);
            return SettingsResult::None;
        }
        if self.dropdown.is_some() {
            if !event.repeat || matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown) {
                self.dropdown_key(key, console);
            }
            return SettingsResult::None;
        }
        if self.searching {
            self.search_key(key, event.text.as_deref(), console);
            return SettingsResult::None;
        }
        if self.edit_numeric(key, event.text.as_deref(), event.repeat, console) {
            return SettingsResult::None;
        }
        if let Some(draft) = &mut self.editing {
            let buffer = &mut draft.text;
            match key {
                KeyCode::Escape => self.editing = None,
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    if let Some(draft) = self.editing.take()
                        && let Some(setting) = self.rows().get(draft.row)
                    {
                        console.set_cvar(setting.cvar, draft.text.trim());
                    }
                    self.refresh(console);
                }
                KeyCode::Backspace => {
                    buffer.pop();
                }
                _ if !event.repeat => {
                    if let Some(text) = event.text.as_deref() {
                        buffer.extend(
                            text.chars()
                                .filter(|c| !c.is_control())
                                .take(128usize.saturating_sub(buffer.len())),
                        );
                    }
                }
                _ => {}
            }
            return SettingsResult::None;
        }
        if event.repeat {
            return SettingsResult::None;
        }
        let classic = self.classic.is_some();
        // Up from the pop-up's first row wraps to its foot, as it has no search.
        if self.popup() {
            if let Some(result) = popup_key(key) {
                return result;
            }
        } else if classic {
            // Classic+: `/` or Up from the first row gives the search field
            // the keyboard; Escape clears a search before it leaves.
            let at_top = self.classic.as_ref().and_then(|c| c.rows().next()) == Some(self.selected);
            match key {
                KeyCode::Slash | KeyCode::NumpadDivide => {
                    self.searching = true;
                    return SettingsResult::None;
                }
                KeyCode::ArrowUp | KeyCode::KeyW if at_top => {
                    self.searching = true;
                    return SettingsResult::None;
                }
                KeyCode::Escape if !self.search.is_empty() => {
                    self.set_search(console, String::new());
                    return SettingsResult::None;
                }
                _ => {}
            }
        }
        if !self.has_rows() {
            return match key {
                KeyCode::Escape => self.back(console),
                _ => SettingsResult::None,
            };
        }
        // SJK: a digit (or '.', ',' and '-' where the slider allows them) typed
        // on a selected slider opens entry with it.
        if let Some(text) = event.text.as_deref()
            && text.starts_with(|c: char| c.is_ascii_digit() || ".,-".contains(c))
            && self.begin_typed(self.selected, text)
        {
            return SettingsResult::None;
        }
        match key {
            KeyCode::Tab | KeyCode::BracketRight if classic => {
                return SettingsResult::ClassicCycle(1);
            }
            KeyCode::BracketLeft if classic => return SettingsResult::ClassicCycle(-1),
            KeyCode::Tab | KeyCode::BracketRight => self.select_tab(console, self.tab + 1),
            KeyCode::BracketLeft => {
                let tab = self.tab.checked_sub(1).unwrap_or(self.tabs().len() - 1);
                self.select_tab(console, tab);
            }
            KeyCode::ArrowUp | KeyCode::KeyW => self.step_selection(-1),
            KeyCode::ArrowDown | KeyCode::KeyS => self.step_selection(1),
            KeyCode::ArrowLeft | KeyCode::KeyA => self.adjust(console, -1),
            KeyCode::ArrowRight | KeyCode::KeyD => self.adjust(console, 1),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space
                if self.action().is_some() && self.selected == self.rows().len() =>
            {
                return self.activate_action(console);
            }
            // SJK: Space keeps stepping a slider; Enter opens its entry.
            KeyCode::Space
                if self.rows().get(self.selected).is_some_and(|setting| {
                    matches!(
                        setting.kind,
                        ValueKind::Integer { .. } | ValueKind::Float { .. }
                    )
                }) =>
            {
                self.adjust(console, 1);
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(setting) = self.rows().get(self.selected) {
                    if matches!(setting.kind, ValueKind::Text) {
                        self.begin_text(console, self.selected);
                    } else if matches!(setting.kind, ValueKind::Resolution) {
                        self.open_resolutions(console);
                    } else if matches!(setting.kind, ValueKind::HudPicker) {
                        self.open_hud_picker(console);
                    } else if classic && self.open_dropdown(console, self.selected) {
                        // Classic+: a choice opens its list; nothing changes yet.
                    } else if !self.begin_numeric(console, self.selected) {
                        self.adjust(console, 1);
                    }
                }
            }
            KeyCode::Escape => return self.back(console),
            // Classic+: back to the default.
            KeyCode::Backspace | KeyCode::Delete if classic => {
                self.reset_to_default(console, self.selected);
            }
            _ => {}
        }
        SettingsResult::None
    }

    fn adjust(&mut self, console: &mut ViewerConsole, direction: i32) {
        let Some(setting) = self.rows().get(self.selected) else {
            return;
        };
        let next = match (setting.kind, console.cvar(setting.cvar)) {
            (ValueKind::Bool, Some(CvarValue::Bool(value))) => (!value).to_string(),
            (ValueKind::Bool, Some(value)) => if switch_on(value) { "0" } else { "1" }.to_owned(),
            (ValueKind::Integer { min, max, step }, Some(CvarValue::Integer(value))) => {
                step_integer(*value, direction, min, max, step).to_string()
            }
            (ValueKind::Float { .. }, Some(CvarValue::Float(value))) => {
                match setting.kind.stepped(*value, direction) {
                    Some(next) => next,
                    None => return,
                }
            }
            (ValueKind::Choice(values), Some(value)) => {
                // Choices name text or integer cvars (`cg_saberTrail` is an integer).
                let value = value.as_text();
                let index = values
                    .iter()
                    .position(|candidate| *candidate == value)
                    .unwrap_or(0);
                values[(index as i32 + direction).rem_euclid(values.len() as i32) as usize]
                    .to_owned()
            }
            (ValueKind::Resolution, _) => {
                self.step_resolution(console, direction);
                return;
            }
            (ValueKind::HudPicker, _) => {
                self.step_hud(console, direction);
                return;
            }
            (ValueKind::DisplayMode, _) => {
                DisplayMode::requested(console)
                    .step(direction, self.exclusive_available())
                    .store(console);
                self.refresh(console);
                return;
            }
            _ => return,
        };
        console.set_cvar(setting.cvar, &next);
        self.refresh(console);
    }

    /// Whether exclusive fullscreen can be offered; assumed until the
    /// monitor facts arrive, since the window falls back to borderless.
    fn exclusive_available(&self) -> bool {
        self.monitor
            .as_ref()
            .is_none_or(|monitor| monitor.exclusive)
    }

    /// Start typing Text row `row`, keeping an edit already open on it.
    fn begin_text(&mut self, console: &ViewerConsole, row: usize) {
        if self.editing.as_ref().is_some_and(|draft| draft.row == row) {
            return;
        }
        let Some(setting) = self.rows().get(row) else {
            return;
        };
        self.numeric = None;
        self.selected = row;
        self.editing = Some(TextDraft {
            row,
            text: value_text(console, setting.cvar),
        });
    }

    /// Whether a typed draft (text or number) is open. While one is, the pointer
    /// does not move the selection away from it.
    fn drafting(&self) -> bool {
        self.editing.is_some() || self.numeric.is_some()
    }

    /// A press on anything but the text draft's own row discards the draft, as
    /// clicking another control discards a numeric one.
    fn press_elsewhere(&mut self, row: Option<usize>) {
        if self
            .editing
            .as_ref()
            .is_some_and(|draft| Some(draft.row) != row)
        {
            self.editing = None;
        }
    }

    /// Hover selects `row` unless a draft is open.
    fn hover_row(&mut self, row: usize) {
        if !self.drafting() {
            self.selected = row;
        }
    }

    /// The wheel moves the selection by `direction` unless a draft is open: in
    /// a classic panel within its rows, otherwise through the scrolled tab.
    fn wheel(&mut self, direction: i32) {
        if direction == 0 || self.drafting() {
            return;
        }
        if let Some(classic) = &self.classic {
            let rows: Vec<usize> = classic.rows().collect();
            if let Some(index) = rows.iter().position(|row| *row == self.selected) {
                let next = (index as i32 + direction).clamp(0, rows.len() as i32 - 1);
                self.selected = rows[next as usize];
            } else if let Some(first) = rows.first() {
                self.selected = *first;
            }
        } else {
            let count = self.row_count();
            if count > 0 {
                self.selected = self.scroll.wheel(direction, count, self.selected);
            }
        }
    }

    fn refresh(&mut self, console: &ViewerConsole) {
        let display = DisplayMode::requested(console).effective(self.exclusive_available());
        self.hud.read(console);
        let hud = self.hud.label();
        self.values.clear();
        self.values
            .extend(self.rows().iter().map(|setting| match setting.kind {
                ValueKind::DisplayMode => display.label().to_owned(),
                ValueKind::HudPicker => hud.clone(),
                ValueKind::Bool => toggle_text(console, setting.cvar),
                _ => row_text(console, setting),
            }));
        self.defaults.clear();
        self.defaults.extend(
            self.rows()
                .iter()
                .map(|setting| classic_view::row_default(console, setting)),
        );
    }
}

/// Settings of `tab` in `section`.
fn section_settings(section: Section, tab: usize) -> &'static [Setting] {
    match section {
        Section::General => settings(tab),
        Section::Renderer => match tab {
            0 => RENDER_IMAGE,
            1 => RENDER_LIGHTING,
            2 => RENDER_SHADOWS,
            3 => RENDER_WEATHER,
            _ => &[],
        },
        Section::Group(group) => group.rows(),
        Section::Search => search::rows(),
    }
}

/// Settings of general tab `tab`.
fn settings(tab: usize) -> &'static [Setting] {
    match tab {
        0 => VIDEO,
        1 => AUDIO,
        2 => HUD,
        3 => CONTROLS,
        4 => GAME,
        5 => NETWORK,
        6 => HUD_OPTIONS,
        7 => TEXT,
        QUICK_TAB => quick::rows(),
        _ => &[],
    }
}
/// A negative minimum on an integer row is one special value below the range,
/// shown as AUTO (`com_maxfps -1`). Stepping moves between it and zero, then
/// along the row's step.
fn step_integer(value: i64, direction: i32, min: i64, max: i64, step: i64) -> i64 {
    if min < 0 {
        if value < 0 {
            return if direction > 0 { 0 } else { min };
        }
        if value == 0 && direction < 0 {
            return min;
        }
        return (value + i64::from(direction) * step).clamp(0, max);
    }
    (value + i64::from(direction) * step).clamp(min, max)
}

/// The value a row shows: AUTO for the one special value below a negative
/// minimum (`com_maxfps -1`), otherwise the cvar's text.
fn row_text(console: &ViewerConsole, setting: &Setting) -> String {
    match (setting.kind, console.cvar(setting.cvar)) {
        (ValueKind::Integer { min, .. }, Some(CvarValue::Integer(value)))
            if min < 0 && *value < 0 =>
        {
            "AUTO".to_owned()
        }
        (ValueKind::Float { .. }, Some(CvarValue::Float(value))) => float_text(*value),
        _ => value_text(console, setting.cvar),
    }
}

/// A slider's number to four decimals at most, keeping one (`0.9`, not
/// `0.8999999761581421` from a single-precision default; `100.0`).
fn float_text(value: f64) -> String {
    let mut text = format!("{value:.4}");
    while text.ends_with('0') && !text.ends_with(".0") {
        text.pop();
    }
    text
}

/// Whether a switch row's cvar is on: true, or any nonzero number.
fn switch_on(value: &CvarValue) -> bool {
    match value {
        CvarValue::Bool(value) => *value,
        CvarValue::Integer(value) => *value != 0,
        CvarValue::Float(value) => *value != 0.0,
        CvarValue::Text(value) => value.trim().parse::<f64>().is_ok_and(|value| value != 0.0),
    }
}

/// ON/OFF for a toggle row; a numeric cvar is on when nonzero.
fn toggle_text(console: &ViewerConsole, name: &str) -> String {
    match console.cvar(name) {
        Some(value @ (CvarValue::Integer(_) | CvarValue::Float(_))) => {
            if switch_on(value) { "ON" } else { "OFF" }.to_owned()
        }
        _ => value_text(console, name),
    }
}

fn value_text(console: &ViewerConsole, name: &str) -> String {
    console.cvar(name).map_or_else(
        || "?".to_owned(),
        |value| match value {
            CvarValue::Bool(v) => {
                if *v {
                    "ON".to_owned()
                } else {
                    "OFF".to_owned()
                }
            }
            _ => value.as_text(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_numbers_lose_float_noise() {
        assert_eq!(float_text(f64::from(0.9_f32)), "0.9");
        assert_eq!(float_text(100.0), "100.0");
        assert_eq!(float_text(0.005), "0.005");
        assert_eq!(float_text(-2.5), "-2.5");
    }

    /// First setup's Menu style row, picked away from the classic panel,
    /// carries on as the modern FIRST SETUP tab, not Interface, which also
    /// has the row.
    #[test]
    fn first_setup_carries_on_as_its_modern_tab() {
        let (_directory, console) = console();
        let mut menu = SettingsMenu::new();
        menu.open_classic_group(
            &console,
            Group::Quick,
            crate::menu::classic::panel::Frame::Main,
        );
        menu.select_cvar(crate::menu::style::CVAR);
        menu.leave_classic();
        menu.continue_modern(&console);
        assert_eq!((menu.section, menu.tab), (Section::General, QUICK_TAB));
        assert_eq!(menu.rows()[menu.selected].cvar, crate::menu::style::CVAR);
        // Interface's own row still carries on as Interface's tab.
        menu.open_classic_group(
            &console,
            Group::Interface,
            crate::menu::classic::panel::Frame::Main,
        );
        menu.select_cvar(crate::menu::style::CVAR);
        menu.leave_classic();
        menu.continue_modern(&console);
        assert_ne!(menu.tab, QUICK_TAB);
        assert_eq!(menu.rows()[menu.selected].cvar, crate::menu::style::CVAR);
    }

    const SECTIONS: [(Section, usize); 2] = [
        (Section::General, TABS.len()),
        (Section::Renderer, RENDERER_TABS.len()),
    ];

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    fn rows() -> impl Iterator<Item = &'static Setting> {
        SECTIONS.into_iter().flat_map(|(section, tabs)| {
            (0..tabs).flat_map(move |tab| section_settings(section, tab))
        })
    }

    #[test]
    fn every_row_names_a_registered_cvar_of_its_kind() {
        let (_directory, console) = console();
        for setting in rows() {
            // Rows whose value is not one cvar's (resolution, display mode, HUD).
            if matches!(
                setting.kind,
                ValueKind::Resolution | ValueKind::DisplayMode | ValueKind::HudPicker
            ) {
                continue;
            }
            let value = console
                .cvar(setting.cvar)
                .unwrap_or_else(|| panic!("{} is not registered", setting.cvar));
            let fits = match (setting.kind, value) {
                (
                    ValueKind::Bool,
                    CvarValue::Bool(_) | CvarValue::Integer(_) | CvarValue::Float(_),
                ) => true,
                // A negative minimum is the one special value below the range (AUTO).
                (ValueKind::Integer { min, max, .. }, CvarValue::Integer(value)) => {
                    (min.min(0)..=max).contains(value) || (min < 0 && *value == min)
                }
                (ValueKind::Float { min, max, .. }, CvarValue::Float(value)) => {
                    (min..=max).contains(value)
                }
                // Choices cycle the cvar's text form, whatever its type.
                (ValueKind::Choice(values), value) => values.contains(&value.as_text().as_str()),
                (ValueKind::Text, CvarValue::Text(_)) => true,
                _ => false,
            };
            assert!(
                fits,
                "{} default {value:?} does not fit its row",
                setting.cvar
            );
        }
    }

    #[test]
    fn the_renderer_tabs_hold_every_renamed_rendering_cvar() {
        // Bookkeeping, diagnostics, the ground HUD (on the HUD tab) and exclusive
        // fullscreen (the Video tab's display-mode row) stay off it.
        const NOT_RENDERER: [&str; 6] = [
            "cl_bindDefaultsVersion",
            "cl_sensitivityScaleVersion",
            "com_maxfpsDefaultVersion",
            "r_dayDebug",
            "cg_groundHud",
            "r_exclusiveFullscreen",
        ];
        let (_directory, console) = console();
        let renderer: Vec<_> = (0..RENDERER_TABS.len())
            .flat_map(|tab| section_settings(Section::Renderer, tab))
            .map(|setting| setting.cvar)
            .collect();
        // JKR's own `jkr_*` cvars, under SJK's names.
        for &(_, name) in crate::cvar_renames::RENAMED {
            assert!(console.cvar(name).is_some(), "{name} is not registered");
            if !NOT_RENDERER.contains(&name) {
                assert!(
                    renderer.contains(&name),
                    "{name} is missing from the renderer tabs"
                );
            }
        }
        for tab in 0..TABS.len() {
            for setting in settings(tab) {
                assert!(
                    !renderer.contains(&setting.cvar),
                    "{} is on both pages",
                    setting.cvar
                );
            }
        }
    }

    #[test]
    fn the_video_row_opens_the_renderer_and_back_returns_to_it() {
        let (_directory, console) = console();
        let mut menu = SettingsMenu::new();
        menu.open_tab(&console, RENDERER_TAB);
        assert_eq!(menu.action(), Some(Action::Renderer));
        assert_eq!(menu.row_count(), VIDEO.len() + 1);
        menu.selected = VIDEO.len();
        assert!(matches!(
            menu.activate_action(&console),
            SettingsResult::None
        ));
        assert_eq!(
            (menu.section, menu.tab, menu.selected),
            (Section::Renderer, 0, 0)
        );
        assert_eq!(menu.tabs(), &RENDERER_TABS);
        assert_eq!(menu.values.len(), RENDER_IMAGE.len());
        menu.select_tab(&console, RENDERER_TABS.len());
        assert_eq!(menu.tab, 0, "renderer tabs wrap within their own section");
        assert!(matches!(menu.back(&console), SettingsResult::None));
        assert_eq!(
            (menu.section, menu.tab, menu.selected),
            (Section::General, RENDERER_TAB, VIDEO.len())
        );
        assert!(matches!(menu.back(&console), SettingsResult::Back));
    }

    #[test]
    fn the_renderer_opened_on_its_own_backs_out_of_the_screen() {
        let (_directory, console) = console();
        let mut menu = SettingsMenu::new();
        menu.open_renderer(&console);
        assert_eq!((menu.section, menu.tab), (Section::Renderer, 0));
        assert_eq!(menu.action(), None);
        assert!(matches!(menu.back(&console), SettingsResult::Back));
        // A later ordinary open is the general screen again.
        menu.open_tab(&console, RENDERER_TAB);
        assert_eq!(menu.section, Section::General);
        assert_eq!(menu.action(), Some(Action::Renderer));
    }

    #[test]
    fn switch_rows_toggle_integer_and_float_cvars() {
        let (_directory, mut console) = console();
        let mut menu = SettingsMenu::new();
        menu.open_tab(&console, RENDERER_TAB);
        menu.enter_renderer(&console);
        for (tab, cvar) in [(0, "r_sceneBloom"), (2, "r_contactShadows")] {
            menu.select_tab(&console, tab);
            menu.selected = menu
                .rows()
                .iter()
                .position(|setting| setting.cvar == cvar)
                .unwrap();
            let before = switch_on(console.cvar(cvar).unwrap());
            menu.adjust(&mut console, 1);
            assert_eq!(switch_on(console.cvar(cvar).unwrap()), !before, "{cvar}");
            assert_eq!(
                menu.values[menu.selected],
                if before { "OFF" } else { "ON" }
            );
            menu.adjust(&mut console, 1);
            assert_eq!(switch_on(console.cvar(cvar).unwrap()), before, "{cvar}");
        }
    }

    #[test]
    fn auto_sits_below_zero_and_steps_join_the_grid() {
        let step = |value, direction| step_integer(value, direction, -1, 2000, 25);
        assert_eq!(step(-1, 1), 0);
        assert_eq!(step(-1, -1), -1);
        assert_eq!(step(0, -1), -1);
        assert_eq!(step(0, 1), 25);
        assert_eq!(step(144, -1), 119);
        assert_eq!(step(1990, 1), 2000);
    }

    #[test]
    fn rows_without_a_special_value_step_as_before() {
        assert_eq!(step_integer(80, 1, 80, 130, 5), 85);
        assert_eq!(step_integer(80, -1, 80, 130, 5), 80);
    }

    #[test]
    fn a_fresh_profile_takes_sols_visual_defaults_and_a_saved_one_keeps_its_values() {
        use crate::world_materials::shadows::day;
        let (_directory, console) = console();
        for cvar in [
            "r_sceneBloom",
            "r_normalMapping",
            "r_specularMapping",
            "r_parallaxMapping",
        ] {
            assert_eq!(console.integer_cvar(cvar), Some(1), "{cvar}");
        }
        assert_eq!(console.float_cvar("r_parallaxStrength"), Some(0.1));
        assert_eq!(console.float_cvar("r_dayHour"), Some(12.0));
        assert_eq!(console.float_cvar(crate::dust_motes::CVAR), Some(1.0));
        // The live state the renderer reads starts from the same defaults.
        assert!(console.post_color.policy().bloom);
        assert_eq!(console.dust_motes.intensity(), 1.0);
        assert_eq!(day::Settings::sample(Some(&console)).hour, 12.0);
        assert_eq!(day::live::Clock::default().values()[0], 12.0);

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.cfg");
        std::fs::write(
            &path,
            "seta r_dayHour \"11\"\nseta r_sceneBloom \"0\"\nseta r_dustMotes \"0\"\n\
             seta r_normalMapping \"0\"\n",
        )
        .unwrap();
        let console = ViewerConsole::new(path).unwrap();
        assert_eq!(console.float_cvar("r_dayHour"), Some(11.0));
        assert_eq!(console.integer_cvar("r_sceneBloom"), Some(0));
        assert!(!console.post_color.policy().bloom);
        assert_eq!(console.dust_motes.intensity(), 0.0);
        assert_eq!(console.integer_cvar("r_normalMapping"), Some(0));
    }
}
