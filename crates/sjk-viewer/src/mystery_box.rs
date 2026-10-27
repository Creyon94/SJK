//! Mystery boxes (Sol's idea, 27/10/2026; `docs/mystery-boxes.md`): boxes SJK's creator
//! places on maps through the hub, which any SJK player opens for credits with `+use`.
//! The hub keeps where they stand, takes the price and rolls what is inside
//! (`sjk_identity::mystery_box`); this module draws the boxes standing on the loaded map
//! file, asks to open the one the player faces, shows and plays what came out, and shows
//! other players' openings (the hub's toy event). No game server knows of them: a box is
//! drawn by SJK clients only and has no collision.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::Vec3;
use sjk_identity::mystery_box::{self as wire, MysteryBox};
use sjk_identity::service::BoxDone;
use sjk_protocol::{PlayerState, UserCommand};
use std::time::{Duration, Instant};

/// The box's model: a retail crate until the box gets its own.
pub(crate) const MODEL: &str = "models/map_objects/imperial/crate_banded.md3";
/// Played when a box opens, for the opener and everyone near.
const OPEN: &str = "sound/effects/crate_slide.mp3";
/// Played over [`OPEN`] when the box paid credits back.
const WIN: &str = "sound/player/holocron.wav";
/// Played over [`OPEN`] for the jackpot.
const JACKPOT: &str = "sound/items/respawn1.wav";
/// Played over [`OPEN`] when the box was empty.
const EMPTY: &str = "sound/movers/doors/door_locked.mp3";
/// `BUTTON_USE`, `+use`.
const BUTTON_USE: u16 = 32;
/// Farthest a player may stand from a box to open it, across the floor.
const REACH: f32 = 80.0;
/// Farthest above or below the box a player may stand to open it.
const REACH_UP: f32 = 96.0;
/// Widest angle between the player's facing and the box, in radians (60 degrees).
const FACING: f32 = std::f32::consts::FRAC_PI_3;
/// How long an opened box hops and turns.
const HOP: Duration = Duration::from_millis(1_600);
/// How high it hops, in game units.
const HOP_HEIGHT: f32 = 18.0;
/// How long an opening waits for the hub before the player may ask again.
const ASK_TIMEOUT: Duration = Duration::from_secs(10);
/// Farthest a box `mysterybox_remove` takes without an id may stand.
const REMOVE_REACH: f32 = 256.0;

/// The creator's console commands, with their help.
pub(crate) const COMMANDS: &[(&str, &str)] = &[
    (
        "mysterybox_place",
        "SJK creator: place a mystery box where you aim, facing you ('server': for this server only)",
    ),
    (
        "mysterybox_remove",
        "SJK creator: remove the mystery box nearest you, or the one with the id given",
    ),
    (
        "mysterybox_list",
        "List the mystery boxes on this map, their ids and the price",
    ),
];

/// Whether `name` is one of [`COMMANDS`].
pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|(command, _)| *command == name)
}

/// One box standing on the loaded map.
#[derive(Clone, Debug, PartialEq)]
struct Here {
    id: i64,
    origin: Vec3,
    /// Facing, in degrees.
    yaw: f32,
    /// When it last opened, for its hop.
    opened: Option<Instant>,
}

/// A sound to play once.
struct Cue {
    path: &'static str,
    origin: [f32; 3],
}

/// A box the creator asked to place, waiting for the map's checksum.
struct PendingPlace {
    origin: Vec3,
    yaw: f32,
    /// `ip:port` for this server only, empty for every server.
    server: String,
}

