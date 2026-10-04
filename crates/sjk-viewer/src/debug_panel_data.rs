//! The debug panel's built-in test list: `assets/debug_panel.txt`, parsed once when
//! the console starts. The file header documents the format; [`parse`] enforces it,
//! and the tests below check the embedded file so a broken edit fails `cargo test`.

use std::fmt;

/// The test list built into the client.
pub(super) const EMBEDDED: &str = include_str!("../assets/debug_panel.txt");

/// Areas an entry may name, in the order open PRs are grouped.
pub(super) const AREAS: [&str; 7] = [
    "Console & chat",
    "Input",
    "Menus & settings",
    "HUD",
    "Rendering & effects",
    "Audio",
    "Gameplay",
];

/// Longest value, in bytes; longer lines would end in an ellipsis on narrow windows.
pub(super) const LINE_LIMIT: usize = 96;

/// Where an entry's change stands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Status {
    /// An open pull request on upstream, merged only into Sol's build.
    Open,
    /// Merged upstream.
    Merged,
    /// In Sol's build only, with no pull request.
    Personal,
}

impl Status {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "merged" => Some(Self::Merged),
            "personal" => Some(Self::Personal),
            _ => None,
        }
    }

    /// Uppercase label shown in the list and the detail pane.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Open => "OPEN PR",
            Self::Merged => "MERGED UPSTREAM",
            Self::Personal => "PERSONAL",
        }
    }
}

/// One change in the build and how to test it.
#[derive(Debug)]
pub(super) struct Entry {
    /// Stable key of the entry's tick.
    pub(super) id: String,
    /// PR numbers, for the checks of the embedded list.
    #[cfg(test)]
    pub(super) prs: Vec<u32>,
    pub(super) status: Status,
    pub(super) area: String,
    pub(super) title: String,
    pub(super) changes: Vec<String>,
    pub(super) tests: Vec<String>,
    pub(super) notes: Vec<String>,
    /// `#33` or `#30, #31`; empty for a personal entry without a PR.
    pub(super) reference: String,
    /// Status and area in capitals, for the list's second line.
    pub(super) meta: String,
    /// `PR #32   /   ISSUE #10`, or empty, for the detail pane.
    pub(super) links: String,
}

/// Why the test list could not be read; `line` is 1-based.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct ParseError {
    pub(super) line: usize,
    pub(super) message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "debug_panel.txt line {}: {}",
            self.line, self.message
        )
    }
}

/// An entry while its block is read; checked and completed by [`Draft::finish`].
struct Draft {
    line: usize,
    id: String,
    prs: Option<Vec<u32>>,
    issues: Option<Vec<u32>>,
    status: Option<Status>,
    area: Option<String>,
    title: Option<String>,
    changes: Vec<String>,
    tests: Vec<String>,
    notes: Vec<String>,
}

