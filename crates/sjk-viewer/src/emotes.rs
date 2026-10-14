//! Emotes (`docs/hub-chat.md`, "Emotes in the client"): short animations an SJK
//! player plays, which other SJK players on the same game server see. They travel
//! through the SJK hub, not the game server, so stock servers and clients know
//! nothing of them.
//!
//! This is the groundwork: the catalogue's format and loader, the `sjkemote` command
//! (named apart from the `emote` some server mods have),
//! and [`ActiveEmotes`], which says which player plays which emote right now. The
//! animations, sounds and art are separate work: an emote's torso and legs
//! animations and sound are named in its catalogue file, and the player's model
//! code reads [`active`] to play them. Until then a received emote is written to the
//! console.
//!
//! The catalogue is data in the game's file system (pk3s or the folder):
//! `emotes/<id>.emote`, a JSON object:
//!
//! ```json
//! {"name": "Wave", "torso": "BOTH_...", "legs": "", "length_ms": 2500,
//!  "loop": false, "sound": "sound/emotes/wave.mp3", "clip": "", "local": false,
//!  "moving": false, "icon": "gfx/emotes/wave.png"}
//! ```
//!
//! `clip` names an animation of a rig model (`docs/rig-models.md`): a player wearing
//! a rig that has it plays it on the rig's own skeleton. `rig` names the model folder
//! the clip comes from: other rig models with the same joint names borrow it. `local` keeps the emote on
//! this client: it plays here and is never sent to the hub (an emote made for a
//! model only this player has). `moving` keeps it playing while the player moves.
//! The sound plays on the player and stops when the emote is cut short. `icon` is a
//! picture (PNG, JPG or TGA) the quick wheel shows for it (`emote_icons.rs`). The id
//! [`STOP`] ends the player's emote.
//!
//! `<id>` is the emote's id at the hub: 1 to 32 of `a` to `z`, `0` to `9` and `_`.

use sjk_identity::Emote;
use std::time::{Duration, Instant};

/// Where the catalogue's files are.
pub(crate) const FOLDER: &str = "emotes";
/// The `emote` command.
pub(crate) const COMMAND: &str = "sjkemote";
/// The id that ends the player's emote (`sjkemote stop`), never a catalogue entry's.
pub(crate) const STOP: &str = "stop";
pub(crate) const HELP: &str =
    "Play an emote the SJK players on this server see (sjkemote alone lists them)";
/// How long an emote the catalogue does not know lasts.
const UNKNOWN_LENGTH: Duration = Duration::from_secs(3);
/// How far a player may move (game units) before their emote stops.
const MOVE_LIMIT: f32 = 24.0;
/// Slots the game numbers.
const SLOTS: usize = 64;

/// One emote of the catalogue.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EmoteDef {
    /// Its id at the hub, the file's name.
    pub(crate) id: String,
    /// What it is called.
    pub(crate) name: String,
    /// The torso animation (`BOTH_...` or `TORSO_...`), or empty.
    pub(crate) torso: String,
    /// The legs animation, or empty.
    pub(crate) legs: String,
    /// How long it plays.
    pub(crate) length: Duration,
    /// Whether it loops until the player moves.
    pub(crate) looping: bool,
    /// The sound it plays, or empty.
    pub(crate) sound: String,
    /// The rig model animation it plays, or empty.
    pub(crate) clip: String,
    /// Played on this client only, never sent to the hub.
    pub(crate) local: bool,
    /// Keeps playing while the player moves.
    pub(crate) moving: bool,
    /// Its picture, or empty.
    pub(crate) icon: String,
    /// The rig model folder (under `models/players`) its clip comes from, or empty.
    pub(crate) rig: String,
}

