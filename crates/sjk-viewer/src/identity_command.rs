//! The `identity` console command: `identity` opens the Identity page, and its
//! words edit the player's hub profile and list who the hub knows on this server
//! (`docs/identity.md`).

use crate::player_identity;
use sjk_identity::{Snapshot, Status};

/// Console command name.
pub(crate) const COMMAND: &str = "identity";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Your SJK identity: open its page; name, bio, key and who subcommands";

const USAGE: &str = "identity [name <text> | bio <text> | key | who [slot]]";

/// What the player asked for.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Action {
    /// No words: open or close the page.
    Toggle,
    /// Show the key id and where the key file is.
    Key,
    /// List the players the hub knows on this server, or one of them.
    Who(Option<u8>),
    /// Set the display name at the hub.
    Name(String),
    /// Set the bio at the hub.
    Bio(String),
}

/// Read the command's words.
pub(crate) fn parse(args: &[String]) -> Result<Action, String> {
    let Some(word) = args.first() else {
        return Ok(Action::Toggle);
    };
    let text = || args[1..].join(" ");
    match word.to_ascii_lowercase().as_str() {
        "key" => Ok(Action::Key),
        "who" => match args.get(1) {
            None => Ok(Action::Who(None)),
            Some(slot) => slot
                .parse()
                .map(|slot| Action::Who(Some(slot)))
                .map_err(|_| format!("who takes a client number, not \"{slot}\"")),
        },
        "name" if args.len() > 1 => Ok(Action::Name(text())),
        "bio" => Ok(Action::Bio(text())),
        "name" => Err("identity name needs the name to use".to_owned()),
        _ => Err(format!("usage: {USAGE}")),
    }
}

/// The lines `identity key` prints.
pub(crate) fn key_lines(snapshot: Option<&Snapshot>, file: &std::path::Path) -> Vec<String> {
    match snapshot {
        Some(snapshot) => vec![
            format!("Key id: {}", snapshot.key_id),
            format!(
                "The private key is in {}. Back it up: losing it loses this identity, and nobody else may have it.",
                file.display()
            ),
        ],
        None => vec!["No identity key yet: turn cl_identity on.".to_owned()],
    }
}

/// The lines `identity who` prints.
pub(crate) fn who_lines(snapshot: &Snapshot, slot: Option<u8>) -> Vec<String> {
    let mut lines = Vec::new();
    for player in &snapshot.players {
        if slot.is_some_and(|wanted| wanted != player.slot) {
            continue;
        }
        let name = if player.name.is_empty() {
            "(no name)"
        } else {
            &player.name
        };
        lines.push(format!(
            "{:>2}  {}  {}{}",
            player.slot,
            name,
            player.key_id,
            if player.verified { "  VERIFIED" } else { "" }
        ));
        if slot.is_some() {
            match snapshot.profiles.get(&player.key_id) {
                Some(profile) if !profile.bio.is_empty() => {
                    lines.extend(profile.bio.lines().map(|line| format!("    {line}")));
                }
                Some(_) => lines.push("    (no bio)".to_owned()),
                None => {
                    player_identity::look_up(&player.key_id);
                    lines.push("    fetching the bio; run this again in a moment".to_owned());
                }
            }
        }
    }
    if lines.is_empty() {
        lines.push(match slot {
            Some(slot) => format!("The hub knows nobody in slot {slot} here."),
            None => "The hub knows no SJK players on this server.".to_owned(),
        });
    }
    lines
}

/// Why a profile change cannot be sent now, if it cannot.
pub(crate) fn profile_blocker(snapshot: Option<&Snapshot>) -> Option<&'static str> {
    match snapshot.map(|snapshot| &snapshot.status) {
        None | Some(Status::Disabled) => Some("Identity is off: turn cl_identity on."),
        Some(Status::NoHub) => Some("There is no hub: set cl_hubUrl."),
        Some(Status::Online) => None,
        Some(_) => Some("Not connected to the hub yet; try again in a moment."),
    }
}

/// `name` and `bio` for the profile after a change to one of them.
pub(crate) fn merged_profile(
    snapshot: &Snapshot,
    name: Option<String>,
    bio: Option<String>,
) -> Result<(String, String), String> {
    let current = snapshot.me.as_ref();
    let name = name
        .or_else(|| current.map(|me| me.name.clone()))
        .filter(|name| !name.is_empty())
        .ok_or("Set a name first: identity name <text>")?;
    let bio = bio
        .or_else(|| current.map(|me| me.bio.clone()))
        .unwrap_or_default();
    Ok((name, bio))
}

