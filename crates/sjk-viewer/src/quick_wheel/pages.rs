//! The quick wheel's pages: what each holds and in what order, kept in
//! `wheel.json` in the profile folder (`GameData/SJK/`) once the player changes
//! them in Settings. Without the file the wheel has its two first pages, General
//! and Weather. The file is JSON rather than cvars because a page is a list and a
//! custom choice a free console command (quotes, semicolons), which a `config.cfg`
//! line would have to escape and whose length a cvar would cap.
//!
//! ```json
//! { "version": 1, "pages": [
//!   { "id": "general", "name": "General", "choices": ["third_person", "hud"] },
//!   { "id": "duels", "name": "Duels", "choices": ["duel", {"name": "Ready", "command": "ready"}] }
//! ] }
//! ```
//!
//! A choice is an action of the catalogue by its id ([`super::catalog::ACTIONS`]),
//! or an object naming a console command. A page's `id` is what `+wheel <id>`
//! opens it with; it is made from the page's first name and kept when the page
//! is renamed, so a bind keeps working.

use super::catalog::{self, ACTIONS, State};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// The file, in the profile folder.
pub(crate) const FILE: &str = "wheel.json";
/// Most pages the wheel has: the dots under its middle stay a short row.
pub(crate) const MAX_PAGES: usize = 8;
/// Most choices a page holds: ten round icons fill the ring with a gap between
/// each, the highlighted one grown, at any window size.
pub(crate) const MAX_CHOICES: usize = 10;
/// Longest page name, and longest name of a custom choice, in characters.
pub(crate) const NAME_CHARS: usize = 20;
pub(crate) const LABEL_CHARS: usize = 24;
/// Longest custom console command, in characters.
pub(crate) const COMMAND_CHARS: usize = 200;

/// One choice of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    /// An action of the catalogue, by its index in [`ACTIONS`].
    Action(usize),
    /// A console command the player named.
    Custom { label: String, command: String },
}

impl Slot {
    /// Its name on the ring.
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Action(index) => ACTIONS[*index].label,
            Self::Custom { label, .. } => label,
        }
    }

    /// The console command it runs.
    pub(crate) fn command(&self) -> &str {
        match self {
            Self::Action(index) => ACTIONS[*index].command,
            Self::Custom { command, .. } => command,
        }
    }

    /// Its picture's index in [`catalog::ICONS`]; a custom choice has none.
    pub(crate) fn icon(&self) -> Option<usize> {
        match self {
            Self::Action(index) => ACTIONS[*index].icon.and_then(catalog::icon_index),
            Self::Custom { .. } => None,
        }
    }

    /// When it counts as in effect.
    pub(crate) fn state(&self) -> State {
        match self {
            Self::Action(index) => ACTIONS[*index].state,
            Self::Custom { .. } => State::None,
        }
    }
}

/// One page of the wheel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Page {
    /// What `+wheel <id>` opens it with: lower-case letters and digits.
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) choices: Vec<Slot>,
}

impl Page {
    fn of(id: &str, name: &str, actions: &[&str]) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            choices: actions
                .iter()
                .filter_map(|id| catalog::action_index(id))
                .map(Slot::Action)
                .collect(),
        }
    }
}

/// The pages the wheel has with nothing saved.
pub(crate) fn defaults() -> Vec<Page> {
    vec![
        Page::of("general", "General", &catalog::GENERAL),
        Page::of("weather", "Weather", &catalog::WEATHER),
    ]
}

/// The pages and where they are kept.
#[derive(Debug)]
pub(crate) struct WheelPages {
    path: Option<PathBuf>,
    pages: Vec<Page>,
}

impl WheelPages {
    /// The pages kept in `directory` (the profile folder), or the defaults when
    /// it has none or the file cannot be read.
    pub(crate) fn load(directory: &Path) -> Self {
        let path = directory.join(FILE);
        let pages = match std::fs::read(&path) {
            Ok(bytes) => parse(&bytes).unwrap_or_else(|| {
                crate::log::progress(format_args!(
                    "warning: {} is not a quick wheel file; using the default pages",
                    path.display()
                ));
                defaults()
            }),
            Err(_) => defaults(),
        };
        Self {
            path: Some(path),
            pages,
        }
    }

    /// The defaults, kept nowhere (tests).
    #[cfg(test)]
    pub(crate) fn unsaved() -> Self {
        Self {
            path: None,
            pages: defaults(),
        }
    }

    pub(crate) fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// Whether the pages are the defaults.
    pub(crate) fn is_default(&self) -> bool {
        self.pages == defaults()
    }

