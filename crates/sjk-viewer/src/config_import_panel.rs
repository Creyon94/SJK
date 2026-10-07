//! The Import page: what a dropped `.cfg` holds (`config_import.rs`), one tick per
//! part, and Enter to copy the ticked parts. Opened by dropping a file on the
//! window or by `firstsetup import`; like the Update page it lives in the console
//! and is drawn in place of it, so it opens over the menus and in a match.

use crate::config_import::{Found, Item};
use crate::menu_widgets::{BACK_TOKEN, FormLayout, MenuCanvas, Scrim};
use crate::text::{TextVertex, UiFont};
use sjk_ui::{FontWeight, InputEvent, Rect, UiEventKind};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Pointer targets of the footer's actions; the rows use their index.
const IMPORT_TOKEN: u16 = 930;
const TICK_TOKEN: u16 = 931;

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Copy the ticked parts ([`Panel::chosen`]).
    Import,
}

enum State {
    /// Opened without a file: says how to give one.
    Waiting,
    Loaded(Found),
    Failed(String),
    /// Imported; the lines say what changed.
    Done(Vec<String>),
}

pub(crate) struct Panel {
    open: bool,
    /// The page opened the console, so closing the page closes it too.
    owns_console: bool,
    /// The file's name, without its folder.
    file: String,
    state: State,
    /// The parts found, in [`Item::ALL`] order, and whether each is ticked.
    rows: Vec<(Item, bool)>,
    selected: usize,
    ui: MenuCanvas,
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            file: String::new(),
            state: State::Waiting,
            rows: Vec::with_capacity(Item::ALL.len()),
            selected: 0,
            ui: MenuCanvas::with_text_capacity(64),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page with `file`'s parts (`None` before a file is given);
    /// `owns_console` when the console was closed before it. Every part found
    /// starts ticked.
    pub(crate) fn open(
        &mut self,
        owns_console: bool,
        file: String,
        found: Option<Result<Found, String>>,
    ) {
        // A file dropped while the page is up keeps the console's owner.
        self.owns_console = if self.open {
            self.owns_console
        } else {
            owns_console
        };
        self.open = true;
        self.file = file;
        self.selected = 0;
        self.rows.clear();
        self.state = match found {
            None => State::Waiting,
            Some(Err(reason)) => State::Failed(reason),
            Some(Ok(found)) => {
                self.rows.extend(
                    Item::ALL
                        .into_iter()
                        .filter(|item| found.has(*item))
                        .map(|item| (item, true)),
                );
                State::Loaded(found)
            }
        };
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    /// The loaded parts and the ticked items, for the import.
    pub(crate) fn chosen(&self) -> Option<(&Found, Vec<Item>)> {
        let State::Loaded(found) = &self.state else {
            return None;
        };
        let items: Vec<Item> = self
            .rows
            .iter()
            .filter(|(_, ticked)| *ticked)
            .map(|(item, _)| *item)
            .collect();
        (!items.is_empty()).then_some((found, items))
    }

    /// Show what the import changed.
    pub(crate) fn finish(&mut self, lines: Vec<String>) {
        self.state = State::Done(lines);
        self.rows.clear();
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    fn toggle(&mut self, row: usize) {
        if let Some((_, ticked)) = self.rows.get_mut(row) {
            *ticked = !*ticked;
        }
    }

    /// What Enter does: import while a file's parts are on show, close otherwise.
    fn primary(&self) -> PanelAction {
        match self.state {
            State::Loaded(_) if self.chosen().is_some() => PanelAction::Import,
            State::Loaded(_) => PanelAction::None,
            _ => PanelAction::Close,
        }
    }

    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        let count = self.rows.len();
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::Enter | KeyCode::NumpadEnter => return self.primary(),
            KeyCode::ArrowUp if count > 0 => self.selected = (self.selected + count - 1) % count,
            KeyCode::ArrowDown | KeyCode::Tab if count > 0 => {
                self.selected = (self.selected + 1) % count;
            }
            KeyCode::Space | KeyCode::ArrowLeft | KeyCode::ArrowRight => self.toggle(self.selected),
            _ => {}
        }
        PanelAction::None
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            Some(IMPORT_TOKEN) => self.primary(),
            Some(TICK_TOKEN) => {
                self.toggle(self.selected);
                PanelAction::None
            }
            Some(row) if usize::from(row) < self.rows.len() => {
                self.selected = usize::from(row);
                self.toggle(self.selected);
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }

    /// The value a row shows for `item`.
    fn value(found: &Found, item: Item) -> String {
        match item {
            Item::Name => plain(found.name.as_deref().unwrap_or_default()),
            Item::Model => {
                let model = found.model.as_deref().unwrap_or_default();
                if found.colors.iter().any(Option::is_some) {
                    format!("{model} + tint")
                } else {
                    model.to_owned()
                }
            }
            Item::Fov => found.fov.clone().unwrap_or_default(),
            Item::Binds => format!(
                "{} key{}, {}",
                found.binds.len(),
                if found.binds.len() == 1 { "" } else { "s" },
                if found.whole_table {
                    "replace yours"
                } else {
                    "added to yours"
                }
            ),
        }
    }

    /// Draw the page over the whole frame; text other overlays appended earlier
    /// this frame is dropped rather than shown through.
    pub(crate) fn append(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        vertices.clear();
        let layout = FormLayout::new(viewport);
        let s = layout.scale;
        self.ui.begin_hero(viewport, 1.0, Scrim::Wide);
        let subtitle = match &self.state {
            State::Loaded(_) => format!("From {}. Tick what to copy into SJK.", self.file),
            _ if self.file.is_empty() => {
                "Bring your name, model, field of view and keys from another client.".to_owned()
            }
            _ => format!("From {}.", self.file),
        };
        self.ui
            .form_header(&layout, "SJK   /   FIRST SETUP", "IMPORT", &subtitle);
        let theme = self.ui.theme();
        let mut hints: Vec<(&str, &str, u16)> = Vec::new();
        match &self.state {
            State::Loaded(found) => {
                for (row, (item, ticked)) in self.rows.iter().enumerate() {
                    let rect = layout.row_rect(row);
                    let selected = row == self.selected;
                    self.ui.form_row_frame(rect, row as u16, selected, s);
                    self.ui.form_label(rect, item.label(), selected, s);
                    let zone = layout.value_zone(rect);
                    let pill = Rect::new(
                        zone.right() - 44.0 * s,
                        rect.y + 14.0 * s,
                        44.0 * s,
                        22.0 * s,
                    );
                    self.ui.toggle_pill(pill, *ticked, theme.accent);
                    let value = Self::value(found, *item);
                    let color = self.ui.form_value_color(selected);
                    self.ui.form_value(
                        &value,
                        Rect::new(zone.x, rect.y, zone.width - 60.0 * s, rect.height),
                        color,
                        s,
                    );
                }
                if found.unknown_keys > 0 {
                    let rect = layout.row_rect(self.rows.len());
                    let note = format!(
                        "{} binding{} use keys SJK does not know and are left out.",
                        found.unknown_keys,
                        if found.unknown_keys == 1 { "" } else { "s" }
                    );
                    self.ui.text(
                        &note,
                        Rect::new(rect.x, rect.y + 16.0 * s, rect.width, 20.0 * s),
                        14.0 * s,
                        theme.muted,
                        FontWeight::Regular,
                        0.2 * s,
                    );
                }
                if self.chosen().is_some() {
                    hints.push(("ENTER", "Import", IMPORT_TOKEN));
                }
                hints.push(("SPACE", "Tick", TICK_TOKEN));
                hints.push(("ESC", "Cancel", BACK_TOKEN));
            }
            State::Waiting => {
                self.card(
                    &layout,
                    "Drop a .cfg file on this window",
                    &[
                        "Drag your config from the other client's folder onto SJK, for example",
                        "GameData/base/jampconfig.cfg, or the one in your mod's folder.",
                        "Or type: firstsetup import \"C:/path/to/jampconfig.cfg\"",
                    ],
                );
                hints.push(("ESC", "Close", BACK_TOKEN));
            }
            State::Failed(reason) => {
                let reason = reason.clone();
                self.card(
                    &layout,
                    "Nothing imported",
                    &[&reason, "Drop another .cfg file to try again."],
                );
                hints.push(("ESC", "Close", BACK_TOKEN));
            }
            State::Done(lines) => {
                let lines = lines.clone();
                let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
                self.card(&layout, "Imported", &lines);
                hints.push(("ENTER", "Done", IMPORT_TOKEN));
            }
        }
        self.ui.form_footer_actions(&layout, &hints);
        self.ui.end_hero();
        self.ui.finish(self.selected as u16);
        self.ui.append_text(vertices, font, viewport);
    }

    /// A headline and a few lines in a card under the header.
    fn card(&mut self, layout: &FormLayout, headline: &str, lines: &[&str]) {
        let s = layout.scale;
        let theme = self.ui.theme();
        let viewport = layout.viewport;
        let x = layout.margin;
        let pad = 28.0 * s;
        let line_height = 24.0 * s;
        let card = Rect::new(
            x,
            layout.rows_y,
            (viewport[0] - x * 2.0).min(820.0 * s),
            pad * 2.0 + 52.0 * s + line_height * lines.len() as f32,
        );
        self.ui.panel(card);
        let width = card.width - pad * 2.0;
        self.ui.text(
            headline,
            Rect::new(card.x + pad, card.y + pad, width, 36.0 * s),
            28.0 * s,
            theme.foreground,
            FontWeight::Semibold,
            0.0,
        );
        for (index, line) in lines.iter().enumerate() {
            self.ui.text(
                line,
                Rect::new(
                    card.x + pad,
                    card.y + pad + 52.0 * s + line_height * index as f32,
                    width,
                    line_height,
                ),
                16.0 * s,
                theme.muted,
                FontWeight::Regular,
                0.2 * s,
            );
        }
    }
}

/// `text` without its `^0`..`^9` colour codes.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '^' && chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
            continue;
        }
        out.push(character);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_import::parse;

    fn loaded(text: &str) -> Panel {
        let mut panel = Panel::new();
        panel.open(true, "jampconfig.cfg".to_owned(), Some(Ok(parse(text))));
        panel
    }

    #[test]
    fn every_part_found_starts_ticked_and_untick_leaves_it_out() {
        let mut panel = loaded("seta name Sol\nbind w +forward\n");
        let rows: Vec<Item> = panel.rows.iter().map(|(item, _)| *item).collect();
        assert_eq!(rows, [Item::Name, Item::Binds]);
        assert_eq!(panel.primary(), PanelAction::Import);
        panel.toggle(0);
        assert_eq!(panel.chosen().unwrap().1, [Item::Binds]);
        panel.toggle(1);
        assert!(panel.chosen().is_none());
        assert_eq!(panel.primary(), PanelAction::None);
    }

    #[test]
    fn a_failed_or_finished_page_closes_on_enter() {
        let mut panel = Panel::new();
        panel.open(false, "notes.txt".to_owned(), Some(Err("no".to_owned())));
        assert_eq!(panel.primary(), PanelAction::Close);
        let mut panel = loaded("seta name Sol\n");
        panel.finish(vec!["Name: Sol".to_owned()]);
        assert!(panel.chosen().is_none());
        assert_eq!(panel.primary(), PanelAction::Close);
    }

    #[test]
    fn a_second_drop_keeps_who_opened_the_console() {
        let mut panel = loaded("seta name Sol\n");
        panel.open(
            false,
            "other.cfg".to_owned(),
            Some(Ok(parse("seta cg_fov 100"))),
        );
        assert!(panel.close());
    }

    #[test]
    fn names_show_without_colour_codes() {
        assert_eq!(plain("^1Sol^7Fox"), "SolFox");
        assert_eq!(plain("a^b ^^9"), "a^b ^");
    }
}
