//! The Hax, SJK's meme toy (`toy_hax`; Sol's idea, 21/10/2026): a player who has it on
//! holds `+use` on another player who has it on, the siege hack bar fills with its
//! sound, and once the hack is done the hacker steers the hacked player for a few
//! seconds (running, walking, jumping, crouching, swinging), watching them from behind.
//!
//! No game server knows of it, and no client can move another player: the hacked
//! player's own client plays the hacker's controls as its own user commands. The hacker's
//! client sends them, many times a second, through the SJK hub's toy relay
//! (`player_identity::relay_toy_event`, `PROTOCOL.md` "Toy relay"), which hands them to
//! the hacked player's client at once. The toy itself travels as a toy in the player's
//! look (`looks.rs`, [`TOY`]), so the hacker's client knows who may be hacked.
//!
//! The exchange, every event from one player to the other (the relay's `target`):
//!
//! - hacker: `hack_<moment>` (a hack began, done at the game server's `moment`), `stop`
//!   (it ended early), `seize` (it is done), then `in_<frame>` ([`Frame`]) while in control;
//! - hacked player: `taken_<until>` (in control until the game server's `until`),
//!   `refused_<why>` ([`Refusal`]), or `stop` (broke free: the toy turned off, died).
//!
//! Only the hacked player's client decides: it takes a seize only from a player the game
//! shows under the name they claimed, not blocked here, who has the toy out, while its own
//! toy is on and its player plays (not in a duel, not carrying a flag, not hacked in the
//! last [`IMMUNE_MS`]). Turning the toy off frees the player at once. Controls are
//! limited to [`CONTROL_BUTTONS`] and the movement keys: no Force power, weapon change,
//! chat or command goes through. The hacker's own player stands still meanwhile, open to
//! attack. Players on stock clients see a player who stands still and one who moves.
//!
//! The hack bar is the siege one (`hud/hack_bar.rs`, the stock `gfx/mp/c_icon_tech`),
//! shown to both, and the loop is the stock hacking sound ([`HACK_SOUND`]); the seize and
//! the release play the stock mind trick's ([`SEIZE_SOUND`], [`RELEASE_SOUND`]).

use crate::GpuState;
use crate::ghost_replay::Drive;
use glam::Vec3;
use sjk_protocol::UserCommand;
use sjk_runtime::EntityId;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// The toy's console command (a toy is a `toy_<name>` command): turn it on or off.
pub(crate) const COMMAND: &str = "toy_hax";
/// The toy's id in a look's `toys` and in relayed toy events.
pub(crate) const TOY: &str = "hax";
/// The toy's icon (quick wheel, Collection, key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_hax.png";
/// The stock siege hacking loop (`G_SetClientSound`, `level.snd_hack`).
pub(crate) const HACK_SOUND: &str = "sound/player/hacking.wav";
/// Played to both when the hacker takes control, and when they let go.
pub(crate) const SEIZE_SOUND: &str = "sound/weapons/force/distract.wav";
pub(crate) const RELEASE_SOUND: &str = "sound/weapons/force/distractstop.wav";
/// How long `+use` is held to hack, in game server milliseconds.
pub(crate) const HACK_MS: i32 = 3_000;
/// How long the hacker steers.
pub(crate) const CONTROL_MS: i32 = 5_000;
/// After steering, the hacker waits this long before hacking again.
const COOLDOWN_MS: i32 = 15_000;
/// After being steered, a player cannot be hacked for this long.
pub(crate) const IMMUNE_MS: i32 = 15_000;
/// A hack holds while the target stays this close (the start needs the cardboard box's
/// reach, `cardboard_box::REACH`).
const HOLD_REACH: f32 = 160.0;
/// A hack holds while the hacker faces the target this much (cosine of the angle).
const HOLD_FACING: f32 = 0.3;
/// The hacker gives up on an answer to a seize after this long.
const ANSWER_WAIT: Duration = Duration::from_secs(2);
/// A warning whose hack came neither to an end nor to a seize goes this long after its
/// moment.
const WARNING_GRACE_MS: i32 = 2_500;
/// Without a frame this long, the hacked player stands still; this long, they are free.
const FRAME_STALE: Duration = Duration::from_millis(400);
const FRAME_LOST: Duration = Duration::from_millis(1_500);
/// Shortest time between two frames sent; one with other keys or buttons goes after
/// [`FRAME_GAP_CHANGED`], else after [`FRAME_GAP`]. The relay takes 40 a second.
const FRAME_GAP_CHANGED: Duration = Duration::from_millis(30);
const FRAME_GAP: Duration = Duration::from_millis(50);
/// `BUTTON_ATTACK`, `BUTTON_WALKING`, `BUTTON_USE` and `BUTTON_ALT_ATTACK`.
const BUTTON_ATTACK: u16 = 1;
const BUTTON_WALKING: u16 = 16;
const BUTTON_USE: u16 = 32;
const BUTTON_ALT_ATTACK: u16 = 128;
/// The buttons a hacker may press for the hacked player: swings and walking.
pub(crate) const CONTROL_BUTTONS: u16 = BUTTON_ATTACK | BUTTON_ALT_ATTACK | BUTTON_WALKING;
/// The hacker's camera: above the hacked player's origin, and how far behind.
const CAMERA_HEIGHT: f32 = 26.0;
const CAMERA_RANGE: f32 = 100.0;
const CAMERA_RISE: f32 = 12.0;
const SLOTS: usize = crate::looks::SLOTS;
/// `PMF_FOLLOW`.
const PMF_FOLLOW: u16 = 4096;
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
const EF_NODRAW: u32 = 1 << 3;

