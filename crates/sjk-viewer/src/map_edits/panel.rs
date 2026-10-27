//! The map editor's panel (`docs/map-edits.md`, "The panel"): the editor's commands as an
//! SJK UI column down the window's right edge, as Camera control's, over the live map. The
//! world-note key opens it on what is under the crosshair while the editor is on; a click
//! on the scene left of the column selects what is under the pointer; Escape (or Aim)
//! hides it to aim again, with a strip at the top saying how to come back.
//!
//! Three pages: Selected (hide it, remap its shader), Place (an MD3 from the game data,
//! then turn, lift, scale or grab the placed model) and Edits (the map's edits, delete or
//! undo); under them the scope of new edits, the last command's answer, Publish, Aim and
//! Editor off. Every button runs the editor's own command ([`super::editor`]), so the
//! console and the panel stay one editor.
//!
//! It is a page of the in-game menu ([`crate::ingame_menu::Page::MapEdit`]), which frees
//! the pointer and keeps clicks from the game; its state lives with the map's edits.

use super::document::{Kind, Scope};
use crate::GpuState;
use crate::ingame_menu::Page;
use crate::menu_widgets::MenuCanvas;
use crate::world_notes::EditTarget;
use sjk_ui::{AbstractAction, InputEvent, PointerButton, Rect, UiEventKind, Vec2};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// The pages, as tabs.
pub(crate) const TAB_SELECTED: u16 = 1;
pub(crate) const TAB_PLACE: u16 = 2;
pub(crate) const TAB_EDITS: u16 = 3;
/// Selected: hide it (or remove a placed model), its whole compiled model, a copy of it.
pub(crate) const HIDE: u16 = 10;
pub(crate) const HIDE_GROUP: u16 = 11;
pub(crate) const COPY: u16 = 12;
/// Selected: the remap's field and buttons.
pub(crate) const REMAP_FIELD: u16 = 13;
pub(crate) const REMAP: u16 = 14;
pub(crate) const UNREMAP: u16 = 15;
/// Place: the search, placing the chosen model, the placed model's controls.
pub(crate) const SEARCH: u16 = 20;
pub(crate) const PLACE: u16 = 21;
pub(crate) const TURN_LEFT: u16 = 22;
pub(crate) const TURN_RIGHT: u16 = 23;
pub(crate) const UP: u16 = 24;
pub(crate) const DOWN: u16 = 25;
pub(crate) const SMALLER: u16 = 26;
pub(crate) const BIGGER: u16 = 27;
pub(crate) const GRAB: u16 = 28;
pub(crate) const REMOVE: u16 = 29;
/// Place, SJK's creator only: a mystery box where you aim, the nearest one removed
/// (`crate::mystery_box`).
pub(crate) const BOX_PLACE: u16 = 32;
pub(crate) const BOX_REMOVE: u16 = 33;
/// Edits: delete the chosen edit, undo the newest.
pub(crate) const DELETE: u16 = 30;
pub(crate) const UNDO: u16 = 31;
/// The foot: the scope's two segments, then the closing buttons.
pub(crate) const SCOPE_ALL: u16 = 40;
pub(crate) const SCOPE_SERVER: u16 = 41;
pub(crate) const PUBLISH: u16 = 42;
pub(crate) const AIM: u16 = 43;
pub(crate) const EDITOR_OFF: u16 = 44;
/// The model list's rows on show are `MODEL_ROW..MODEL_ROW + MODEL_ROWS`, the edits'
/// `EDIT_ROW..`.
pub(crate) const MODEL_ROW: u16 = 100;
pub(crate) const MODEL_ROWS: usize = 7;
pub(crate) const EDIT_ROW: u16 = 200;
pub(crate) const EDIT_ROWS: usize = 12;

/// Longest text the fields take.
const FIELD_MAX: usize = 96;
/// What Turn, Up and Down, Smaller and Bigger change by.
const TURN: f32 = 15.0;
const LIFT: f32 = 8.0;
const GROW: f32 = 1.1;

/// The page on show.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Tab {
    #[default]
    Selected,
    Place,
    Edits,
}

impl Tab {
    pub(crate) fn token(self) -> u16 {
        match self {
            Self::Selected => TAB_SELECTED,
            Self::Place => TAB_PLACE,
            Self::Edits => TAB_EDITS,
        }
    }
}