    /// The page `name` names: its id, else its name, ignoring case and spaces.
    pub(crate) fn find(&self, name: &str) -> Option<usize> {
        let wanted = squeeze(name);
        if wanted.is_empty() {
            return None;
        }
        self.pages
            .iter()
            .position(|page| page.id == wanted)
            .or_else(|| {
                self.pages
                    .iter()
                    .position(|page| squeeze(&page.name) == wanted)
            })
    }

    /// Add a page named `name` (a fresh "Page n" when blank) at the end; its
    /// index, or `None` when the wheel has [`MAX_PAGES`].
    pub(crate) fn add_page(&mut self, name: &str) -> Option<usize> {
        if self.pages.len() >= MAX_PAGES {
            return None;
        }
        let name = match clean(name, NAME_CHARS) {
            name if name.is_empty() => self.fresh_name(),
            name => name,
        };
        let id = self.unique_id(&name, None);
        self.pages.push(Page {
            id,
            name,
            choices: Vec::new(),
        });
        self.save();
        Some(self.pages.len() - 1)
    }

    /// The name the next page added gets unless it is given one.
    pub(crate) fn fresh_name(&self) -> String {
        (self.pages.len() + 1..)
            .map(|number| format!("Page {number}"))
            .find(|name| self.pages.iter().all(|page| page.name != *name))
            .unwrap_or_default()
    }

    /// Rename page `page` (a blank name keeps the old one). A page named for the
    /// first time (`first`, just added) takes its id from the name too.
    pub(crate) fn rename_page(&mut self, page: usize, name: &str, first: bool) {
        let name = clean(name, NAME_CHARS);
        if name.is_empty() || page >= self.pages.len() {
            return;
        }
        if first {
            self.pages[page].id = self.unique_id(&name, Some(page));
        }
        self.pages[page].name = name;
        self.save();
    }

    /// Remove page `page`, unless it is the last one left.
    pub(crate) fn remove_page(&mut self, page: usize) -> bool {
        if self.pages.len() <= 1 || page >= self.pages.len() {
            return false;
        }
        self.pages.remove(page);
        self.save();
        true
    }

    /// Move page `page` one place up (`direction` -1) or down (1); its new index.
    pub(crate) fn move_page(&mut self, page: usize, direction: i32) -> usize {
        let Some(to) = step(page, direction, self.pages.len()) else {
            return page;
        };
        self.pages.swap(page, to);
        self.save();
        to
    }

    /// Put `slot` on page `page`: in place of choice `at`, or after the last
    /// choice when `at` is `None` (unless the page is full). The choice's index.
    pub(crate) fn set_choice(
        &mut self,
        page: usize,
        at: Option<usize>,
        slot: Slot,
    ) -> Option<usize> {
        let slot = match slot {
            Slot::Custom { label, command } => {
                let (label, command) = (clean(&label, LABEL_CHARS), clean(&command, COMMAND_CHARS));
                if label.is_empty() || command.is_empty() {
                    return None;
                }
                Slot::Custom { label, command }
            }
            Slot::Action(index) if index < ACTIONS.len() => Slot::Action(index),
            Slot::Action(_) => return None,
        };
        let choices = &mut self.pages.get_mut(page)?.choices;
        let index = match at {
            Some(at) if at < choices.len() => {
                choices[at] = slot;
                at
            }
            _ if choices.len() < MAX_CHOICES => {
                choices.push(slot);
                choices.len() - 1
            }
            _ => return None,
        };
        self.save();
        Some(index)
    }

    /// Remove choice `at` of page `page`.
    pub(crate) fn remove_choice(&mut self, page: usize, at: usize) {
        let Some(choices) = self.pages.get_mut(page).map(|page| &mut page.choices) else {
            return;
        };
        if at < choices.len() {
            choices.remove(at);
            self.save();
        }
    }

    /// Move choice `at` of page `page` one place earlier (-1) or later (1) round
    /// the ring; its new index.
    pub(crate) fn move_choice(&mut self, page: usize, at: usize, direction: i32) -> usize {
        let Some(choices) = self.pages.get_mut(page).map(|page| &mut page.choices) else {
            return at;
        };
        let Some(to) = step(at, direction, choices.len()) else {
            return at;
        };
        choices.swap(at, to);
        self.save();
        to
    }

    /// Back to the two first pages: the file is removed, so later versions'
    /// defaults reach this profile too.
    pub(crate) fn restore_defaults(&mut self) {
        self.pages = defaults();
        if let Some(path) = &self.path
            && path.exists()
            && let Err(error) = std::fs::remove_file(path)
        {
            crate::log::progress(format_args!(
                "warning: {} not removed: {error}",
                path.display()
            ));
            self.save();
        }
    }