/// A bundled file's bytes, by its name in `assets/hax`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/hax/", $name))
    };
}

/// The bundled icon at its game path (made by `scripts/hax_assets.py`).
const FILES: [(&str, &[u8]); 1] = [(ICON, bundled!("toy_hax.png"))];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK Hax", FILES.iter().copied())?;
    Ok(())
}

/// The hacker's controls at one moment: the movement keys, the buttons
/// ([`CONTROL_BUTTONS`]) and the view the hacked player turns to (the game server's
/// angles, 16-bit), numbered so the newest wins.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Frame {
    seq: u32,
    forward: i8,
    right: i8,
    up: i8,
    buttons: u16,
    pitch: u16,
    yaw: u16,
}

impl Frame {
    /// `in_<seq>_<forward>_<right>_<up>_<buttons>_<pitch>_<yaw>`.
    fn event(&self) -> String {
        format!(
            "in_{}_{}_{}_{}_{}_{}_{}",
            self.seq, self.forward, self.right, self.up, self.buttons, self.pitch, self.yaw
        )
    }

    fn parse(rest: &str) -> Option<Self> {
        let mut parts = rest.split('_');
        let mut next = || parts.next();
        let frame = Self {
            seq: next()?.parse().ok()?,
            forward: next()?.parse().ok()?,
            right: next()?.parse().ok()?,
            up: next()?.parse().ok()?,
            buttons: next()?.parse::<u16>().ok()? & CONTROL_BUTTONS,
            pitch: next()?.parse().ok()?,
            yaw: next()?.parse().ok()?,
        };
        next().is_none().then_some(frame)
    }

    /// Whether the keys or buttons differ from `other`'s (not the view).
    fn presses_differ(&self, other: &Self) -> bool {
        (self.forward, self.right, self.up, self.buttons)
            != (other.forward, other.right, other.up, other.buttons)
    }
}

/// Why a hacked player's client turned a seize down.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refusal {
    /// Its toy is off, or it is busy with another hack.
    Busy,
    /// Not playing: spectating, dead, at the intermission.
    Away,
    Duel,
    Flag,
    /// Hacked a moment ago ([`IMMUNE_MS`]).
    Immune,
}

impl Refusal {
    fn word(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Away => "away",
            Self::Duel => "duel",
            Self::Flag => "flag",
            Self::Immune => "immune",
        }
    }

    fn from_word(word: &str) -> Self {
        match word {
            "away" => Self::Away,
            "duel" => Self::Duel,
            "flag" => Self::Flag,
            "immune" => Self::Immune,
            _ => Self::Busy,
        }
    }

    /// Why, as the hacker reads it after the player's name.
    fn words(self) -> &'static str {
        match self {
            Self::Busy => "is busy",
            Self::Away => "is not playing",
            Self::Duel => "is in a duel",
            Self::Flag => "carries the flag",
            Self::Immune => "was hacked a moment ago",
        }
    }
}

/// A relayed event of the toy, as read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Event {
    Hack(i32),
    Stop,
    Seize,
    Frame(Frame),
    Taken(i32),
    Refused(Refusal),
}

impl Event {
    pub(crate) fn parse(event: &str) -> Option<Self> {
        if let Some(rest) = event.strip_prefix("in_") {
            return Frame::parse(rest).map(Self::Frame);
        }
        if let Some(moment) = event.strip_prefix("hack_") {
            return moment.parse().ok().map(Self::Hack);
        }
        if let Some(until) = event.strip_prefix("taken_") {
            return until.parse().ok().map(Self::Taken);
        }
        if let Some(why) = event.strip_prefix("refused_") {
            return Some(Self::Refused(Refusal::from_word(why)));
        }
        match event {
            "stop" => Some(Self::Stop),
            "seize" => Some(Self::Seize),
            _ => None,
        }
    }

