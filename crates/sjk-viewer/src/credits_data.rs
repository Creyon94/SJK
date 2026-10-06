//! Parser for `assets/credits.txt`, built into the client and shown by the
//! credits page (`credits.rs`). The file's header gives the format; the tests
//! here check the real file.

/// The built-in credits.
pub(super) const EMBEDDED: &str = include_str!("../assets/credits.txt");

#[derive(Debug)]
pub(super) struct Section {
    pub(super) title: String,
    pub(super) cards: Vec<Card>,
}

/// One person (or project) of a section.
#[derive(Debug)]
pub(super) struct Card {
    pub(super) name: String,
    /// "@handle", or empty.
    pub(super) github: String,
    pub(super) role: String,
    pub(super) did: Vec<String>,
    pub(super) links: Vec<Link>,
}

/// A clickable address on a card.
#[derive(Debug)]
pub(super) struct Link {
    /// What the card shows: the label, or the address without `https://`.
    pub(super) label: String,
    pub(super) url: String,
}

impl Card {
    /// The GitHub profile behind `github`, if there is one.
    pub(super) fn github_url(&self) -> Option<String> {
        let handle = self.github.strip_prefix('@')?;
        Some(format!("https://github.com/{handle}"))
    }
}

/// `https://...` or `Label | https://...`.
fn parse_link(value: &str) -> Option<Link> {
    let (label, url) = match value.split_once('|') {
        Some((label, url)) => (label.trim(), url.trim()),
        None => {
            let url = value.trim();
            (
                url.trim_start_matches("https://").trim_end_matches('/'),
                url,
            )
        }
    };
    let host = url.strip_prefix("https://")?;
    if label.is_empty() || host.is_empty() || url.contains(char::is_whitespace) {
        return None;
    }
    Some(Link {
        label: label.to_owned(),
        url: url.to_owned(),
    })
}

pub(super) fn parse(text: &str) -> Result<Vec<Section>, String> {
    let mut sections: Vec<Section> = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let number = number + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.is_ascii() {
            return Err(format!("credits.txt line {number}: not ASCII"));
        }
        if let Some(title) = line.strip_prefix("==") {
            sections.push(Section {
                title: title.trim().to_owned(),
                cards: Vec::new(),
            });
            continue;
        }
        let Some(section) = sections.last_mut() else {
            return Err(format!(
                "credits.txt line {number}: text before the first \"==\" section"
            ));
        };
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section.cards.push(Card {
                name: name.trim().to_owned(),
                github: String::new(),
                role: String::new(),
                did: Vec::new(),
                links: Vec::new(),
            });
            continue;
        }
        let Some(card) = section.cards.last_mut() else {
            return Err(format!(
                "credits.txt line {number}: a key before the first [name]"
            ));
        };
        let Some((key, value)) = line.split_once(':') else {
            return Err(format!(
                "credits.txt line {number}: expected \"key: value\""
            ));
        };
        let value = value.trim().to_owned();
        match key.trim() {
            "github" => card.github = format!("@{}", value.trim_start_matches('@')),
            "role" => card.role = value,
            "did" => card.did.push(value),
            "link" => match parse_link(&value) {
                Some(link) => card.links.push(link),
                None => {
                    return Err(format!(
                        "credits.txt line {number}: a link is \"https://...\" or \"Label | https://...\""
                    ));
                }
            },
            other => return Err(format!("credits.txt line {number}: unknown key {other:?}")),
        }
    }
    for section in &sections {
        if section.cards.is_empty() {
            return Err(format!(
                "credits.txt: section {:?} has no card",
                section.title
            ));
        }
        if let Some(card) = section.cards.iter().find(|card| card.role.is_empty()) {
            return Err(format!("credits.txt: {} has no role", card.name));
        }
    }
    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_credits_parse_with_the_team_first() {
        let sections = parse(EMBEDDED).unwrap();
        let team = &sections[0];
        let names: Vec<_> = team.cards.iter().map(|card| card.name.as_str()).collect();
        assert_eq!(names, ["Sol"]);
        assert_eq!(team.cards[0].github, "@Sol-Vulpes");
        assert_eq!(
            team.cards[0].github_url().as_deref(),
            Some("https://github.com/Sol-Vulpes")
        );
        let cards: Vec<_> = sections.iter().flat_map(|section| &section.cards).collect();
        for name in ["Bishop", "Creyon"] {
            assert!(cards.iter().any(|card| card.name == name), "{name} missing");
        }
        assert!(cards.iter().flat_map(|card| &card.links).count() > 0);
    }

    #[test]
    fn format_errors_name_their_line() {
        let ok = "== A\n[X]\ngithub: @x\nrole: r\ndid: one\ndid: two\n";
        let sections = parse(ok).unwrap();
        assert_eq!(sections[0].cards[0].github, "@x");
        assert_eq!(sections[0].cards[0].did, ["one", "two"]);
        assert!(parse("[X]\nrole: r\n").is_err());
        assert!(parse("== A\nrole: r\n").is_err());
        assert!(parse("== A\n[X]\nrank: r\n").is_err());
        assert!(parse("== A\n[X]\ndid: no role\n").is_err());
        assert!(parse("== A\n").is_err());
    }

    #[test]
    fn links_are_https_with_an_optional_label() {
        let card =
            "== A\n[X]\nrole: r\nlink: https://a.example/b/\nlink: Pulls | https://c.example\n";
        let sections = parse(card).unwrap();
        let links = &sections[0].cards[0].links;
        assert_eq!(links[0].label, "a.example/b");
        assert_eq!(links[0].url, "https://a.example/b/");
        assert_eq!(links[1].label, "Pulls");
        assert_eq!(links[1].url, "https://c.example");
        assert!(sections[0].cards[0].github_url().is_none());
        for bad in [
            "http://a.example",
            "Label | ftp://a",
            "https://",
            " | https://a",
            "https://a b",
        ] {
            let text = format!("== A\n[X]\nrole: r\nlink: {bad}\n");
            assert!(parse(&text).is_err(), "{bad}");
        }
    }
}