    /// `name` as an id no other page than `except` has: its letters and digits in
    /// lower case, starting with a letter, numbered when taken.
    fn unique_id(&self, name: &str, except: Option<usize>) -> String {
        let mut base = squeeze(name);
        if !base.starts_with(|c: char| c.is_ascii_lowercase()) {
            base.insert_str(0, "page");
        }
        let taken = |id: &str| {
            self.pages
                .iter()
                .enumerate()
                .any(|(index, page)| Some(index) != except && page.id == id)
        };
        if !taken(&base) {
            return base;
        }
        (2..)
            .map(|number| format!("{base}{number}"))
            .find(|id| !taken(id))
            .unwrap_or(base)
    }

    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        if let Err(error) = write(path, &self.pages) {
            crate::log::progress(format_args!(
                "warning: quick wheel pages not saved to {}: {error}",
                path.display()
            ));
        }
    }
}

/// `text`'s ASCII letters and digits in lower case: an id, and how `+wheel`
/// compares a name.
fn squeeze(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// `text` trimmed, without control characters, at most `chars` long.
pub(crate) fn clean(text: &str, chars: usize) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(chars)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// The index `direction` places from `index` among `count`, if there is one.
fn step(index: usize, direction: i32, count: usize) -> Option<usize> {
    let to = index as i64 + i64::from(direction.signum());
    (index < count && (0..count as i64).contains(&to) && to != index as i64).then_some(to as usize)
}

/// The pages a `wheel.json` holds; `None` when it is not one.
fn parse(bytes: &[u8]) -> Option<Vec<Page>> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let items = value.get("pages")?.as_array()?;
    let mut pages = WheelPages {
        path: None,
        pages: Vec::new(),
    };
    for item in items.iter().take(MAX_PAGES) {
        let Some(name) = item
            .get("name")
            .and_then(Value::as_str)
            .map(|name| clean(name, NAME_CHARS))
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        let choices = item
            .get("choices")
            .and_then(Value::as_array)
            .map(|choices| choices.iter().filter_map(slot).take(MAX_CHOICES).collect())
            .unwrap_or_default();
        let wanted = item.get("id").and_then(Value::as_str).unwrap_or(&name);
        let id = pages.unique_id(wanted, None);
        pages.pages.push(Page { id, name, choices });
    }
    (!pages.pages.is_empty()).then_some(pages.pages)
}

/// A choice as `wheel.json` writes it; `None` for an unknown action or an
/// incomplete custom one.
fn slot(value: &Value) -> Option<Slot> {
    if let Some(id) = value.as_str() {
        return catalog::action_index(id).map(Slot::Action);
    }
    let text = |key: &str, chars: usize| {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(|text| clean(text, chars))
            .filter(|text| !text.is_empty())
    };
    Some(Slot::Custom {
        label: text("name", LABEL_CHARS)?,
        command: text("command", COMMAND_CHARS)?,
    })
}

