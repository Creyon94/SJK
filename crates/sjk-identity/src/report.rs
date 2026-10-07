//! Bug reports and world notes sent to the hub (`PROTOCOL.md`, "Bug reports" and
//! "World notes"). The hub decides; these are the same rules, so the game can refuse a
//! character as it is typed and say what is wrong before anything is sent.

/// Shortest and longest report text, in characters, after whitespace is normalised.
pub const TEXT_MIN: usize = 10;
/// Longest report text, in characters.
pub const TEXT_MAX: usize = 600;
/// The punctuation a report may use besides letters, digits and spaces.
pub const PUNCTUATION: &str = ".,!?'-:()";
const RUN_MAX: usize = 6;
const LETTERS_MIN: usize = 5;
const WORDS_MIN: usize = 2;
/// Shortest and longest world note text, in characters, after whitespace is normalised.
pub const NOTE_MIN: usize = 3;
/// Longest world note text, in characters.
pub const NOTE_MAX: usize = 500;
const NOTE_LETTERS_MIN: usize = 2;

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
    /// The in-game name the player wears; empty takes the one the service knows.
    pub name: String,
}

/// A world note: the player's text about what they aimed at, and where.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldNote {
    /// What the player wrote ([`note_text`] checks it).
    pub text: String,
    /// The map, `maps/<name>.bsp`, or empty.
    pub map: String,
    /// The client's build.
    pub build: String,
    /// The game server's `ip:port`, or empty.
    pub server: String,
    /// `setviewpos` back to the view: x, y, z and yaw.
    pub view: Option<[f32; 4]>,
    /// The aimed point and the surface's normal there.
    pub hit: Option<[f32; 3]>,
    /// The surface's normal at [`WorldNote::hit`].
    pub normal: Option<[f32; 3]>,
    /// The aimed shader ([`field`] empties a name the hub would refuse).
    pub shader: String,
    /// The BSP draw surface's index.
    pub surface: Option<u32>,
    /// How the surface is lit (`lightmapped`, `vertex-lit`).
    pub lighting: String,
    /// From the eye to the aimed point.
    pub distance: Option<f32>,
    /// The aimed entity's class name.
    pub entity: String,
    /// The in-game name the player wears; empty takes the one the service knows.
    pub name: String,
}

/// Whether a report may contain `c`.
pub fn allowed(c: char) -> bool {
    c.is_alphanumeric() || c == ' ' || PUNCTUATION.contains(c)
}

/// The report text as the hub stores it, or why the hub would refuse it.
pub fn text(raw: &str) -> Result<String, &'static str> {
    check(raw, TEXT_MIN, TEXT_MAX, LETTERS_MIN, WORDS_MIN).map_err(|refusal| match refusal {
        Refusal::Character => CHARACTERS,
        Refusal::Short => "a report needs at least 10 characters",
        Refusal::Long => "a report is at most 600 characters",
        Refusal::Noise => "a report needs a few real words",
    })
}

/// The note text as the hub stores it, or why the hub would refuse it.
pub fn note_text(raw: &str) -> Result<String, &'static str> {
    check(raw, NOTE_MIN, NOTE_MAX, NOTE_LETTERS_MIN, 1).map_err(|refusal| match refusal {
        Refusal::Character => CHARACTERS,
        Refusal::Short => "a note needs at least 3 characters",
        Refusal::Long => "a note is at most 500 characters",
        Refusal::Noise => "a note needs a few letters",
    })
}

/// A context field as the hub takes it: `raw` trimmed when it is at most `max` ASCII
/// letters, digits and `_ - . /` and spaces, else empty (an odd name is left out
/// rather than the whole note refused).
pub fn field(raw: &str, max: usize) -> String {
    let value = raw.trim();
    if value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-./ ".contains(&b))
    {
        value.to_owned()
    } else {
        String::new()
    }
}

const CHARACTERS: &str = "only letters, digits, spaces and . , ! ? ' - : ( ) are allowed";

enum Refusal {
    Character,
    Short,
    Long,
    Noise,
}

fn check(
    raw: &str,
    min: usize,
    max: usize,
    letters_min: usize,
    words_min: usize,
) -> Result<String, Refusal> {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if !text.chars().all(allowed) {
        return Err(Refusal::Character);
    }
    let length = text.chars().count();
    if length < min {
        return Err(Refusal::Short);
    }
    if length > max {
        return Err(Refusal::Long);
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
    if letters < letters_min || words < words_min || longest > RUN_MAX {
        return Err(Refusal::Noise);
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

    #[test]
    fn notes_take_the_alphabet_with_shorter_bounds() {
        assert_eq!(note_text(" too  shiny ").as_deref(), Ok("too shiny"));
        assert!(note_text("Where video ???").is_ok());
        assert!(note_text("ok").is_err());
        assert!(note_text("1234 5678").is_err());
        assert!(note_text("a \"quoted\" word").is_err());
        assert_eq!(
            field(" textures/mp/s_ylight_red ", 64),
            "textures/mp/s_ylight_red"
        );
        assert_eq!(field("models/x+y", 64), "");
        assert_eq!(field(&"a".repeat(65), 64), "");
    }
}
