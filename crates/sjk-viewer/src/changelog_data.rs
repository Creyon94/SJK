//! Parser for `CHANGELOG.md`, built into the client and shown by the changelog
//! page (`changelog.rs`). The file's comment gives the format; the tests here
//! check the real file, so `cargo test` catches a broken edit.

/// The repository's changelog, as built.
pub(super) const EMBEDDED: &str = include_str!("../../../CHANGELOG.md");

/// One release, or the changes on `main` since the last one.
#[derive(Debug)]
pub(super) struct Release {
    /// "2026.1005.1 (Alpha)", or "Unreleased".
    pub(super) title: String,
    /// dd/mm/yyyy; empty for "Unreleased".
    pub(super) date: String,
    /// Introduction paragraphs, each joined into one line.
    pub(super) intro: Vec<String>,
    pub(super) changes: Vec<Change>,
    /// "<date>   /   <n> changes" for the list row.
    pub(super) meta: String,
}

#[derive(Debug)]
pub(super) struct Change {
    pub(super) text: String,
    /// Who made it: "Sol", "Creyon", "Sol, after JoF EJK".
    pub(super) credit: String,
}

/// Inline markdown the page does not draw: code spans become plain text.
fn plain(text: &str) -> String {
    text.replace('`', "")
}

pub(super) fn parse(text: &str) -> Result<Vec<Release>, String> {
    let mut releases: Vec<Release> = Vec::new();
    let mut in_comment = false;
    let mut paragraph = String::new();
    for (number, line) in text.lines().enumerate() {
        let number = number + 1;
        let line = line.trim_end();
        if !line.is_ascii() {
            return Err(format!("CHANGELOG.md line {number}: not ASCII"));
        }
        if in_comment {
            in_comment = !line.contains("-->");
            continue;
        }
        if line.trim_start().starts_with("<!--") {
            in_comment = !line.contains("-->");
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            flush(&mut releases, &mut paragraph);
            let (title, date) = match heading.split_once('|') {
                Some((title, date)) => (title.trim(), date.trim()),
                None => (heading.trim(), ""),
            };
            if date.is_empty() != (title == "Unreleased") {
                return Err(format!(
                    "CHANGELOG.md line {number}: a release needs \"| dd/mm/yyyy\" (Unreleased has none)"
                ));
            }
            if !date.is_empty() && !is_eu_date(date) {
                return Err(format!(
                    "CHANGELOG.md line {number}: date {date:?} is not dd/mm/yyyy"
                ));
            }
            releases.push(Release {
                title: title.to_owned(),
                date: date.to_owned(),
                intro: Vec::new(),
                changes: Vec::new(),
                meta: String::new(),
            });
            continue;
        }
        let Some(release) = releases.last_mut() else {
            continue; // The page's own introduction.
        };
        if let Some(item) = line.strip_prefix("- ") {
            flush(&mut releases, &mut paragraph);
            let (text, credit) = item
                .strip_suffix(")_")
                .and_then(|rest| rest.rsplit_once(" _("))
                .ok_or_else(|| {
                    format!("CHANGELOG.md line {number}: a change must end with \" _(<credit>)_\"")
                })?;
            let release = releases.last_mut().expect("checked above");
            release.changes.push(Change {
                text: plain(text.trim()),
                credit: credit.trim().to_owned(),
            });
        } else if line.trim().is_empty() {
            flush(&mut releases, &mut paragraph);
        } else if !release.changes.is_empty() {
            return Err(format!(
                "CHANGELOG.md line {number}: text after a release's changes"
            ));
        } else {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(line.trim());
        }
    }
    flush(&mut releases, &mut paragraph);
    for release in &mut releases {
        let count = release.changes.len();
        let plural = if count == 1 { "change" } else { "changes" };
        release.meta = if release.date.is_empty() {
            format!("NOT RELEASED YET   /   {count} {plural}")
        } else {
            format!("{}   /   {count} {plural}", release.date)
        };
    }
    Ok(releases)
}

/// End the introduction paragraph being gathered, if any.
fn flush(releases: &mut [Release], paragraph: &mut String) {
    if paragraph.is_empty() {
        return;
    }
    if let Some(release) = releases.last_mut() {
        release.intro.push(plain(paragraph));
    }
    paragraph.clear();
}

fn is_eu_date(date: &str) -> bool {
    let parts: Vec<&str> = date.split('/').collect();
    let digits =
        |part: &str, len: usize| part.len() == len && part.bytes().all(|b| b.is_ascii_digit());
    parts.len() == 3
        && digits(parts[0], 2)
        && digits(parts[1], 2)
        && digits(parts[2], 4)
        && (1..=31).contains(&parts[0].parse::<u8>().unwrap_or(0))
        && (1..=12).contains(&parts[1].parse::<u8>().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_changelog_parses_and_credits_every_change() {
        let releases = parse(EMBEDDED).unwrap();
        assert!(releases.len() >= 3);
        assert!(releases.iter().any(|r| r.title == "0.1.0-alpha.1"));
        for release in &releases {
            assert!(
                !release.changes.is_empty(),
                "{} lists no change",
                release.title
            );
            for change in &release.changes {
                assert!(!change.credit.is_empty(), "{}", change.text);
                assert!(!change.text.contains('`'), "{}", change.text);
            }
        }
        // Only the first section may be the unreleased one.
        for release in &releases[1..] {
            assert_ne!(release.title, "Unreleased");
        }
    }

    #[test]
    fn sections_intro_items_and_errors() {
        let text = "# Changelog\nPage intro.\n<!--\n## not a release\n-->\n\n## 1.0 | 05/10/2026\n\nFirst `line`\nsame paragraph.\n\nSecond.\n\n- Did a thing _(Sol, after JoF EJK)_\n- Another _(Creyon)_\n";
        let releases = parse(text).unwrap();
        assert_eq!(releases.len(), 1);
        let release = &releases[0];
        assert_eq!(
            (release.title.as_str(), release.date.as_str()),
            ("1.0", "05/10/2026")
        );
        assert_eq!(release.intro, ["First line same paragraph.", "Second."]);
        assert_eq!(release.changes[0].text, "Did a thing");
        assert_eq!(release.changes[0].credit, "Sol, after JoF EJK");
        assert_eq!(release.meta, "05/10/2026   /   2 changes");

        assert!(parse("## 1.0 | 2026-10-05\n- x _(Sol)_\n").is_err());
        assert!(parse("## 1.0\n- x _(Sol)_\n").is_err());
        assert!(parse("## 1.0 | 05/10/2026\n- no credit\n").is_err());
        assert!(parse("## Unreleased\n- x _(Sol)_\n").is_ok());
    }
}
