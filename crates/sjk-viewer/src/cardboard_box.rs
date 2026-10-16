//! The cardboard box, SJK's own toy (`toy_box`; Sol's idea, 16/10/2026): a shipping
//! carton the player hides in, Metal Gear style. While it is on, the player crouches
//! in it (the user commands hold crouch, [`CardboardBox::shape_command`]) and its
//! music plays on them. No game server knows of it: it travels through the SJK hub as
//! a toy in the player's look (`looks.rs`, [`TOY`]), so other SJK players on the server
//! see the box over that player and hear the music; players on stock clients see a
//! crouching player.
//!
//! Another SJK player who presses `+use` facing a boxed player within reach lifts the
//! box ([`pick_target`]): the "spotted" sound plays, the box goes at once on the
//! lifter's screen, and the hub relays a toy event ([`EVENT_SPOTTED`]) on which the
//! boxed player's client takes it off (and tells its look) and every other SJK client
//! hides it and plays the sound. A player who dies in the box plays its death sound
//! where they fell, on every SJK client that sees it; the box is lost with the life
//! (off at the respawn). It is out at every start, like Illuminate.
//!
//! The carton's model, picture, shader and icon are bundled ([`FILES`], made by
//! `scripts/cardboard_box_assets.py`) and mounted below all game data, so a PK3 with
//! the same paths replaces them. The sounds ([`MUSIC`], [`SPOTTED`], [`DEATH`]) are
//! game paths; where they are missing nothing plays.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::{Quat, Vec2, Vec3};
use sjk_protocol::{EntityState, PlayerState, UserCommand};
use sjk_vfs::{VfsError, VirtualFileSystem};
use std::time::{Duration, Instant};

/// The toy's console command (a toy is a `toy_<name>` command).
pub(crate) const COMMAND: &str = "toy_box";
/// The toy's id in a look's `toys` and in toy events (`PROTOCOL.md`, "Looks").
pub(crate) const TOY: &str = "cardboard_box";
/// The toy event of a box lifted off a player.
pub(crate) const EVENT_SPOTTED: &str = "spotted";
pub(crate) const MODEL: &str = "models/sjk/cardboard_box.md3";
/// The music that plays on a boxed player, looping.
pub(crate) const MUSIC: &str = "sound/sjk/cardboard_box/music.wav";
/// The sound of a box lifted off a player.
pub(crate) const SPOTTED: &str = "sound/sjk/cardboard_box/spotted.wav";
/// The sound of a player who dies in the box.
pub(crate) const DEATH: &str = "sound/sjk/cardboard_box/death.wav";

/// A bundled file's bytes, by its name in `assets/cardboard_box`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/cardboard_box/", $name))
    };
}

/// The bundled files at their game paths.
const FILES: [(&str, &[u8]); 4] = [
    (MODEL, bundled!("cardboard_box.md3")),
    (
        "models/sjk/cardboard_box.jpg",
        bundled!("cardboard_box.jpg"),
    ),
    (
        "shaders/sjk_cardboard_box.shader",
        bundled!("cardboard_box.shader"),
    ),
    ("gfx/sjk/toy_box.png", bundled!("toy_box.png")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), VfsError> {
    vfs.mount_memory("SJK cardboard box", FILES.iter().copied())?;
    Ok(())
}

const SLOTS: usize = crate::looks::SLOTS;
/// `BUTTON_USE`, `+use`.
const BUTTON_USE: u16 = 32;
/// The crouch a held `+movedown` sends.
const CROUCH: i8 = -127;
/// How far from the lifter's origin a box can be lifted: the stock `+use` reach
/// (`USE_DISTANCE` 64, `g_active.c`) and a little, as the box is wider than the player.
pub(crate) const REACH: f32 = 96.0;
/// How far off the lifter's facing a box can be lifted: the cosine of 60 degrees.
const FACING: f32 = 0.5;
/// How long a box lifted here stays hidden while the boxed player's look still says it
/// is on (their client takes it off on the hub's event and tells its look): past this
/// it shows again, as their look says.
const LIFT_HOLD: Duration = Duration::from_secs(15);
/// From the player's origin down to their feet (`DEFAULT_MINS_2`).
const FEET: f32 = -24.0;
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
const EF_NODRAW: u32 = 1 << 8;
const PW_CLOAKED: u32 = 1 << 11;
/// `PMF_FOLLOW`: the player state is that of the player being followed.
const PMF_FOLLOW: u16 = 4096;

/// A one-shot sound to play: on `slot`, at `origin` (`None`: the player's own, in both
/// ears).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cue {
    pub(crate) path: &'static str,
    pub(crate) slot: u8,
    pub(crate) origin: Option<[f32; 3]>,
}