/// Whether `id` may name an emote at the hub.
pub(crate) fn valid_id(id: &str) -> bool {
    (1..=32).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// The fields a catalogue file may hold.
const FIELDS: [&str; 11] = [
    "name",
    "torso",
    "legs",
    "length_ms",
    "loop",
    "sound",
    "clip",
    "local",
    "moving",
    "icon",
    "rig",
];
/// The longest animation or sound name.
const PATH_MAX: usize = 64;
/// The shortest and longest emote that is not a loop, in milliseconds.
const LENGTH_MS: std::ops::RangeInclusive<u64> = 100..=60_000;

/// Read one catalogue file: `id` from its name, `text` its contents.
pub(crate) fn parse(id: &str, text: &str) -> Result<EmoteDef, String> {
    if !valid_id(id) {
        return Err(format!(
            "{id:?} is not an emote id (1 to 32 of a to z, 0 to 9 and _)"
        ));
    }
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("not JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "not a JSON object".to_owned())?;
    if let Some(unknown) = object.keys().find(|key| !FIELDS.contains(&key.as_str())) {
        return Err(format!("unknown field {unknown:?}"));
    }
    let text_field = |field: &str, max: usize| -> Result<String, String> {
        match object.get(field) {
            None => Ok(String::new()),
            Some(serde_json::Value::String(text))
                if text.chars().count() <= max && !text.chars().any(char::is_control) =>
            {
                Ok(text.clone())
            }
            Some(_) => Err(format!("{field} must be text of at most {max} characters")),
        }
    };
    let name = text_field("name", 32)?;
    if name.trim().is_empty() {
        return Err("name is needed".to_owned());
    }
    let flag = |field: &str| match object.get(field) {
        None => Ok(false),
        Some(serde_json::Value::Bool(value)) => Ok(*value),
        Some(_) => Err(format!("{field} must be true or false")),
    };
    let looping = flag("loop")?;
    let length = match object.get("length_ms") {
        None if looping => 0,
        Some(value) => value
            .as_u64()
            .filter(|ms| LENGTH_MS.contains(ms))
            .ok_or_else(|| "length_ms must be 100 to 60000".to_owned())?,
        None => return Err("length_ms is needed unless the emote loops".to_owned()),
    };
    Ok(EmoteDef {
        id: id.to_owned(),
        name,
        torso: text_field("torso", PATH_MAX)?,
        legs: text_field("legs", PATH_MAX)?,
        length: Duration::from_millis(length),
        looping,
        sound: text_field("sound", PATH_MAX)?,
        clip: text_field("clip", PATH_MAX)?,
        local: flag("local")?,
        moving: flag("moving")?,
        icon: text_field("icon", PATH_MAX)?,
        rig: {
            let rig = text_field("rig", PATH_MAX)?;
            if rig.contains(['/', '\\']) || rig.contains("..") {
                return Err("rig must be a model folder's name".to_owned());
            }
            rig
        },
    })
}

/// The catalogue in `vfs`: every `emotes/<id>.emote` that reads, by id; the ones that
/// do not are reported to the log by name.
pub(crate) fn load(vfs: &sjk_vfs::VirtualFileSystem) -> Vec<EmoteDef> {
    let mut catalogue = Vec::new();
    for listed in vfs.list_files(FOLDER, ".emote") {
        let path = format!("{FOLDER}/{listed}");
        let id = listed.trim_end_matches(".emote");
        let read = vfs
            .read(&path)
            .map_err(|error| error.to_string())
            .and_then(|asset| asset.ok_or_else(|| "missing".to_owned()))
            .and_then(|asset| String::from_utf8(asset.bytes).map_err(|_| "not UTF-8".to_owned()))
            .and_then(|text| parse(id, &text));
        match read {
            Ok(def) => catalogue.push(def),
            Err(why) => crate::log::progress(format_args!("emotes: {path}: {why}")),
        }
    }
    catalogue.retain(|def| {
        let reserved = def.id == STOP;
        if reserved {
            crate::log::progress(format_args!(
                "emotes: {FOLDER}/{STOP}.emote: the id is reserved"
            ));
        }
        !reserved
    });
    catalogue.sort_by(|a, b| a.id.cmp(&b.id));
    catalogue.dedup_by(|a, b| a.id == b.id);
    catalogue
}

/// One emote being played.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ActiveEmote {
    /// The emote's id.
    pub(crate) emote: String,
    pub(crate) started: Instant,
    /// When it ends, unless the player moves first; `None` while a loop plays.
    pub(crate) until: Option<Instant>,
    /// Where the player stood when it started.
    pub(crate) origin: [f32; 3],
    /// Its sound, or empty.
    pub(crate) sound: String,
    /// Whether it keeps playing while the player moves.
    pub(crate) moving: bool,
}

/// A sound to start on, or stop from, a player: what the audio does with emotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cue {
    Start { slot: u8, sound: String },
    Stop { slot: u8 },
}

