//! Retained tabbed settings UI backed directly by archived shell cvars.

use crate::console::ViewerConsole;
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextVertex, UiFont};
use jkr_shell::CvarValue;
use jkr_ui::{DrawList, Rect};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

mod catalog;
mod classic_view;
mod display;
mod numeric;
mod pointer;
mod resolution;
mod resolution_list;
mod scroll;
mod view;

pub(crate) use catalog::RESOLUTIONS;
use catalog::*;
pub(crate) use display::{
    DisplayMode, EXCLUSIVE_CVAR, MonitorModes, exclusive_supported, exclusive_video_mode,
};
use resolution::{PickResult, ResolutionChoice, ResolutionPicker};
pub(crate) enum SettingsResult {
    None,
    Back,
    OpenKeybinds,
    /// A button of the classic panel screen around the options: index into
    /// its page's slots.
    Classic(usize),
    /// Move the classic panel to the next (1) or previous (-1) group.
    ClassicCycle(i32),
}

/// The rows a classic option panel shows: a span of the tab, and where its
/// slider bars sit across a row.
struct ClassicRows {
    rows: std::ops::Range<usize>,
    slider_span: (f32, f32),
}

/// A Text setting being typed. Its row is fixed when typing starts, so Enter
/// writes the setting that was opened even if the pointer moved meanwhile.
struct TextDraft {
    row: usize,
    text: String,
}

pub(crate) struct SettingsMenu {
    tab: usize,
    selected: usize,
    /// Which rows show; keeps the selection on screen.
    scroll: scroll::RowScroll,
    values: Vec<String>,
    editing: Option<TextDraft>,
    /// What the window's monitor offers; asked for each time the screen opens.
    monitor: Option<MonitorModes>,
    /// The screen opened and wants fresh [`MonitorModes`].
    wants_monitor: bool,
    /// Scratch list of the resolutions on offer.
    choices: Vec<ResolutionChoice>,
    /// The resolution list, open over the form.
    picker: ResolutionPicker,
    numeric: Option<crate::menu_widgets::numeric::NumericEdit>,
    /// Set while the screen is a classic option panel.
    classic: Option<ClassicRows>,
    ui: MenuCanvas,
}

impl SettingsMenu {
    pub(crate) fn new() -> Self {
        Self {
            tab: 0,
            selected: 0,
            scroll: scroll::RowScroll::new(),
            values: Vec::with_capacity(12),
            editing: None,
            monitor: None,
            wants_monitor: false,
            choices: Vec::with_capacity(48),
            picker: ResolutionPicker::new(),
            numeric: None,
            classic: None,
            ui: MenuCanvas::new(),
        }
    }

    /// Index of the tab that carries the "Key bindings" row.
    pub(crate) fn keybinds_tab() -> usize {
        KEYBINDS_TAB
    }

