//! Console side of the Import page (see `config_import_panel.rs`): opening it on a
//! dropped file, copying the ticked parts into the profile, and routing keys and
//! pointer events to it while it is open, as for the Update page.

use super::config_import_panel::PanelAction;
use super::*;
use crate::config_import::{self, COLOR_CVARS, Found, Item};
use sjk_ui::InputEvent;
use std::path::Path;

impl ViewerConsole {
    /// Show the Import page with what `path` holds, or, without a file, how to
    /// give one; opens the console if needed.
    pub(crate) fn open_config_import(&mut self, path: Option<&Path>) {
        let owns_console = !self.open;
        if !self.open {
            self.set_open(true);
        }
        self.browser.close();
        self.debug_panel.close();
        self.changelog.close();
        self.credits.close();
        self.update_panel.close();
        self.identity_panel.close();
        self.dead_key.settle();
        let file = path
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(path) = path {
            self.shell.push_log(format!("Import: {}", path.display()));
        }
        self.config_import
            .open(owns_console, file, path.map(config_import::read));
    }

    fn close_config_import(&mut self) {
        if self.config_import.close() {
            self.set_open(false);
        }
    }

    /// Copy the chosen parts of `found` into the profile; one line per part
    /// for the page.
    fn import_config(&mut self, found: &Found, items: &[Item]) -> Vec<String> {
        let mut lines = Vec::with_capacity(items.len());
        for item in items {
            match item {
                Item::Name => {
                    if let Some(name) = &found.name
                        && self.set_cvar("name", name)
                    {
                        lines.push(format!("Name: {name}"));
                    }
                }
                Item::Model => {
                    if let Some(model) = &found.model
                        && self.set_cvar("model", model)
                    {
                        for (cvar, value) in COLOR_CVARS.iter().zip(&found.colors) {
                            if let Some(value) = value {
                                self.set_cvar(cvar, value);
                            }
                        }
                        lines.push(format!("Model: {model}"));
                    }
                }
                Item::Fov => {
                    if let Some(fov) = &found.fov
                        && self.set_cvar("cg_fov", fov)
                    {
                        lines.push(format!("Field of view: {fov}"));
                    }
                }
                Item::Binds => {
                    let count = self.import_binds(found);
                    lines.push(format!(
                        "Key bindings: {count} key{}{}",
                        if count == 1 { "" } else { "s" },
                        if found.whole_table {
                            ", the rest unbound"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }
        for line in &lines {
            self.shell.push_log(format!("Imported {line}"));
        }
        lines
    }

    /// Bind the file's keys; returns how many were bound. A whole table (the
    /// file starts with `unbindall`) replaces the player's, keeping the keys the
    /// editor locks and giving SJK's own actions their default keys where the
    /// file left those free; a partial file only adds its keys.
    fn import_binds(&mut self, found: &Found) -> usize {
        let mut binds = self.shell.binds.clone();
        if found.whole_table {
            let locked: Vec<(String, String)> = binds
                .iter()
                .filter(|(key, _)| keybind_editor::is_locked_key(key))
                .map(|(key, command)| (key.to_owned(), command.to_owned()))
                .collect();
            binds.clear();
            for (key, command) in locked {
                let _ = binds.bind(&key, command);
            }
        }
        let mut count = 0;
        for (key, script) in &found.binds {
            if !keybind_editor::is_locked_key(key) && binds.bind(key, script.clone()).is_ok() {
                count += 1;
            }
        }
        if found.whole_table {
            keybind_editor::migrate_missing_defaults(&mut binds);
        }
        self.shell.binds = binds;
        self.persist();
        count
    }

    fn config_import_action(&mut self, action: PanelAction) {
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.close_config_import(),
            PanelAction::Import => {
                let chosen = self
                    .config_import
                    .chosen()
                    .map(|(found, items)| (found.clone(), items));
                let lines = match chosen {
                    Some((found, items)) => self.import_config(&found, &items),
                    None => Vec::new(),
                };
                self.config_import.finish(lines);
            }
        }
    }

    /// Give a pressed key to the open page; false when the page is closed.
    pub(super) fn config_import_key(&mut self, event: &KeyEvent) -> bool {
        if !self.config_import.is_open() {
            return false;
        }
        let action = self.config_import.handle_key(event);
        self.config_import_action(action);
        true
    }

    /// Give a pointer event to the open page; false when the page is closed.
    pub(super) fn config_import_pointer(&mut self, event: InputEvent) -> bool {
        if !self.config_import.is_open() {
            return false;
        }
        let action = self.config_import.handle_pointer(event);
        self.config_import_action(action);
        true
    }

    /// Draw the page in place of the console; false when it is not shown.
    pub(super) fn append_config_import(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) -> bool {
        if !(self.open && self.config_import.is_open()) {
            return false;
        }
        self.config_import.append(vertices, font, viewport);
        true
    }

    /// The page's draw list while it is shown.
    pub(super) fn config_import_draw_list(&self) -> Option<&sjk_ui::DrawList> {
        (self.open && self.config_import.is_open()).then(|| self.config_import.draw_list())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_import::parse;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    #[test]
    fn the_chosen_parts_are_copied_and_the_others_left_alone() {
        let (_directory, mut console) = console();
        let found = parse(
            "seta name \"^1Sol\"\nseta model \"tavion/default\"\nseta char_color_red 10\n\
             seta cg_fov 105\n",
        );
        let lines = console.import_config(&found, &[Item::Name, Item::Model]);
        assert_eq!(lines.len(), 2);
        assert_eq!(console.text_cvar("name").unwrap(), "^1Sol");
        assert_eq!(console.text_cvar("model").unwrap(), "tavion/default");
        assert_eq!(console.integer_cvar("char_color_red"), Some(10));
        assert_ne!(console.float_cvar("cg_fov"), Some(105.0));
    }

    #[test]
    fn a_whole_table_replaces_the_binds_but_keeps_locked_keys_and_free_defaults() {
        let (_directory, mut console) = console();
        let escape = console.shell.binds.get("ESCAPE").map(str::to_owned);
        let found = parse("unbindall\nbind w \"+back\"\nbind ESCAPE \"quit\"\n");
        console.import_config(&found, &[Item::Binds]);
        assert_eq!(console.shell.binds.get("w"), Some("+back"));
        assert_eq!(console.shell.binds.get("ESCAPE").map(str::to_owned), escape);
        // +forward lost its key to the file, so it is not given one back on W.
        assert!(console.keys_for_command("+forward").is_empty());
    }

    #[test]
    fn a_partial_file_only_adds_its_keys() {
        let (_directory, mut console) = console();
        let before = console.shell.binds.iter().count();
        let found = parse("bind F11 \"say hi\"\n");
        console.import_config(&found, &[Item::Binds]);
        assert_eq!(console.shell.binds.get("F11"), Some("say hi"));
        assert!(console.shell.binds.iter().count() >= before);
    }
}