/// What a click or key asks the editor for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Hide,
    HideGroup,
    Copy,
    Remap,
    Unremap,
    Place,
    Turn(f32),
    Lift(f32),
    Scale(f32),
    Grab,
    Remove,
    BoxPlace,
    BoxRemove,
    Delete,
    Undo,
    Scope(Scope),
    Publish,
    Aim,
    EditorOff,
}

/// What the selection is, as the panel names it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Picked {
    /// A `misc_model_static` of the map.
    Prop,
    /// A model an edit placed.
    Placed,
    /// A BSP surface; `soup` when the compiler merged a model into it (Hide whole model).
    Surface { soup: bool },
    /// An entity the game needs (a door, a breakable): remapped, never hidden.
    Entity,
    /// Nothing an edit changes.
    Other,
}

/// The selection as shown.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Selected {
    pub(crate) picked: Picked,
    /// Its name line: the model's path, or the surface's number.
    pub(crate) name: String,
    /// Its detail line: the shader, the class, the distance.
    pub(crate) detail: String,
    /// Whether it has a shader to remap.
    pub(crate) remappable: bool,
}

/// One edit in the Edits list.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EditRow {
    pub(crate) id: u32,
    pub(crate) text: String,
    /// It applies on this server.
    pub(crate) here: bool,
}

/// What the panel shows of the editor and the map, read again when they change.
#[derive(Clone, Debug, Default)]
pub(crate) struct Facts {
    /// The map's name, or empty while it loads.
    pub(crate) map: String,
    /// The map's checksum is known (edits can be made).
    pub(crate) ready: bool,
    pub(crate) edit_count: usize,
    pub(crate) unpublished: bool,
    pub(crate) revision: u64,
    pub(crate) selection: Option<Selected>,
    /// The current placed model's line, when there is one.
    pub(crate) placed: Option<String>,
    /// The map's edits, newest first.
    pub(crate) rows: Vec<EditRow>,
    pub(crate) scope: Scope,
    /// The server new edits for this server would name, when on one.
    pub(crate) server: Option<String>,
    pub(crate) grabbing: bool,
    /// The first key bound to the world note (the editor's select key).
    pub(crate) select_key: String,
}

/// The panel's state.
#[derive(Default)]
pub(crate) struct Panel {
    pub(crate) tab: Tab,
    /// The control the keyboard is on.
    pub(crate) focus: u16,
    pub(crate) remap: String,
    pub(crate) search: String,
    pub(crate) facts: Facts,
    /// Every MD3 the game data holds outside the player models, read once per game data.
    models: Vec<String>,
    /// The game data [`Self::models`] was read from (its address).
    models_from: usize,
    /// The indices of [`Self::models`] the search keeps.
    pub(crate) shown: Vec<u32>,
    pub(crate) model_row: usize,
    pub(crate) model_first: usize,
    pub(crate) edit_row: usize,
    pub(crate) edit_first: usize,
    /// The last command's answer, and whether it was a refusal.
    pub(crate) status: String,
    pub(crate) status_error: bool,
    /// Built by the view: where the scene ends (window pixels; a press left of it selects),
    /// and the lists' areas for the wheel.
    pub(crate) scene_right: f32,
    pub(crate) model_list: Option<Rect>,
    pub(crate) edit_list: Option<Rect>,
    /// What [`Facts`] were read for: the document's revision, the current placed model,
    /// grabbing, and whether the map's checksum was known.
    seen: Option<(u64, Option<u32>, bool, bool)>,
}

/// Whether `path` (the game data's spelling) is a model a map edit can place: an MD3
/// outside the player models.
fn placeable(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md3") && lower.starts_with("models/") && !lower.starts_with("models/players/")
}

/// Whether `path` matches every word of `query` (lower case).
fn matches(path: &str, query: &str) -> bool {
    let path = path.to_ascii_lowercase();
    query.split_whitespace().all(|word| path.contains(word))
}

impl Panel {
    /// Lay the panel out in the SJK UI's look.
    pub(crate) fn build(&mut self, canvas: &mut MenuCanvas, viewport: [f32; 2]) {
        super::panel_view::build(self, canvas, viewport);
    }

