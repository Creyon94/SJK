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
pub(crate) const ENTRIES: [Entry; 5] = [
    Entry {
        label: "Changelog",
        hint: "What changed in each SJK release, and who made it",
    },
    Entry {
        label: "Credits",
        hint: "The people who make SJK",
    },
    Entry {
        label: "Identity",
        hint: "Your SJK key and profile, and the players the hub knows here",
    },
    Entry {
        label: "Report a bug",
        hint: "Tell the SJK team what went wrong; it goes to the SJK hub",
    },
    Entry {
        label: "Report a player",
        hint: "Everyone here; tell the SJK team about a cheater or a troll",
    },
];

/// Rows of the entries.
pub(crate) const CHANGELOG: usize = 0;
pub(crate) const CREDITS: usize = 1;
pub(crate) const IDENTITY: usize = 2;
pub(crate) const REPORT: usize = 3;
pub(crate) const REPORT_PLAYER: usize = 4;
