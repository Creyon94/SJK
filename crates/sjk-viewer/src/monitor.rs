//! The Monitor, SJK's meme toy (`toy_monitor`; Sol's idea, 26/10/2026, after the old
//! sandbox-game joke of hurling a big CRT monitor at someone): a player with the toy on
//! throws an old square monitor (`monitor_throw`, [`MODEL`]); one that hits another
//! player who has the toy on too knocks them down as a ragdoll and sends them flying,
//! the way the monitor struck them.
//!
//! - **Opt-in:** the switch (`toy_monitor`, the Collection's Toys tab, the quick wheel)
//!   is both the right to throw and the consent to be hit; it stays as last switched
//!   ([`crate::toy_saved::MONITOR_CVAR`]). A monitor passes through a player without it.
//!   Nobody throws or is hit while dead, spectating, in a duel, with the flag, limp,
//!   hacked or held by the server (as for the Ragdoll), and a throw waits
//!   [`COOLDOWN`] after the last.
//! - **The throw:** from the eyes, the way the player looks, at [`THROW_SPEED`] plus
//!   the thrower's own speed, tumbling end over end. Held (`+monitor_throw`, the key
//!   bindings' Throw monitor), the throw charges while a ring fills round the crosshair
//!   and a tube's hum rises ([`CHARGE_SOUND`]), and goes on the release, up to
//!   [`MAX_THROW_SPEED`] once held [`CHARGE_TIME`]; a charged throw is heard as one
//!   ([`THROW_HARD_SOUND`]) by everyone, from its speed. The monitor is a solid of eight
//!   corners ([`flight`]): it bounces off the world, topples and comes to rest on a
//!   face, and goes [`LINGER`] after.
//! - **The hit:** the struck player's client alone decides it, against its own body (a
//!   standing player's box, or a ragdoll's points): the monitor must come at them
//!   faster than [`MIN_HIT_SPEED`]. Their body falls limp ([`crate::ragdoll`]) and takes
//!   a push at the point the monitor met ([`crate::ragdoll::Strike`]): the monitor's
//!   speed against theirs times [`TRANSFER`], [`LIFT`] upwards, at most [`MAX_LAUNCH`];
//!   a blow off the body's centre turns it. A ragdoll can be struck again where it lies
//!   or flies. The monitor bounces back off the body. The player gets up by themselves
//!   once the body has lain still a little, or with `toy_ragdoll`. The thrower sees a
//!   hit marker on their crosshair and hears [`HIT_SOUND`] when the struck player's
//!   blow reaches them (red for a hard one).
//!
//! - **Unseen:** in a duel (the player's own or the one they follow, and on Duel and
//!   Power Duel servers) no monitor is drawn or heard, nor any mark shown; and
//!   [`SHOW_CVAR`] at 0 hides every monitor and its sounds in the world, everywhere (the
//!   player's own throw ring and hit marker stay). Neither changes who may be hit: the
//!   switch does.
//!
//! No game server knows of it: the player's real body stays where it was and walks to
//! the ragdoll, as for the Ragdoll. Every SJK client on the server sees the same throw:
//! the thrower's client relays `throw_<number>_<moment>_<x>_<y>_<z>_<vx>_<vy>_<vz>`
//! (the toy relay, `PROTOCOL.md`, "Toy relay") to each SJK player in turn, the nearest
//! first, one every [`SEND_GAP`], and each runs the same flight from it in steps of the
//! game server's clock. A hit travels as the Ragdoll's `hit_` event, which names the
//! monitor so that every client bounces it off the body at the same step. Players on
//! stock clients see nothing.