    fn text(&self) -> String {
        match self {
            Self::Hack(moment) => format!("hack_{moment}"),
            Self::Stop => "stop".to_owned(),
            Self::Seize => "seize".to_owned(),
            Self::Frame(frame) => frame.event(),
            Self::Taken(until) => format!("taken_{until}"),
            Self::Refused(why) => format!("refused_{}", why.word()),
        }
    }
}

/// Where the player stands in a hack.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    Idle,
    /// Holding `+use` on `target`; the hack is done at `done_at`.
    Hacking {
        target: u8,
        done_at: i32,
    },
    /// The hack is done; waiting for `target`'s answer since `since`.
    Seizing {
        target: u8,
        since: Instant,
    },
    /// Steering `target` until `until`. `frozen` is the own view the commands keep (the
    /// game server's angles) and `camera` the camera's own angles to give back (pitch,
    /// yaw in radians), both taken at the first command.
    Controlling {
        target: u8,
        until: i32,
        frozen: Option<[i32; 3]>,
        camera: Option<(f32, f32)>,
    },
    /// `hacker` hacks the player; done at `done_at`.
    Targeted {
        hacker: u8,
        done_at: i32,
    },
    /// `hacker` steers the player until `until`, with their newest frame (when it came)
    /// and the weapon held when it began.
    Controlled {
        hacker: u8,
        until: i32,
        frame: Option<(Frame, Instant)>,
        since: Instant,
        weapon: Option<u8>,
    },
}

/// The toy's state.
pub(crate) struct Hax {
    on: bool,
    server: Option<SocketAddr>,
    role: Role,
    /// `+use` in the last command, and a press since the last frame.
    use_held: bool,
    use_pressed: bool,
    /// The game server time before which the player may not hack, nor be hacked.
    ready_at: i32,
    immune_until: i32,
    /// The last frame sent, and when.
    sent: Option<(Frame, Instant)>,
    seq: u32,
    /// The one-shot sounds to play.
    cues: Vec<&'static str>,
    sounds_registered: Option<u64>,
}

impl Default for Hax {
    fn default() -> Self {
        Self {
            on: false,
            server: None,
            role: Role::Idle,
            use_held: false,
            use_pressed: false,
            ready_at: i32::MIN,
            immune_until: i32::MIN,
            sent: None,
            seq: 0,
            cues: Vec::with_capacity(4),
            sounds_registered: None,
        }
    }
}

/// The game server's 16-bit angle for `degrees` (`ANGLE2SHORT`).
fn short(degrees: f32) -> u16 {
    ((degrees * 65_536.0 / 360.0) as i32 & 0xFFFF) as u16
}

/// Degrees for a 16-bit angle, from -180 to 180.
fn degrees(short: u16) -> f32 {
    let degrees = f32::from(short) * (360.0 / 65_536.0);
    if degrees > 180.0 {
        degrees - 360.0
    } else {
        degrees
    }
}

/// A command that keeps `view` (the game server's angles) with `delta`, and nothing
/// else of `input` but its weapon and Force power: no key, no button.
fn still(input: &UserCommand, view: [i32; 3], delta: [i32; 3]) -> UserCommand {
    UserCommand {
        angles: std::array::from_fn(|i| view[i].wrapping_sub(delta[i]) & 0xFFFF),
        forward_move: 0,
        right_move: 0,
        up_move: 0,
        buttons: 0,
        generic_command: 0,
        ..*input
    }
}

/// The command `frame` makes from `input` with `delta`: its keys, buttons and view
/// (the roll stays the player's), `weapon` held; `None` when it is out of date, which
/// stands still at its view.
fn steered(
    input: &UserCommand,
    frame: &Frame,
    fresh: bool,
    delta: [i32; 3],
    weapon: Option<u8>,
) -> UserCommand {
    let view = [
        i32::from(frame.pitch),
        i32::from(frame.yaw),
        input.angles[2].wrapping_add(delta[2]),
    ];
    let mut command = still(input, view, delta);
    if fresh {
        command.forward_move = frame.forward;
        command.right_move = frame.right;
        command.up_move = frame.up;
        command.buttons = frame.buttons & CONTROL_BUTTONS;
    }
    if let Some(weapon) = weapon {
        command.weapon = weapon;
    }
    command
}

impl Hax {
    /// `toy_hax`: turn the toy on or off.
    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// Turn it on (`true`) or off.
    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Whether the player's own toy is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Note `+use` in the command about to be sent (the start of a hack is a press).
    pub(crate) fn observe_use(&mut self, command: &UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
    }