impl Draft {
    fn new(id: &str, line: usize) -> Self {
        Self {
            line,
            id: id.to_owned(),
            prs: None,
            issues: None,
            status: None,
            area: None,
            title: None,
            changes: Vec::new(),
            tests: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        fn once<T>(slot: &mut Option<T>, key: &str, value: T) -> Result<(), String> {
            if slot.replace(value).is_some() {
                return Err(format!("{key} given twice"));
            }
            Ok(())
        }
        match key {
            "pr" => once(&mut self.prs, key, numbers(value)?),
            "issue" => once(&mut self.issues, key, numbers(value)?),
            "status" => {
                let status = Status::parse(value)
                    .ok_or_else(|| format!("status {value:?} is not open, merged or personal"))?;
                once(&mut self.status, key, status)
            }
            "area" => {
                if !AREAS.contains(&value) {
                    return Err(format!("unknown area {value:?}"));
                }
                once(&mut self.area, key, value.to_owned())
            }
            "title" => once(&mut self.title, key, value.to_owned()),
            "change" => {
                self.changes.push(value.to_owned());
                Ok(())
            }
            "test" => {
                self.tests.push(value.to_owned());
                Ok(())
            }
            "note" => {
                self.notes.push(value.to_owned());
                Ok(())
            }
            _ => Err(format!("unknown key {key:?}")),
        }
    }

    fn finish(self) -> Result<Entry, ParseError> {
        let error = |message: &str| ParseError {
            line: self.line,
            message: format!("[{}] {message}", self.id),
        };
        let status = self.status.ok_or_else(|| error("has no status"))?;
        let area = self.area.ok_or_else(|| error("has no area"))?;
        let title = self.title.ok_or_else(|| error("has no title"))?;
        if self.tests.is_empty() {
            return Err(error("has no test step"));
        }
        let prs = self.prs.unwrap_or_default();
        if prs.is_empty() && status != Status::Personal {
            return Err(error("has no pr (only personal entries may omit it)"));
        }
        let issues = self.issues.unwrap_or_default();
        let reference = join_numbers("#", &prs);
        let meta = format!("{}   /   {}", status.label(), area.to_uppercase());
        let mut links = String::new();
        if !prs.is_empty() {
            links = format!("PR {reference}");
        }
        if !issues.is_empty() {
            if !links.is_empty() {
                links.push_str("   /   ");
            }
            links.push_str("ISSUE ");
            links.push_str(&join_numbers("#", &issues));
        }
        Ok(Entry {
            id: self.id,
            #[cfg(test)]
            prs,
            status,
            area,
            title,
            changes: self.changes,
            tests: self.tests,
            notes: self.notes,
            reference,
            meta,
            links,
        })
    }
}

/// Read a test list in the format the file header describes.
pub(super) fn parse(text: &str) -> Result<Vec<Entry>, ParseError> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut draft: Option<Draft> = None;
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let fail = |message: String| ParseError { line, message };
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !trimmed.is_ascii() {
            return Err(fail("text must be ASCII".into()));
        }
        if let Some(id) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            {
                return Err(fail(format!(
                    "id {id:?} must be lowercase letters, digits and '-'"
                )));
            }
            if entries.iter().any(|entry| entry.id == id)
                || draft.as_ref().is_some_and(|draft| draft.id == id)
            {
                return Err(fail(format!("id {id:?} is used twice")));
            }
            if let Some(done) = draft.replace(Draft::new(id, line)) {
                entries.push(done.finish()?);
            }
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            return Err(fail(format!(
                "expected \"key: value\" or \"[id]\", found {trimmed:?}"
            )));
        };
        let (key, value) = (key.trim(), value.trim());
        let Some(current) = draft.as_mut() else {
            return Err(fail(format!("{key:?} comes before the first [id]")));
        };
        if value.is_empty() {
            return Err(fail(format!("{key} is empty")));
        }
        if value.len() > LINE_LIMIT {
            return Err(fail(format!(
                "{key} is {} characters, more than {LINE_LIMIT}",
                value.len()
            )));
        }
        current.set(key, value).map_err(fail)?;
    }
    if let Some(done) = draft {
        entries.push(done.finish()?);
    }
    Ok(entries)
}

/// Comma-separated positive numbers, with an optional leading `#` on each.
fn numbers(value: &str) -> Result<Vec<u32>, String> {
    value
        .split(',')
        .map(|part| {
            let part = part.trim();
            part.strip_prefix('#')
                .unwrap_or(part)
                .parse::<u32>()
                .ok()
                .filter(|&number| number > 0)
                .ok_or_else(|| format!("{part:?} is not a number"))
        })
        .collect()
}