mod flight;

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use crate::ragdoll::{By, JOINTS, STEP_MS};
use flight::{Deflect, Flight, Throw};
use glam::Vec3;
use sjk_runtime::EntityId;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// The toy's console command: the switch (throw, and be hit).
pub(crate) const COMMAND: &str = "toy_monitor";
/// Throw a monitor.
pub(crate) const THROW_COMMAND: &str = "monitor_throw";
/// The held bind: charge a throw while held, throw on the release.
pub(crate) const CHARGE_COMMAND: &str = "+monitor_throw";
/// The release of [`CHARGE_COMMAND`].
pub(crate) const CHARGE_RELEASE: &str = "-monitor_throw";
/// `cg_toyMonitorShow` (archived, on by default): off, no monitor is drawn, and none of
/// their sounds in the world is heard, on this PC.
pub(crate) const SHOW_CVAR: &str = "cg_toyMonitorShow";
/// The toy's id in relayed toy events.
pub(crate) const TOY: &str = "monitor";
/// The toy's icon (quick wheel, Collection, key bindings).
pub(crate) const ICON: &str = "gfx/sjk/toy_monitor.png";
/// The monitor: an old beige CRT, 20 units wide, its screen facing +X, its centre at
/// the origin.
pub(crate) const MODEL: &str = "models/sjk/monitor.md3";
/// Heard as a monitor leaves the hand, as it strikes a body, and as it knocks on the
/// world (all made by `scripts/monitor_assets.py`).
pub(crate) const THROW_SOUND: &str = "sound/sjk/monitor/throw.wav";
pub(crate) const BONK_SOUND: &str = "sound/sjk/monitor/bonk.wav";
pub(crate) const KNOCK_SOUND: &str = "sound/sjk/monitor/knock.wav";
/// Heard by the thrower while a throw charges; heard as a charged throw leaves the
/// hand; heard by the thrower when their monitor strikes someone.
pub(crate) const CHARGE_SOUND: &str = "sound/sjk/monitor/charge.wav";
pub(crate) const THROW_HARD_SOUND: &str = "sound/sjk/monitor/throw_hard.wav";
pub(crate) const HIT_SOUND: &str = "sound/sjk/monitor/hit.wav";
/// The throw event: this, then [`Event::text`]'s numbers.
const THROW_PREFIX: &str = "throw_";
/// How fast a monitor leaves the hand, units a second (the thrower's speed added).
const THROW_SPEED: f32 = 1_100.0;
/// A throw held this long goes this fast (units a second, the thrower's speed added);
/// one held less, in between.
const CHARGE_TIME: Duration = Duration::from_millis(1_000);
const MAX_THROW_SPEED: f32 = 2_600.0;
/// A throw held this long is put down (its release was lost, the window left).
const CHARGE_LIMIT: Duration = Duration::from_secs(6);
/// A throw leaving the hand faster than this is heard as a charged one.
const HARD_SPEED: f32 = 1_800.0;
/// Where it leaves the hand: this far ahead of the eyes.
const HAND: f32 = 20.0;
/// The shortest time between two throws of a player.
const COOLDOWN: Duration = Duration::from_millis(1_200);
/// Between two throw events sent (the relay takes 40 a second from a player, which the
/// other toys share).
const SEND_GAP: Duration = Duration::from_millis(40);
/// The thrower's own monitor cannot hit them for this many steps (it leaves the hand).
const SELF_SAFE_STEPS: u32 = 25;
/// A monitor slower than this against the player (units a second) does not knock them
/// down: one lying there is only in the way.
const MIN_HIT_SPEED: f32 = 250.0;
/// A struck body's push: the monitor's speed against it times this, and this much
/// upwards, at most this fast (the hardest blow a ragdoll takes, which the clients
/// before the charged throw take too). More than a real monitor would: it is the joke.
const TRANSFER: f32 = 1.5;
const LIFT: f32 = 260.0;
const MAX_LAUNCH: f32 = crate::ragdoll::MAX_PUSH as f32;
/// A monitor bounces off a body with this share of the body's push, back, and this
/// much upwards (units a second).
const BOUNCE_BACK: f32 = 0.5;
const BOUNCE_UP: i32 = 150;
/// After a hit the player cannot be struck again for this long by any monitor, and for
/// this long by the same one.
const IMMUNE: i64 = 200;
const IMMUNE_SAME: i64 = 600;
/// A monitor at rest goes this long after, fading out over [`FADE`]; one flying this
/// many steps goes whatever it does.
const LINGER: Duration = Duration::from_secs(10);
const FADE: f32 = 0.4;
/// The most monitors a player has out (the oldest goes), and shown in all.
const PER_THROWER: usize = 4;
const MAX_SHOWN: usize = 32;
/// A throw heard this far after its moment, or before it, is dropped (ms).
const LATE_MS: i64 = 8_000;
const EARLY_MS: i64 = 3_000;
/// A monitor's knocks on the world are heard this far apart at least.
const KNOCK_GAP: Duration = Duration::from_millis(120);
/// A monitor's way over a step is tried for a body every this many units (a fast one
/// goes farther in a step than a limb is thick).
const SWEEP_PIECE: f32 = 6.0;
/// The hit marker shows this long; a hit heard later than this after it happened (ms)
/// shows none. A blow this hard marks red.
const MARK_TIME: Duration = Duration::from_millis(350);
const MARK_LATE_MS: i64 = 1_500;
const STRONG_PUSH: f32 = 2_400.0;
/// The charge's sound, on its own source so its release stops it.
const CHARGE_SOURCE: sjk_audio::SourceId = sjk_audio::SourceId(0x5100_0001);
const CHARGE_CHANNEL: sjk_audio::ChannelId = sjk_audio::ChannelId(1);
/// The numbers of a player's monitors go round this many.
const NUMBERS: u8 = 100;
const SLOTS: usize = crate::looks::SLOTS;

/// A bundled file's bytes, by its name in `assets/monitor`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/monitor/", $name))
    };
}

/// The bundled files at their game paths (made by `scripts/monitor_assets.py`).
const FILES: [(&str, &[u8]); 10] = [
    (MODEL, bundled!("monitor.md3")),
    ("models/sjk/monitor.jpg", bundled!("monitor.jpg")),
    ("shaders/sjk_monitor.shader", bundled!("monitor.shader")),
    (ICON, bundled!("toy_monitor.png")),
    (THROW_SOUND, bundled!("throw.wav")),
    (BONK_SOUND, bundled!("bonk.wav")),
    (KNOCK_SOUND, bundled!("knock.wav")),
    (CHARGE_SOUND, bundled!("charge.wav")),
    (THROW_HARD_SOUND, bundled!("throw_hard.wav")),
    (HIT_SOUND, bundled!("hit.wav")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK Monitor", FILES.iter().copied())?;
    Ok(())
}

/// A throw event: the monitor's number among its thrower's, and the throw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Event {
    number: u8,
    throw: Throw,
}

impl Event {
    fn text(&self) -> String {
        let Throw {
            moment,
            origin: [x, y, z],
            velocity: [vx, vy, vz],
        } = self.throw;
        format!(
            "{THROW_PREFIX}{}_{moment}_{x}_{y}_{z}_{vx}_{vy}_{vz}",
            self.number
        )
    }