/// The music on a boxed player this frame: `slot`, at `origin` (`None`: the player's
/// own, in both ears).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Music {
    pub(crate) slot: u8,
    pub(crate) origin: Option<[f32; 3]>,
}

/// The toy's state: the player's own box, the boxes lifted here, who was seen dead,
/// and the sounds to play. Fixed-size but for the two short lists, which keep their
/// room: a frame allocates nothing.
pub(crate) struct CardboardBox {
    /// The player's own box is on.
    on: bool,
    /// The user commands hold crouch: the box is on and the player plays.
    crouching: bool,
    /// Whether the player was alive at the last frame (`None` out of a game).
    own_alive: Option<bool>,
    /// `+use` went down in a user command since the last frame.
    use_pressed: bool,
    /// `+use` was down in the last user command.
    use_held: bool,
    /// Other players' boxes lifted here, and when.
    lifted: [Option<Instant>; SLOTS],
    /// The slots seen dead in the last snapshot (bits).
    dead: u64,
    /// Sounds to play, taken by the audio each frame.
    cues: Vec<Cue>,
    /// The music this frame.
    music: Vec<Music>,
    /// The sounds were handed to the audio.
    sounds_registered: bool,
    /// Cartons to draw without a session, for the world shots: origin and yaw.
    #[cfg(test)]
    pub(crate) shot_cartons: Vec<([f32; 3], f32)>,
}

impl Default for CardboardBox {
    fn default() -> Self {
        Self {
            on: false,
            crouching: false,
            own_alive: None,
            use_pressed: false,
            use_held: false,
            lifted: [None; SLOTS],
            dead: 0,
            cues: Vec::with_capacity(4),
            music: Vec::with_capacity(8),
            sounds_registered: false,
            #[cfg(test)]
            shot_cartons: Vec::new(),
        }
    }
}

impl CardboardBox {
    /// `toy_box`: put the box on, or take it off.
    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// Put it on (`true`) or take it off.
    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Whether the player's own box is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Hold crouch in `command` while the box is on and the player plays, as a held
    /// `+movedown` would (over a jump), and note a `+use` press for [`Self::update`].
    pub(crate) fn shape_command(&mut self, command: &mut UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
        if self.crouching {
            command.up_move = CROUCH;
        }
    }

    /// Hold crouch in a command that is only shown, never sent (the prediction's
    /// preview between commands).
    pub(crate) fn shape_preview(&self, command: &mut UserCommand) {
        if self.crouching {
            command.up_move = CROUCH;
        }
    }

