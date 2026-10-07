//! Importing a player's setup from another client: a `.cfg` file (Jedi Academy's
//! `jampconfig.cfg`, EternalJK's or JA+'s) dropped on the window, or named to
//! `firstsetup import`, gives the player's name, model, field of view and key
//! bindings. The Import page (`config_import_panel.rs`) lists what was found and
//! copies the ticked parts; nothing else in the file is read or run.

use std::path::Path;

/// Files larger than this are not configs.
const MAX_BYTES: u64 = 1024 * 1024;

/// What the Import page can copy, in its order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Item {
    Name,
    /// `model`, with the `char_color_*` tint when the file sets it.
    Model,
    Fov,
    Binds,
}

impl Item {
    pub(crate) const ALL: [Self; 4] = [Self::Name, Self::Model, Self::Fov, Self::Binds];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Model => "Model",
            Self::Fov => "Field of view",
            Self::Binds => "Key bindings",
        }
    }
}

/// The importable parts of one config file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Found {
    pub(crate) name: Option<String>,
    pub(crate) model: Option<String>,
    /// `char_color_red`, `_green` and `_blue`, copied with the model.
    pub(crate) colors: [Option<String>; 3],
    pub(crate) fov: Option<String>,
    /// Key and script of each binding, in file order, one per key.
    pub(crate) binds: Vec<(String, String)>,
    /// The file clears every binding first (`unbindall`), as a client's saved
    /// config does: its bindings are a whole table that replaces the player's.
    pub(crate) whole_table: bool,
    /// Bindings left out because SJK does not know their key.
    pub(crate) unknown_keys: usize,
}

/// The `char_color_*` cvars, in [`Found::colors`] order.
pub(crate) const COLOR_CVARS: [&str; 3] = ["char_color_red", "char_color_green", "char_color_blue"];

impl Found {
    pub(crate) fn has(&self, item: Item) -> bool {
        match item {
            Item::Name => self.name.is_some(),
            Item::Model => self.model.is_some(),
            Item::Fov => self.fov.is_some(),
            Item::Binds => !self.binds.is_empty(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        !Item::ALL.into_iter().any(|item| self.has(item))
    }

    fn set_cvar(&mut self, name: &str, value: &str) {
        let value = value.to_owned();
        match name.to_ascii_lowercase().as_str() {
            "name" if !value.trim().is_empty() => self.name = Some(value),
            "model" if !value.trim().is_empty() => self.model = Some(value),
            "cg_fov" if value.trim().parse::<f64>().is_ok_and(|fov| fov > 0.0) => {
                self.fov = Some(value.trim().to_owned());
            }
            other => {
                if let Some(slot) = COLOR_CVARS.iter().position(|cvar| *cvar == other)
                    && value.trim().parse::<i64>().is_ok()
                {
                    self.colors[slot] = Some(value.trim().to_owned());
                }
            }
        }
    }

    fn bind(&mut self, key: &str, script: String) {
        let Some(key) = sjk_shell::key_names::canonical_key(key) else {
            self.unknown_keys += 1;
            return;
        };
        self.binds.retain(|(old, _)| !old.eq_ignore_ascii_case(key));
        if !script.is_empty() {
            self.binds.push((key.to_owned(), script));
        }
    }
}

/// Why a file could not be imported, as the page says it.
pub(crate) fn read(path: &Path) -> Result<Found, String> {
    let is_cfg = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("cfg"));
    if !is_cfg {
        return Err("Only .cfg files can be imported.".to_owned());
    }
    let size = std::fs::metadata(path)
        .map_err(|error| format!("Could not read it: {error}."))?
        .len();
    if size > MAX_BYTES {
        return Err("It is too large to be a config.".to_owned());
    }
    let bytes = std::fs::read(path).map_err(|error| format!("Could not read it: {error}."))?;
    let found = parse(&sjk_shell::decode_config_text(bytes));
    if found.is_empty() {
        return Err("No name, model, field of view or key bindings were found in it.".to_owned());
    }
    Ok(found)
}

/// The importable parts of a config's text. Only `seta`/`set` (and a bare
/// `name`, `model` or `cg_fov`), `bind`, `unbind` and `unbindall` are read.
pub(crate) fn parse(text: &str) -> Found {
    let mut found = Found::default();
    for line in text.lines() {
        let Ok(commands) = sjk_shell::split_commands(without_comment(line)) else {
            continue;
        };
        for command in commands {
            let Ok(tokens) = sjk_shell::tokenize(&command) else {
                continue;
            };
            let Some((verb, rest)) = tokens.split_first() else {
                continue;
            };
            match (verb.to_ascii_lowercase().as_str(), rest) {
                ("seta" | "set" | "sets" | "setu", [name, value, ..]) => {
                    found.set_cvar(name, value);
                }
                ("name" | "model" | "cg_fov", [value]) => found.set_cvar(verb, value),
                ("bind", [key, script @ ..]) if !script.is_empty() => {
                    found.bind(key, script.join(" "));
                }
                ("unbind", [key]) => found.bind(key, String::new()),
                ("unbindall", []) => {
                    found.binds.clear();
                    found.whole_table = true;
                }
                _ => {}
            }
        }
    }
    found
}

/// `line` up to a `//` comment outside quotes.
fn without_comment(line: &str) -> &str {
    let mut quoted = false;
    let bytes = line.as_bytes();
    for (index, &byte) in bytes.iter().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'/' if !quoted && bytes.get(index + 1) == Some(&b'/') => return &line[..index],
            _ => {}
        }
    }
    line
}