    /// Rows of tab `tab` (without the key-bindings row).
    #[cfg(test)]
    pub(crate) fn tab_len(tab: usize) -> usize {
        settings(tab).len()
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
        self.tab = tab.min(TABS.len() - 1);
        self.selected = 0;
        self.editing = None;
        self.picker.close();
        self.wants_monitor = true;
        self.numeric = None;
        self.classic = None;
        self.refresh(console);
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

    /// Rows keyboard focus moves through: the classic panel's span, or the
    /// whole tab (with its key-bindings row).
    fn row_span(&self) -> std::ops::Range<usize> {
        match &self.classic {
            Some(classic) => classic.rows.clone(),
            None => 0..settings(self.tab).len() + usize::from(self.tab == KEYBINDS_TAB),
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
        if self.picker.is_open() {
            self.resolution_key(key, event.repeat, console);
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
                        && let Some(setting) = settings(self.tab).get(draft.row)
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
        let span = self.row_span();
        if span.is_empty() {
            return match key {
                KeyCode::Escape => SettingsResult::Back,
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
        let classic = self.classic.is_some();
        match key {
            KeyCode::Tab | KeyCode::BracketRight if classic => {
                return SettingsResult::ClassicCycle(1);
            }
            KeyCode::BracketLeft if classic => return SettingsResult::ClassicCycle(-1),
            KeyCode::Tab | KeyCode::BracketRight => {
                self.tab = (self.tab + 1) % TABS.len();
                self.selected = 0;
                self.refresh(console);
            }
            KeyCode::BracketLeft => {
                self.tab = self.tab.checked_sub(1).unwrap_or(TABS.len() - 1);
                self.selected = 0;
                self.refresh(console);
            }
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.selected = if self.selected <= span.start {
                    span.end - 1
                } else {
                    (self.selected - 1).min(span.end - 1)
                };
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.selected = if self.selected + 1 >= span.end || self.selected < span.start {
                    span.start
                } else {
                    self.selected + 1
                };
            }
            KeyCode::ArrowLeft | KeyCode::KeyA => self.adjust(console, -1),
            KeyCode::ArrowRight | KeyCode::KeyD => self.adjust(console, 1),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space
                if self.tab == KEYBINDS_TAB && self.selected == settings(KEYBINDS_TAB).len() =>
            {
                return SettingsResult::OpenKeybinds;
            }
            // SJK: Space keeps stepping a slider; Enter opens its entry.
            KeyCode::Space
                if settings(self.tab)
                    .get(self.selected)
                    .is_some_and(|setting| {
                        matches!(
                            setting.kind,
                            ValueKind::Integer { .. } | ValueKind::Float { .. }
                        )
                    }) =>
            {
                self.adjust(console, 1);
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(setting) = settings(self.tab).get(self.selected) {
                    if matches!(setting.kind, ValueKind::Text) {
                        self.begin_text(console, self.selected);
                    } else if matches!(setting.kind, ValueKind::Resolution) {
                        self.open_resolutions(console);
                    } else if !self.begin_numeric(console, self.selected) {
                        self.adjust(console, 1);
                    }
                }
            }
            KeyCode::Escape => return SettingsResult::Back,
            _ => {}
        }
        SettingsResult::None
    }

    fn adjust(&mut self, console: &mut ViewerConsole, direction: i32) {
        let Some(setting) = settings(self.tab).get(self.selected) else {
            return;
        };
        let next = match (setting.kind, console.cvar(setting.cvar)) {
            (ValueKind::Bool, Some(CvarValue::Bool(value))) => (!value).to_string(),
            (ValueKind::Bool, Some(CvarValue::Integer(value))) => {
                if *value != 0 { "0" } else { "1" }.to_owned()
            }
            (ValueKind::Integer { min, max, step }, Some(CvarValue::Integer(value))) => {
                step_integer(*value, direction, min, max, step).to_string()
            }
            (ValueKind::Float { .. }, Some(CvarValue::Float(value))) => {
                match setting.kind.stepped(*value, direction) {
                    Some(next) => next,
                    None => return,
                }
            }
            (ValueKind::Choice(values), Some(CvarValue::Text(value))) => {
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
        let Some(setting) = settings(self.tab).get(row) else {
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
        if self.classic.is_some() {
            let span = self.row_span();
            if !span.is_empty() {
                self.selected = (self.selected as i32 + direction)
                    .clamp(span.start as i32, span.end as i32 - 1)
                    as usize;
            }
        } else {
            let count = settings(self.tab).len() + usize::from(self.tab == KEYBINDS_TAB);
            if count > 0 {
                self.selected = self.scroll.wheel(direction, count, self.selected);
            }
        }
    }

    fn refresh(&mut self, console: &ViewerConsole) {
        let display = DisplayMode::requested(console).effective(self.exclusive_available());
        self.values.clear();
        self.values
            .extend(settings(self.tab).iter().map(|setting| match setting.kind {
                ValueKind::DisplayMode => display.label().to_owned(),
                ValueKind::Bool => toggle_text(console, setting.cvar),
                _ => row_text(console, setting),
            }));
    }
}

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
        _ => value_text(console, setting.cvar),
    }
}

/// ON/OFF for a toggle row; an integer cvar is on when nonzero.
fn toggle_text(console: &ViewerConsole, name: &str) -> String {
    match console.cvar(name) {
        Some(CvarValue::Integer(value)) => if *value != 0 { "ON" } else { "OFF" }.to_owned(),
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
    use super::step_integer;

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
}