/// Which player plays which emote, by slot.
pub(crate) struct ActiveEmotes {
    slots: [Option<ActiveEmote>; SLOTS],
    /// Sounds to start and stop, taken by the audio each frame.
    cues: Vec<Cue>,
}

impl Default for ActiveEmotes {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            cues: Vec::new(),
        }
    }
}

impl ActiveEmotes {
    /// The emote `slot` plays, if any.
    pub(crate) fn get(&self, slot: u8) -> Option<&ActiveEmote> {
        self.slots.get(usize::from(slot))?.as_ref()
    }

    /// `slot` starts `emote` at `now`, standing at `origin`; `def` is its catalogue
    /// entry if this client has it.
    pub(crate) fn start(
        &mut self,
        slot: u8,
        emote: &str,
        def: Option<&EmoteDef>,
        origin: [f32; 3],
        now: Instant,
    ) {
        let Some(entry) = self.slots.get_mut(usize::from(slot)) else {
            return;
        };
        let until = match def {
            Some(def) if def.looping => None,
            Some(def) => Some(now + def.length),
            None => Some(now + UNKNOWN_LENGTH),
        };
        if entry.as_ref().is_some_and(|old| !old.sound.is_empty()) {
            self.cues.push(Cue::Stop { slot });
        }
        let sound = def.map_or_else(String::new, |def| def.sound.clone());
        if !sound.is_empty() {
            self.cues.push(Cue::Start {
                slot,
                sound: sound.clone(),
            });
        }
        *entry = Some(ActiveEmote {
            emote: emote.to_owned(),
            started: now,
            until,
            origin,
            sound,
            moving: def.is_some_and(|def| def.moving),
        });
    }

    /// End `slot`'s emote now, and its sound.
    pub(crate) fn stop(&mut self, slot: u8) {
        if let Some(old) = self.slots.get_mut(usize::from(slot)).and_then(Option::take)
            && !old.sound.is_empty()
        {
            self.cues.push(Cue::Stop { slot });
        }
    }

    /// The sounds to start and stop since the last call.
    pub(crate) fn take_cues(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.cues)
    }

    /// End the emotes that are over at `now`, or whose player moved away (unless the
    /// emote keeps playing while moving) or left (`position` says where each slot's
    /// player is; `None` for one that left). An emote cut short stops its sound; one
    /// that ran its length lets the sound finish.
    pub(crate) fn update(&mut self, now: Instant, position: impl Fn(u8) -> Option<[f32; 3]>) {
        for (slot, entry) in self.slots.iter_mut().enumerate() {
            let Some(active) = entry else { continue };
            let Ok(slot) = u8::try_from(slot) else {
                continue;
            };
            let over = active.until.is_some_and(|until| now >= until);
            let gone = match position(slot) {
                None => true,
                Some(_) if active.moving => false,
                Some(at) => {
                    let d: f32 = at
                        .iter()
                        .zip(active.origin)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum();
                    d > MOVE_LIMIT * MOVE_LIMIT
                }
            };
            if gone && !active.sound.is_empty() {
                self.cues.push(Cue::Stop { slot });
            }
            if over || gone {
                *entry = None;
            }
        }
    }
}

/// The catalogue (once a game's files are mounted) and the emotes playing now.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) catalogue: Option<Vec<EmoteDef>>,
    pub(crate) active: ActiveEmotes,
    /// Whether the catalogue's sounds were handed to the audio to load.
    pub(crate) sounds_loaded: bool,
}

static STATE: std::sync::LazyLock<std::sync::Mutex<State>> =
    std::sync::LazyLock::new(std::sync::Mutex::default);

pub(crate) fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The emote the player in `slot` plays now, for the player's model code.
// The seam the stock animations and sounds plug into; rig clips use `playing`.
#[allow(dead_code)]
pub(crate) fn active(slot: u8) -> Option<ActiveEmote> {
    lock().active.get(slot).cloned()
}

/// A rig clip the player in `slot` plays at `now`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Playing {
    pub(crate) clip: String,
    /// Seconds since it started.
    pub(crate) seconds: f32,
    /// Seconds before it ends, unless the player moves first; `None` for a loop.
    pub(crate) remaining: Option<f32>,
    pub(crate) looping: bool,
}