    /// The sounds to play since the last call.
    pub(crate) fn take_cues(&mut self) -> std::vec::Drain<'_, Cue> {
        self.cues.drain(..)
    }

    /// The music on boxed players this frame.
    pub(crate) fn music(&self) -> &[Music] {
        &self.music
    }

    /// Whether the box of `slot` (not the player's own) was lifted here and is hidden
    /// while their look catches up.
    pub(crate) fn lifted(&self, slot: usize) -> bool {
        self.lifted.get(slot).is_some_and(Option::is_some)
    }

    /// The player's own life: alive (`Some(true)`), dead, or out of a game (`None`).
    /// Dying in the box plays its death sound; the box is lost with the life, off when
    /// the player is alive again.
    fn own_life(&mut self, alive: Option<bool>, slot: Option<u8>) {
        match (self.own_alive, alive) {
            (Some(true), Some(false)) if self.on => self.cues.push(Cue {
                path: DEATH,
                slot: slot.unwrap_or_default(),
                origin: None,
            }),
            (Some(false), Some(true)) if self.on => self.on = false,
            _ => {}
        }
        self.own_alive = alive;
        self.crouching = self.on && alive == Some(true);
    }

    /// A toy event relayed for this server, from `sender` (whose claim matches what the
    /// game shows) about `target`; `own` is the player's slot, `origin` where the target
    /// stands if the game draws them, `boxed` whether this client shows their box.
    fn spotted(
        &mut self,
        target: u8,
        own: Option<u8>,
        origin: Option<[f32; 3]>,
        boxed: bool,
        now: Instant,
    ) {
        if Some(target) == own {
            if self.on {
                self.on = false;
                self.crouching = false;
                self.cues.push(Cue {
                    path: SPOTTED,
                    slot: target,
                    origin: None,
                });
            }
            return;
        }
        let Some(lifted) = self.lifted.get_mut(usize::from(target)) else {
            return;
        };
        if boxed && lifted.is_none() {
            self.cues.push(Cue {
                path: SPOTTED,
                slot: target,
                origin: origin.or(Some([0.0; 3])),
            });
        }
        *lifted = Some(now);
    }

    /// Lift the box of `target` here, standing at `origin`: the sound, and the box goes.
    fn lift(&mut self, target: u8, origin: [f32; 3], now: Instant) {
        if let Some(lifted) = self.lifted.get_mut(usize::from(target)) {
            *lifted = Some(now);
            self.cues.push(Cue {
                path: SPOTTED,
                slot: target,
                origin: Some(origin),
            });
        }
    }

    /// Forget a lift once the slot's look no longer has the box (`worn` says who has it
    /// on), or after [`LIFT_HOLD`].
    fn expire_lifts(&mut self, worn: impl Fn(usize) -> bool, now: Instant) {
        for (slot, lifted) in self.lifted.iter_mut().enumerate() {
            if lifted.is_some_and(|at| !worn(slot) || now.saturating_duration_since(at) > LIFT_HOLD)
            {
                *lifted = None;
            }
        }
    }

    /// Note who is dead in this snapshot (`dead` bits over the slots present in
    /// `seen`): a boxed player (`boxed`) newly dead plays the death sound at `origin`.
    fn deaths(
        &mut self,
        seen: u64,
        dead: u64,
        boxed: impl Fn(usize) -> bool,
        origin: impl Fn(usize) -> [f32; 3],
    ) {
        let newly = dead & !self.dead & seen;
        for slot in 0..SLOTS {
            if newly & (1 << slot) != 0 && boxed(slot) {
                self.cues.push(Cue {
                    path: DEATH,
                    slot: slot as u8,
                    origin: Some(origin(slot)),
                });
            }
        }
        self.dead = (self.dead & !seen) | (dead & seen);
    }

    /// Every other player's box is forgotten: no game, or another server.
    fn clear_others(&mut self) {
        self.lifted = [None; SLOTS];
        self.dead = 0;
    }
}

/// The player `+use` lifts the box of, if any: the nearest of `boxed` (slot and origin)
/// within [`REACH`] of `origin` and in front of `yaw` (radians), on the level plane.
pub(crate) fn pick_target(
    origin: Vec3,
    yaw: f32,
    boxed: impl IntoIterator<Item = (u8, Vec3)>,
) -> Option<(u8, Vec3)> {
    let facing = Vec2::from_angle(yaw);
    boxed
        .into_iter()
        .filter_map(|(slot, at)| {
            let distance = at.distance(origin);
            let flat = (at - origin).truncate().normalize_or_zero();
            (distance <= REACH && (flat == Vec2::ZERO || flat.dot(facing) >= FACING))
                .then_some((distance, slot, at))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, slot, at)| (slot, at))
}