    /// The models to place: read from `vfs` the first time the Place page needs them.
    fn read_models(&mut self, vfs: Option<&sjk_vfs::VirtualFileSystem>) {
        let Some(vfs) = vfs else {
            return;
        };
        let from = std::ptr::from_ref(vfs) as usize;
        if self.models_from == from {
            return;
        }
        self.models_from = from;
        self.models = vfs
            .paths()
            .into_iter()
            .map(|path| path.as_str().to_owned())
            .filter(|path| placeable(path))
            .collect();
        self.models.sort();
        self.models.dedup();
        self.refilter();
    }

    #[cfg(test)]
    pub(crate) fn set_models(&mut self, models: &[&str]) {
        self.models = models.iter().map(|model| (*model).to_owned()).collect();
        self.refilter();
    }

    fn refilter(&mut self) {
        let query = self.search.to_ascii_lowercase();
        self.shown.clear();
        self.shown.extend(
            self.models
                .iter()
                .enumerate()
                .filter(|(_, model)| matches(model, &query))
                .map(|(index, _)| index as u32),
        );
        self.model_row = 0;
        self.model_first = 0;
    }

    /// The model chosen in the list.
    pub(crate) fn chosen_model(&self) -> Option<&str> {
        self.shown
            .get(self.model_row)
            .and_then(|&index| self.models.get(index as usize))
            .map(String::as_str)
    }

    /// The model list's row `index` on show, for the view.
    pub(crate) fn shown_model(&self, index: usize) -> Option<&str> {
        self.shown
            .get(index)
            .and_then(|&index| self.models.get(index as usize))
            .map(String::as_str)
    }

    /// The edit chosen in the list.
    pub(crate) fn chosen_edit(&self) -> Option<u32> {
        self.facts.rows.get(self.edit_row).map(|row| row.id)
    }

    pub(super) fn show(&mut self, tab: Tab) {
        self.tab = tab;
        self.focus = tab.token();
    }

    /// Keep `row` of `count` rows on show in a list of `rows` from `first`.
    fn settle(row: &mut usize, first: &mut usize, count: usize, rows: usize) {
        *row = (*row).min(count.saturating_sub(1));
        if *row < *first {
            *first = *row;
        } else if *row >= *first + rows {
            *first = *row + 1 - rows;
        }
        *first = (*first).min(count.saturating_sub(rows));
    }

    /// Move the chosen row of the list the keyboard is on by `by`; whether it moved.
    fn move_in_list(&mut self, by: isize) -> bool {
        let (row, first, count, rows, base) = if self.is_model_row(self.focus) {
            (
                &mut self.model_row,
                &mut self.model_first,
                self.shown.len(),
                MODEL_ROWS,
                MODEL_ROW,
            )
        } else if self.is_edit_row(self.focus) {
            (
                &mut self.edit_row,
                &mut self.edit_first,
                self.facts.rows.len(),
                EDIT_ROWS,
                EDIT_ROW,
            )
        } else {
            return false;
        };
        let wanted = row.saturating_add_signed(by);
        if count == 0 || wanted >= count || (by < 0 && *row == 0) {
            return false;
        }
        *row = wanted;
        Self::settle(row, first, count, rows);
        self.focus = base + (*row - *first) as u16;
        true
    }

    fn is_model_row(&self, token: u16) -> bool {
        (MODEL_ROW..MODEL_ROW + MODEL_ROWS as u16).contains(&token)
    }

    fn is_edit_row(&self, token: u16) -> bool {
        (EDIT_ROW..EDIT_ROW + EDIT_ROWS as u16).contains(&token)
    }

    /// Up, Down or Tab: along a list, else the next control in the drawn order.
    pub(crate) fn step(&mut self, canvas: &mut MenuCanvas, forward: bool) {
        if self.move_in_list(if forward { 1 } else { -1 }) {
            return;
        }
        let direction = if forward {
            AbstractAction::Next
        } else {
            AbstractAction::Previous
        };
        if let Some(token) = canvas.action(direction) {
            self.focus = token;
            // Into a list at its chosen row.
            if self.is_model_row(token) {
                self.focus = MODEL_ROW + (self.model_row.saturating_sub(self.model_first)) as u16;
            } else if self.is_edit_row(token) {
                self.focus = EDIT_ROW + (self.edit_row.saturating_sub(self.edit_first)) as u16;
            }
        }
    }

