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
        assert_eq!(names, ["Sol", "Bishop"]);
        assert_eq!(team.cards[0].github, "@Sol-Vulpes");
        assert!(
            sections
                .iter()
                .flat_map(|section| &section.cards)
                .any(|card| card.name == "Creyon")
        );
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
}
