//! The hub's wire format (`PROTOCOL.md` in Sol-Vulpes/SJK-hub): signed
//! requests, the JSON the hub answers with, and how a claim's name is matched
//! to the name the game shows.

use crate::keys::Identity;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// A player's public profile as the hub serves it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Profile {
    /// First 16 hex digits of the SHA-256 of the public key.
    pub key_id: String,
    /// The public key, unpadded base64url.
    pub key: String,
    /// Display name: the name the operator gave the key, else the in-game name it
    /// last wore; empty while the hub has seen neither.
    pub name: String,
    /// Free text the player wrote about themselves.
    pub bio: String,
    /// Whether the hub's operator vouches for this key.
    pub verified: bool,
    /// Registration time, unix seconds.
    pub created: i64,
    /// In-game names the key has worn, most recent first (absent from hubs older
    /// than the name history).
    #[serde(default)]
    pub names: Vec<WornName>,
    /// Medals the SJK team gave the key, in the hub's catalogue order (absent from
    /// hubs older than medals).
    #[serde(default)]
    pub medals: Vec<Medal>,
}

/// One medal the SJK team gave a key: recognition only, it grants nothing.
///
/// A profile carries every field; a presence entry only `id` and `count`, the rest
/// keeping their defaults. A client shows only the ids it knows.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Medal {
    /// The medal's id (`early_tester`, `bug_hunter`, ...).
    pub id: String,
    /// How many times it was given: 1 unless the medal is repeatable.
    #[serde(default = "one")]
    pub count: u32,
    /// When it was last given, unix seconds; 0 when not sent (presence).
    #[serde(default)]
    pub awarded: i64,
    /// Public plain text from the team about why, often empty. Never markup.
    #[serde(default)]
    pub note: String,
}

fn one() -> u32 {
    1
}

/// One in-game name a key has worn, as the hub saw it in registrations and claims.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WornName {
    /// The name with its colour codes.
    pub name: String,
    /// When the hub first saw it, unix seconds.
    pub first_seen: i64,
    /// When the hub last saw it, unix seconds.
    pub last_seen: i64,
}

/// One live claim on a game server and who made it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Presence {
    /// The game slot the claimant says they hold.
    pub slot: u8,
    /// The name the claimant says the game shows for them.
    pub claimed_name: String,
    /// The claimant's key id.
    pub key_id: String,
    /// The claimant's display name.
    pub name: String,
    /// Whether the hub's operator vouches for the claimant.
    pub verified: bool,
    /// The claimant's medals, ids and counts only (absent from hubs older than
    /// medals), so the scoreboard shows them without a request per player.
    #[serde(default)]
    pub medals: Vec<Medal>,
}

/// The text a request's signature covers.
pub fn signed_text(
    method: &str,
    path_and_query: &str,
    ts: i64,
    nonce: &str,
    body: &[u8],
) -> String {
    let hex: String = Sha256::digest(body)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("SJK-HUB-v1\n{method}\n{path_and_query}\n{ts}\n{nonce}\n{hex}")
}

/// The `Authorization` header value for a request signed by `identity`.
pub fn authorization(
    identity: &Identity,
    method: &str,
    path_and_query: &str,
    body: &[u8],
    ts: i64,
    nonce_bytes: [u8; 12],
) -> String {
    let nonce = B64.encode(nonce_bytes);
    let signature = identity.sign_text(&signed_text(method, path_and_query, ts, &nonce, body));
    format!(
        "SJK-Sig key={}, ts={ts}, nonce={nonce}, sig={signature}",
        identity.public_key_text()
    )
}

/// The form two player names are compared in: lower case, Quake colour codes
/// removed, only letters and digits kept (the hub uses the same rule for
/// display names).
pub fn normal_form(name: &str) -> String {
    let mut out = String::new();
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '^' {
            // `Q_IsColorString`: `^` plus any character but another `^`.
            if chars.peek().is_some_and(|&next| next != '^') {
                let _ = chars.next();
            }
        } else if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// Whether a claim's name and the name the game shows are one player's name.
///
/// The game server cleans names (length, spaces, duplicates), so the exact text
/// can differ; the normal forms must agree and must not be empty.
pub fn names_match(claimed: &str, shown: &str) -> bool {
    let claimed = normal_form(claimed);
    !claimed.is_empty() && claimed == normal_form(shown)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A request every implementation of `PROTOCOL.md` must sign to this exact
    /// header; the hub's tests carry the same vector.
    #[test]
    fn the_known_answer_header_matches_the_protocol() {
        let identity = Identity::from_seed([7; 32]);
        assert_eq!(
            authorization(
                &identity,
                "POST",
                "/v1/claim",
                br#"{"server":"1.2.3.4:29070","slot":3,"name":"Sol"}"#,
                1_000_000,
                [1; 12]
            ),
            KNOWN_ANSWER
        );
    }

    const KNOWN_ANSWER: &str = "SJK-Sig key=6kpsY-KcUgq-9VB7Ey7F-ZVHdq6-vnuSQh7qaRRG0iw, ts=1000000, nonce=AQEBAQEBAQEBAQEB, sig=IKLlpqaSWIXqXK7Q77SsGJiE4BIdUTjkSWv2zyr0951WKByhEShSL-Ow70Rgc3znokl0Sf4rQK2VjNkPEm9uCQ";

    #[test]
    fn names_match_through_colours_and_case() {
        assert!(names_match("^1Sol", "sol"));
        assert!(names_match("S.o.l", "^2SOL"));
        assert!(!names_match("Sol", "Fox"));
        assert!(!names_match("^1^2", "^3"));
    }

    #[test]
    fn profiles_and_presence_parse() {
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5}"#,
        )
        .unwrap();
        assert!(profile.verified);
        assert!(profile.names.is_empty(), "an older hub sends no history");
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":false,"created":5,
                "names":[{"name":"^1Sol","first_seen":1,"last_seen":9}]}"#,
        )
        .unwrap();
        assert_eq!(profile.names[0].name, "^1Sol");
        let presence: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false}"#,
        )
        .unwrap();
        assert_eq!(presence.slot, 3);
        assert!(presence.medals.is_empty(), "an older hub sends no medals");
    }

    #[test]
    fn medals_parse_in_profiles_and_presence() {
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5,
                "medals":[{"id":"early_tester","count":1,"awarded":1791000000,"note":""},
                          {"id":"bug_hunter","count":2,"awarded":1791300000,"note":"The ^1fog^7 bug"},
                          {"id":"from_the_future","count":1,"awarded":1,"note":""}]}"#,
        )
        .unwrap();
        assert!(profile.names.is_empty());
        let ids: Vec<&str> = profile
            .medals
            .iter()
            .map(|medal| medal.id.as_str())
            .collect();
        assert_eq!(ids, ["early_tester", "bug_hunter", "from_the_future"]);
        assert_eq!(profile.medals[1].count, 2);
        assert_eq!(profile.medals[1].awarded, 1_791_300_000);
        assert_eq!(profile.medals[1].note, "The ^1fog^7 bug");
        let presence: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false,
                "medals":[{"id":"early_tester","count":1},{"id":"jof_clan"}]}"#,
        )
        .unwrap();
        assert_eq!(
            presence.medals,
            [
                Medal {
                    id: "early_tester".to_owned(),
                    count: 1,
                    awarded: 0,
                    note: String::new(),
                },
                Medal {
                    id: "jof_clan".to_owned(),
                    count: 1,
                    awarded: 0,
                    note: String::new(),
                },
            ]
        );
    }
}