    /// The wheel over a list scrolls it; whether it was over one.
    fn scroll(&mut self, position: Vec2, delta: Vec2) -> bool {
        let inside = |rect: Option<Rect>| {
            rect.is_some_and(|rect| {
                position.x >= rect.x
                    && position.x <= rect.right()
                    && position.y >= rect.y
                    && position.y <= rect.bottom()
            })
        };
        let lines = if delta.y > 0.0 { -3 } else { 3 };
        let (first, count, rows) = if inside(self.model_list) {
            (&mut self.model_first, self.shown.len(), MODEL_ROWS)
        } else if inside(self.edit_list) {
            (&mut self.edit_first, self.facts.rows.len(), EDIT_ROWS)
        } else {
            return false;
        };
        *first = first
            .saturating_add_signed(lines)
            .min(count.saturating_sub(rows));
        true
    }

    /// A pointer event on the column.
    pub(crate) fn pointer(&mut self, canvas: &mut MenuCanvas, event: InputEvent) -> Option<Action> {
        if let InputEvent::PointerWheel { position, delta } = event
            && self.scroll(position, delta)
        {
            return None;
        }
        let event = canvas.pointer(event)?;
        let token = event.token?;
        if matches!(event.kind, UiEventKind::Hover | UiEventKind::HoverEnter) {
            // Hovering a list's row does not choose it; a click does.
            if !self.is_model_row(token) && !self.is_edit_row(token) {
                self.focus = token;
            }
            return None;
        }
        if event.kind != UiEventKind::Activate {
            return None;
        }
        self.focus = token;
        self.activate(token)
    }

    /// Enter, Space or a click on `token`.
    pub(crate) fn activate(&mut self, token: u16) -> Option<Action> {
        match token {
            TAB_SELECTED => self.show(Tab::Selected),
            TAB_PLACE => self.show(Tab::Place),
            TAB_EDITS => self.show(Tab::Edits),
            HIDE => return Some(Action::Hide),
            HIDE_GROUP => return Some(Action::HideGroup),
            COPY => return Some(Action::Copy),
            REMAP | REMAP_FIELD => return Some(Action::Remap),
            UNREMAP => return Some(Action::Unremap),
            PLACE | SEARCH => return Some(Action::Place),
            TURN_LEFT => return Some(Action::Turn(TURN)),
            TURN_RIGHT => return Some(Action::Turn(-TURN)),
            UP => return Some(Action::Lift(LIFT)),
            DOWN => return Some(Action::Lift(-LIFT)),
            SMALLER => return Some(Action::Scale(1.0 / GROW)),
            BIGGER => return Some(Action::Scale(GROW)),
            GRAB => return Some(Action::Grab),
            REMOVE => return Some(Action::Remove),
            BOX_PLACE => return Some(Action::BoxPlace),
            BOX_REMOVE => return Some(Action::BoxRemove),
            DELETE => return Some(Action::Delete),
            UNDO => return Some(Action::Undo),
            SCOPE_ALL => return Some(Action::Scope(Scope::AllServers)),
            SCOPE_SERVER => return Some(Action::Scope(Scope::ThisServer)),
            PUBLISH => return Some(Action::Publish),
            AIM => return Some(Action::Aim),
            EDITOR_OFF => return Some(Action::EditorOff),
            row if self.is_model_row(row) => {
                let index = self.model_first + usize::from(row - MODEL_ROW);
                if index < self.shown.len() {
                    // A click on the chosen row places it.
                    if index == self.model_row {
                        return Some(Action::Place);
                    }
                    self.model_row = index;
                }
            }
            row if self.is_edit_row(row) => {
                let index = self.edit_first + usize::from(row - EDIT_ROW);
                if index < self.facts.rows.len() {
                    self.edit_row = index;
                }
            }
            _ => {}
        }
        None
    }

    /// The field the keyboard types in, if it is on one.
    fn field(&mut self) -> Option<&mut String> {
        match self.focus {
            REMAP_FIELD => Some(&mut self.remap),
            SEARCH => Some(&mut self.search),
            _ => None,
        }
    }

