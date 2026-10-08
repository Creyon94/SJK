//! Staff requests (`PROTOCOL.md`, "Staff"): what a key the hub's operator made staff
//! may do from inside the game: find players, give and take back medals, clear
//! achievements. The hub refuses every one from a key that is not staff; the client
//! only offers them to a player whose own profile says `staff`.

use crate::wire::Profile;

/// One staff request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaffRequest {
    /// Find players: 16 hex digits for one key, a part of a name, or empty for the
    /// players seen most recently.
    Search(String),
    /// Give a medal.
    Award {
        /// The key that gets it.
        key_id: String,
        /// The medal's id.
        medal: String,
        /// Why, shown with the medal; may be empty.
        note: String,
    },
    /// Take one award of a medal back.
    Unaward {
        /// The key that holds it.
        key_id: String,
        /// The medal's id.
        medal: String,
    },
    /// Clear achievements.
    ClearAchievements {
        /// The key whose achievements go.
        key_id: String,
        /// The achievement's id, or empty for every one.
        id: String,
    },
}

impl StaffRequest {
    /// The key the request changes, if it changes one.
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::Search(_) => None,
            Self::Award { key_id, .. }
            | Self::Unaward { key_id, .. }
            | Self::ClearAchievements { key_id, .. } => Some(key_id),
        }
    }
}

/// What the staff requests brought back.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StaffState {
    /// Counts answers, so a reader can tell a new one.
    pub serial: u64,
    /// The players the last search found, updated by later answers about them.
    pub players: Vec<Profile>,
    /// What the last request came to: done, or why not.
    pub message: String,
    /// Whether the last request failed.
    pub failed: bool,
    /// A request is on its way.
    pub busy: bool,
}

impl StaffState {
    /// Put `profile`, the hub's answer about a key, in place of that key's entry.
    pub fn replace(&mut self, profile: &Profile) {
        if let Some(entry) = self
            .players
            .iter_mut()
            .find(|entry| entry.key_id == profile.key_id)
        {
            *entry = profile.clone();
        } else {
            self.players.insert(0, profile.clone());
        }
    }
}

/// What a request's success is called on the page.
pub(crate) fn done(request: &StaffRequest, count: usize) -> String {
    match request {
        StaffRequest::Search(query) if query.is_empty() => {
            format!("{count} players seen lately")
        }
        StaffRequest::Search(_) => match count {
            0 => "Nobody found".to_owned(),
            1 => "1 player found".to_owned(),
            count => format!("{count} players found"),
        },
        StaffRequest::Award { medal, .. } => format!("Gave {medal}"),
        StaffRequest::Unaward { medal, .. } => format!("Took back {medal}"),
        StaffRequest::ClearAchievements { id, .. } if id.is_empty() => {
            "Cleared every achievement".to_owned()
        }
        StaffRequest::ClearAchievements { id, .. } => format!("Cleared {id}"),
    }
}
