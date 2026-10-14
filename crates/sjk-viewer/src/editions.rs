//! Editions: the seasons of everything collectible (14/10/2026, Sol's request). Each
//! collectible belongs to one edition, the stamp of when it came: **Alpha** for the
//! whole alpha, and later editions as they open. Holocron cards already carry theirs
//! (the hub stamps a copy at opening, [`crate::cards`]); the shaders, medals,
//! achievements and toys take theirs from their catalogue entries here in the client.
//! One table says which editions exist and how each reads, so a new edition is one
//! line in [`ALL`] and the items put in it name its id.

/// The alpha's edition, the first.
pub(crate) const ALPHA: &str = "alpha";

/// One edition: its id (the hub's, as a card carries it) and how it reads.
#[derive(Debug)]
pub(crate) struct Edition {
    pub(crate) id: &'static str,
    /// The full stamp: "Alpha Edition".
    pub(crate) name: &'static str,
    /// The short tag for small spaces: "Alpha".
    pub(crate) short: &'static str,
}

/// Every edition the client knows, the oldest first.
pub(crate) const ALL: [Edition; 1] = [Edition {
    id: ALPHA,
    name: "Alpha Edition",
    short: "Alpha",
}];

/// The edition `id` names, if the client knows it.
pub(crate) fn find(id: &str) -> Option<&'static Edition> {
    ALL.iter().find(|edition| edition.id == id)
}

/// `id` with a capital first letter, for an edition this client does not know yet.
fn capitalised(id: &str) -> String {
    let mut chars = id.chars();
    let first = chars.next().map(|c| c.to_ascii_uppercase());
    format!("{}{}", first.unwrap_or(' '), chars.as_str())
}

/// What players read for edition `id`: "Alpha Edition"; an id the client does not know
/// reads as itself with "Edition" after it, and none as nothing.
pub(crate) fn name(id: &str) -> String {
    match (id, find(id)) {
        ("", _) => String::new(),
        (_, Some(edition)) => edition.name.to_owned(),
        (other, None) => format!("{} Edition", capitalised(other)),
    }
}

/// The short tag of edition `id`: "Alpha"; empty for none.
pub(crate) fn short(id: &str) -> String {
    match (id, find(id)) {
        ("", _) => String::new(),
        (_, Some(edition)) => edition.short.to_owned(),
        (other, None) => capitalised(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edition_reads_in_full_and_short() {
        assert_eq!(name("alpha"), "Alpha Edition");
        assert_eq!(short("alpha"), "Alpha");
        assert_eq!(name("zeta"), "Zeta Edition");
        assert_eq!(short("zeta"), "Zeta");
        assert_eq!((name(""), short("")), (String::new(), String::new()));
    }

    #[test]
    fn every_item_names_an_edition_the_table_has() {
        for skin in crate::unlockables::ALL.iter() {
            assert!(find(skin.edition).is_some(), "{}", skin.id);
        }
        for medal in crate::medals::Medal::ALL {
            assert!(find(medal.edition()).is_some(), "{medal:?}");
        }
        for kind in &crate::achievements::ALL {
            assert!(find(kind.edition).is_some(), "{}", kind.id);
        }
        let ids: Vec<_> = ALL.iter().map(|edition| edition.id).collect();
        let mut sorted = ids.clone();
        sorted.dedup();
        assert_eq!(ids, sorted);
    }
}