    /// A key while the panel is open (the caller has the modifiers): an action, or none.
    pub(crate) fn key(
        &mut self,
        canvas: &mut MenuCanvas,
        event: &KeyEvent,
        shift: bool,
        control: bool,
    ) -> Option<Action> {
        if event.state != ElementState::Pressed {
            return None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return None;
        };
        match key {
            KeyCode::Escape if !event.repeat => return Some(Action::Aim),
            KeyCode::ArrowUp => self.step(canvas, false),
            KeyCode::ArrowDown => self.step(canvas, true),
            KeyCode::Tab if !event.repeat => self.step(canvas, !shift),
            KeyCode::PageUp => {
                let _ = self.move_in_list(-(MODEL_ROWS as isize));
            }
            KeyCode::PageDown => {
                let _ = self.move_in_list(MODEL_ROWS as isize);
            }
            KeyCode::ArrowLeft | KeyCode::ArrowRight if !event.repeat => {
                let right = key == KeyCode::ArrowRight;
                match self.focus {
                    TAB_SELECTED | TAB_PLACE | TAB_EDITS => {
                        let tabs = [Tab::Selected, Tab::Place, Tab::Edits];
                        let at = tabs.iter().position(|tab| *tab == self.tab).unwrap_or(0);
                        let next = if right { (at + 1) % 3 } else { (at + 2) % 3 };
                        self.show(tabs[next]);
                    }
                    SCOPE_ALL | SCOPE_SERVER => {
                        self.focus = if right { SCOPE_SERVER } else { SCOPE_ALL };
                        return Some(Action::Scope(if right {
                            Scope::ThisServer
                        } else {
                            Scope::AllServers
                        }));
                    }
                    _ => {}
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter if !event.repeat => {
                return self.activate(self.focus);
            }
            KeyCode::KeyZ if control && !event.repeat => return Some(Action::Undo),
            KeyCode::Delete if self.is_edit_row(self.focus) && !event.repeat => {
                return Some(Action::Delete);
            }
            _ => {
                let search = self.focus == SEARCH;
                if let Some(field) = self.field() {
                    let text = event.text.as_deref();
                    if crate::text_entry::edit(field, key, text, shift, control, FIELD_MAX, true)
                        && search
                    {
                        self.refilter();
                    }
                } else if key == KeyCode::Space && !event.repeat {
                    return self.activate(self.focus);
                }
            }
        }
        None
    }

    /// Show what a command answered: its last line, or why it was refused.
    pub(super) fn answer(&mut self, result: Result<Vec<String>, String>) {
        match result {
            Ok(lines) => {
                self.status = lines.last().cloned().unwrap_or_default();
                self.status_error = false;
            }
            Err(error) => {
                self.status = error;
                self.status_error = true;
            }
        }
    }

    /// Read the facts again on the next frame.
    pub(super) fn stale(&mut self) {
        self.seen = None;
    }
}

impl GpuState {
    /// Whether the editor's panel is on show.
    pub(crate) fn map_edit_panel_open(&self) -> bool {
        self.game_menu && self.game_menu_page == Page::MapEdit
    }

    /// Open the panel over the live map, the pointer freed for it.
    pub(crate) fn open_map_edit_panel(&mut self) {
        if !self.map_edits.editor.active {
            return;
        }
        let panel = &mut self.map_edits.panel;
        panel.stale();
        if panel.focus == 0 {
            panel.focus = panel.tab.token();
        }
        self.gameplay_input.release_keys();
        self.game_menu = true;
        self.game_menu_page = Page::MapEdit;
        self.release_pointer();
    }

    /// Hide the panel to aim again; the editor stays on.
    pub(crate) fn close_map_edit_panel(&mut self) {
        if self.map_edit_panel_open() {
            self.game_menu = false;
            self.game_menu_page = Page::Main;
            self.capture_pointer();
        }
    }

    /// A key while the panel is open: all of them are its own. False when it is not open.
    pub(crate) fn map_edit_key(&mut self, event: &KeyEvent) -> bool {
        if !self.map_edit_panel_open() {
            return false;
        }
        let (shift, control) = self.console.as_ref().map_or((false, false), |console| {
            (console.shift_held(), console.control_held())
        });
        if let Some(action) =
            self.map_edits
                .panel
                .key(self.in_game_menu.canvas_mut(), event, shift, control)
        {
            self.apply_map_edit_action(action);
        }
        true
    }

    /// A pointer event while the panel is open: the column's, or a press on the scene,
    /// which selects what is under the pointer.
    pub(crate) fn map_edit_pointer(&mut self, event: InputEvent) {
        if let InputEvent::PointerPress {
            position,
            button: PointerButton::Primary,
        } = event
            && position.x < self.map_edits.panel.scene_right
        {
            self.map_edit_pick_at([position.x, position.y]);
            let panel = &mut self.map_edits.panel;
            panel.show(Tab::Selected);
            panel.stale();
            return;
        }
        if let Some(action) = self
            .map_edits
            .panel
            .pointer(self.in_game_menu.canvas_mut(), event)
        {
            self.apply_map_edit_action(action);
        }
    }

    /// Run what `action` asks with the editor's commands, and show their answer.
    fn apply_map_edit_action(&mut self, action: Action) {
        let text = |words: &[&str]| words.iter().map(|word| (*word).to_owned()).collect();
        let (name, args): (&str, Vec<String>) = match action {
            Action::Aim => {
                self.close_map_edit_panel();
                return;
            }
            Action::BoxPlace | Action::BoxRemove => {
                let (name, args) = match action {
                    Action::BoxPlace => match self.map_edits.editor.scope() {
                        Scope::ThisServer => ("mysterybox_place", text(&["server"])),
                        Scope::AllServers => ("mysterybox_place", vec![]),
                    },
                    _ => ("mysterybox_remove", vec![]),
                };
                let result = self.mystery_box_command(name, &args);
                self.map_edits.panel.answer(result);
                return;
            }
            Action::Hide => ("mapedit_hide", vec![]),
            Action::HideGroup => ("mapedit_hide", text(&["group"])),
            Action::Copy => ("mapedit_place", vec![]),
            Action::Remap => {
                let shader = self.map_edits.panel.remap.trim().to_owned();
                if shader.is_empty() {
                    let panel = &mut self.map_edits.panel;
                    panel.answer(Err("Type the shader to draw instead first".into()));
                    panel.focus = REMAP_FIELD;
                    return;
                }
                ("mapedit_remap", vec![shader])
            }
            Action::Unremap => ("mapedit_remap", text(&["-"])),
            Action::Place => {
                let panel = &mut self.map_edits.panel;
                let Some(model) = panel.chosen_model().map(str::to_owned) else {
                    panel.answer(Err("Choose a model to place first".into()));
                    return;
                };
                ("mapedit_place", vec![model])
            }
            Action::Turn(degrees) => ("mapedit_turn", vec![degrees.to_string()]),
            Action::Lift(units) => ("mapedit_move", vec![units.to_string()]),
            Action::Scale(factor) => ("mapedit_scale", vec![factor.to_string()]),
            Action::Grab => ("mapedit_grab", vec![]),
            Action::Remove => match self.map_edits.editor.current() {
                Some(id) => ("mapedit_delete", vec![id.to_string()]),
                None => {
                    self.map_edits
                        .panel
                        .answer(Err("No placed model: place one first".into()));
                    return;
                }
            },
            Action::Delete => match self.map_edits.panel.chosen_edit() {
                Some(id) => ("mapedit_delete", vec![id.to_string()]),
                None => {
                    self.map_edits
                        .panel
                        .answer(Err("Choose an edit in the list first".into()));
                    return;
                }
            },
            Action::Undo => ("mapedit_undo", vec![]),
            Action::Scope(Scope::AllServers) => ("mapedit_scope", text(&["all"])),
            Action::Scope(Scope::ThisServer) => ("mapedit_scope", text(&["server"])),
            Action::Publish => ("mapedit_publish", vec![]),
            Action::EditorOff => ("mapedit", text(&["off"])),
        };
        let result = self.map_edit_command(name, &args);
        let ok = result.is_ok();
        let panel = &mut self.map_edits.panel;
        panel.answer(result);
        panel.stale();
        match action {
            Action::Remap if ok => panel.remap.clear(),
            // A grabbed model follows the aim: back to aiming.
            Action::Grab if ok && self.map_edits.editor.grabbing() => self.close_map_edit_panel(),
            Action::EditorOff if ok => self.close_map_edit_panel(),
            _ => {}
        }
    }

    /// The strip over play while the editor is on and its panel hidden; whether it is
    /// drawn this frame.
    pub(crate) fn append_map_edit_strip(&mut self, viewport: [f32; 2]) -> bool {
        if !self.map_edits.editor.active || self.map_edit_panel_open() {
            return false;
        }
        self.refresh_map_edit_panel();
        let state = &mut self.map_edits;
        super::panel_view::build_strip(&state.panel.facts, &mut state.strip.0, viewport);
        crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        )
        .append(&self.map_edits.strip.0, viewport);
        true
    }

    /// Select what is under `point` (physical pixels) for the editor.
    fn map_edit_pick_at(&mut self, point: [f32; 2]) {
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let picked = self.world_notes.select_for_edit_at(
            crate::world_notes::Scene {
                bsp: &self.bsp,
                surfaces: &self.decal_surfaces,
                scratch: &mut self.trace_scratch,
                vfs: self.vfs.as_deref(),
                map: &map,
                remaps: Vec::new(),
                markers: Vec::new(),
                entities: self.map_edits.pick_entities(&self.bsp),
            },
            point,
        );
        if picked.is_none() {
            self.map_edits
                .panel
                .answer(Err("Nothing there to edit".into()));
        }
    }

    /// Once a frame while the panel is open: read the facts again when the edits, the
    /// placed model or the grab changed (allocation-free otherwise).
    pub(crate) fn refresh_map_edit_panel(&mut self) {
        let editor = &self.map_edits.editor;
        let key = (
            self.map_edits.revision,
            editor.current(),
            editor.grabbing(),
            self.map_edits.map().is_some(),
        );
        if self.map_edits.panel.seen == Some(key) {
            return;
        }
        self.map_edits.panel.seen = Some(key);
        let selection = self.map_edit_selected();
        let server = self.map_edits_server();
        let select_key = self.console.as_ref().map_or_else(String::new, |console| {
            console
                .keys_for_command("worldnote")
                .first()
                .map(|key| {
                    let mut name = String::new();
                    sjk_shell::key_names::push_display_key(&mut name, key);
                    name
                })
                .unwrap_or_default()
        });
        let state = &self.map_edits;
        let document = state.document.as_ref();
        let current = state.editor.current();
        let placed = current.and_then(|id| {
            document
                .and_then(|document| document.edits.iter().find(|edit| edit.id == id))
                .filter(|edit| matches!(edit.kind, Kind::Place { .. }))
                .map(super::document::Edit::describe)
        });
        let rows = document.map_or_else(Vec::new, |document| {
            document
                .edits
                .iter()
                .rev()
                .map(|edit| EditRow {
                    id: edit.id,
                    text: edit.describe(),
                    here: edit.applies_on(server.as_deref()),
                })
                .collect()
        });
        let facts = Facts {
            map: state
                .map()
                .map(|(map, _)| map.to_owned())
                .or_else(|| document.map(|document| document.map.clone()))
                .unwrap_or_default(),
            ready: state.map().is_some(),
            edit_count: document.map_or(0, |document| document.edits.len()),
            unpublished: document.is_some_and(|document| document.unpublished),
            revision: document.map_or(0, |document| document.revision),
            selection,
            placed,
            rows,
            scope: state.editor.scope(),
            server,
            grabbing: state.editor.grabbing(),
            select_key,
        };
        let vfs = self.vfs.clone();
        let panel = &mut self.map_edits.panel;
        panel.facts = facts;
        Panel::settle(
            &mut panel.edit_row,
            &mut panel.edit_first,
            panel.facts.rows.len(),
            EDIT_ROWS,
        );
        panel.read_models(vfs.as_deref());
    }

    /// The selection, as the panel shows it.
    fn map_edit_selected(&self) -> Option<Selected> {
        let selection = self.world_notes.waiting_selection()?;
        let summary = selection.summary();
        let lump =
            sjk_entity::parse_entity_lump(self.bsp.entities()).map_or(0, |entities| entities.len());
        Some(match selection.edit_target() {
            EditTarget::Model {
                number,
                classname,
                model,
                shader,
            } => {
                let picked = if self.map_edits.placed_edit(number, lump).is_some() {
                    Picked::Placed
                } else if self.static_models.is_prop(number as u32) {
                    Picked::Prop
                } else {
                    Picked::Entity
                };
                let class = if picked == Picked::Placed {
                    "placed by an edit"
                } else {
                    classname.as_str()
                };
                Selected {
                    picked,
                    name: model,
                    detail: match &shader {
                        Some(shader) => format!("{shader}, {class}"),
                        None => class.to_owned(),
                    },
                    remappable: shader.is_some(),
                }
            }
            EditTarget::Surface {
                index,
                shader,
                kind,
            } => Selected {
                picked: Picked::Surface {
                    soup: self
                        .bsp
                        .render()
                        .surfaces()
                        .get(index)
                        .is_some_and(|surface| surface.kind == sjk_bsp::SurfaceKind::TriangleSoup),
                },
                name: shader,
                detail: format!("surface {index}, {kind}"),
                remappable: true,
            },
            EditTarget::Other => Selected {
                picked: Picked::Other,
                name: "Not something edits change".to_owned(),
                detail: summary,
                remappable: false,
            },
        })
    }
}

#[cfg(test)]
impl GpuState {
    /// The world shots: the editor on (no staff check), what is under the crosshair
    /// selected, or under `point` (physical pixels) when given, and the panel open on `tab`.
    pub(crate) fn shot_map_edit_panel(&mut self, tab: Tab, point: Option<[f32; 2]>) {
        self.map_edits.editor.active = true;
        self.world_notes.hold_selection(true);
        match point {
            Some(point) => self.map_edit_pick_at(point),
            None => {
                let _ = self.map_edit_pick();
            }
        }
        self.open_map_edit_panel();
        self.map_edits.panel.show(tab);
        self.map_edits.panel.stale();
    }