/// The rig clip the player in `slot` plays at `now`, if their emote names one.
pub(crate) fn playing(slot: u8, now: Instant) -> Option<Playing> {
    let state = lock();
    let active = state.active.get(slot)?;
    let def = state
        .catalogue
        .as_ref()?
        .iter()
        .find(|def| def.id == active.emote)?;
    (!def.clip.is_empty()).then(|| Playing {
        clip: def.clip.clone(),
        seconds: now.saturating_duration_since(active.started).as_secs_f32(),
        remaining: active
            .until
            .map(|until| until.saturating_duration_since(now).as_secs_f32()),
        looping: def.looping,
    })
}

/// What `emote` alone prints: the catalogue, or how emotes come.
pub(crate) fn listing(catalogue: &[EmoteDef]) -> Vec<String> {
    if catalogue.is_empty() {
        return vec![
            "No emotes are installed yet (emotes/<id>.emote in the game's files).".to_owned(),
            "sjkemote <id> still sends any id to the SJK players here.".to_owned(),
        ];
    }
    catalogue
        .iter()
        .map(|def| {
            let length = if def.looping {
                "loops".to_owned()
            } else {
                format!("{:.1} s", def.length.as_secs_f32())
            };
            format!("{} - {}, {length}", def.id, def.name)
        })
        .chain(std::iter::once(format!("{COMMAND} {STOP} ends yours.")))
        .collect()
}

