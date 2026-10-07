//! Bug reports sent to the hub (`PROTOCOL.md`, "Bug reports"). The hub decides; these
//! are the same rules, so the game can refuse a character as it is typed and say what
//! is wrong before anything is sent.

/// Shortest and longest report text, in characters, after whitespace is normalised.
pub const TEXT_MIN: usize = 10;
/// Longest report text, in characters.
pub const TEXT_MAX: usize = 600;
/// The punctuation a report may use besides letters, digits and spaces.
pub const PUNCTUATION: &str = ".,!?'-:()";
const RUN_MAX: usize = 6;
const LETTERS_MIN: usize = 5;
const WORDS_MIN: usize = 2;

/// A bug report: the tester's text and where they were.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BugReport {
    /// What the tester wrote ([`text`] checks it).
    pub text: String,
    /// The map, `maps/<name>.bsp`, or empty.
    pub map: String,
    /// The client's build.
    pub build: String,
    /// The game server's `ip:port`, or empty.
    pub server: String,
}

/// Whether a report may contain `c`.
pub fn allowed(c: char) -> bool {
    c.is_alphanumeric() || c == ' ' || PUNCTUATION.contains(c)
}

/// The report text as the hub stores it, or why the hub would refuse it.
pub fn text(raw: &str) -> Result<String, &'static str> {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if !text.chars().all(allowed) {
        return Err("only letters, digits, spaces and . , ! ? ' - : ( ) are allowed");
    }
    let length = text.chars().count();
    if length < TEXT_MIN {
        return Err("a report needs at least 10 characters");
    }
    if length > TEXT_MAX {
        return Err("a report is at most 600 characters");
    }
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let words = text
        .split(' ')
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count();
    let mut run = (None, 0);
    let longest = text.chars().fold(0, |longest, c| {
        run = if run.0 == Some(c) {
            (Some(c), run.1 + 1)
        } else {
            (Some(c), 1)
        };
        longest.max(run.1)
    });
    if letters < LETTERS_MIN || words < WORDS_MIN || longest > RUN_MAX {
        return Err("a report needs a few real words");
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules_match_the_hubs() {
        assert_eq!(
            text(" The door\nflickers on ffa3! ").as_deref(),
            Ok("The door flickers on ffa3!")
        );
        assert!(text("Le sol brille trop, été").is_ok());
        assert!(text("<b>bold</b> text here").is_err());
        assert!(text("short").is_err());
        assert!(text("aaaaaaaaaaaa bb").is_err());
        assert!(text(&"word ".repeat(200)).is_err());
        assert!(!allowed('<') && !allowed('"') && !allowed('/') && !allowed('\u{202e}'));
        assert!(allowed('é') && allowed('7') && allowed('?'));
    }
}