fn join_numbers(prefix: &str, numbers: &[u32]) -> String {
    let mut text = String::new();
    for (index, number) in numbers.iter().enumerate() {
        if index > 0 {
            text.push_str(", ");
        }
        text.push_str(prefix);
        text.push_str(&number.to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn embedded() -> Vec<Entry> {
        parse(EMBEDDED).unwrap_or_else(|error| panic!("{error}"))
    }

    #[test]
    fn embedded_list_parses_with_required_fields() {
        let entries = embedded();
        assert!(!entries.is_empty());
        for entry in &entries {
            assert!(!entry.title.is_empty(), "{}", entry.id);
            assert!(!entry.tests.is_empty(), "{}", entry.id);
            assert!(!entry.changes.is_empty(), "{} has no change line", entry.id);
            assert!(
                entry.changes.len() <= 3,
                "{} has too many change lines",
                entry.id
            );
            assert!(AREAS.contains(&entry.area.as_str()), "{}", entry.id);
            if entry.status != Status::Personal {
                assert!(!entry.prs.is_empty(), "{}", entry.id);
            }
        }
    }

    #[test]
    fn embedded_ids_and_prs_are_unique() {
        let entries = embedded();
        let mut ids = HashSet::new();
        let mut prs = HashSet::new();
        for entry in &entries {
            assert!(ids.insert(entry.id.as_str()), "id {} twice", entry.id);
            // One upstream PR can supersede several of Sol's (e.g. #88 for #6 and #33).
            if entry.status != Status::Open {
                continue;
            }
            for pr in &entry.prs {
                assert!(prs.insert(*pr), "PR #{pr} listed twice");
            }
        }
    }

    #[test]
    fn embedded_list_covers_the_build() {
        let entries = embedded();
        let listed: HashSet<u32> = entries.iter().flat_map(|entry| entry.prs.clone()).collect();
        // Sol's PRs in this build: open ones, and merged ones not yet dropped.
        let in_build = [
            29, 34, 35, 36, 37, 38, 39, 40, 42, 43, 44, 45, 46, 47, 49, 59,
        ]
        .into_iter()
        .chain([
            60, 61, 62, 63, 66, 70, 71, 72, 78, 86, 87, 97, 98, 100, 101, 105,
        ])
        .chain([
            106, 107, 108, 109, 110, 111, 113, 114, 115, 116, 117, 118, 119, 120, 121,
        ]);
        for pr in in_build {
            assert!(listed.contains(&pr), "PR #{pr} is missing");
        }
    }

    #[test]
    fn embedded_order_groups_open_prs_by_area_then_merged() {
        let rank = |entry: &Entry| match entry.status {
            Status::Open => AREAS.iter().position(|area| *area == entry.area).unwrap(),
            Status::Personal => AREAS.len(),
            Status::Merged => AREAS.len() + 1,
        };
        let entries = embedded();
        for pair in entries.windows(2) {
            assert!(
                rank(&pair[0]) <= rank(&pair[1]),
                "{} should come after {}",
                pair[0].id,
                pair[1].id
            );
        }
    }

    #[test]
    fn entries_get_their_display_labels() {
        let entries = parse(
            "[a]\npr: 30, #31\nissue: 26\nstatus: open\narea: HUD\ntitle: T\ntest: Step\n\
             [b]\nstatus: personal\narea: Audio\ntitle: U\ntest: Step\nnote: N\n",
        )
        .unwrap();
        assert_eq!(entries[0].prs, [30, 31]);
        assert_eq!(entries[0].reference, "#30, #31");
        assert_eq!(entries[0].meta, "OPEN PR   /   HUD");
        assert_eq!(entries[0].links, "PR #30, #31   /   ISSUE #26");
        assert_eq!(entries[1].reference, "");
        assert_eq!(entries[1].links, "");
        assert_eq!(entries[1].notes, ["N"]);
    }

    #[test]
    fn broken_lists_name_the_line() {
        let base = "[a]\npr: 1\nstatus: open\narea: HUD\ntitle: T\ntest: Step\n";
        let cases = [
            (
                "[a]\nstatus: open\narea: HUD\ntitle: T\ntest: S\n",
                1,
                "no pr",
            ),
            (
                "[a]\npr: 1\nstatus: open\narea: HUD\ntest: S\n",
                1,
                "no title",
            ),
            (
                "[a]\npr: 1\nstatus: open\narea: HUD\ntitle: T\n",
                1,
                "no test",
            ),
            ("[a]\npr: 1\nstatus: soon\n", 3, "status"),
            ("[a]\narea: Sky\n", 2, "unknown area"),
            ("[a]\ncolour: red\n", 2, "unknown key"),
            ("[a]\ntitle: T\ntitle: U\n", 3, "twice"),
            ("title: T\n", 1, "before the first"),
            ("[A b]\n", 1, "lowercase"),
            ("[a]\ntitle: caf\u{e9}\n", 2, "ASCII"),
            ("[a]\npr: six\n", 2, "not a number"),
            ("[a]\njust text\n", 2, "key: value"),
        ];
        for (text, line, message) in cases {
            let error = parse(text).unwrap_err();
            assert_eq!(error.line, line, "{text:?}: {error}");
            assert!(error.message.contains(message), "{text:?}: {error}");
        }
        let doubled = format!("{base}{base}");
        let error = parse(&doubled).unwrap_err();
        assert_eq!(error.line, 7);
        assert!(error.message.contains("used twice"), "{error}");
        let long = format!("[a]\ntitle: {}\n", "x".repeat(LINE_LIMIT + 1));
        assert!(parse(&long).unwrap_err().message.contains("more than"));
    }
}