impl crate::GpuState {
    /// `identity <words>`: the subcommands that are not "open the page".
    pub(crate) fn identity_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let action = parse(args)?;
        let snapshot = player_identity::snapshot();
        match action {
            Action::Toggle => Ok(Vec::new()),
            Action::Key => {
                let file = self
                    .console
                    .as_ref()
                    .map(|console| console.config_directory().join("identity.key"))
                    .unwrap_or_default();
                Ok(key_lines(snapshot.as_ref(), &file))
            }
            Action::Who(slot) => match &snapshot {
                Some(snapshot) => Ok(who_lines(snapshot, slot)),
                None => Err("Identity is off: turn cl_identity on.".to_owned()),
            },
            Action::Name(text) => self.send_profile(snapshot, Some(text), None),
            Action::Bio(text) => self.send_profile(snapshot, None, Some(text)),
        }
    }

    fn send_profile(
        &mut self,
        snapshot: Option<Snapshot>,
        name: Option<String>,
        bio: Option<String>,
    ) -> Result<Vec<String>, String> {
        if let Some(reason) = profile_blocker(snapshot.as_ref()) {
            return Err(reason.to_owned());
        }
        let snapshot = snapshot.ok_or("Identity is off: turn cl_identity on.")?;
        let (name, bio) = merged_profile(&snapshot, name, bio)?;
        player_identity::set_profile(name, bio);
        Ok(vec![
            "Sent to the hub; the Identity page shows the result.".to_owned(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::{Presence, Profile};
    use std::collections::HashMap;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            status: Status::Online,
            key_id: "0123456789abcdef".to_owned(),
            me: Some(Profile {
                key_id: "0123456789abcdef".to_owned(),
                key: String::new(),
                name: "Sol".to_owned(),
                bio: "hi".to_owned(),
                verified: false,
                created: 0,
            }),
            server: None,
            players: vec![Presence {
                slot: 3,
                claimed_name: "^1Fox".to_owned(),
                key_id: "fedcba9876543210".to_owned(),
                name: "Fox".to_owned(),
                verified: true,
            }],
            profiles: HashMap::new(),
            notice: None,
            revision: 0,
        }
    }

    #[test]
    fn words_become_actions() {
        assert_eq!(parse(&words("")), Ok(Action::Toggle));
        assert_eq!(parse(&words("key")), Ok(Action::Key));
        assert_eq!(parse(&words("WHO")), Ok(Action::Who(None)));
        assert_eq!(parse(&words("who 3")), Ok(Action::Who(Some(3))));
        assert_eq!(
            parse(&words("name Sol the Fox")),
            Ok(Action::Name("Sol the Fox".to_owned()))
        );
        assert_eq!(parse(&words("bio")), Ok(Action::Bio(String::new())));
        assert!(parse(&words("name")).is_err());
        assert!(parse(&words("who x")).is_err());
        assert!(
            parse(&words("frobnicate"))
                .unwrap_err()
                .starts_with("usage")
        );
    }

    #[test]
    fn who_lists_slots_and_marks_verified_players() {
        let lines = who_lines(&snapshot(), None);
        assert_eq!(lines, [" 3  Fox  fedcba9876543210  VERIFIED"]);
        assert_eq!(
            who_lines(&snapshot(), Some(9)),
            ["The hub knows nobody in slot 9 here."]
        );
        let mut empty = snapshot();
        empty.players.clear();
        assert_eq!(
            who_lines(&empty, None),
            ["The hub knows no SJK players on this server."]
        );
    }

    #[test]
    fn who_with_a_slot_shows_a_fetched_bio() {
        let mut shown = snapshot();
        shown.profiles.insert(
            "fedcba9876543210".to_owned(),
            Profile {
                key_id: "fedcba9876543210".to_owned(),
                key: String::new(),
                name: "Fox".to_owned(),
                bio: "line one\nline two".to_owned(),
                verified: true,
                created: 0,
            },
        );
        let lines = who_lines(&shown, Some(3));
        assert_eq!(lines[1..], ["    line one", "    line two"]);
    }

    #[test]
    fn a_profile_change_keeps_the_other_field_and_needs_a_name() {
        assert_eq!(
            merged_profile(&snapshot(), Some("Vulpes".into()), None),
            Ok(("Vulpes".to_owned(), "hi".to_owned()))
        );
        assert_eq!(
            merged_profile(&snapshot(), None, Some("new".into())),
            Ok(("Sol".to_owned(), "new".to_owned()))
        );
        let mut unnamed = snapshot();
        unnamed.me.as_mut().unwrap().name.clear();
        assert!(merged_profile(&unnamed, None, Some("x".into())).is_err());
        assert!(merged_profile(&unnamed, Some("Sol".into()), None).is_ok());
    }

    #[test]
    fn profile_changes_wait_for_an_online_hub() {
        assert!(profile_blocker(None).is_some());
        let mut state = snapshot();
        assert_eq!(profile_blocker(Some(&state)), None);
        for status in [Status::Disabled, Status::NoHub, Status::Registering] {
            state.status = status;
            assert!(profile_blocker(Some(&state)).is_some());
        }
    }
}