    /// The command to send in place of `input` while the player steers or is steered,
    /// `delta` the player's `delta_angles` and `camera` the camera's angles (pitch, yaw
    /// in radians, pitch up). Steering, it also hands the frame to the relay when due.
    pub(crate) fn drive(
        &mut self,
        input: &UserCommand,
        delta: [i32; 3],
        camera: (f32, f32),
    ) -> Option<Drive> {
        let now = Instant::now();
        match &mut self.role {
            Role::Controlling {
                target,
                frozen,
                camera: kept,
                ..
            } => {
                let view = *frozen.get_or_insert_with(|| {
                    std::array::from_fn(|i| input.angles[i].wrapping_add(delta[i]) & 0xFFFF)
                });
                kept.get_or_insert(camera);
                let target = *target;
                let frame = Frame {
                    seq: self.seq,
                    forward: input.forward_move,
                    right: input.right_move,
                    up: input.up_move,
                    buttons: input.buttons & CONTROL_BUTTONS,
                    pitch: short(-camera.0.to_degrees()),
                    yaw: short(camera.1.to_degrees()),
                };
                let due = match self.sent {
                    None => true,
                    Some((last, at)) => {
                        let since = now.duration_since(at);
                        since >= FRAME_GAP
                            || (since >= FRAME_GAP_CHANGED && frame.presses_differ(&last))
                    }
                };
                if due {
                    self.seq = self.seq.wrapping_add(1);
                    let frame = Frame {
                        seq: self.seq,
                        ..frame
                    };
                    crate::player_identity::relay_toy_event(TOY, frame.event(), target, true);
                    self.sent = Some((frame, now));
                }
                Some(Drive {
                    command: still(input, view, delta),
                    view: None,
                })
            }
            Role::Controlled { frame, weapon, .. } => {
                let held = *weapon.get_or_insert(input.weapon);
                let Some((frame, at)) = frame else {
                    let view =
                        std::array::from_fn(|i| input.angles[i].wrapping_add(delta[i]) & 0xFFFF);
                    return Some(Drive {
                        command: still(input, view, delta),
                        view: None,
                    });
                };
                let fresh = now.duration_since(*at) < FRAME_STALE;
                Some(Drive {
                    command: steered(input, frame, fresh, delta, Some(held)),
                    view: Some([degrees(frame.pitch), degrees(frame.yaw)]),
                })
            }
            _ => None,
        }
    }

    /// Shape a command that is only shown, never sent (the prediction's preview between
    /// commands), as [`Hax::drive`] would, without sending anything.
    pub(crate) fn shape_preview(&self, command: &mut UserCommand, delta: [i32; 3]) {
        match &self.role {
            Role::Controlling {
                frozen: Some(view), ..
            } => *command = still(command, *view, delta),
            Role::Controlled {
                frame: Some((frame, at)),
                weapon,
                ..
            } => {
                let fresh = at.elapsed() < FRAME_STALE;
                *command = steered(command, frame, fresh, delta, *weapon);
            }
            _ => {}
        }
    }

    /// The slot the player steers, while steering.
    fn steering(&self) -> Option<u8> {
        match self.role {
            Role::Controlling { target, .. } => Some(target),
            _ => None,
        }
    }

    /// Forget any hack: no game, or another server.
    fn leave(&mut self) {
        self.server = None;
        self.role = Role::Idle;
        self.ready_at = i32::MIN;
        self.immune_until = i32::MIN;
        self.sent = None;
    }
}

/// Send `event` to the player in `slot`.
fn relay(event: Event, slot: u8) {
    crate::player_identity::relay_toy_event(TOY, event.text(), slot, false);
}

impl GpuState {
    /// `toy_hax`: turn the toy on or off.
    pub(crate) fn toggle_hax(&mut self) {
        self.hax.toggle();
    }