    fn parse(event: &str) -> Option<Self> {
        let mut parts = event.strip_prefix(THROW_PREFIX)?.split('_');
        let mut number = || parts.next()?.parse::<i32>().ok();
        let monitor = u8::try_from(number()?).ok().filter(|n| *n < NUMBERS)?;
        let moment = number()?;
        let origin = [number()?, number()?, number()?];
        let velocity = [number()?, number()?, number()?];
        if parts.next().is_some()
            || origin.iter().any(|c| c.abs() > 65_536)
            || velocity.iter().any(|c| c.abs() > 4_000)
        {
            return None;
        }
        Some(Self {
            number: monitor,
            throw: Throw {
                moment,
                origin,
                velocity,
            },
        })
    }
}

/// A monitor out: whose, its number, its flight, who has been told of it (the
/// player's own, bits by slot), since when it rests and when it last knocked.
struct Shown {
    thrower: u8,
    number: u8,
    flight: Flight,
    told: u64,
    rest: Option<Instant>,
    knocked: Option<Instant>,
}

impl Shown {
    fn new(thrower: u8, number: u8, throw: &Throw) -> Self {
        Self {
            thrower,
            number,
            flight: Flight::new(throw),
            told: 0,
            rest: None,
            knocked: None,
        }
    }

    /// How much of it shows (it fades out once it has lain [`LINGER`]); 0: gone.
    fn level(&self) -> f32 {
        self.rest.map_or(1.0, |rest| {
            let over = rest.elapsed().saturating_sub(LINGER).as_secs_f32();
            (1.0 - over / FADE).max(0.0)
        })
    }
}

/// A sound of the toy to play: its path, where (`None`: in both ears), and how loud.
struct Cue {
    path: &'static str,
    at: Option<Vec3>,
    volume: f32,
}

/// The toy's state.
#[derive(Default)]
pub(crate) struct Monitor {
    on: bool,
    /// A throw was asked since the last frame, charged this much (0 to 1).
    throw_asked: Option<f32>,
    /// The throw held (`+monitor_throw`): since when it charges (the press, or the end
    /// of the cooldown), and whether its sound began.
    charging: Option<(Instant, bool)>,
    /// The charge's sound is to stop.
    stop_charge: bool,
    /// The last hit of the player's monitors on someone: when it was heard, and hard.
    mark: Option<(Instant, bool)>,
    /// This frame: the player is in a duel (nothing of the toy is seen or heard), and
    /// [`SHOW_CVAR`] is off (no monitor seen, nor heard in the world).
    duel: bool,
    unseen: bool,
    server: Option<SocketAddr>,
    shown: Vec<Shown>,
    next_number: u8,
    last_throw: Option<Instant>,
    tell_at: Option<Instant>,
    /// The last time the player was struck, and by which monitor.
    struck: Option<(i64, u8, u8)>,
    cues: Vec<Cue>,
    sounds_registered: Option<u64>,
    movers: crate::ragdoll::Movers,
    /// The game server time last stepped to (the presentation's).
    time: i64,
}

impl Monitor {
    /// `toy_monitor`: on or off.
    pub(crate) fn toggle(&mut self) {
        self.on = !self.on;
    }

    /// On (`true`) or off.
    pub(crate) fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Whether the toy is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// `monitor_throw`: throw one at the next frame.
    pub(crate) fn throw(&mut self) {
        self.throw_asked.get_or_insert(0.0);
    }

    /// Put down a throw held, if any.
    fn put_down(&mut self) {
        if self.charging.take().is_some() {
            self.stop_charge = true;
        }
    }

    /// How much the throw held has charged (0 to 1), if one is.
    fn charge(&self) -> Option<f32> {
        let (since, _) = self.charging?;
        Some(charge_of(Instant::now().saturating_duration_since(since)))
    }

    /// What the HUD draws for the toy now (nothing in a duel).
    fn marks(&self) -> crate::hud::MonitorMarks {
        if self.duel {
            return crate::hud::MonitorMarks::default();
        }
        crate::hud::MonitorMarks {
            charge: self.charge(),
            hit: self.mark.and_then(|(when, hard)| {
                let age = when.elapsed().as_secs_f32() / MARK_TIME.as_secs_f32();
                (age < 1.0).then_some((age, hard))
            }),
        }
    }

    /// Forget every monitor: no game, or another server.
    fn leave(&mut self) {
        self.server = None;
        self.shown.clear();
        self.struck = None;
        self.tell_at = None;
        self.mark = None;
        self.put_down();
    }

    /// The monitor `thrower`'s number `number`, if out.
    fn find(&mut self, thrower: u8, number: u8) -> Option<&mut Shown> {
        self.shown
            .iter_mut()
            .find(|shown| shown.thrower == thrower && shown.number == number)
    }

    /// Put out a monitor, in place of one of the same thrower and number; the thrower's
    /// oldest goes past [`PER_THROWER`], the oldest of all past [`MAX_SHOWN`].
    fn put(&mut self, shown: Shown) {
        self.shown
            .retain(|other| (other.thrower, other.number) != (shown.thrower, shown.number));
        let theirs = self
            .shown
            .iter()
            .filter(|other| other.thrower == shown.thrower)
            .count();
        if theirs >= PER_THROWER
            && let Some(oldest) = self
                .shown
                .iter()
                .position(|other| other.thrower == shown.thrower)
        {
            self.shown.remove(oldest);
        }
        if self.shown.len() >= MAX_SHOWN {
            self.shown.remove(0);
        }
        self.shown.push(shown);
    }
}

/// Send `event` to the player in `slot`.
fn relay(event: Event, slot: u8) {
    crate::player_identity::relay_toy_event(TOY, event.text(), slot, false);
}