    /// The world shots: type `query` in the Place page's search.
    pub(crate) fn shot_map_edit_search(&mut self, query: &str) {
        let panel = &mut self.map_edits.panel;
        panel.search = query.to_owned();
        panel.refilter();
        panel.focus = SEARCH;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_md3s_outside_the_player_models_are_placed() {
        assert!(placeable("models/map_objects/yavin/tree01.md3"));
        assert!(placeable("Models/Items/Shield.MD3"));
        assert!(!placeable("models/players/kyle/model.glm"));
        assert!(!placeable("models/players/kyle/head.md3"));
        assert!(!placeable("textures/yavin/tree.md3"));
        assert!(!placeable("models/map_objects/tree.glm"));
    }

    #[test]
    fn the_search_keeps_paths_with_every_word() {
        let mut panel = Panel::default();
        panel.set_models(&[
            "models/map_objects/yavin/tree01.md3",
            "models/map_objects/yavin/rock.md3",
            "models/map_objects/korriban/tree.md3",
        ]);
        assert_eq!(panel.shown.len(), 3);
        panel.search = "Yavin TREE".to_owned();
        panel.refilter();
        assert_eq!(
            panel.chosen_model(),
            Some("models/map_objects/yavin/tree01.md3")
        );
        assert_eq!(panel.shown.len(), 1);
        panel.search = "nothing".to_owned();
        panel.refilter();
        assert_eq!(panel.chosen_model(), None);
    }

    #[test]
    fn a_list_keeps_its_chosen_row_on_show() {
        let (mut row, mut first) = (11, 0);
        Panel::settle(&mut row, &mut first, 20, 8);
        assert_eq!((row, first), (11, 4));
        Panel::settle(&mut row, &mut first, 5, 8);
        assert_eq!((row, first), (4, 0));
        let (mut row, mut first) = (0, 0);
        Panel::settle(&mut row, &mut first, 0, 8);
        assert_eq!((row, first), (0, 0));
    }

    #[test]
    fn buttons_ask_for_their_command() {
        let mut panel = Panel::default();
        assert_eq!(panel.activate(HIDE), Some(Action::Hide));
        assert_eq!(panel.activate(TURN_RIGHT), Some(Action::Turn(-TURN)));
        assert_eq!(
            panel.activate(SCOPE_SERVER),
            Some(Action::Scope(Scope::ThisServer))
        );
        assert_eq!(panel.activate(TAB_EDITS), None);
        assert_eq!(panel.tab, Tab::Edits);
        // A model row chooses, then a second click places.
        panel.set_models(&["models/a.md3", "models/b.md3"]);
        assert_eq!(panel.activate(MODEL_ROW + 1), None);
        assert_eq!(panel.chosen_model(), Some("models/b.md3"));
        assert_eq!(panel.activate(MODEL_ROW + 1), Some(Action::Place));
    }
}