    /// Say `text` on the centre of the screen.
    fn hax_centre(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text,
            None,
            Instant::now(),
        );
    }

    /// The name the game shows in `slot`, or "Someone".
    fn hax_name(&self, slot: u8) -> String {
        self.live_session
            .as_ref()
            .and_then(|session| {
                crate::player_identity::shown_name(session.game_state(), usize::from(slot))
            })
            .unwrap_or_else(|| "Someone".to_owned())
    }

    /// The frame's toy logic: the relay's events, the hack under way, the steering's end,
    /// the hack bar and the hacking sound.
    pub(crate) fn update_hax(&mut self) {
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            crate::player_identity::set_relay_listen(false);
            self.end_hax_role();
            self.hax.leave();
            self.hud.set_toy_hack(None);
            return;
        };
        let server = session.server();
        if self.hax.server != Some(server) {
            self.end_hax_role();
            self.hax.leave();
            self.hax.server = Some(server);
        }
        crate::player_identity::set_relay_listen(self.hax.on);
        let events = crate::player_identity::take_relay_events(Some(server));
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let snapshot = session.latest_snapshot();
        let game_state = session.game_state();
        let now = snapshot.server_time;
        let own = u8::try_from(game_state.client_num).ok();
        let gametype = game_state
            .config_string(0)
            .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"))
            .unwrap_or(0);
        let spared = crate::order66::spared(&snapshot.player, gametype);
        // The relay's events: a sender counts only when the game shows the name they
        // claimed, and never one blocked on this PC.
        let heard: Vec<(u8, Event)> = events
            .iter()
            .filter(|event| {
                event.toy == TOY
                    && Some(event.slot) != own
                    && !self.muted_players.contains(u16::from(event.slot))
                    && crate::player_identity::shown_name(game_state, usize::from(event.slot))
                        .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            })
            .filter_map(|event| Some((event.slot, Event::parse(&event.event)?)))
            .collect();
        for (slot, event) in heard {
            self.hax_event(slot, event, now, spared);
        }
        self.hax_hack(now, spared, own);
        self.hax_progress(now, spared);
        let bar = match self.hax.role {
            Role::Hacking { done_at, .. } | Role::Targeted { done_at, .. } => {
                Some((done_at - now) as f32 / HACK_MS as f32)
            }
            _ => None,
        };
        self.hud.set_toy_hack(bar);
    }

    /// One relayed event from the player in `slot`.
    fn hax_event(
        &mut self,
        slot: u8,
        event: Event,
        now: i32,
        spared: Option<crate::order66::Spared>,
    ) {
        let holds = self.looks.toy(usize::from(slot), TOY);
        match (event, self.hax.role) {
            // A hack begins on the player.
            (Event::Hack(done_at), Role::Idle)
                if self.hax.on && holds && now >= self.hax.immune_until =>
            {
                self.hax.role = Role::Targeted {
                    hacker: slot,
                    done_at,
                };
                let name = self.hax_name(slot);
                self.hax_centre(format!("^2{name}^7 is hacking you!\n^3Get away"));
            }
            (Event::Stop, Role::Targeted { hacker, .. }) if hacker == slot => {
                self.hax.role = Role::Idle;
            }
            (Event::Seize, role) => {
                let busy = !matches!(role, Role::Idle)
                    && !matches!(role, Role::Targeted { hacker, .. } if hacker == slot);
                let why = if !self.hax.on || !holds || busy {
                    Some(Refusal::Busy)
                } else if now < self.hax.immune_until {
                    Some(Refusal::Immune)
                } else {
                    spared.map(|spared| match spared {
                        crate::order66::Spared::Away | crate::order66::Spared::Dead => {
                            Refusal::Away
                        }
                        crate::order66::Spared::Duel => Refusal::Duel,
                        crate::order66::Spared::Flag => Refusal::Flag,
                    })
                };
                if let Some(why) = why {
                    relay(Event::Refused(why), slot);
                    if matches!(role, Role::Targeted { hacker, .. } if hacker == slot) {
                        self.hax.role = Role::Idle;
                    }
                    return;
                }
                let until = now + CONTROL_MS;
                relay(Event::Taken(until), slot);
                self.hax.role = Role::Controlled {
                    hacker: slot,
                    until,
                    frame: None,
                    since: Instant::now(),
                    weapon: None,
                };
                self.hax.cues.push(SEIZE_SOUND);
                let name = self.hax_name(slot);
                self.hax_centre(format!(
                    "^1HACKED\n^2{name}^7 has your controls\n^3toy_hax^7 off breaks free"
                ));
            }
            (Event::Frame(frame), Role::Controlled { hacker, .. }) if hacker == slot => {
                if let Role::Controlled { frame: kept, .. } = &mut self.hax.role
                    && kept.is_none_or(|(old, _)| {
                        frame.seq > old.seq || old.seq - frame.seq > u32::MAX / 2
                    })
                {
                    *kept = Some((frame, Instant::now()));
                }
            }
            (Event::Stop, Role::Controlled { hacker, .. }) if hacker == slot => {
                self.hax_freed(now, false);
            }
            // The answers to the player's own seize.
            (Event::Taken(until), Role::Seizing { target, .. }) if target == slot => {
                self.hax.role = Role::Controlling {
                    target,
                    until,
                    frozen: None,
                    camera: None,
                };
                self.hax.sent = None;
                self.hax.cues.push(SEIZE_SOUND);
                let name = self.hax_name(slot);
                self.hax_centre(format!("^2HACKED^7\nYou steer {name}"));
            }
            (Event::Refused(why), Role::Seizing { target, .. }) if target == slot => {
                self.hax.role = Role::Idle;
                let name = self.hax_name(slot);
                self.hax_centre(format!("^1Hack failed^7\n{name}^7 {}", why.words()));
            }
            (Event::Stop, Role::Controlling { target, .. } | Role::Seizing { target, .. })
                if target == slot =>
            {
                let name = self.hax_name(slot);
                self.end_hax_role();
                self.hax_centre(format!("{name}^7 broke free"));
            }
            _ => {}
        }
    }

    /// The player's own hack: one begins on a `+use` press facing a holder of the toy
    /// within reach, holds while `+use` does and the target stays near and in front, and
    /// is seized when its moment comes.
    fn hax_hack(&mut self, now: i32, spared: Option<crate::order66::Spared>, own: Option<u8>) {
        let pressed = std::mem::take(&mut self.hax.use_pressed);
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let snapshot = session.latest_snapshot();
        let from = Vec3::from_array(
            self.local_prediction
                .predicted_state()
                .map_or_else(|| snapshot.player.origin(), |state| state.origin),
        );
        let world = &self.live_world;
        let origin_of = |slot: u8| {
            world
                .entity(EntityId::new(u64::from(slot) + 1))
                .map(|entity| Vec3::from_array(entity.sample(i64::from(now)).translation))
        };
        match self.hax.role {
            Role::Idle if pressed && self.hax.on && spared.is_none() => {
                let looks = &self.looks;
                let candidates = snapshot.entities.iter().filter_map(|entity| {
                    let slot = u8::try_from(entity.number()).ok()?;
                    (usize::from(slot) < SLOTS
                        && Some(slot) != own
                        && entity.entity_type() == ET_PLAYER
                        && entity.e_flags() & (EF_DEAD | EF_NODRAW) == 0
                        && looks.toy(usize::from(slot), TOY))
                    .then(|| Some((slot, origin_of(slot)?)))
                    .flatten()
                });
                let Some((target, _)) =
                    crate::cardboard_box::pick_target(from, self.camera_yaw, candidates)
                else {
                    return;
                };
                if now < self.hax.ready_at {
                    let wait = (self.hax.ready_at - now + 999) / 1000;
                    self.hax_centre(format!("Hax again in {wait} s"));
                    return;
                }
                let done_at = now + HACK_MS;
                self.hax.role = Role::Hacking { target, done_at };
                relay(Event::Hack(done_at), target);
            }
            Role::Hacking { target, done_at } => {
                let facing = glam::Vec2::from_angle(self.camera_yaw);
                let holds = self.hax.on
                    && self.hax.use_held
                    && spared.is_none()
                    && self.looks.toy(usize::from(target), TOY)
                    && origin_of(target).is_some_and(|at| {
                        let way = (at - from).truncate();
                        at.distance(from) <= HOLD_REACH
                            && (way.length_squared() < 1.0
                                || way.normalize().dot(facing) >= HOLD_FACING)
                    });
                if !holds {
                    self.hax.role = Role::Idle;
                    relay(Event::Stop, target);
                } else if now >= done_at {
                    self.hax.role = Role::Seizing {
                        target,
                        since: Instant::now(),
                    };
                    relay(Event::Seize, target);
                }
            }
            _ => {}
        }
    }

    /// The ends that come with time: an unanswered seize, the steering's time, a warning
    /// that led nowhere, a hacked player whose hacker went quiet; and the player's own
    /// toy turned off or their player no longer playing.
    fn hax_progress(&mut self, now: i32, spared: Option<crate::order66::Spared>) {
        match self.hax.role {
            Role::Seizing { target, since } if since.elapsed() >= ANSWER_WAIT => {
                self.hax.role = Role::Idle;
                let name = self.hax_name(target);
                self.hax_centre(format!("^1Hack failed^7\n{name}^7 did not answer"));
            }
            Role::Controlling { until, .. } if now >= until => self.end_hax_role(),
            Role::Controlling { .. } | Role::Seizing { .. } | Role::Hacking { .. }
                if !self.hax.on || spared.is_some() =>
            {
                self.end_hax_role();
            }
            Role::Targeted { done_at, .. } if !self.hax.on || now > done_at + WARNING_GRACE_MS => {
                self.hax.role = Role::Idle;
            }
            Role::Controlled {
                until,
                frame,
                since,
                ..
            } => {
                let heard = frame.map_or(since, |(_, at)| at);
                if now >= until {
                    self.hax_freed(now, true);
                } else if !self.hax.on || spared.is_some() || heard.elapsed() >= FRAME_LOST {
                    self.hax_freed(now, false);
                }
            }
            _ => {}
        }
    }

    /// The player is free again: the hacker is told unless the time was up (`timely`),
    /// and the player cannot be hacked for [`IMMUNE_MS`].
    fn hax_freed(&mut self, now: i32, timely: bool) {
        if let Role::Controlled { hacker, .. } = self.hax.role {
            if !timely {
                relay(Event::Stop, hacker);
            }
            self.hax.cues.push(RELEASE_SOUND);
            self.hax_centre("^2Free again".to_owned());
        }
        self.hax.role = Role::Idle;
        self.hax.immune_until = now + IMMUNE_MS;
    }

    /// End the player's own hack, steering included: the target is told, the camera
    /// turns back to where it was, and the cooldown starts after a steering.
    fn end_hax_role(&mut self) {
        let now = self
            .live_session
            .as_ref()
            .map_or(0, |session| session.latest_snapshot().server_time);
        match self.hax.role {
            Role::Hacking { target, .. } | Role::Seizing { target, .. } => {
                relay(Event::Stop, target);
            }
            Role::Controlling {
                target,
                until,
                camera,
                ..
            } => {
                if now < until {
                    relay(Event::Stop, target);
                }
                if let Some((pitch, yaw)) = camera {
                    self.camera_pitch = pitch;
                    self.camera_yaw = yaw;
                }
                self.hax.cues.push(RELEASE_SOUND);
                self.hax.ready_at = now + COOLDOWN_MS;
            }
            Role::Controlled { .. } => {
                self.hax_freed(now, false);
                return;
            }
            _ => {}
        }
        self.hax.role = Role::Idle;
        self.hax.sent = None;
    }

    /// The hacking sound's loop this frame: on the hacker (in both ears for the own
    /// player), for the hacker and the player they hack.
    pub(crate) fn hax_loop(&self, presentation_time: i64) -> Option<(u8, Option<[f32; 3]>)> {
        match self.hax.role {
            Role::Hacking { .. } => {
                let own = self.live_session.as_ref()?.game_state().client_num;
                Some((u8::try_from(own).ok()?, None))
            }
            Role::Targeted { hacker, .. } => {
                let origin = self
                    .live_world
                    .entity(EntityId::new(u64::from(hacker) + 1))?
                    .sample(presentation_time)
                    .translation;
                Some((hacker, Some(origin)))
            }
            _ => None,
        }
    }

    /// Play the toy's one-shot sounds; they are handed to the audio the first time there
    /// is one.
    pub(crate) fn play_hax_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.hax.cues.clear();
            return;
        };
        if self.hax.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, [HACK_SOUND, SEIZE_SOUND, RELEASE_SOUND]);
            self.hax.sounds_registered = Some(audio.generation());
        }
        let own = self
            .live_session
            .as_ref()
            .map_or(0, |session| session.game_state().client_num.max(0) as u32);
        for path in self.hax.cues.drain(..) {
            audio.play_local(
                path,
                1.0,
                sjk_audio::SourceId(own),
                crate::audio::TOY_CHANNEL,
            );
        }
    }
}