/// How much a throw held `held` has charged: 0 to 1 over [`CHARGE_TIME`].
fn charge_of(held: Duration) -> f32 {
    (held.as_secs_f32() / CHARGE_TIME.as_secs_f32()).clamp(0.0, 1.0)
}

/// How fast a throw charged `charge` (0 to 1) leaves the hand, before the thrower's own
/// speed.
fn throw_speed(charge: f32) -> f32 {
    THROW_SPEED + (MAX_THROW_SPEED - THROW_SPEED) * charge.clamp(0.0, 1.0)
}

/// The sound of a throw leaving the hand at `velocity`.
fn throw_sound(velocity: Vec3) -> &'static str {
    if velocity.length() > HARD_SPEED {
        THROW_HARD_SOUND
    } else {
        THROW_SOUND
    }
}

/// The way `pitch` and `yaw` (degrees) look.
fn looking(pitch: f32, yaw: f32) -> Vec3 {
    let (pitch, yaw) = (pitch.to_radians(), yaw.to_radians());
    Vec3::new(
        yaw.cos() * pitch.cos(),
        yaw.sin() * pitch.cos(),
        -pitch.sin(),
    )
}

/// The player's own body as a monitor may strike it this frame.
enum Target {
    /// Standing: their box (low and high corners), their speed and the points of a
    /// body standing there.
    Standing {
        low: Vec3,
        high: Vec3,
        velocity: Vec3,
        points: [Vec3; JOINTS],
    },
    /// Limp: the ragdoll's points.
    Limp([Vec3; JOINTS]),
}

impl Target {
    /// The point of the body nearest a monitor at `centre`, if the monitor reaches the
    /// body, and the body's speed.
    fn struck(&self, centre: Vec3) -> Option<(u8, Vec3)> {
        let nearest = |points: &[Vec3; JOINTS]| {
            (0..JOINTS)
                .min_by(|a, b| {
                    points[*a]
                        .distance_squared(centre)
                        .total_cmp(&points[*b].distance_squared(centre))
                })
                .map(|joint| joint as u8)
        };
        match self {
            Self::Standing {
                low,
                high,
                velocity,
                points,
            } => {
                let closest = centre.clamp(*low, *high);
                (closest.distance(centre) <= flight::RADIUS * 0.8)
                    .then(|| nearest(points).map(|joint| (joint, *velocity)))
                    .flatten()
            }
            Self::Limp(points) => {
                let joint = nearest(points)?;
                (points[usize::from(joint)].distance(centre) <= flight::RADIUS * 0.8 + 4.0)
                    .then_some((joint, Vec3::ZERO))
            }
        }
    }

    /// The first point of a monitor's way from `from` to `to` (a step) where it reaches
    /// the body, with what [`Target::struck`] tells there.
    fn struck_along(&self, from: Vec3, to: Vec3) -> Option<(Vec3, u8, Vec3)> {
        let pieces = ((to - from).length() / SWEEP_PIECE).ceil().max(1.0) as u32;
        (1..=pieces).find_map(|piece| {
            let at = from.lerp(to, piece as f32 / pieces as f32);
            self.struck(at).map(|(joint, body)| (at, joint, body))
        })
    }

    /// Where the body is, for the way a monitor comes at it.
    fn centre(&self) -> Vec3 {
        match self {
            Self::Standing { low, high, .. } => (*low + *high) * 0.5,
            Self::Limp(points) => points[0],
        }
    }
}

/// The push a body takes from a monitor going `monitor` against its own `body` speed;
/// `None` when too slow to knock it down.
fn launch(monitor: Vec3, body: Vec3) -> Option<[i32; 3]> {
    let against = monitor - body;
    if against.length() < MIN_HIT_SPEED {
        return None;
    }
    let push = (against * TRANSFER + Vec3::Z * LIFT).clamp_length_max(MAX_LAUNCH);
    Some(push.to_array().map(|c| c.round() as i32))
}

/// The bounce a monitor takes off a body pushed by `push`.
fn bounce(push: [i32; 3], step: u16) -> Deflect {
    let back = |c: i32| (-(c as f32) * BOUNCE_BACK).round() as i32;
    Deflect {
        step: u32::from(step),
        push: [back(push[0]), back(push[1]), BOUNCE_UP],
    }
}

impl GpuState {
    /// `toy_monitor`: on or off, said on the centre of the screen.
    pub(crate) fn toggle_monitor(&mut self) {
        self.monitor.toggle();
        self.monitor_centre(if self.monitor.on() {
            "Monitor: throw with your throw key, hold it to charge; monitors knock you flying"
        } else {
            "Monitor off: monitors pass through you"
        });
    }

    /// `monitor_throw`: throw one.
    pub(crate) fn throw_monitor(&mut self) {
        self.monitor.throw();
    }

    /// `+monitor_throw` (`pressed`) begins charging a throw, `-monitor_throw` throws it.
    /// A throw pressed during the cooldown begins charging once it ends.
    pub(crate) fn charge_monitor(&mut self, pressed: bool) {
        if !pressed {
            if let Some((since, _)) = self.monitor.charging {
                let charge = charge_of(Instant::now().saturating_duration_since(since));
                self.monitor.put_down();
                self.monitor.throw_asked = Some(charge);
            }
            return;
        }
        if self.monitor.charging.is_some() {
            return;
        }
        if let Some(why) = self.monitor_refusal() {
            self.monitor_centre(why);
            return;
        }
        let now = Instant::now();
        let ready = self
            .monitor
            .last_throw
            .map_or(now, |last| (last + COOLDOWN).max(now));
        self.monitor.charging = Some((ready, false));
    }