fn write(path: &Path, pages: &[Page]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pages: Vec<Value> = pages
        .iter()
        .map(|page| {
            let choices: Vec<Value> = page
                .choices
                .iter()
                .map(|choice| match choice {
                    Slot::Action(index) => json!(ACTIONS[*index].id),
                    Slot::Custom { label, command } => json!({"name": label, "command": command}),
                })
                .collect();
            json!({"id": page.id, "name": page.name, "choices": choices})
        })
        .collect();
    let bytes = serde_json::to_vec_pretty(&json!({"version": 1, "pages": pages}))
        .map_err(std::io::Error::other)?;
    // Written whole beside the file, then moved over it, so a crash never
    // leaves half a file.
    let partial = path.with_extension("json.partial");
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(label: &str, command: &str) -> Slot {
        Slot::Custom {
            label: label.to_owned(),
            command: command.to_owned(),
        }
    }

    #[test]
    fn without_a_file_the_wheel_has_general_and_weather() {
        let directory = tempfile::tempdir().unwrap();
        let pages = WheelPages::load(directory.path());
        assert!(pages.is_default());
        let names: Vec<_> = pages
            .pages()
            .iter()
            .map(|page| page.name.as_str())
            .collect();
        assert_eq!(names, ["General", "Weather"]);
        assert_eq!(pages.pages()[0].choices.len(), 8);
        assert_eq!(pages.pages()[1].choices[2].command(), "r_weatherForce 2");
        assert_eq!(pages.pages()[0].choices[0].label(), "Third person");
        // Nothing is written until something changes.
        assert!(!directory.path().join(FILE).exists());
        // Binds name a page by id or name, whatever the case and spacing.
        assert_eq!(pages.find("weather"), Some(1));
        assert_eq!(pages.find(" General "), Some(0));
        assert_eq!(pages.find("hail"), None);
        assert_eq!(pages.find(""), None);
    }

    #[test]
    fn edits_are_saved_and_read_back_as_made() {
        let directory = tempfile::tempdir().unwrap();
        let mut pages = WheelPages::load(directory.path());
        let duels = pages.add_page("  Duels ").unwrap();
        assert_eq!(pages.pages()[duels].id, "duels");
        let duel = catalog::action_index("duel").unwrap();
        assert_eq!(pages.set_choice(duels, None, Slot::Action(duel)), Some(0));
        assert_eq!(
            pages.set_choice(duels, None, custom("Ready", "say \"ready\"; ready")),
            Some(1)
        );
        // A rename keeps the id a bind opens it with.
        pages.rename_page(duels, "1v1 duels", false);
        assert_eq!(pages.pages()[duels].id, "duels");
        assert_eq!(pages.find("duels"), Some(duels));
        assert_eq!(pages.find("1v1 Duels"), Some(duels));
        pages.move_page(duels, -1);
        assert_eq!(pages.pages()[1].name, "1v1 duels");
        pages.move_choice(1, 1, -1);
        assert_eq!(pages.pages()[1].choices[0].label(), "Ready");
        let saved = WheelPages::load(directory.path());
        assert_eq!(saved.pages(), pages.pages());
        assert!(!saved.is_default());
        // Back to the defaults: the file goes.
        pages.restore_defaults();
        assert!(pages.is_default());
        assert!(!directory.path().join(FILE).exists());
        assert!(WheelPages::load(directory.path()).is_default());
    }

    #[test]
    fn limits_hold_and_the_last_page_stays() {
        let mut pages = WheelPages::unsaved();
        // Full pages take no more; a full wheel no more pages.
        assert_eq!(pages.set_choice(0, None, Slot::Action(0)), Some(8));
        assert_eq!(pages.set_choice(0, None, Slot::Action(0)), Some(9));
        assert_eq!(pages.set_choice(0, None, Slot::Action(0)), None);
        // In place of a choice still works on a full page.
        assert_eq!(pages.set_choice(0, Some(9), Slot::Action(2)), Some(9));
        while pages.add_page("").is_some() {}
        assert_eq!(pages.pages().len(), MAX_PAGES);
        assert_eq!(pages.pages()[2].name, "Page 3");
        assert_eq!(pages.pages()[2].id, "page3");
        // Blank or unfinished custom choices are refused, long ones cut.
        assert_eq!(pages.set_choice(2, None, custom(" ", "kill")), None);
        assert_eq!(pages.set_choice(2, None, custom("Kill", "")), None);
        let long = "x".repeat(40);
        let at = pages.set_choice(2, None, custom(&long, "kill")).unwrap();
        assert_eq!(pages.pages()[2].choices[at].label().len(), LABEL_CHARS);
        while pages.remove_page(0) {}
        assert_eq!(pages.pages().len(), 1);
        // Moves past the ends stay put.
        assert_eq!(pages.move_page(0, -1), 0);
        assert_eq!(pages.move_choice(0, 0, -1), 0);
    }

    #[test]
    fn a_first_name_gives_the_page_its_id_and_ids_stay_unique() {
        let mut pages = WheelPages::unsaved();
        let page = pages.add_page("").unwrap();
        pages.rename_page(page, "Weather", true);
        assert_eq!(pages.pages()[page].id, "weather2");
        let other = pages.add_page("42").unwrap();
        assert_eq!(pages.pages()[other].id, "page42");
        // A blank rename keeps the name.
        pages.rename_page(other, "   ", false);
        assert_eq!(pages.pages()[other].name, "42");
    }

    #[test]
    fn a_hand_written_or_broken_file_reads_what_it_can() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE);
        std::fs::write(
            &path,
            br#"{"pages": [
                {"name": "Mine", "choices": ["rain", "hail", {"name": "Hi", "command": "say hi"},
                    {"name": "No command"}, 7]},
                {"choices": ["rain"]},
                {"id": "Mine", "name": "Copy", "choices": []}
            ]}"#,
        )
        .unwrap();
        let pages = WheelPages::load(directory.path());
        assert_eq!(pages.pages().len(), 2);
        let mine = &pages.pages()[0];
        assert_eq!((mine.id.as_str(), mine.name.as_str()), ("mine", "Mine"));
        assert_eq!(mine.choices.len(), 2);
        assert_eq!(mine.choices[1], custom("Hi", "say hi"));
        assert_eq!(pages.pages()[1].id, "mine2");
        std::fs::write(&path, b"not json").unwrap();
        assert!(WheelPages::load(directory.path()).is_default());
        std::fs::write(&path, br#"{"pages": []}"#).unwrap();
        assert!(WheelPages::load(directory.path()).is_default());
    }
}