/// Whether the game draws a player entity: alive, not hidden and not cloaked.
fn drawn(entity: &EntityState) -> bool {
    entity.entity_type() == ET_PLAYER
        && entity.e_flags() & (EF_DEAD | EF_NODRAW) == 0
        && entity.powerups() & PW_CLOAKED == 0
}

/// Whether the player whose state is `player` is alive and in the match (not
/// spectating, nor at the intermission).
fn alive(player: &PlayerState) -> bool {
    // `pmtype_t`: PM_SPECTATOR 4, PM_DEAD 5, PM_INTERMISSION 7, PM_SPINTERMISSION 8.
    player.health() > 0 && !matches!(player.movement_type(), 4 | 5 | 7 | 8)
}

/// The carton over a player standing at `origin` (their entity's), facing `yaw`
/// (radians).
fn carton(origin: [f32; 3], yaw: f32) -> ActorInstance {
    ActorInstance::new(
        [origin[0], origin[1], origin[2] + FEET],
        Quat::from_rotation_z(yaw).to_array(),
        [1.0; 3],
    )
}

impl GpuState {
    /// `toy_box`: put the box on, or take it off.
    pub(crate) fn toggle_cardboard_box(&mut self) {
        self.cardboard_box.toggle();
    }

    /// The frame's toy logic, after the looks: the own life, the hub's events, a
    /// `+use` press and the deaths in the box.
    pub(crate) fn update_cardboard_box(&mut self, now: Instant) {
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.cardboard_box.own_life(None, None);
            self.cardboard_box.use_pressed = false;
            self.cardboard_box.clear_others();
            return;
        };
        let snapshot = session.latest_snapshot();
        let game_state = session.game_state();
        let own = u8::try_from(game_state.client_num).ok();
        let player = &snapshot.player;
        let own_playing = alive(player) && player.movement_flags() & PMF_FOLLOW == 0;
        let spectating = player.movement_flags() & PMF_FOLLOW != 0
            || matches!(player.movement_type(), 4 | 7 | 8);
        self.cardboard_box
            .own_life((!spectating).then_some(own_playing), own);
        let world = &self.live_world;
        let presentation = i64::from(snapshot.server_time);
        let origin_of = |slot: usize| {
            world
                .entity(sjk_runtime::EntityId::new(slot as u64 + 1))
                .map(|entity| entity.sample(presentation).translation)
        };
        let looks = &self.looks;
        let boxed = |slot: usize| Some(slot) != own.map(usize::from) && looks.toy(slot, TOY);
        // The hub's events: a sender counts only when the game shows the name they claimed.
        for event in crate::player_identity::take_toy_events(Some(session.server())) {
            if event.toy != TOY || event.event != EVENT_SPOTTED || Some(event.slot) == own {
                continue;
            }
            let shown = crate::player_identity::shown_name(game_state, usize::from(event.slot));
            if !shown.is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown)) {
                continue;
            }
            let target = usize::from(event.target);
            let shows = boxed(target) && !self.cardboard_box.lifted(target);
            self.cardboard_box
                .spotted(event.target, own, origin_of(target), shows, now);
        }
        // `+use` facing a boxed player within reach lifts the box.
        if std::mem::take(&mut self.cardboard_box.use_pressed) && own_playing {
            let from = self
                .local_prediction
                .predicted_state()
                .map_or_else(|| player.origin(), |state| state.origin);
            let candidates = snapshot.entities.iter().filter_map(|entity| {
                let slot = usize::from(entity.number());
                (slot < SLOTS && boxed(slot) && !self.cardboard_box.lifted(slot) && drawn(entity))
                    .then(|| Some((slot as u8, Vec3::from_array(origin_of(slot)?))))
                    .flatten()
            });
            if let Some((target, at)) =
                pick_target(Vec3::from_array(from), self.camera_yaw, candidates)
            {
                self.cardboard_box.lift(target, at.to_array(), now);
                crate::player_identity::toy_event(TOY, EVENT_SPOTTED, target);
            }
        }
        // Deaths in the box, of the players in this snapshot.
        let (mut seen, mut dead) = (0_u64, 0_u64);
        for entity in &snapshot.entities {
            let slot = usize::from(entity.number());
            if slot < SLOTS && entity.entity_type() == ET_PLAYER {
                seen |= 1 << slot;
                if entity.e_flags() & EF_DEAD != 0 {
                    dead |= 1 << slot;
                }
            }
        }
        let lifted = self.cardboard_box.lifted;
        self.cardboard_box.deaths(
            seen,
            dead,
            |slot| boxed(slot) && lifted[slot].is_none(),
            |slot| origin_of(slot).unwrap_or_default(),
        );
        self.cardboard_box.expire_lifts(boxed, now);
    }

    /// Add this frame's cartons, and note the music: the player's own (drawn in mirrors
    /// only in first person), then every other boxed player the game draws.
    pub(crate) fn submit_cardboard_boxes(&mut self, presentation_time: i64) {
        self.cardboard_box.music.clear();
        let mesh = self
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance.model == MODEL);
        #[cfg(test)]
        if let Some(mesh) = mesh {
            for &(origin, yaw) in &self.cardboard_box.shot_cartons {
                self.object_groups[mesh].push(carton(origin, yaw));
            }
        }
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            return;
        };
        let snapshot = session.latest_snapshot();
        let slots = crate::looks::ViewSlots::of(session.game_state(), &snapshot.player);
        let followed = slots.followed().filter(|&slot| slot < SLOTS);
        let first_person = !self.third_person;
        let mut cartons = [None::<(usize, ActorInstance)>; SLOTS];
        if self.cardboard_box.crouching
            && let Some(own) = slots.own.filter(|&slot| slot < SLOTS)
        {
            let origin = self
                .local_prediction
                .predicted_state()
                .map_or_else(|| snapshot.player.origin(), |state| state.origin);
            let mut instance = carton(origin, self.camera_yaw);
            if first_person {
                instance.view_flags |= 1;
            }
            cartons[own] = Some((own, instance));
            self.cardboard_box.music.push(Music {
                slot: own as u8,
                origin: None,
            });
        }
        let lifted = self.cardboard_box.lifted;
        let looks = &self.looks;
        let shows =
            |slot: usize| Some(slot) != slots.own && looks.toy(slot, TOY) && lifted[slot].is_none();
        // The player the view follows: their entity is not in the snapshot.
        if let Some(slot) = followed
            && shows(slot)
            && alive(&snapshot.player)
        {
            let origin = snapshot.player.origin();
            let mut instance = carton(origin, snapshot.player.view_angles()[1].to_radians());
            if first_person {
                instance.view_flags |= 1;
            }
            cartons[slot] = Some((slot, instance));
            self.cardboard_box.music.push(Music {
                slot: slot as u8,
                origin: Some(origin),
            });
        }
        for entity in &snapshot.entities {
            let slot = usize::from(entity.number());
            if slot >= SLOTS || Some(slot) == followed || !shows(slot) || !drawn(entity) {
                continue;
            }
            let Some(presented) = self
                .live_world
                .entity(sjk_runtime::EntityId::new(slot as u64 + 1))
            else {
                continue;
            };
            let origin = presented.sample(presentation_time).translation;
            let yaw = presented
                .sample_pose(presentation_time)
                .map_or(0.0, |pose| pose.view_angles_degrees[1]);
            cartons[slot] = Some((slot, carton(origin, yaw.to_radians())));
            self.cardboard_box.music.push(Music {
                slot: slot as u8,
                origin: Some(origin),
            });
        }
        if let Some(mesh) = mesh {
            for (_, instance) in cartons.into_iter().flatten() {
                self.object_groups[mesh].push(instance);
            }
        }
    }

    /// Play the box's one-shot sounds; the sounds are handed to the audio the first time
    /// there is one.
    pub(crate) fn play_cardboard_box_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.cardboard_box.cues.clear();
            return;
        };
        if !self.cardboard_box.sounds_registered
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [MUSIC, SPOTTED, DEATH]);
            self.cardboard_box.sounds_registered = true;
        }
        for cue in self.cardboard_box.take_cues() {
            let source = sjk_audio::SourceId(u32::from(cue.slot));
            match cue.origin {
                Some(origin) => {
                    audio.play_on_channel(cue.path, 1.0, origin, source, crate::audio::TOY_CHANNEL)
                }
                None => audio.play_local(cue.path, 1.0, source, crate::audio::TOY_CHANNEL),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_files_mount_and_the_model_is_an_open_carton_on_the_floor() {
        let mut vfs = VirtualFileSystem::new();
        mount(&mut vfs).unwrap();
        for (path, _) in FILES {
            assert!(vfs.read(path).unwrap().is_some(), "{path}");
        }
        let model = sjk_model::Md3::parse(&vfs.read(MODEL).unwrap().unwrap().bytes).unwrap();
        let surface = &model.surfaces[0];
        assert_eq!(surface.shaders, ["models/sjk/cardboard_box"]);
        // Four sides and the top, no bottom.
        assert_eq!((surface.frames[0].len(), surface.triangles.len()), (20, 10));
        let (mut low, mut high) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for vertex in &surface.frames[0] {
            let position = Vec3::from_array(vertex.position);
            low = low.min(position);
            high = high.max(position);
        }
        // Wider and taller than a crouched player (30 x 30 x 40), from the floor up.
        assert!((low.z).abs() < 1e-3 && high.z > 40.0, "{low} {high}");
        assert!(low.x < -15.0 && high.x > 15.0 && low.y < -15.0 && high.y > 15.0);
        // Clockwise seen from outside, stock's front side.
        let centre = Vec3::new(0.0, 0.0, high.z / 2.0);
        for [a, b, c] in &surface.triangles {
            let [a, b, c] =
                [a, b, c].map(|&i| Vec3::from_array(surface.frames[0][i as usize].position));
            let facing = (b - a).cross(c - a);
            assert!(facing.dot((a + b + c) / 3.0 - centre) < 0.0);
        }
        let icon = image::load_from_memory(FILES[3].1).unwrap();
        assert_eq!((icon.width(), icon.height()), (128, 128));
    }

    #[test]
    fn the_box_holds_crouch_only_while_on_and_playing_and_notes_use_presses() {
        let mut toy = CardboardBox::default();
        let mut command = UserCommand {
            up_move: 127,
            ..UserCommand::default()
        };
        toy.own_life(Some(true), Some(2));
        toy.shape_command(&mut command);
        assert_eq!(command.up_move, 127, "off: the jump goes");
        toy.toggle();
        toy.own_life(Some(true), Some(2));
        toy.shape_command(&mut command);
        assert_eq!(command.up_move, CROUCH, "on: crouch over the jump");
        // Dead, or out of a game, nothing is held.
        toy.own_life(None, None);
        command.up_move = 0;
        toy.shape_command(&mut command);
        assert_eq!(command.up_move, 0);
        // A held `+use` is one press.
        command.buttons = BUTTON_USE;
        toy.shape_command(&mut command);
        assert!(std::mem::take(&mut toy.use_pressed));
        toy.shape_command(&mut command);
        assert!(!toy.use_pressed);
        command.buttons = 0;
        toy.shape_command(&mut command);
        command.buttons = BUTTON_USE;
        toy.shape_command(&mut command);
        assert!(toy.use_pressed);
    }

    #[test]
    fn dying_in_the_box_plays_its_sound_and_the_box_is_lost_at_the_respawn() {
        let mut toy = CardboardBox::default();
        toy.set_on(true);
        toy.own_life(Some(true), Some(4));
        toy.own_life(Some(false), Some(4));
        let cues: Vec<_> = toy.take_cues().collect();
        assert_eq!(
            cues,
            [Cue {
                path: DEATH,
                slot: 4,
                origin: None
            }]
        );
        assert!(
            toy.on(),
            "kept while dead, so others still see whose death it was"
        );
        assert!(!toy.crouching);
        toy.own_life(Some(true), Some(4));
        assert!(!toy.on() && !toy.crouching);
        // Without the box, a death is silent.
        toy.own_life(Some(false), Some(4));
        assert_eq!(toy.take_cues().count(), 0);
    }

    #[test]
    fn a_spotted_event_takes_the_own_box_off_or_hides_anothers() {
        let now = Instant::now();
        let mut toy = CardboardBox::default();
        toy.set_on(true);
        toy.own_life(Some(true), Some(1));
        toy.spotted(1, Some(1), None, false, now);
        assert!(!toy.on() && !toy.crouching);
        assert_eq!(toy.take_cues().next().unwrap().path, SPOTTED);
        // Again (a repeat): the box is already off, nothing plays.
        toy.spotted(1, Some(1), None, false, now);
        assert_eq!(toy.take_cues().count(), 0);
        // Another player's: hidden, the sound where they stand, once.
        toy.spotted(6, Some(1), Some([1.0, 2.0, 3.0]), true, now);
        assert!(toy.lifted(6));
        let cue = toy.take_cues().next().unwrap();
        assert_eq!((cue.slot, cue.origin), (6, Some([1.0, 2.0, 3.0])));
        toy.spotted(6, Some(1), Some([1.0, 2.0, 3.0]), true, now);
        assert_eq!(toy.take_cues().count(), 0, "lifted here already");
        // The lift ends when their look drops the box, or after the hold.
        toy.expire_lifts(|_| true, now + Duration::from_secs(1));
        assert!(toy.lifted(6));
        toy.expire_lifts(|_| true, now + LIFT_HOLD + Duration::from_secs(1));
        assert!(!toy.lifted(6));
        toy.lift(7, [0.0; 3], now);
        toy.expire_lifts(|slot| slot != 7, now);
        assert!(!toy.lifted(7));
    }

    #[test]
    fn a_boxed_player_newly_dead_plays_the_death_sound_once() {
        let mut toy = CardboardBox::default();
        let boxed = |slot: usize| slot == 3;
        let at = |_| [5.0, 0.0, 0.0];
        toy.deaths(0b1111, 0, boxed, at);
        toy.deaths(0b1111, 0b1100, boxed, at);
        let cues: Vec<_> = toy.take_cues().collect();
        assert_eq!(cues.len(), 1, "slot 2 died without a box");
        assert_eq!((cues[0].path, cues[0].slot), (DEATH, 3));
        toy.deaths(0b1111, 0b1100, boxed, at);
        assert_eq!(toy.take_cues().count(), 0, "still dead");
        // Out of the snapshot, the slot keeps what was seen.
        toy.deaths(0b0111, 0, boxed, at);
        toy.deaths(0b1111, 0b1000, boxed, at);
        assert_eq!(toy.take_cues().count(), 0);
    }

    #[test]
    fn use_lifts_the_nearest_box_in_reach_and_in_front() {
        let here = Vec3::ZERO;
        let east = 0.0;
        let boxed = [
            (1, Vec3::new(80.0, 0.0, 0.0)),
            (2, Vec3::new(40.0, 10.0, 0.0)),
            (3, Vec3::new(-30.0, 0.0, 0.0)),
            (4, Vec3::new(200.0, 0.0, 0.0)),
        ];
        assert_eq!(
            pick_target(here, east, boxed).map(|(slot, _)| slot),
            Some(2)
        );
        // Behind only: none.
        assert_eq!(pick_target(here, east, [boxed[2]]), None);
        // Too far: none.
        assert_eq!(pick_target(here, east, [boxed[3]]), None);
        // Facing west, the one behind is in front.
        assert_eq!(
            pick_target(here, std::f32::consts::PI, boxed).map(|(slot, _)| slot),
            Some(3)
        );
    }
}
