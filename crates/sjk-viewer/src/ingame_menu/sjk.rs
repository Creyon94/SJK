//! The in-game SJK pop-up: SJK's own screens, reached from the SJK button
//! left of About on the classic bar, or the SJK row of the modern game menu.
//! Add an entry to [`ENTRIES`] and its action in `game_menu_actions.rs`
//! (`activate_sjk_row`).

/// One entry: its label and the line describing it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Entry {
    pub(crate) label: &'static str,
    pub(crate) hint: &'static str,
}

/// The pop-up's entries, top to bottom; the modern page adds Back after them.
pub(crate) const ENTRIES: [Entry; 1] = [Entry {
    label: "Changelog",
    hint: "What changed in each SJK release, and who made it",
}];

/// Row of the Changelog entry.
pub(crate) const CHANGELOG: usize = 0;