impl crate::GpuState {
    /// A file was dropped on the window: show the Import page with what it holds.
    pub(crate) fn file_dropped(&mut self, path: &Path) {
        if let Some(console) = &mut self.console {
            console.open_config_import(Some(path));
        }
        self.sync_cursor_policy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_config_gives_name_model_fov_and_the_whole_bind_table() {
        let found = parse(
            "// generated by Jedi Academy, do not modify\n\
             unbindall\n\
             bind TAB \"+scores\"\n\
             bind w \"+forward\"\n\
             bind MOUSE1 \"+attack\"\n\
             seta name \"^1Sol^7Fox\"\n\
             seta model \"kyle/default\"\n\
             seta char_color_red \"200\"\n\
             seta cg_fov \"100\"\n\
             seta sensitivity \"3\"\n",
        );
        assert_eq!(found.name.as_deref(), Some("^1Sol^7Fox"));
        assert_eq!(found.model.as_deref(), Some("kyle/default"));
        assert_eq!(found.colors[0].as_deref(), Some("200"));
        assert_eq!(found.colors[1], None);
        assert_eq!(found.fov.as_deref(), Some("100"));
        assert!(found.whole_table);
        assert_eq!(found.binds.len(), 3);
        assert_eq!(found.binds[1].1, "+forward");
    }

    #[test]
    fn binds_keep_the_last_script_per_key_and_join_unquoted_words() {
        let found = parse(
            "bind x say hello there // greet\n\
             bind X \"say bye; wave\"\n\
             bind f \"+use\"\n\
             unbind f\n\
             bind NOSUCHKEY \"+attack\"\n",
        );
        assert!(!found.whole_table);
        assert_eq!(found.binds.len(), 1);
        assert_eq!(found.binds[0].1, "say bye; wave");
        assert_eq!(found.unknown_keys, 1);
    }

    #[test]
    fn comments_bare_cvars_and_bad_values() {
        let found = parse(
            "name \"a // b\" // not this\n\
             cg_fov wide\n\
             seta model \"\"\n\
             set cg_fov 110; seta snaps 40\n",
        );
        assert_eq!(found.name.as_deref(), Some("a // b"));
        assert_eq!(found.model, None);
        assert_eq!(found.fov.as_deref(), Some("110"));
    }

    #[test]
    fn a_config_with_none_of_the_parts_is_empty() {
        assert!(parse("seta snaps 40\nexec other.cfg\n").is_empty());
    }

    #[test]
    fn only_cfg_files_are_read() {
        let directory = tempfile::tempdir().unwrap();
        let text = directory.path().join("notes.txt");
        std::fs::write(&text, "seta name x").unwrap();
        assert!(read(&text).unwrap_err().contains(".cfg"));
        let config = directory.path().join("jampconfig.CFG");
        std::fs::write(&config, "seta name x").unwrap();
        assert_eq!(read(&config).unwrap().name.as_deref(), Some("x"));
        let empty = directory.path().join("autoexec.cfg");
        std::fs::write(&empty, "seta snaps 40").unwrap();
        assert!(read(&empty).is_err());
    }
}