/// The hacker's camera while steering: behind the steered player, turned as the
/// hacker's own camera is (the mouse answers at once, the player follows after the
/// relay's way); `None` otherwise, or while the game does not show them.
pub(crate) fn camera(state: &mut GpuState, time: i64) -> Option<(Vec3, Vec3)> {
    let target = state.hax.steering()?;
    let session = state.live_session.as_ref()?;
    let game = session.game_state();
    let snapshot = session.snapshot_at_or_before(time as i32);
    let shown = snapshot.entities.iter().any(|entity| {
        u8::try_from(entity.number()) == Ok(target) && entity.entity_type() == ET_PLAYER
    }) && snapshot.player.movement_flags() & PMF_FOLLOW == 0;
    if !shown {
        return None;
    }
    let origin = state
        .live_world
        .entity(EntityId::new(u64::from(target) + 1))?
        .sample(time)
        .translation;
    let (pitch, yaw) = (state.camera_pitch, state.camera_yaw);
    let forward = Vec3::new(
        yaw.cos() * pitch.cos(),
        yaw.sin() * pitch.cos(),
        pitch.sin(),
    );
    let focus = Vec3::from_array(origin) + Vec3::Z * CAMERA_HEIGHT;
    let wanted = focus - forward * CAMERA_RANGE + Vec3::Z * CAMERA_RISE;
    let position = crate::movement_collision::camera_trace(
        &state.bsp,
        &mut state.trace_scratch,
        Some((game, snapshot)),
        time as i32,
        focus,
        wanted,
    );
    Some((position, position + forward * 256.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame {
            seq: 12,
            forward: 127,
            right: -127,
            up: 0,
            buttons: BUTTON_ATTACK | BUTTON_WALKING,
            pitch: 65_000,
            yaw: 16_384,
        }
    }

    #[test]
    fn events_read_back_as_written_and_fit_the_relay() {
        let events = [
            Event::Hack(123_456),
            Event::Stop,
            Event::Seize,
            Event::Frame(frame()),
            Event::Frame(Frame {
                seq: u32::MAX,
                forward: -128,
                right: -128,
                up: -128,
                buttons: CONTROL_BUTTONS,
                pitch: u16::MAX,
                yaw: u16::MAX,
            }),
            Event::Taken(-5),
            Event::Refused(Refusal::Duel),
            Event::Refused(Refusal::Immune),
        ];
        for event in events {
            let text = event.text();
            // The hub's rule: 1 to 64 of a-z, 0-9, _ and -.
            assert!(text.len() <= 64, "{text}");
            assert!(
                text.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || b == b'_'
                    || b == b'-'),
                "{text}"
            );
            assert_eq!(Event::parse(&text), Some(event), "{text}");
        }
    }

    #[test]
    fn a_frame_carries_only_the_allowed_buttons_and_nothing_else_parses() {
        let parsed = Event::parse("in_1_0_0_0_65535_0_0");
        let Some(Event::Frame(frame)) = parsed else {
            panic!("{parsed:?}");
        };
        assert_eq!(frame.buttons, CONTROL_BUTTONS);
        for bad in [
            "in_1_0_0_0_0_0",
            "in_1_0_0_0_0_0_0_0",
            "in_x_0_0_0_0_0_0",
            "hack_",
            "go",
        ] {
            assert_eq!(Event::parse(bad), None, "{bad}");
        }
        assert_eq!(
            Event::parse("refused_what"),
            Some(Event::Refused(Refusal::Busy))
        );
    }

    #[test]
    fn a_steered_command_turns_to_the_frames_view_through_the_delta() {
        let input = UserCommand {
            angles: [100, 200, 300],
            forward_move: -50,
            buttons: 512 | BUTTON_USE,
            weapon: 3,
            generic_command: 26,
            ..UserCommand::default()
        };
        let delta = [1_000, -2_000, 50];
        let command = steered(&input, &frame(), true, delta, Some(5));
        // The server adds the delta back: the frame's view, the player's own roll.
        let view: [i32; 3] =
            std::array::from_fn(|i| command.angles[i].wrapping_add(delta[i]) & 0xFFFF);
        assert_eq!(view, [65_000, 16_384, 350]);
        assert_eq!(
            (command.forward_move, command.right_move, command.up_move),
            (127, -127, 0)
        );
        assert_eq!(command.buttons, BUTTON_ATTACK | BUTTON_WALKING);
        assert_eq!((command.weapon, command.generic_command), (5, 0));
        // Out of date: the view holds, nothing is pressed.
        let stale = steered(&input, &frame(), false, delta, None);
        assert_eq!(stale.angles, command.angles);
        assert_eq!(
            (stale.forward_move, stale.right_move, stale.buttons),
            (0, 0, 0)
        );
        assert_eq!(stale.weapon, 3);
    }

    #[test]
    fn the_hacker_stands_still_at_the_view_taken_and_sends_frames_at_the_relays_pace() {
        let mut hax = Hax {
            role: Role::Controlling {
                target: 4,
                until: 10_000,
                frozen: None,
                camera: None,
            },
            ..Hax::default()
        };
        let delta = [10, 20, 0];
        let input = UserCommand {
            angles: [1_000, 2_000, 0],
            forward_move: 127,
            buttons: BUTTON_ATTACK | 512,
            ..UserCommand::default()
        };
        let drive = hax
            .drive(&input, delta, (0.5, 1.0))
            .expect("steering drives");
        assert_eq!(drive.view, None, "the camera stays the hacker's");
        assert_eq!(drive.command.angles, [1_000, 2_000, 0]);
        assert_eq!((drive.command.forward_move, drive.command.buttons), (0, 0));
        let (sent, _) = hax.sent.expect("the first frame goes at once");
        assert_eq!((sent.forward, sent.buttons), (127, BUTTON_ATTACK));
        assert_eq!(sent.yaw, short(1.0_f32.to_degrees()));
        assert_eq!(sent.pitch, short(-0.5_f32.to_degrees()));
        // The view stays the one taken, even when the input turns.
        let turned = UserCommand {
            angles: [5_000, 6_000, 0],
            ..input
        };
        let drive = hax.drive(&turned, delta, (0.0, 0.0)).unwrap();
        assert_eq!(drive.command.angles, [1_000, 2_000, 0]);
        assert_eq!(
            hax.sent.unwrap().0.seq,
            sent.seq,
            "too soon for the next frame"
        );
        assert!(matches!(
            hax.role,
            Role::Controlling {
                camera: Some((0.5, 1.0)),
                ..
            }
        ));
    }

    #[test]
    fn the_short_angle_round_trips() {
        for degrees_in in [-179.0_f32, -90.0, -1.0, 0.0, 45.0, 89.0, 179.0] {
            let back = degrees(short(degrees_in));
            assert!((back - degrees_in).abs() < 0.01, "{degrees_in} -> {back}");
        }
    }
}