/// The boxes of the loaded map, the opening under way and the sounds to play.
#[derive(Default)]
pub(crate) struct MysteryBoxes {
    /// The hub list's revision the boxes were taken from, and the map, checksum and
    /// server they were taken for.
    taken: Option<(u64, String, i32, Option<String>)>,
    here: Vec<Here>,
    /// What opening one costs, as the hub said.
    price: u64,
    /// The rigid mesh of [`MODEL`] once loaded, and the mesh count it was looked up at.
    mesh: Option<usize>,
    meshes_seen: usize,
    use_pressed: bool,
    use_held: bool,
    /// The box asked to open and when, until the hub answers.
    asked: Option<(i64, Instant)>,
    /// The last hub answer read.
    answer_seen: u64,
    /// The box the player was last told how to open, while they stay at it.
    hinted: Option<i64>,
    pending_place: Option<PendingPlace>,
    cues: Vec<Cue>,
    sounds_registered: Option<u64>,
}

impl MysteryBoxes {
    /// Note a `+use` press in `command` for the frame's logic.
    pub(crate) fn observe_use(&mut self, command: &UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
    }

    /// Whether [`MODEL`] must be loaded: a box stands here and its mesh is not there.
    pub(crate) fn wants_model(&self) -> bool {
        !self.here.is_empty() && self.mesh.is_none()
    }

    fn clear(&mut self) {
        self.taken = None;
        self.here.clear();
        self.asked = None;
        self.hinted = None;
        self.use_pressed = false;
    }

    /// Take the boxes standing on `map` of `checksum` on `server` from `boxes`.
    fn take(&mut self, boxes: &wire::Boxes, map: &str, checksum: i32, server: Option<&str>) {
        let old = std::mem::take(&mut self.here);
        self.price = boxes.price;
        self.here = boxes
            .boxes
            .iter()
            .filter(|placed| placed.stands_on(map, checksum, server))
            .map(|placed| Here {
                id: placed.id,
                origin: Vec3::from_array(placed.origin),
                yaw: placed.yaw,
                opened: old
                    .iter()
                    .find(|here| here.id == placed.id)
                    .and_then(|here| here.opened),
            })
            .collect();
        if self.here.is_empty() {
            self.hinted = None;
        }
    }

    fn opened(&mut self, id: i64, outcome: &str, won: u64, now: Instant) {
        let Some(here) = self.here.iter_mut().find(|here| here.id == id) else {
            return;
        };
        here.opened = Some(now);
        let origin = here.origin.to_array();
        self.cues.push(Cue { path: OPEN, origin });
        let over = match outcome {
            "jackpot" => JACKPOT,
            _ if won > 0 => WIN,
            _ => EMPTY,
        };
        self.cues.push(Cue { path: over, origin });
    }
}