    /// Why the player may not throw now, if they may not.
    fn monitor_refusal(&self) -> Option<&'static str> {
        if !self.monitor.on {
            Some("Turn the Monitor toy on to throw one")
        } else if self.ragdoll.on() || !self.ragdoll.strikable() {
            Some("No monitor right now")
        } else {
            None
        }
    }

    /// Say `text` on the centre of the screen.
    fn monitor_centre(&mut self, text: &str) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text.to_owned(),
            None,
            Instant::now(),
        );
    }

    /// The frame's toy logic, from the relay's events: throws heard, the player's own
    /// throw, and those to tell of it. After the Ragdoll's, whose blows bounce the
    /// monitors.
    pub(crate) fn update_monitor(&mut self, relay_events: &[sjk_identity::ToyEvent]) {
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            if self.monitor.server.is_some() {
                self.monitor.leave();
            }
            self.monitor.put_down();
            self.monitor.throw_asked = None;
            self.hud
                .set_monitor_marks(crate::hud::MonitorMarks::default());
            return;
        };
        let server = session.server();
        if self.monitor.server != Some(server) {
            self.monitor.leave();
            self.monitor.server = Some(server);
        }
        let game_state = session.game_state();
        let gametype = game_state
            .config_string(0)
            .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"))
            .unwrap_or(0);
        self.monitor.duel = crate::order66::duelling(&session.latest_snapshot().player, gametype);
        self.monitor.unseen = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(SHOW_CVAR))
            == Some(false);
        let own = u8::try_from(game_state.client_num).ok();
        let now = self.monitor.time;
        let heard: Vec<(u8, Event)> = relay_events
            .iter()
            .filter(|event| {
                event.toy == TOY
                    && Some(event.slot) != own
                    && usize::from(event.slot) < SLOTS
                    && !self.muted_players.contains(u16::from(event.slot))
                    && crate::player_identity::shown_name(game_state, usize::from(event.slot))
                        .is_some_and(|shown| sjk_identity::names_match(&event.claimed_name, &shown))
            })
            .filter_map(|event| Some((event.slot, Event::parse(&event.event)?)))
            .collect();
        for (slot, event) in heard {
            let moment = i64::from(event.throw.moment);
            let known = self
                .monitor
                .find(slot, event.number)
                .is_some_and(|shown| *shown.flight.throw() == event.throw);
            if known || moment > now + EARLY_MS || moment < now - LATE_MS {
                continue;
            }
            self.monitor
                .put(Shown::new(slot, event.number, &event.throw));
            if moment >= now - 500 {
                self.monitor.cues.push(Cue {
                    path: throw_sound(Vec3::from_array(event.throw.velocity.map(|c| c as f32))),
                    at: Some(Vec3::from_array(event.throw.origin.map(|c| c as f32))),
                    volume: 0.9,
                });
            }
        }
        // The monitors that struck a body bounce off it.
        for hit in self.ragdoll.take_bounces() {
            let deflect = bounce(hit.push, hit.by.step);
            let Some(shown) = self.monitor.find(hit.by.thrower, hit.by.number) else {
                continue;
            };
            let when = i64::from(shown.flight.throw().moment)
                + i64::from(hit.by.step) * i64::from(STEP_MS);
            if !shown.flight.deflect(deflect) {
                shown.flight = shown.flight.rerun(deflect);
            }
            shown.rest = None;
            let at = Some(shown.flight.centre());
            self.monitor.cues.push(Cue {
                path: BONK_SOUND,
                at,
                volume: 1.0,
            });
            // The player's own monitor struck someone else, just now: the hit marker.
            if Some(hit.by.thrower) == own && Some(hit.struck) != own && now - when <= MARK_LATE_MS
            {
                let hard = Vec3::from_array(hit.push.map(|c| c as f32)).length() >= STRONG_PUSH;
                self.monitor.mark = Some((Instant::now(), hard));
                self.monitor.cues.push(Cue {
                    path: HIT_SOUND,
                    at: None,
                    volume: if hard { 1.0 } else { 0.8 },
                });
            }
        }
        self.monitor_charging();
        if let Some(charge) = self.monitor.throw_asked.take() {
            self.monitor_throw_now(own, charge);
        }
        self.monitor_tell(own);
        self.hud.set_monitor_marks(self.monitor.marks());
    }

    /// A throw held: its sound once it begins charging; put down when the player may
    /// no longer throw, or held too long.
    fn monitor_charging(&mut self) {
        let Some((since, sounded)) = self.monitor.charging else {
            return;
        };
        let now = Instant::now();
        if self.monitor_refusal().is_some() || now.saturating_duration_since(since) > CHARGE_LIMIT {
            self.monitor.put_down();
            return;
        }
        if !sounded && now >= since {
            self.monitor.charging = Some((since, true));
            self.monitor.cues.push(Cue {
                path: CHARGE_SOUND,
                at: None,
                volume: 0.7,
            });
        }
    }

    /// The player throws a monitor charged `charge` (0 to 1) now, if they may.
    fn monitor_throw_now(&mut self, own: Option<u8>, charge: f32) {
        let Some(own) = own else {
            return;
        };
        if let Some(why) = self.monitor_refusal() {
            self.monitor_centre(why);
            return;
        }
        if self
            .monitor
            .last_throw
            .is_some_and(|last| last.elapsed() < COOLDOWN)
        {
            return;
        }
        let Some(state) = self.local_prediction.predicted_state() else {
            return;
        };
        let eyes = Vec3::from_array(state.origin) + Vec3::Z * state.view_height as f32;
        let way = looking(state.view_angles[0], state.view_angles[1]);
        let velocity = way * throw_speed(charge) + Vec3::from_array(state.velocity);
        // Out of the hand, short of a wall in the way.
        let mut movers = std::mem::take(&mut self.monitor.movers);
        let hand = {
            let mut world = crate::ragdoll::Scene::new(
                &self.bsp,
                &mut self.trace_scratch,
                self.live_session.as_ref(),
                &mut movers,
            );
            let wanted = eyes + way * HAND;
            crate::ragdoll::World::trace(&mut world, eyes, wanted, flight::RADIUS * 0.5)
                .map_or(wanted, |hit| hit.stop)
        };
        self.monitor.movers = movers;
        let throw = Throw {
            moment: self.monitor.time as i32,
            origin: hand.to_array().map(|c| c.round() as i32),
            velocity: velocity
                .to_array()
                .map(|c| c.clamp(-3_999.0, 3_999.0).round() as i32),
        };
        let number = self.monitor.next_number;
        self.monitor.next_number = (number + 1) % NUMBERS;
        self.monitor.last_throw = Some(Instant::now());
        self.monitor.put(Shown::new(own, number, &throw));
        self.monitor.cues.push(Cue {
            path: throw_sound(velocity),
            at: Some(hand),
            volume: 0.9,
        });
    }

    /// Tell the next SJK player on the server of the player's newest monitor not yet
    /// told of, the nearest first.
    fn monitor_tell(&mut self, own: Option<u8>) {
        let Some(own) = own else {
            return;
        };
        let now = Instant::now();
        if self.monitor.tell_at.is_some_and(|at| now < at) {
            return;
        }
        let sjk = self.looks.sjk_slots() & !(1 << own);
        let time = self.monitor.time;
        let world = &self.live_world;
        let Some(shown) = self
            .monitor
            .shown
            .iter_mut()
            .rev()
            .find(|shown| shown.thrower == own && sjk & !shown.told != 0)
        else {
            return;
        };
        let from = Vec3::from_array(shown.flight.throw().origin.map(|c| c as f32));
        let Some(slot) = (0..SLOTS as u8)
            .filter(|slot| (sjk & !shown.told) & (1 << slot) != 0)
            .min_by(|a, b| {
                let distance = |slot: &u8| {
                    world
                        .entity(EntityId::new(u64::from(*slot) + 1))
                        .map_or(f32::MAX, |entity| {
                            Vec3::from_array(entity.sample(time).translation).distance(from)
                        })
                };
                distance(a).total_cmp(&distance(b))
            })
        else {
            return;
        };
        shown.told |= 1 << slot;
        self.monitor.tell_at = Some(now + SEND_GAP);
        relay(
            Event {
                number: shown.number,
                throw: *shown.flight.throw(),
            },
            slot,
        );
    }

    /// The player's own body as a monitor may strike it, if it may be struck.
    fn monitor_target(&self) -> Option<Target> {
        if !self.monitor.on || !self.ragdoll.strikable() {
            return None;
        }
        if let Some(points) = self.ragdoll.own_points() {
            return Some(Target::Limp(points));
        }
        let state = self.local_prediction.predicted_state()?;
        let origin = Vec3::from_array(state.origin);
        // The player's box: `playerMins`/`playerMaxs`, the top lower crouched.
        let top = (state.view_height as f32 + 4.0).max(16.0);
        Some(Target::Standing {
            low: origin + Vec3::new(-15.0, -15.0, -24.0),
            high: origin + Vec3::new(15.0, 15.0, top),
            velocity: Vec3::from_array(state.velocity),
            points: crate::ragdoll::standing(origin, state.view_angles[1]),
        })
    }

    /// Step every monitor on to `time` (game server milliseconds, the presentation's),
    /// strike the player with one that reaches them, and let rested ones go. Before the
    /// Ragdoll's bodies are stepped, so a blow lands this frame.
    pub(crate) fn step_monitors(&mut self, time: i64) {
        self.monitor.time = time;
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            return;
        };
        let own = u8::try_from(session.game_state().client_num).ok();
        let target = self.monitor_target();
        let struck = self.monitor.struck;
        let mut hits = Vec::new();
        let mut movers = std::mem::take(&mut self.monitor.movers);
        let mut world = crate::ragdoll::Scene::new(
            &self.bsp,
            &mut self.trace_scratch,
            Some(session),
            &mut movers,
        );
        for (index, shown) in self.monitor.shown.iter_mut().enumerate() {
            let since = (time - i64::from(shown.flight.throw().moment)).max(0);
            let steps = (since / i64::from(STEP_MS)) as u32;
            let mut knock = 0.0_f32;
            while shown.flight.steps() < steps {
                let step = shown.flight.steps() + 1;
                let before = shown.flight.centre();
                shown.flight.run_to(step, &mut world);
                knock = knock.max(shown.flight.knock);
                let Some(target) = target.as_ref() else {
                    continue;
                };
                let at =
                    i64::from(shown.flight.throw().moment) + i64::from(step) * i64::from(STEP_MS);
                let immune = struck.is_some_and(|(when, thrower, number)| {
                    at - when < IMMUNE
                        || ((thrower, number) == (shown.thrower, shown.number)
                            && at - when < IMMUNE_SAME)
                });
                if immune
                    || (Some(shown.thrower) == own && step < SELF_SAFE_STEPS)
                    || shown.flight.asleep()
                {
                    continue;
                }
                let Some((centre, joint, body)) =
                    target.struck_along(before, shown.flight.centre())
                else {
                    continue;
                };
                let velocity = shown.flight.velocity();
                if velocity.dot(target.centre() - centre) <= 0.0 {
                    continue;
                }
                if let Some(push) = launch(velocity, body) {
                    hits.push((index, at, joint, push, step));
                    // The rest of its flight waits for the bounce, next frame.
                    break;
                }
            }
            // The knocks of this frame's steps (not while fast-forwarding).
            if knock > flight::KNOCK_SPEED
                && steps.saturating_sub(shown.flight.steps()) < 8
                && shown
                    .knocked
                    .is_none_or(|knocked| knocked.elapsed() >= KNOCK_GAP)
            {
                shown.knocked = Some(Instant::now());
                self.monitor.cues.push(Cue {
                    path: KNOCK_SOUND,
                    at: Some(shown.flight.centre()),
                    volume: (knock / 900.0).clamp(0.25, 1.0),
                });
            }
            if shown.flight.asleep() {
                shown.rest.get_or_insert_with(Instant::now);
            }
        }
        self.monitor.movers = movers;
        self.monitor.shown.retain(|shown| shown.level() > 0.0);
        // At most one blow a frame: the first.
        if let Some((index, at, joint, push, step)) = hits.into_iter().next() {
            let Some(shown) = self.monitor.shown.get(index) else {
                return;
            };
            let by = By {
                thrower: shown.thrower,
                number: shown.number,
                step: u16::try_from(step).unwrap_or(u16::MAX),
            };
            // Struck or not (a fall takes so many blows), it is immune a while, so the
            // monitor flies on.
            self.ragdoll_struck(at, joint, push, by);
            self.monitor.struck = Some((at, by.thrower, by.number));
        }
    }

    /// Add this frame's monitors, between their last two steps.
    pub(crate) fn submit_monitors(&mut self, presentation_time: i64) {
        if self.monitor.duel || self.monitor.unseen {
            return;
        }
        let Some(mesh) = self
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance.model == MODEL)
        else {
            return;
        };
        for shown in &self.monitor.shown {
            let since = (presentation_time - i64::from(shown.flight.throw().moment)).max(0);
            let fraction = if shown.flight.asleep() {
                1.0
            } else {
                (since % i64::from(STEP_MS)) as f32 / STEP_MS as f32
            };
            let (centre, turn) = shown.flight.pose(fraction);
            self.object_groups[mesh].push(ActorInstance::new(
                centre.to_array(),
                turn.to_array(),
                [shown.level(); 3],
            ));
        }
    }

    /// Play the toy's sounds of the frame.
    pub(crate) fn play_monitor_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.monitor.cues.clear();
            return;
        };
        if self.monitor.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(
                vfs,
                [
                    THROW_SOUND,
                    BONK_SOUND,
                    KNOCK_SOUND,
                    CHARGE_SOUND,
                    THROW_HARD_SOUND,
                    HIT_SOUND,
                ],
            );
            self.monitor.sounds_registered = Some(audio.generation());
        }
        if std::mem::take(&mut self.monitor.stop_charge) {
            audio.stop_toy(CHARGE_SOURCE, CHARGE_CHANNEL);
        }
        for Cue { path, at, volume } in self.monitor.cues.drain(..) {
            // In a duel nothing is heard; unseen, no monitor in the world is.
            if self.monitor.duel || (self.monitor.unseen && at.is_some()) {
                continue;
            }
            let (source, channel) = if path == CHARGE_SOUND {
                (CHARGE_SOURCE, CHARGE_CHANNEL)
            } else {
                (sjk_audio::SourceId(0x5100_0000), sjk_audio::ChannelId(0))
            };
            audio.play_toy(
                crate::toy_sounds::Toy::Monitor,
                path,
                volume,
                at.map(|at| at.to_array()),
                source,
                channel,
            );
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: monitor `number` of the player in `slot` thrown from
    /// `origin` at `velocity`, shown `steps` steps on.
    pub(crate) fn shot_monitor(
        &mut self,
        slot: u8,
        number: u8,
        origin: [i32; 3],
        velocity: [i32; 3],
        steps: u32,
    ) {
        let throw = Throw {
            moment: 0,
            origin,
            velocity,
        };
        let mut shown = Shown::new(slot, number, &throw);
        let mut movers = crate::ragdoll::Movers::default();
        shown.flight.run_to(
            steps,
            &mut crate::ragdoll::Scene::new(&self.bsp, &mut self.trace_scratch, None, &mut movers),
        );
        self.monitor.put(shown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throw_events_read_back_as_written_and_fit_the_relay() {
        let event = Event {
            number: 99,
            throw: Throw {
                moment: -2_147_000_000,
                origin: [-65_536, 65_536, -12_345],
                velocity: [-3_999, 3_999, -4_000],
            },
        };
        let text = event.text();
        assert!(text.len() <= 64, "{text}");
        assert!(
            text.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-'),
            "{text}"
        );
        assert_eq!(Event::parse(&text), Some(event));
        assert_eq!(Event::parse("throw_100_0_0_0_0_0_0_0"), None, "number 100");
        assert_eq!(Event::parse("throw_1_0_0_0_0_0_0_4001"), None, "too fast");
        assert_eq!(Event::parse("throw_1_0_0_0_0_0_0"), None);
        assert_eq!(Event::parse("fall_1_0_0_0_0_0_0_0"), None);
    }

    #[test]
    fn a_slow_monitor_does_not_knock_anyone_down_and_a_fast_one_sends_them_flying() {
        assert_eq!(launch(Vec3::new(200.0, 0.0, 0.0), Vec3::ZERO), None);
        // Running away from it as fast as it comes: nothing.
        assert_eq!(
            launch(Vec3::new(1_100.0, 0.0, 0.0), Vec3::new(900.0, 0.0, 0.0)),
            None
        );
        let push = launch(Vec3::new(1_100.0, 0.0, 0.0), Vec3::ZERO).unwrap();
        assert_eq!(push, [1_650, 0, 260]);
        let hard = launch(Vec3::new(3_000.0, 3_000.0, 0.0), Vec3::ZERO).unwrap();
        assert!(hard.iter().all(|c| c.abs() <= crate::ragdoll::MAX_PUSH));
        let length = Vec3::from_array(hard.map(|c| c as f32)).length();
        assert!(length <= MAX_LAUNCH + 1.0, "{length}");
        // The monitor goes back off the body, slower, and up.
        let deflect = bounce(push, 30);
        assert_eq!(deflect.step, 30);
        assert_eq!(deflect.push, [-825, 0, BOUNCE_UP]);
    }

    #[test]
    fn a_standing_player_is_struck_where_the_monitor_meets_them() {
        let origin = Vec3::new(0.0, 0.0, 24.0);
        let target = Target::Standing {
            low: origin + Vec3::new(-15.0, -15.0, -24.0),
            high: origin + Vec3::new(15.0, 15.0, 40.0),
            velocity: Vec3::ZERO,
            points: crate::ragdoll::standing(origin, 0.0),
        };
        // High: the head; low: a foot; far off: nothing.
        let (head, _) = target.struck(Vec3::new(-20.0, 0.0, 62.0)).unwrap();
        assert_eq!(usize::from(head), 2);
        let (foot, _) = target.struck(Vec3::new(-20.0, -4.0, 6.0)).unwrap();
        assert!(matches!(foot, 11 | 14), "{foot}");
        assert!(target.struck(Vec3::new(-40.0, 0.0, 40.0)).is_none());
    }

    #[test]
    fn a_held_throw_charges_to_its_top_speed_and_is_heard_as_charged() {
        assert_eq!(charge_of(Duration::ZERO), 0.0);
        assert_eq!(charge_of(CHARGE_TIME / 2), 0.5);
        assert_eq!(charge_of(CHARGE_TIME * 3), 1.0);
        assert_eq!(throw_speed(0.0), THROW_SPEED);
        assert_eq!(throw_speed(1.0), MAX_THROW_SPEED);
        assert_eq!(throw_speed(4.0), MAX_THROW_SPEED);
        assert_eq!(throw_sound(Vec3::X * throw_speed(0.0)), THROW_SOUND);
        assert_eq!(throw_sound(Vec3::X * throw_speed(1.0)), THROW_HARD_SOUND);
        // A full charge, running, still fits the throw event's limits.
        let fastest = MAX_THROW_SPEED + 1_000.0;
        assert!(fastest < 4_000.0);
        // A charged monitor's blow is the hardest a ragdoll takes, and marks red.
        let push = launch(Vec3::X * MAX_THROW_SPEED, Vec3::ZERO).unwrap();
        let length = Vec3::from_array(push.map(|c| c as f32)).length();
        assert!(
            (STRONG_PUSH..=MAX_LAUNCH + 1.0).contains(&length),
            "{length}"
        );
        let plain = launch(Vec3::X * THROW_SPEED, Vec3::ZERO).unwrap();
        assert!(Vec3::from_array(plain.map(|c| c as f32)).length() < STRONG_PUSH);
    }

    #[test]
    fn a_fast_monitor_strikes_a_body_it_passes_through_within_a_step() {
        let points = crate::ragdoll::standing(Vec3::new(0.0, 0.0, 24.0), 0.0);
        let target = Target::Limp(points);
        let head = points[2];
        // 2,600 units a second: 41.6 units a step, through the head and out.
        let from = head - Vec3::X * 20.8;
        let to = head + Vec3::X * 20.8;
        assert!(target.struck(from).is_none() && target.struck(to).is_none());
        let (at, joint, _) = target.struck_along(from, to).unwrap();
        assert_eq!(usize::from(joint), 2);
        assert!(at.x < head.x, "met on the way in: {at}");
        assert!(
            target
                .struck_along(from + Vec3::Z * 60.0, to + Vec3::Z * 60.0)
                .is_none()
        );
    }

    #[test]
    fn a_player_keeps_four_monitors_out_and_a_number_replaces_its_own() {
        let mut monitor = Monitor::default();
        let throw = |moment| Throw {
            moment,
            origin: [0; 3],
            velocity: [100, 0, 0],
        };
        for number in 0..6 {
            monitor.put(Shown::new(3, number, &throw(i32::from(number))));
        }
        monitor.put(Shown::new(4, 0, &throw(0)));
        let mine: Vec<u8> = monitor
            .shown
            .iter()
            .filter(|shown| shown.thrower == 3)
            .map(|shown| shown.number)
            .collect();
        assert_eq!(mine, [2, 3, 4, 5]);
        monitor.put(Shown::new(3, 4, &throw(99)));
        assert_eq!(monitor.find(3, 4).unwrap().flight.throw().moment, 99);
        assert_eq!(monitor.shown.len(), 5);
    }
}
