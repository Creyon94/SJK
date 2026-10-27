//! Mystery boxes (the hub's `PROTOCOL.md`, "Mystery boxes", since 27/10/2026): boxes
//! SJK's creator places on maps, which players open for credits. The hub keeps where
//! they are, takes the price and rolls what is inside; this crate reads the list, asks
//! to open one and carries the creator's placing. The viewer draws them
//! (`sjk-viewer`'s `mystery_box.rs`).

use crate::hub::HubError;
use crate::wire::Profile;
use serde::{Deserialize, Serialize};

/// The toy id of the feed event the hub makes when a box is opened.
pub const TOY: &str = "mystery_box";

/// One placed box.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MysteryBox {
    /// The hub's id, never given again.
    pub id: i64,
    /// The map's name as the game gives it (`mp/ffa3`).
    pub map: String,
    /// The map's BSP checksum: a box stands in one exact file.
    pub checksum: i32,
    /// `ip:port` for one server only, empty for every server.
    #[serde(default)]
    pub server: String,
    /// Where it stands, in game units.
    pub origin: [f32; 3],
    /// Facing, in degrees.
    pub yaw: f32,
}

impl MysteryBox {
    /// Whether the box stands on map `map` (any case) of checksum `checksum`, on `server`.
    pub fn stands_on(&self, map: &str, checksum: i32, server: Option<&str>) -> bool {
        self.checksum == checksum
            && self.map.eq_ignore_ascii_case(map)
            && (self.server.is_empty() || Some(self.server.as_str()) == server)
    }
}

/// One thing a box may hold, as the hub rolls it.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct Outcome {
    /// Its id (`empty`, `jackpot`, ...).
    pub id: String,
    /// How likely it is, per mille.
    pub per_mille: u32,
    /// The credits it pays.
    pub credits: u64,
}

/// `GET /v1/mystery-boxes`: every placed box, the price and the odds.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Boxes {
    /// Every placed box.
    pub boxes: Vec<MysteryBox>,
    /// What opening one costs, in credits.
    pub price: u64,
    /// Shortest time between two openings by one person, in seconds.
    #[serde(default)]
    pub cooldown: u64,
    /// What a box may hold.
    #[serde(default)]
    pub outcomes: Vec<Outcome>,
}

impl Boxes {
    /// Whether any box stands on a map called `map` (any checksum), so the viewer knows
    /// to read the loaded map's checksum.
    pub fn on_map(&self, map: &str) -> bool {
        self.boxes
            .iter()
            .any(|placed| placed.map.eq_ignore_ascii_case(map))
    }
}

/// One opening, as the opener is answered.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Opened {
    /// The box opened.
    #[serde(rename = "box")]
    pub box_id: i64,
    /// What was inside (`empty`, `refund`, `double`, `jackpot`, or a newer hub's).
    pub outcome: String,
    /// What it cost.
    pub price: u64,
    /// The credits it paid.
    pub won: u64,
    /// The opener's credits after.
    pub balance: u64,
}

/// `POST /v1/mystery-box/open`'s answer.
#[derive(Clone, Debug, Deserialize)]
pub struct OpenAnswer {
    /// What came out.
    pub opened: Opened,
    /// The opener's profile after.
    pub profile: Profile,
}

/// What a feed event of [`TOY`] says: box `box` gave `outcome` (`open_<box>_<outcome>`).
pub fn parse_event(event: &str) -> Option<(i64, &str)> {
    let rest = event.strip_prefix("open_")?;
    let (id, outcome) = rest.split_once('_')?;
    Some((id.parse().ok()?, outcome)).filter(|(_, outcome)| !outcome.is_empty())
}

/// Why an opening, a placing or a removal failed, in the player's words.
pub fn refusal(error: &HubError) -> String {
    let words = match error {
        HubError::Rejected { code, message, .. } => match code.as_str() {
            "not_enough_credits" => "Not enough credits.",
            "box_cooldown" => "The box is still closing.",
            "no_such_box" => "That box is not here any more.",
            "not_on_server" => "The hub does not see you on this server yet.",
            "opening_staff_only" => "Mystery boxes open with holocrons: soon.",
            "too_many_boxes" => "As many boxes are placed as the hub keeps: remove one first.",
            "creator_only" => "Only SJK's creator places boxes.",
            _ => return message.clone(),
        },
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub has no mystery boxes yet."
        }
        other => return other.to_string(),
    };
    words.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_reads_as_the_hub_sends_it() {
        let boxes: Boxes = serde_json::from_str(
            r#"{"boxes":[{"id":3,"map":"mp/ffa3","checksum":-1234,"server":"","origin":[1,2,3],
                "yaw":90,"placed":5}],"price":100,"cooldown":3,
                "outcomes":[{"id":"empty","per_mille":500,"credits":0}]}"#,
        )
        .unwrap();
        assert_eq!(boxes.price, 100);
        let placed = &boxes.boxes[0];
        assert!(placed.stands_on("MP/FFA3", -1234, Some("1.2.3.4:29070")));
        assert!(!placed.stands_on("mp/ffa3", 7, None));
        assert!(boxes.on_map("mp/ffa3") && !boxes.on_map("mp/ffa2"));
        let one = MysteryBox {
            server: "1.2.3.4:29070".to_owned(),
            ..placed.clone()
        };
        assert!(one.stands_on("mp/ffa3", -1234, Some("1.2.3.4:29070")));
        assert!(!one.stands_on("mp/ffa3", -1234, Some("5.6.7.8:29070")));
        assert!(!one.stands_on("mp/ffa3", -1234, None));
    }

    #[test]
    fn feed_events_name_the_box_and_what_came_out() {
        assert_eq!(parse_event("open_12_jackpot"), Some((12, "jackpot")));
        assert_eq!(parse_event("open_12_big_win"), Some((12, "big_win")));
        assert_eq!(parse_event("open_x_empty"), None);
        assert_eq!(parse_event("open_12_"), None);
        assert_eq!(parse_event("spotted"), None);
    }
}