/// The box among `boxes` the player at `from` facing `yaw` (radians) may open: within
/// [`REACH`] across the floor and [`REACH_UP`] up or down, within [`FACING`] of their
/// facing; the nearest of those.
fn pick_box<'a>(from: Vec3, yaw: f32, boxes: impl IntoIterator<Item = &'a Here>) -> Option<i64> {
    let facing = glam::Vec2::from_angle(yaw);
    boxes
        .into_iter()
        .filter_map(|here| {
            let way = (here.origin - from).truncate();
            let across = way.length();
            let up = (here.origin.z - from.z).abs();
            let faced = across < 1.0 || facing.angle_to(way).abs() <= FACING;
            (across <= REACH && up <= REACH_UP && faced).then_some((here.id, across))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// How an opening reads for the opener.
fn result_text(outcome: &str, price: u64, won: u64) -> String {
    match outcome {
        "empty" => format!("The box was empty: -{price} credits"),
        "refund" => format!("Your {price} credits back"),
        "double" => format!("Double! +{won} credits"),
        "jackpot" => format!("JACKPOT! +{won} credits"),
        _ if won > 0 => format!("+{won} credits"),
        _ => format!("Nothing inside: -{price} credits"),
    }
}

/// `pmtype_t`: the player plays (not dead, spectating or in intermission).
fn plays(player: &PlayerState) -> bool {
    /// `PMF_FOLLOW`: the player state is that of the player being followed.
    const PMF_FOLLOW: u16 = 4096;
    player.health() > 0
        && !matches!(player.movement_type(), 4 | 5 | 7 | 8)
        && player.movement_flags() & PMF_FOLLOW == 0
}

impl GpuState {
    fn mystery_box_print(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::Print,
            text,
            None,
            Instant::now(),
        );
    }

    fn mystery_box_centre(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text,
            None,
            Instant::now(),
        );
    }

    /// The frame's mystery box logic, after the looks: the boxes of the loaded map,
    /// the hub's answers, a `+use` press, the hint and other players' openings
    /// (`events`, this server's toy events).
    pub(crate) fn update_mystery_boxes(&mut self, events: &[sjk_identity::ToyEvent]) {
        let now = Instant::now();
        self.read_mystery_box_answer(now);
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.mystery_boxes.clear();
            return;
        };
        let server = session.server().to_string();
        // The map's checksum is read when a box may stand on it (`map_edits`).
        let map_name = crate::remap_blocked_maps::map_name(&self.world_load_map);
        let (revision, on_map) = crate::player_identity::with_snapshot(|snapshot| {
            (
                snapshot.mystery_boxes_revision,
                snapshot
                    .mystery_boxes
                    .as_ref()
                    .is_some_and(|boxes| boxes.on_map(map_name)),
            )
        })
        .unwrap_or_default();
        if on_map || self.mystery_boxes.pending_place.is_some() {
            self.map_edits.want_checksum(map_name);
        }
        let Some((map, checksum)) = self.map_edits.map().map(|(map, c)| (map.to_owned(), c)) else {
            self.mystery_boxes.here.clear();
            return;
        };
        self.place_pending_mystery_box(&map, checksum);
        let fresh = self.mystery_boxes.taken.as_ref().is_some_and(
            |(taken, taken_map, taken_checksum, taken_server)| {
                *taken == revision
                    && *taken_checksum == checksum
                    && *taken_map == map
                    && taken_server.as_deref() == Some(server.as_str())
            },
        );
        if !fresh {
            let boxes = crate::player_identity::with_snapshot(|snapshot| {
                snapshot.mystery_boxes.clone().unwrap_or_default()
            })
            .unwrap_or_default();
            self.mystery_boxes
                .take(&boxes, &map, checksum, Some(&server));
            self.mystery_boxes.taken = Some((revision, map, checksum, Some(server.clone())));
        }
        if self.mystery_boxes.meshes_seen != self.object_meshes.len() {
            self.mystery_boxes.meshes_seen = self.object_meshes.len();
            self.mystery_boxes.mesh = self.object_meshes.iter().position(|mesh| {
                mesh.appearance.variant.is_empty() && mesh.appearance.model == MODEL
            });
        }
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let own = u8::try_from(session.game_state().client_num).ok();
        // Other players' openings: the hub made the event, so it holds what came out.
        for event in events {
            if event.toy != wire::TOY || Some(event.slot) == own {
                continue;
            }
            if let Some((id, outcome)) = wire::parse_event(&event.event) {
                let won = u64::from(outcome != "empty");
                self.mystery_boxes.opened(id, outcome, won, now);
            }
        }
        if self.mystery_boxes.here.is_empty() {
            self.mystery_boxes.use_pressed = false;
            return;
        }
        let player = &session.latest_snapshot().player;
        if !plays(player) {
            self.mystery_boxes.use_pressed = false;
            self.mystery_boxes.hinted = None;
            return;
        }
        let from = self
            .local_prediction
            .predicted_state()
            .map_or_else(|| player.origin(), |state| state.origin);
        let at = pick_box(
            Vec3::from_array(from),
            self.camera_yaw,
            &self.mystery_boxes.here,
        );
        let pressed = std::mem::take(&mut self.mystery_boxes.use_pressed);
        if at != self.mystery_boxes.hinted {
            self.mystery_boxes.hinted = at;
            if at.is_some() {
                let price = self.mystery_boxes.price;
                self.mystery_box_centre(format!(
                    "Mystery box: press Use to open it ({price} credits)"
                ));
            }
        }
        let (Some(id), true) = (at, pressed) else {
            return;
        };
        if self
            .mystery_boxes
            .asked
            .is_some_and(|(_, asked)| now.duration_since(asked) < ASK_TIMEOUT)
        {
            return;
        }
        let credits = crate::player_identity::with_snapshot(|snapshot| {
            snapshot.me.as_ref().map(|me| me.credits)
        })
        .flatten();
        if credits.is_some_and(|credits| credits < self.mystery_boxes.price) {
            self.mystery_box_centre(format!(
                "Not enough credits: the box costs {}",
                self.mystery_boxes.price
            ));
            return;
        }
        if crate::player_identity::with_service(|service| service.open_mystery_box(id, server))
            .is_some()
        {
            self.mystery_boxes.asked = Some((id, now));
        }
    }

    /// Act on the hub's last answer to a mystery box request, once.
    fn read_mystery_box_answer(&mut self, now: Instant) {
        let seen = self.mystery_boxes.answer_seen;
        let Some((serial, outcome)) = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .mystery_box
                .as_ref()
                .filter(|(serial, _)| *serial != seen)
                .cloned()
        })
        .flatten() else {
            return;
        };
        self.mystery_boxes.answer_seen = serial;
        let asked = self.mystery_boxes.asked.take();
        match outcome {
            Ok(BoxDone::Opened(opened)) => {
                self.mystery_boxes
                    .opened(opened.box_id, &opened.outcome, opened.won, now);
                let text = result_text(&opened.outcome, opened.price, opened.won);
                self.mystery_box_centre(text.clone());
                self.mystery_box_print(format!(
                    "Mystery box: {text} ({} credits left)",
                    opened.balance
                ));
            }
            Ok(BoxDone::Placed(id)) => {
                self.mystery_box_print(format!("Mystery box {id} placed"));
            }
            Ok(BoxDone::Removed(id)) => {
                self.mystery_box_print(format!("Mystery box {id} removed"));
            }
            Err(why) if asked.is_some() => self.mystery_box_centre(why),
            Err(why) => self.mystery_box_print(format!("Mystery box: {why}")),
        }
    }

    /// Send the box the creator asked to place once the map's checksum is known.
    fn place_pending_mystery_box(&mut self, map: &str, checksum: i32) {
        let Some(pending) = self.mystery_boxes.pending_place.take() else {
            return;
        };
        let placed = MysteryBox {
            id: 0,
            map: map.to_owned(),
            checksum,
            server: pending.server,
            origin: pending.origin.to_array(),
            yaw: pending.yaw,
        };
        if crate::player_identity::with_service(|service| service.place_mystery_box(placed))
            .is_none()
        {
            self.mystery_box_print("Mystery box: not connected to the SJK hub".to_owned());
        }
    }

    /// `mysterybox_place`, `mysterybox_remove` and `mysterybox_list`.
    pub(crate) fn mystery_box_command(
        &mut self,
        name: &str,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        if name == "mysterybox_list" {
            return Ok(self.mystery_box_list());
        }
        if !crate::player_identity::is_creator() {
            return Err("Only SJK's creator places and removes mystery boxes".into());
        }
        let session = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .ok_or("Join a game server first: boxes stand on a server's map")?;
        let server = session.server().to_string();
        match name {
            "mysterybox_place" => {
                let one_server = match args.first().map(String::as_str) {
                    None => false,
                    Some("server") => true,
                    Some(other) => return Err(format!("mysterybox_place [server], not {other}")),
                };
                let (eye, forward) = self.world_notes.view().ok_or("No view to aim from")?;
                let hit = self
                    .decal_surfaces
                    .ray_hit(eye, forward, 4096.0)
                    .ok_or("Aim at the floor where the box should stand")?;
                let yaw = forward.y.atan2(forward.x).to_degrees() + 180.0;
                self.mystery_boxes.pending_place = Some(PendingPlace {
                    origin: eye + forward * hit.distance,
                    yaw: yaw.rem_euclid(360.0),
                    server: if one_server { server } else { String::new() },
                });
                let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
                self.map_edits.want_checksum(&map);
                Ok(vec![format!(
                    "Placing a mystery box on {map}{}...",
                    if one_server { " for this server" } else { "" }
                )])
            }
            "mysterybox_remove" => {
                let id = match args.first() {
                    Some(id) => id.parse().map_err(|_| format!("Not a box id: {id}"))?,
                    None => {
                        let from = Vec3::from_array(session.latest_snapshot().player.origin());
                        self.mystery_boxes
                            .here
                            .iter()
                            .map(|here| (here.id, here.origin.distance(from)))
                            .filter(|(_, distance)| *distance <= REMOVE_REACH)
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|(id, _)| id)
                            .ok_or("No mystery box near you: give its id (mysterybox_list)")?
                    }
                };
                crate::player_identity::with_service(|service| service.remove_mystery_box(id))
                    .ok_or("Not connected to the SJK hub")?;
                Ok(vec![format!("Removing mystery box {id}...")])
            }
            _ => Err(format!("Unknown command {name}")),
        }
    }

    fn mystery_box_list(&self) -> Vec<String> {
        let here = &self.mystery_boxes.here;
        if here.is_empty() {
            return vec!["No mystery box on this map".to_owned()];
        }
        let from = self
            .live_session
            .as_ref()
            .map(|session| Vec3::from_array(session.latest_snapshot().player.origin()));
        let mut lines = vec![format!(
            "{} mystery boxes here, {} credits to open:",
            here.len(),
            self.mystery_boxes.price
        )];
        for placed in here {
            let away = from.map_or(String::new(), |from| {
                format!(", {:.0} units away", placed.origin.distance(from))
            });
            lines.push(format!(
                "  {}: at {:.0} {:.0} {:.0}{away}",
                placed.id, placed.origin.x, placed.origin.y, placed.origin.z
            ));
        }
        lines
    }

    /// Add this frame's boxes to the scene: each standing, an opened one hopping and
    /// turning for [`HOP`].
    pub(crate) fn append_mystery_box_instances(&mut self) {
        let Some(mesh) = self.mystery_boxes.mesh else {
            return;
        };
        let now = Instant::now();
        let group = &mut self.object_groups[mesh];
        for here in &self.mystery_boxes.here {
            let t = here
                .opened
                .map(|opened| now.duration_since(opened).as_secs_f32() / HOP.as_secs_f32())
                .filter(|t| *t < 1.0);
            let (lift, turn) = t.map_or((0.0, 0.0), |t| {
                ((t * std::f32::consts::PI).sin() * HOP_HEIGHT, t * 360.0)
            });
            let origin = here.origin + Vec3::Z * lift;
            group.push(ActorInstance::new(
                origin.to_array(),
                sjk_client::legacy_angles_to_quaternion([0.0, here.yaw + turn, 0.0]),
                [1.0; 3],
            ));
        }
    }

    /// Play the boxes' one-shot sounds; the sounds are handed to the audio the first
    /// time there is one.
    pub(crate) fn play_mystery_box_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.mystery_boxes.cues.clear();
            return;
        };
        if self.mystery_boxes.cues.is_empty() {
            return;
        }
        if self.mystery_boxes.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [OPEN, WIN, JACKPOT, EMPTY]);
            self.mystery_boxes.sounds_registered = Some(audio.generation());
        }
        for cue in self.mystery_boxes.cues.drain(..) {
            audio.play_toy(
                crate::toy_sounds::Toy::MysteryBox,
                cue.path,
                1.0,
                Some(cue.origin),
                sjk_audio::SourceId(0),
                crate::audio::TOY_CHANNEL,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn here(id: i64, origin: [f32; 3]) -> Here {
        Here {
            id,
            origin: Vec3::from_array(origin),
            yaw: 0.0,
            opened: None,
        }
    }

    #[test]
    fn the_box_faced_within_reach_is_the_one_opened() {
        let boxes = [here(1, [60.0, 0.0, 0.0]), here(2, [40.0, 10.0, 0.0])];
        // Facing +x: the nearer of the two.
        assert_eq!(pick_box(Vec3::new(0.0, 0.0, 24.0), 0.0, &boxes), Some(2));
        // Facing away: neither.
        assert_eq!(
            pick_box(Vec3::new(0.0, 0.0, 24.0), std::f32::consts::PI, &boxes),
            None
        );
        // Too far across, or too far up.
        assert_eq!(
            pick_box(Vec3::new(-50.0, 0.0, 24.0), 0.0, &boxes[..1]),
            None
        );
        assert_eq!(pick_box(Vec3::new(0.0, 0.0, 200.0), 0.0, &boxes), None);
        // Standing on it: any facing.
        assert_eq!(
            pick_box(Vec3::new(60.0, 0.0, 40.0), 2.0, &boxes[..1]),
            Some(1)
        );
    }

    #[test]
    fn boxes_are_taken_for_their_map_file_and_server() {
        let mut state = MysteryBoxes::default();
        let placed = |id, map: &str, checksum, server: &str| MysteryBox {
            id,
            map: map.to_owned(),
            checksum,
            server: server.to_owned(),
            origin: [1.0, 2.0, 3.0],
            yaw: 90.0,
        };
        let boxes = wire::Boxes {
            boxes: vec![
                placed(1, "mp/ffa3", 5, ""),
                placed(2, "mp/ffa3", 6, ""),
                placed(3, "mp/ffa3", 5, "1.2.3.4:29070"),
                placed(4, "mp/ffa2", 5, ""),
            ],
            price: 100,
            ..Default::default()
        };
        state.take(&boxes, "mp/ffa3", 5, Some("5.6.7.8:29070"));
        let ids: Vec<i64> = state.here.iter().map(|here| here.id).collect();
        assert_eq!(ids, [1]);
        state.take(&boxes, "mp/ffa3", 5, Some("1.2.3.4:29070"));
        let ids: Vec<i64> = state.here.iter().map(|here| here.id).collect();
        assert_eq!(ids, [1, 3]);
        assert!(state.wants_model());
        assert_eq!(state.price, 100);
    }

    #[test]
    fn an_opening_hops_and_plays_what_came_out() {
        let mut state = MysteryBoxes {
            here: vec![here(7, [0.0; 3])],
            ..Default::default()
        };
        let now = Instant::now();
        state.opened(7, "jackpot", 500, now);
        assert_eq!(state.here[0].opened, Some(now));
        let paths: Vec<&str> = state.cues.iter().map(|cue| cue.path).collect();
        assert_eq!(paths, [OPEN, JACKPOT]);
        state.cues.clear();
        state.opened(7, "empty", 0, now);
        assert_eq!(state.cues[1].path, EMPTY);
        // A box not here: nothing.
        state.cues.clear();
        state.opened(8, "double", 200, now);
        assert!(state.cues.is_empty());
    }

    #[test]
    fn results_read_as_the_player_is_told() {
        assert_eq!(
            result_text("empty", 100, 0),
            "The box was empty: -100 credits"
        );
        assert_eq!(result_text("jackpot", 100, 500), "JACKPOT! +500 credits");
        assert_eq!(result_text("newer_thing", 100, 300), "+300 credits");
        assert_eq!(
            result_text("newer_thing", 100, 0),
            "Nothing inside: -100 credits"
        );
    }
}
