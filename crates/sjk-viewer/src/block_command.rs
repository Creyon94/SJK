//! The `block` and `unblock` console commands (`docs/hub-chat.md`, "Blocking a
//! player"): `block` alone lists the players blocked on this PC, `block <player>` blocks
//! one of the server being played (`target` for the one under the crosshair, a slot
//! number, or part of the name the game shows), and `unblock <player|all>` lifts a block,
//! by a name of the list (so it works away from the server too) or a player here. The
//! list is the one the sender cards' Block keeps ([`crate::player_mutes`]).

use crate::chat_mutes::Mute;
use crate::emotes::plain_name;

/// Console command names.
pub(crate) const COMMAND: &str = "block";
pub(crate) const UNBLOCK_COMMAND: &str = "unblock";
/// Help texts for completion and `cmdlist`.
pub(crate) const HELP: &str = "Block an SJK player on this PC (no chat, Kyle, no sounds, no emotes): block <player|target>; block alone lists them";
pub(crate) const UNBLOCK_HELP: &str = "Lift a block: unblock <player|all>";

/// The console's lines for the list.
pub(crate) fn listing(blocked: &[Mute]) -> Vec<String> {
    if blocked.is_empty() {
        return vec!["Nobody is blocked on this PC".to_owned()];
    }
    std::iter::once(format!("Blocked on this PC ({}):", blocked.len()))
        .chain(blocked.iter().map(|mute| {
            let key = mute.key_id.as_deref().unwrap_or("by name");
            format!("  {}^7  ({key})", mute.name)
        }))
        .collect()
}

/// The entry of `blocked` that `word` names: a whole name (colours and case aside) or
/// the key first, else the only one whose name holds it.
pub(crate) fn find<'a>(blocked: &'a [Mute], word: &str) -> Result<Option<&'a Mute>, String> {
    let wanted = plain_name(word);
    if wanted.is_empty() {
        return Ok(None);
    }
    if let Some(exact) = blocked
        .iter()
        .find(|mute| plain_name(&mute.name) == wanted || mute.key_id.as_deref() == Some(word))
    {
        return Ok(Some(exact));
    }
    let matching: Vec<&Mute> = blocked
        .iter()
        .filter(|mute| plain_name(&mute.name).contains(&wanted))
        .collect();
    match matching[..] {
        [] => Ok(None),
        [one] => Ok(Some(one)),
        _ => Err(format!(
            "{word} matches {}: say more of the name",
            matching
                .iter()
                .map(|mute| crate::emotes::plain_name_shown(&mute.name))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

impl crate::GpuState {
    /// The player `word` names on the server being played: their slot, the name the
    /// game shows (colours kept) and their SJK key when the hub's claim covers them.
    fn blockable(&self, word: &str) -> Result<(String, String, Option<String>), String> {
        let Some(session) = self.live_session.as_ref() else {
            return Err("Join a server to block a player there".to_owned());
        };
        let own = u8::try_from(session.latest_snapshot().player.client_num()).ok();
        let (slot, plain) = self.emote_player(own, word)?;
        let shown = crate::player_identity::shown_name(session.game_state(), usize::from(slot))
            .unwrap_or_else(|| plain.clone());
        let key = crate::player_identity::hub_mark(slot, &shown).map(|mark| mark.key_id);
        Ok((plain, shown, key))
    }

    /// `block [player]`.
    pub(crate) fn block_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let word = args.get(1..).unwrap_or_default().join(" ");
        let word = word.trim();
        if word.is_empty() {
            return Ok(listing(&crate::player_mutes::entries()));
        }
        let (plain, shown, key) = self.blockable(word)?;
        if crate::player_mutes::is_muted(key.as_deref(), &shown) {
            return Err(format!("{plain} is already blocked"));
        }
        crate::player_mutes::set_muted(key.as_deref(), &shown, true);
        Ok(vec![match key {
            Some(_) => format!("{plain} is blocked on this PC (unblock {plain})"),
            None => format!(
                "{plain} is blocked on this PC by name: the hub does not know them, so anyone wearing the name is (unblock {plain})"
            ),
        }])
    }

    /// `unblock <player|all>`.
    pub(crate) fn unblock_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let word = args.get(1..).unwrap_or_default().join(" ");
        let word = word.trim();
        if word.is_empty() {
            return Err(UNBLOCK_HELP.to_owned());
        }
        if word.eq_ignore_ascii_case(crate::emotes::ALL) {
            let count = crate::player_mutes::clear();
            return Ok(vec![format!("Unblocked {count} player(s)")]);
        }
        let blocked = crate::player_mutes::entries();
        let entry = match find(&blocked, word)? {
            Some(entry) => entry.clone(),
            None => {
                // A player here, by the name they show now.
                if self.live_session.is_none() {
                    return Err(format!("Nobody blocked is called {word}"));
                }
                let (plain, shown, key) = self.blockable(word)?;
                if !crate::player_mutes::is_muted(key.as_deref(), &shown) {
                    return Err(format!("{plain} is not blocked"));
                }
                Mute {
                    key_id: key,
                    name: shown,
                }
            }
        };
        crate::player_mutes::set_muted(entry.key_id.as_deref(), &entry.name, false);
        Ok(vec![format!(
            "{} is unblocked",
            crate::emotes::plain_name_shown(&entry.name)
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mute(key: Option<&str>, name: &str) -> Mute {
        Mute {
            key_id: key.map(str::to_owned),
            name: name.to_owned(),
        }
    }

    #[test]
    fn a_name_finds_its_entry_whole_names_first() {
        let list = [
            mute(Some("0123456789abcdef"), "^1Troll"),
            mute(None, "Trollface"),
            mute(None, "Sol"),
        ];
        assert_eq!(
            find(&list, "troll").unwrap(),
            Some(&list[0]),
            "whole name first"
        );
        assert_eq!(find(&list, "face").unwrap(), Some(&list[1]));
        assert_eq!(
            find(&list, "0123456789abcdef").unwrap(),
            Some(&list[0]),
            "by key"
        );
        assert_eq!(find(&list, "nobody").unwrap(), None);
        assert!(find(&list, "o").is_err(), "several match");
    }

    #[test]
    fn the_listing_says_how_each_is_matched() {
        assert_eq!(listing(&[]), ["Nobody is blocked on this PC"]);
        let lines = listing(&[mute(Some("0123456789abcdef"), "^1Troll"), mute(None, "Sol")]);
        assert_eq!(lines[0], "Blocked on this PC (2):");
        assert!(lines[1].contains("0123456789abcdef"));
        assert!(lines[2].contains("by name"));
    }
}