/// Whether to believe `emote`: the game shows `shown` in its slot, and the claim the
/// hub took the emote's slot from was made under that name (the badges' rule).
pub(crate) fn trusted(emote: &Emote, shown: Option<&str>) -> bool {
    shown.is_some_and(|shown| sjk_identity::names_match(&emote.claimed_name, shown))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAVE: &str = r#"{"name": "Wave", "torso": "BOTH_ENGAGETAUNT", "legs": "",
        "length_ms": 2500, "loop": false, "sound": "sound/emotes/wave.mp3"}"#;

    fn wave() -> EmoteDef {
        parse("wave", WAVE).unwrap()
    }

    #[test]
    fn a_rig_clip_emote_reads_its_clip_and_stays_local() {
        let def = parse(
            "kane_dance",
            r#"{"name": "Dance", "clip": "kane_dance", "length_ms": 3400, "local": true}"#,
        )
        .unwrap();
        assert_eq!((def.clip.as_str(), def.local), ("kane_dance", true));
        assert!(!wave().local && wave().clip.is_empty());
        assert!(parse("x", r#"{"name": "X", "length_ms": 500, "local": 1}"#).is_err());
    }

    #[test]
    fn an_emote_file_parses_and_bad_ones_are_named() {
        let def = wave();
        assert_eq!(def.id, "wave");
        assert_eq!(def.name, "Wave");
        assert_eq!(def.torso, "BOTH_ENGAGETAUNT");
        assert_eq!(def.length, Duration::from_millis(2500));
        assert!(!def.looping);
        assert_eq!(def.sound, "sound/emotes/wave.mp3");
        // Only the name is required; a loop needs no length.
        let sit = parse(
            "sit",
            r#"{"name": "Sit", "legs": "BOTH_SIT1", "loop": true}"#,
        )
        .unwrap();
        assert!(sit.looping && sit.torso.is_empty());
        for (id, text, why) in [
            ("Wave", WAVE, "id"),
            ("wave", "not json", "JSON"),
            ("wave", r#"{"torso": "X"}"#, "name"),
            ("wave", r#"{"name": "Wave"}"#, "length_ms"),
            ("wave", r#"{"name": "Wave", "length_ms": 50}"#, "length_ms"),
            (
                "wave",
                r#"{"name": "Wave", "length_ms": 1000, "extra": 1}"#,
                "extra",
            ),
        ] {
            let error = parse(id, text).unwrap_err();
            assert!(error.contains(why), "{id} {text}: {error}");
        }
    }

    #[test]
    fn the_listing_names_each_emote_or_says_none_is_installed() {
        assert!(listing(&[])[0].starts_with("No emotes are installed"));
        let sit = parse("sit", r#"{"name": "Sit", "loop": true}"#).unwrap();
        assert_eq!(
            listing(&[sit, wave()]),
            [
                "sit - Sit, loops",
                "wave - Wave, 2.5 s",
                "sjkemote stop ends yours."
            ]
        );
    }

    #[test]
    fn emote_ids_are_the_hubs() {
        assert!(valid_id("wave"));
        assert!(valid_id("sit_2"));
        assert!(!valid_id(""));
        assert!(!valid_id("Wave"));
        assert!(!valid_id(&"a".repeat(33)));
    }

    #[test]
    fn known_emotes_last_their_length_and_unknown_ones_three_seconds() {
        let t0 = Instant::now();
        let mut active = ActiveEmotes::default();
        active.start(3, "wave", Some(&wave()), [0.0; 3], t0);
        active.start(4, "dance", None, [0.0; 3], t0);
        let here = |_| Some([0.0; 3]);
        active.update(t0 + Duration::from_millis(2_400), here);
        assert_eq!(active.get(3).map(|e| e.emote.as_str()), Some("wave"));
        active.update(t0 + Duration::from_millis(2_600), here);
        assert!(active.get(3).is_none());
        assert!(active.get(4).is_some());
        active.update(t0 + Duration::from_secs(3), here);
        assert!(active.get(4).is_none());
    }

    #[test]
    fn a_moving_emote_survives_walking_and_sounds_start_and_stop() {
        let t0 = Instant::now();
        let dance = parse(
            "dance",
            r#"{"name": "Dance", "length_ms": 6800, "moving": true,
                "sound": "sound/emotes/dance.wav"}"#,
        )
        .unwrap();
        let mut active = ActiveEmotes::default();
        active.start(5, "dance", Some(&dance), [0.0; 3], t0);
        assert_eq!(
            active.take_cues(),
            vec![Cue::Start {
                slot: 5,
                sound: "sound/emotes/dance.wav".into()
            }]
        );
        active.update(t0 + Duration::from_secs(1), |_| Some([500.0, 0.0, 0.0]));
        assert!(active.get(5).is_some(), "it dances on while running");
        // Running its length lets the sound finish.
        active.update(t0 + Duration::from_secs(7), |_| Some([500.0, 0.0, 0.0]));
        assert!(active.get(5).is_none() && active.take_cues().is_empty());
        // Cut short (stopped, or replaced), the sound stops.
        active.start(5, "dance", Some(&dance), [0.0; 3], t0);
        active.start(5, "wave", Some(&wave()), [0.0; 3], t0);
        active.stop(5);
        assert_eq!(
            active.take_cues(),
            vec![
                Cue::Start {
                    slot: 5,
                    sound: "sound/emotes/dance.wav".into()
                },
                Cue::Stop { slot: 5 },
                Cue::Start {
                    slot: 5,
                    sound: "sound/emotes/wave.mp3".into()
                },
                Cue::Stop { slot: 5 },
            ]
        );
        assert!(active.get(5).is_none());
    }

    #[test]
    fn moving_or_leaving_ends_an_emote_and_a_loop_lasts_until_then() {
        let t0 = Instant::now();
        let sit = parse("sit", r#"{"name": "Sit", "loop": true}"#).unwrap();
        let mut active = ActiveEmotes::default();
        active.start(1, "sit", Some(&sit), [100.0, 0.0, 0.0], t0);
        active.start(2, "wave", Some(&wave()), [0.0; 3], t0);
        active.start(70, "wave", None, [0.0; 3], t0);
        assert!(active.get(70).is_none(), "no such slot");
        let later = t0 + Duration::from_secs(600);
        active.update(later, |slot| (slot == 1).then_some([110.0, 0.0, 0.0]));
        assert!(active.get(1).is_some(), "a loop, moved a little");
        assert!(active.get(2).is_none(), "left the server");
        active.update(later, |_| Some([130.0, 0.0, 0.0]));
        assert!(active.get(1).is_none(), "walked off");
    }

    #[test]
    fn an_emote_needs_the_slot_name_to_match_the_claim() {
        let emote = Emote {
            id: 1,
            at: 0,
            slot: 3,
            claimed_name: "^2Sol".to_owned(),
            key_id: "0123456789abcdef".to_owned(),
            emote: "wave".to_owned(),
        };
        assert!(trusted(&emote, Some("Sol")));
        assert!(
            !trusted(&emote, Some("Fox")),
            "someone else in the slot now"
        );
        assert!(!trusted(&emote, None), "nobody in the slot");
    }
}
