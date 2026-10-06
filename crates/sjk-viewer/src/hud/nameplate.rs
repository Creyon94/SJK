//! MMO-style nameplates over players (and, optionally, NPCs).
//!
//! Far away a player shows only a small, dim name. Closer, the name rises and a
//! framed plate fades in beneath it with health, shield and Force bars, and the
//! weapon the player holds beside it, haloed in the saber stance's colour.
//! Names come from the chat roster and are drawn in the classic HUD font with
//! their colour codes; the layout maths is in [`super::nameplate_math`]. What the
//! server does not send is estimated, once per snapshot: Force in
//! [`super::force_estimate`], health and shield in [`super::vitals_estimate`],
//! each as a range whose uncertainty the bar shows as a grey haze. The plain
//! TaystJK names stay in [`super::identification`] (`cg_drawPlayerNames`); a
//! nameplate replaces them.
use super::estimate::Range;
use super::force_estimate::{self, Calibration, Estimator};
use super::identification::{Camera, friend_icon, info_number, unoccluded};
use super::nameplate_math::{self as math, Rows, Stack};
use super::vitals_estimate;
use crate::{TextVertex, UiFont, chat::ChatOverlay, console::ViewerConsole};
use glam::Vec3;
use sjk_bsp::{Aabb, Bsp, TraceScratch};
use sjk_client::TeamInfoTable;
use sjk_game_jka::force_powers::FP_DRAIN;
use sjk_protocol::{GameState, Snapshot};
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{
    Color, DrawCommand, DrawList, FontWeight, Gradient, Rect, TextAlign, TextId, TextOverflow,
    TextureId,
};

/// Text ids below this are player slots; above it, `NPC_TEXT + class_t`.
const NPC_TEXT: u32 = 1024;
/// Text id of the "?" over a bar the estimate cannot fill.
const UNKNOWN_TEXT: u32 = 1000;
/// A health or shield range this wide (shares of a full bar) is too unsure to show:
/// the bar dims and a "?" stands over it until something narrows it.
const UNSURE_WIDTH: f32 = 0.6;
/// A shield whose high bound is under this share is known to be empty.
const EMPTY_SHARE: f32 = 0.005;
/// `ET_PLAYER` and `ET_NPC` entity types, `EF_DEAD`, and `PW_FORCE_BOON`.
const ET_PLAYER: u8 = 1;
const ET_NPC: u8 = 13;
const EF_DEAD: u32 = 2;
const PW_FORCE_BOON: u32 = 14;
/// `GT_JEDIMASTER` and the first team game type.
const GT_JEDIMASTER: i32 = 2;
/// `WP_SABER`.
const WP_SABER: u8 = 3;
const GT_TEAM: i32 = 6;
/// Tags drawn at most: every client slot, plus a bounded number of NPCs.
const MAX_PLAYER_TAGS: usize = 32;
const MAX_NPC_TAGS: usize = 16;
const MAX_TAGS: usize = MAX_PLAYER_TAGS + MAX_NPC_TAGS;
/// Entity numbers are ten bits on the wire.
const ENTITY_SLOTS: usize = 1024;
/// A plate unseen this long fades in again instead of resuming.
const STALE_MILLIS: i64 = 250;
/// Opacity of a plate behind a wall when `cg_nameplateWalls` is on.
const WALL_OPACITY: f32 = 0.35;
/// Allowed overshoot of the screen (in half-screens) before a plate is dropped.
const SCREEN_MARGIN: f32 = 1.15;
/// Smallest size, as a share of full, a plate at the end of its range shrinks to.
const MIN_SCALE: f32 = 0.6;
/// Opacity of a name too far for a plate.
const FAR_NAME_OPACITY: f32 = 0.7;
/// Full health and armour, for teammates (`tinfo` sends points, not a share).
const FULL_POINTS: f32 = 100.0;
/// Power icons shown over a name at most.
const MAX_ICONS: usize = 4;
/// Force powers whose icon shows while they are on, dark side first, then the
/// light side and neutral ones, as `forcePowers_t` indices. Push, pull, jump and
/// the saber powers are left out: they last a moment and would only flicker.
const ICON_POWERS: [u8; 11] = [7, 6, 13, 8, 9, 10, 2, 0, 12, 5, 14];
/// Frame colour of players who follow no team.
const NEUTRAL_ACCENT: Color = Color::new(0.78, 0.82, 0.9, 0.9);
/// Frame colour of NPC plates.
const NPC_ACCENT: Color = Color::new(0.95, 0.8, 0.35, 0.9);
/// The grey haze over a bar's uncertain stretch, at its thickest.
const HAZE: Color = Color::new(0.8, 0.82, 0.86, 0.6);
/// A range narrower than this share of the bar is drawn sharp.
const HAZE_MIN_WIDTH: f32 = 0.02;
/// Backdrop of the power and weapon icons.
const ICON_BACKDROP: Color = Color::new(0.03, 0.04, 0.06, 0.55);
/// Halo opacity of a holstered saber, as a share of a lit one's.
const HOLSTERED_HALO: f32 = 0.4;
/// `WP_NONE`.
const WP_NONE: u8 = 0;
/// Milliseconds between reads of which players are verified.
const VERIFIED_EVERY: i64 = 1_000;

/// Register the nameplate settings.
pub(super) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        "cg_nameplate",
        true,
        CvarFlags::ARCHIVE,
        "MMO-style nameplates over players; they replace cg_drawPlayerNames",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateRange",
        3000_i64,
        CvarFlags::ARCHIVE,
        "Distance in units out to which nameplates show",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateNear",
        1000_i64,
        CvarFlags::ARCHIVE,
        "Distance in units inside which a nameplate shows its bars",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateScale",
        0.5_f64,
        CvarFlags::ARCHIVE,
        "Nameplate text size",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateBars",
        2_i64,
        CvarFlags::ARCHIVE,
        "Nameplate bars: 0 none, 1 allies only, 2 everyone",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateForce",
        true,
        CvarFlags::ARCHIVE,
        "Nameplate Force bar, estimated from the player's powers",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplatePredict",
        true,
        CvarFlags::ARCHIVE,
        "Estimate the health and shield the server does not send, from the hits, pains and pickups it does",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateWeapon",
        true,
        CvarFlags::ARCHIVE,
        "Icon of the weapon a player holds beside the nameplate, a saber's haloed in its stance's colour",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateIcons",
        true,
        CvarFlags::ARCHIVE,
        "Icons of the Force powers a player has on, over the nameplate when close",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateWalls",
        false,
        CvarFlags::ARCHIVE,
        "Show nameplates dimmed through walls",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateNpcs",
        false,
        CvarFlags::ARCHIVE,
        "Nameplates on NPCs: their class and health",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateSelf",
        false,
        CvarFlags::ARCHIVE,
        "Your own nameplate over your head in third person, with your real bars",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateDebug",
        false,
        CvarFlags::NONE,
        "Log what the server sends about other players every two seconds",
    ))?;
    Ok(())
}

/// Settings, sampled once per frame.
#[derive(Clone, Copy)]
struct Settings {
    enabled: bool,
    range: f32,
    near: f32,
    scale: f32,
    bars: i64,
    force: bool,
    predict: bool,
    weapon: bool,
    walls: bool,
    npcs: bool,
    icons: bool,
    friends: bool,
    /// The local player's own plate, in third person.
    own: bool,
    debug: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            range: 3000.0,
            near: 1000.0,
            scale: 0.5,
            bars: 2,
            force: true,
            predict: true,
            weapon: true,
            walls: false,
            npcs: false,
            icons: true,
            friends: true,
            own: false,
            debug: false,
        }
    }
}

/// One visible plate, collected in `update` and drawn in `append`.
#[derive(Clone, Copy)]
struct Entry {
    /// Player slot, or NPC entity number.
    number: u16,
    /// `class_t` for an NPC, zero for a player.
    npc_class: u8,
    /// Screen position of the point above the head.
    point: [f32; 2],
    distance: f32,
    /// Size multiplier from distance.
    scale: f32,
    /// Smoothed opacity, distance fade included.
    alpha: f32,
    /// How much of the plate shows, 0 (far: name only) to 1.
    detail: f32,
    /// How close the player is, 0 (far) to 1: the same ramp, whether or not there are bars.
    proximity: f32,
    /// Force powers to show icons for (`forcePowers_t` indices), `power_count` of them.
    powers: [u8; MAX_ICONS],
    power_count: u8,
    /// Shares of a full bar.
    health: Option<Range>,
    shield: Option<Range>,
    force: Option<Range>,
    /// The weapon held (`WP_NONE` for no icon), the saber style (`fireflag`, which
    /// carries `fd.saberAnimLevel`) and whether the blade is put away.
    weapon: u8,
    style: u8,
    holstered: bool,
    accent: Color,
    icon: Option<Color>,
    names_allowed: bool,
    /// The SJK hub's operator vouches for this player: the gold badge after the name.
    verified: bool,
}

impl Entry {
    fn rows(&self) -> Rows {
        Rows {
            health: self.health.is_some(),
            shield: self.shield.is_some(),
            force: self.force.is_some(),
        }
    }
}

/// Smoothed opacity of one entity's plate.
#[derive(Clone, Copy, Default)]
struct Fade {
    alpha: f32,
    seen: i64,
}

/// Fixed-capacity plate state: at most 32 clients and 16 NPCs, no owned name strings.
pub(crate) struct State {
    /// Shapes and text IDs submitted through the normal HUD renderer.
    pub(crate) list: DrawList,
    settings: Settings,
    entries: Vec<Entry>,
    fades: Box<[Fade; ENTITY_SLOTS]>,
    last_update: i64,
    last_debug: i64,
    force: Estimator,
    calibration: Calibration,
    /// Where the Force regeneration pace came from, for the debug log.
    regen_source: &'static str,
    vitals: vitals_estimate::Estimator,
    /// The health, shield and Force colours of the HUD in use, which the bars take.
    colors: BarColors,
    /// Slots the hub's operator vouches for, as bits, and when they were last read.
    verified: u32,
    verified_read: Option<i64>,
    /// Where the local player stands, while their own plate may show (third person).
    own_origin: Option<Vec3>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            list: DrawList::new(MAX_TAGS * 40 + 8),
            settings: Settings::default(),
            entries: Vec::with_capacity(MAX_TAGS),
            fades: Box::new([Fade::default(); ENTITY_SLOTS]),
            last_update: 0,
            last_debug: 0,
            force: Estimator::default(),
            calibration: Calibration::default(),
            regen_source: "default",
            vitals: vitals_estimate::Estimator::default(),
            colors: BarColors::default(),
            verified: 0,
            verified_read: None,
            own_origin: None,
        }
    }
}

impl State {
    /// Sample settings once, outside text emission.
    pub(crate) fn sample(&mut self, console: Option<&ViewerConsole>) {
        let flag =
            |name: &str, default: bool| console.and_then(|c| c.bool_cvar(name)).unwrap_or(default);
        let number = |name: &str, default: f32| {
            console
                .and_then(|c| c.integer_cvar(name))
                .map_or(default, |value| value as f32)
        };
        self.settings = Settings {
            enabled: flag("cg_nameplate", true),
            range: number("cg_nameplaterange", 3000.0).clamp(500.0, 10_000.0),
            near: number("cg_nameplatenear", 1000.0).clamp(0.0, 10_000.0),
            scale: crate::cgame_options::scalar(console, "cg_nameplatescale", 0.5).clamp(0.1, 3.0),
            bars: console
                .and_then(|c| c.integer_cvar("cg_nameplatebars"))
                .unwrap_or(2),
            force: flag("cg_nameplateforce", true),
            predict: flag("cg_nameplatepredict", true),
            weapon: flag("cg_nameplateweapon", true),
            walls: flag("cg_nameplatewalls", false),
            npcs: flag("cg_nameplatenpcs", false),
            icons: flag("cg_nameplateicons", true),
            friends: flag("cg_drawfriend", true),
            own: flag("cg_nameplateself", false),
            debug: flag("cg_nameplatedebug", false),
        };
    }

    /// Whether nameplates replace the plain overhead names.
    pub(crate) fn enabled(&self) -> bool {
        self.settings.enabled
    }

    /// Take the bars' colours from the HUD in use (its health, armour and Force
    /// meters); `None` for one it names none for (the game-data HUD draws
    /// pictures), which gets the retail colour.
    pub(crate) fn set_hud_colors(
        &mut self,
        health: Option<Color>,
        shield: Option<Color>,
        force: Option<Color>,
    ) {
        self.colors = BarColors {
            health: health.unwrap_or(math::HEALTH_COLOR),
            shield: shield.unwrap_or(math::SHIELD_COLOR),
            force: force.unwrap_or(math::FORCE_COLOR),
        };
    }

    /// Read which slots are verified (`read` asks the identity service) at most once a
    /// second, so the names it compares are not rebuilt every frame.
    pub(crate) fn refresh_verified(&mut self, now: i64, read: impl FnOnce() -> u32) {
        if self
            .verified_read
            .is_some_and(|last| (0..VERIFIED_EVERY).contains(&(now - last)))
        {
            return;
        }
        self.verified_read = Some(now);
        self.verified = read();
    }

    /// Where the local player stands, for their own plate; `None` hides it (first
    /// person, where it would sit inside the camera).
    pub(crate) fn set_own_origin(&mut self, origin: Option<[f32; 3]>) {
        self.own_origin = origin.map(Vec3::from_array);
    }

    /// Feed one accepted snapshot to the estimates. Every snapshot is observed
    /// once, in order, whether or not plates are shown, so no event is missed.
    pub(crate) fn observe_snapshot(&mut self, snapshot: &Snapshot, game: &GameState) {
        let mode = info_number(game.config_string(0), "g_gametype");
        self.vitals.observe(snapshot, game, mode);
        self.observe_force(snapshot, game, mode);
        let drained = self.vitals.drained();
        for slot in 0..32_u16 {
            if drained & (1 << slot) != 0 {
                self.force.drained(slot);
            }
        }
    }

    /// Drop every collected plate and drawn shape.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.list.clear();
    }

    /// Smooth one entity's opacity towards `target`.
    fn fade(&mut self, number: u16, target: f32, now: i64, step: f32) -> f32 {
        let fade = &mut self.fades[usize::from(number) % ENTITY_SLOTS];
        if now - fade.seen > STALE_MILLIS {
            fade.alpha = 0.0;
        }
        fade.seen = now;
        fade.alpha += (target - fade.alpha).clamp(-step, step);
        fade.alpha
    }

    /// Collect visible players using fixed scratch and the actual rendered camera.
    pub(crate) fn update(
        &mut self,
        snapshot: &Snapshot,
        game: &GameState,
        world: &sjk_runtime::World,
        team_info: &TeamInfoTable,
        now: i64,
        camera: Camera,
        bsp: &Bsp,
        scratch: &mut TraceScratch,
        hidden: bool,
    ) {
        self.clear();
        let step = (now - self.last_update).clamp(0, 100) as f32 / math::FADE_MILLIS;
        self.last_update = now;
        let settings = self.settings;
        if !settings.enabled {
            return;
        }
        let mode = info_number(game.config_string(0), "g_gametype");
        let local = snapshot.player.client_num();
        if settings.debug && now - self.last_debug >= 2_000 {
            self.last_debug = now;
            self.log_players(snapshot, game, team_info);
        }
        if hidden {
            return;
        }
        let restrictions = info_number(game.config_string(0), "restricts");
        let local_team = snapshot.player.team() as i32;
        let local_duel = info_number(game.config_string(1131 + usize::from(local)), "ds");
        let master = snapshot
            .entities
            .iter()
            .any(|e| e.number() < 32 && e.is_jedi_master());
        let local_master = snapshot.player.is_jedi_master();
        let mut npcs = 0;
        for entity in &snapshot.entities {
            let number = entity.number();
            let player = entity.entity_type() == ET_PLAYER && number < 32 && number != local;
            let npc = !player
                && settings.npcs
                && entity.entity_type() == ET_NPC
                && usize::from(number) < ENTITY_SLOTS
                && super::npc_class::name(entity.npc_class()).is_some()
                && entity.health() > 0
                && npcs < MAX_NPC_TAGS;
            if !(player || npc)
                || entity.e_flags() & EF_DEAD != 0
                || entity.client_bitflag(local)
                || entity.powerups() & (1 << 11) != 0
            {
                continue;
            }
            let (accent, icon, names_allowed, ally) = if player {
                let info = game.config_string(1131 + usize::from(number));
                let team = info_number(info, "t");
                if info.is_none() || team == 3 {
                    continue;
                }
                let icon = if settings.friends {
                    friend_icon(
                        mode,
                        local_team,
                        team,
                        local_duel,
                        info_number(info, "ds"),
                        master,
                        local_master,
                        entity.is_jedi_master(),
                    )
                } else {
                    None
                };
                if icon.is_none() && restrictions & 64 != 0 {
                    continue;
                }
                let ally = mode >= GT_TEAM && (team == 1 || team == 2) && team == local_team;
                (team_accent(mode, team), icon, restrictions & 64 == 0, ally)
            } else {
                (NPC_ACCENT, None, true, false)
            };
            let Some(presented) = world.entity(sjk_runtime::EntityId::new(u64::from(number) + 1))
            else {
                continue;
            };
            let origin = Vec3::from_array(presented.sample(now).translation);
            let distance = origin.distance(camera.eye);
            if distance >= settings.range {
                continue;
            }
            let anchor =
                origin + Vec3::Z * (math::head_height(entity.solid()) + math::HEAD_CLEARANCE);
            let Some(point) = camera.project_within(anchor, SCREEN_MARGIN) else {
                continue;
            };
            let trace = bsp.trace_box_with(
                scratch,
                camera.eye.to_array(),
                origin.to_array(),
                Aabb::new([0.0; 3], [0.0; 3]).unwrap(),
                1 | 0x0200_0000,
            );
            let visibility = if unoccluded(trace.fraction, trace.start_solid, trace.all_solid) {
                1.0
            } else if settings.walls {
                WALL_OPACITY
            } else {
                0.0
            };
            let alpha = self.fade(
                number,
                math::distance_fade(distance, settings.range) * visibility,
                now,
                step,
            );
            if alpha < 0.02 {
                continue;
            }
            let bars = names_allowed && (settings.bars == 2 || (settings.bars == 1 && ally));
            let (health, shield) = if bars {
                let predicted = (player && settings.predict).then_some(&self.vitals);
                bar_values(entity, number, ally && player, team_info, predicted)
            } else {
                (None, None)
            };
            let force = (bars && player && settings.force)
                .then(|| self.force.ratio(number))
                .flatten();
            let rows = Rows {
                health: health.is_some(),
                shield: shield.is_some(),
                force: force.is_some(),
            };
            let (powers, power_count) = if settings.icons && player && names_allowed {
                icon_powers(entity.force_powers_active())
            } else {
                ([0; MAX_ICONS], 0)
            };
            if npc {
                npcs += 1;
            }
            self.entries.push(Entry {
                number,
                npc_class: if npc { entity.npc_class() } else { 0 },
                point,
                distance,
                scale: math::distance_scale(distance, settings.range, MIN_SCALE),
                alpha,
                detail: if rows.any() {
                    math::detail(distance, settings.near)
                } else {
                    0.0
                },
                proximity: math::detail(distance, settings.near),
                powers,
                power_count,
                health,
                shield,
                force,
                weapon: if player && settings.weapon && names_allowed {
                    entity.weapon()
                } else {
                    WP_NONE
                },
                style: entity.fire_flag(),
                holstered: entity.saber_holstered() != 0,
                accent,
                icon,
                names_allowed,
                verified: player && self.verified & (1 << number) != 0,
            });
            if self.entries.len() == MAX_TAGS {
                break;
            }
        }
        if settings.own {
            self.own_plate(snapshot, mode, now, camera, step);
        }
        // Far plates first, so a near plate covers a far one.
        self.entries
            .sort_unstable_by(|a, b| b.distance.total_cmp(&a.distance));
    }

    /// `cg_nameplateSelf`: the local player's own plate over their head in third
    /// person, with the real health, shield and Force the server sends them.
    fn own_plate(&mut self, snapshot: &Snapshot, mode: i32, now: i64, camera: Camera, step: f32) {
        let settings = self.settings;
        let player = &snapshot.player;
        let Some(origin) = self.own_origin else {
            return;
        };
        if player.health() <= 0 || player.is_spectator() || self.entries.len() >= MAX_TAGS {
            return;
        }
        let local = player.client_num();
        let distance = origin.distance(camera.eye);
        if distance >= settings.range {
            return;
        }
        // `CROUCH_VIEWHEIGHT` is 12, the standing one 36: the box top drops to 16.
        let head = if player.view_height() < 24 {
            16.0
        } else {
            math::head_height(0)
        };
        let Some(point) = camera.project_within(
            origin + Vec3::Z * (head + math::HEAD_CLEARANCE),
            SCREEN_MARGIN,
        ) else {
            return;
        };
        let alpha = self.fade(
            local,
            math::distance_fade(distance, settings.range),
            now,
            step,
        );
        if alpha < 0.02 {
            return;
        }
        let bars = settings.bars != 0;
        let full = player.max_health().max(1) as f32;
        let exact = |value: i32, full: f32| Range::exact(value.max(0) as f32).share_over(full);
        let shield = Some(exact(player.armor(), full));
        let entry = Entry {
            number: local,
            npc_class: 0,
            point,
            distance,
            scale: math::distance_scale(distance, settings.range, MIN_SCALE),
            alpha,
            detail: if bars {
                math::detail(distance, settings.near)
            } else {
                0.0
            },
            proximity: math::detail(distance, settings.near),
            powers: [0; MAX_ICONS],
            power_count: 0,
            health: bars.then(|| exact(player.health(), full)),
            shield: shield.filter(|_| bars),
            force: (bars && settings.force).then(|| {
                exact(
                    i32::from(player.force_power()),
                    sjk_game_jka::force_powers::FORCE_POWER_MAX as f32,
                )
            }),
            weapon: if settings.weapon {
                player.weapon()
            } else {
                WP_NONE
            },
            style: player.saber_style(),
            holstered: player.saber_holstered() != 0,
            accent: team_accent(mode, player.team() as i32),
            icon: None,
            names_allowed: true,
            verified: self.verified & (1 << local) != 0,
        };
        let (powers, power_count) = if settings.icons {
            icon_powers(player.force_powers_active())
        } else {
            ([0; MAX_ICONS], 0)
        };
        self.entries.push(Entry {
            powers,
            power_count,
            ..entry
        });
    }

    /// `cg_nameplateDebug`: what the server sends about each other player, to
    /// settle whether enemy health reaches the client.
    fn log_players(&self, snapshot: &Snapshot, game: &GameState, team_info: &TeamInfoTable) {
        let force_keys: Vec<String> = game
            .config_string(0)
            .map(|info| {
                String::from_utf8_lossy(info)
                    .trim_start_matches('\\')
                    .split('\\')
                    .collect::<Vec<_>>()
                    .chunks(2)
                    .filter(|pair| {
                        let key = pair[0].to_ascii_lowercase();
                        key.contains("force") || key.contains("regen")
                    })
                    .map(|pair| pair.join("="))
                    .collect()
            })
            .unwrap_or_default();
        eprintln!(
            "nameplate: regen pace {:.0} ms/point ({}; measured {:?}), server info force keys {force_keys:?}, pain events {:?}",
            self.force.regen_millis(),
            self.regen_source,
            self.calibration.millis_per_point().map(f32::round),
            self.vitals.pain_report(),
        );
        let local = snapshot.player.client_num();
        eprintln!(
            "nameplate: own Force actual {} estimated {}",
            snapshot.player.force_power(),
            shown(self.force.ratio(local).map(|r| r.map(|v| v * 100.0))),
        );
        for entity in &snapshot.entities {
            let number = entity.number();
            if entity.entity_type() != ET_PLAYER || number >= 32 || number == local {
                continue;
            }
            let team = team_info
                .entries()
                .iter()
                .find(|row| u16::from(row.client_num) == number)
                .map(|row| (row.health, row.armor));
            eprintln!(
                "nameplate: client {number} health {} max {} hp~{} armor~{} powers {:#x} fp~{} weapon {} style {} tinfo {team:?}",
                entity.health(),
                entity.max_health(),
                shown(self.vitals.health(number)),
                shown(self.vitals.armor(number)),
                entity.force_powers_active(),
                shown(self.force.ratio(number).map(|r| r.map(|v| v * 100.0))),
                entity.weapon(),
                entity.fire_flag(),
            );
        }
    }

    /// Feed every player's Force use to the estimator, shown or not, and
    /// measure the server's regeneration pace from the local player's own pool.
    ///
    /// The local player is tracked too, but never shown: its estimate is
    /// compared with its real pool in the `cg_nameplateDebug` log.
    fn observe_force(&mut self, snapshot: &Snapshot, game: &GameState, mode: i32) {
        let time = snapshot.server_time;
        let player = &snapshot.player;
        let boon = player.powerup_active(PW_FORCE_BOON as usize, time);
        let master = mode == GT_JEDIMASTER && player.is_jedi_master();
        self.calibration.observe(
            time,
            i32::from(player.force_power()),
            player.force_powers_active() & !(1 << FP_DRAIN) == 0
                && !player.saber_in_flight()
                && !(player.weapon() == WP_SABER
                    && sjk_game_jka::saber_rules::in_special(player.saber_move()))
                && !boon
                && !master,
        );
        let info = game
            .config_string(0)
            .and_then(|b| sjk_client::LegacyClientInfo::new(b).integer("g_forceRegenTime"))
            .map(|millis| millis as f32);
        let measured = self.calibration.millis_per_point();
        self.regen_source = if measured.is_some() {
            "measured"
        } else if info.is_some() {
            "serverinfo"
        } else {
            "default"
        };
        self.force
            .set_regen_millis(measured.or(info), measured.is_some());
        for entity in &snapshot.entities {
            let number = entity.number();
            if entity.entity_type() != ET_PLAYER || number >= 32 {
                continue;
            }
            let regen_multiplier = if entity.powerups() & (1 << PW_FORCE_BOON) != 0 {
                6.0
            } else if mode == GT_JEDIMASTER && entity.is_jedi_master() {
                4.0
            } else {
                1.0
            };
            self.force.observe(
                number,
                force_estimate::Observation {
                    time,
                    active: entity.force_powers_active(),
                    torso_animation: entity.torso_animation(),
                    saber_in_flight: entity.saber_in_flight(),
                    saber_special: entity.weapon() == WP_SABER
                        && sjk_game_jka::saber_rules::in_special(entity.saber_move()),
                    regen_multiplier,
                    dead: entity.e_flags() & EF_DEAD != 0,
                },
            );
        }
    }

    /// Text id of an entry's label.
    fn text_id(entry: &Entry) -> TextId {
        if entry.npc_class == 0 {
            TextId(u32::from(entry.number))
        } else {
            TextId(NPC_TEXT + u32::from(entry.npc_class))
        }
    }

    /// Lay the collected plates out as shapes and text commands.
    fn build<'a>(
        &mut self,
        label: &dyn Fn(TextId) -> &'a str,
        icons: &[Option<TextureId>],
        weapons: &super::icons::Icons,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        self.list.clear();
        let unit = crate::ui_scale::height_scale(viewport[1]);
        for index in 0..self.entries.len() {
            let entry = self.entries[index];
            let u = unit * entry.scale;
            let size = 36.0 * self.settings.scale * u;
            let id = Self::text_id(&entry);
            let rows = if entry.names_allowed {
                entry.rows()
            } else {
                Rows::default()
            };
            let [x, y] = entry.point;
            let marker = entry.icon.map(|color| (color, 7.0 * u));
            let marker_height = marker.map_or(0.0, |(_, side)| side + 2.0 * u);
            let _ = self.list.push(DrawCommand::PushOpacity(entry.alpha));
            if let Some((color, side)) = marker {
                let _ = self.list.push(DrawCommand::SolidRect {
                    rect: Rect::new(x - side * 0.5, y - side, side, side),
                    color,
                });
            }
            let bottom = y - marker_height - 2.0 * u;
            let stack_height = Stack::height(rows, u);
            // The name rises as the plate fades in beneath it.
            let line = size * 1.15;
            let top = bottom - stack_height * entry.detail - line;
            if entry.names_allowed && !label(id).is_empty() {
                let width = (320.0 * unit).min(viewport[0]);
                let left = (x - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
                let opacity = FAR_NAME_OPACITY + (1.0 - FAR_NAME_OPACITY) * entry.proximity;
                let tint = if entry.npc_class == 0 {
                    Color::new(1.0, 1.0, 1.0, opacity)
                } else {
                    Color::new(1.0, 0.95, 0.75, opacity)
                };
                let _ = self.list.push(DrawCommand::Text {
                    rect: Rect::new(left, top, width, line),
                    text: id,
                    size,
                    color: tint,
                    align: TextAlign::Center,
                    overflow: TextOverflow::Ellipsis,
                    weight: FontWeight::Regular,
                    letter_spacing: 0.0,
                });
                if entry.verified {
                    let text = crate::text::visible_text_width(
                        font,
                        label(id),
                        size / font.height.max(1.0),
                    )
                    .min(width);
                    let side = size * 0.95;
                    let badge = Rect::new(
                        (left + (width + text) * 0.5 + size * 0.15).min(viewport[0] - side),
                        top + (line - side) * 0.5,
                        side,
                        side,
                    );
                    let _ = self.list.push(DrawCommand::TexturedQuad {
                        rect: badge,
                        texture: crate::ui_renderer::VERIFIED_TEXTURE,
                        color: Color::new(1.0, 1.0, 1.0, opacity),
                    });
                }
            }
            if entry.power_count > 0 && entry.proximity > 0.01 {
                self.power_row(&entry, icons, [x, top], size, u, viewport);
            }
            // The weapon sits left of the plate, or of where it would be beside the name.
            let mut weapon_centre = top + line * 0.5;
            if rows.any() && entry.detail > 0.01 {
                let stack = Stack::new(x, bottom, rows, u, viewport);
                weapon_centre = stack.frame.y + stack.frame.height * 0.5;
                let _ = self.list.push(DrawCommand::PushOpacity(entry.detail));
                self.plate(&entry, &stack, u);
                let _ = self.list.push(DrawCommand::PopOpacity);
            }
            if entry.weapon != WP_NONE && entry.proximity > 0.01 {
                self.weapon_icon(&entry, weapons, [x, weapon_centre], u, viewport);
            }
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
    }

    /// The held weapon's icon, centred vertically on `centre_y` left of the plate,
    /// in a round backdrop; a saber's is ringed and haloed in its stance's colour
    /// (the Radial HUD's), dimmed while the blade is put away.
    fn weapon_icon(
        &mut self,
        entry: &Entry,
        weapons: &super::icons::Icons,
        [x, centre_y]: [f32; 2],
        u: f32,
        viewport: [f32; 2],
    ) {
        let Some(texture) = weapons.weapon_select(entry.weapon, false, entry.style) else {
            return;
        };
        let side = 22.0 * u;
        let width = Stack::WIDTH * u;
        let plate_left = (x - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
        let rect = Rect::new(
            (plate_left - 4.0 * u - side).max(0.0),
            centre_y - side * 0.5,
            side,
            side,
        );
        let radius = side * 0.5;
        let halo = (entry.weapon == WP_SABER && entry.style != 0).then(|| {
            let strength = if entry.holstered { HOLSTERED_HALO } else { 1.0 };
            (super::radial::saber_style_color(entry.style), strength)
        });
        let _ = self.list.push(DrawCommand::PushOpacity(entry.proximity));
        if let Some((color, strength)) = halo {
            let grow = 3.0 * u;
            let _ = self.list.push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    rect.x - grow,
                    rect.y - grow,
                    side + grow * 2.0,
                    side + grow * 2.0,
                ),
                radius: radius + grow,
                color: Color::new(color.r, color.g, color.b, 0.3 * strength),
            });
        }
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect,
            radius,
            color: ICON_BACKDROP,
        });
        if let Some((color, strength)) = halo {
            let _ = self.list.push(DrawCommand::Border {
                rect,
                radius,
                width: (1.6 * u).max(1.0),
                color: Color::new(color.r, color.g, color.b, 0.95 * strength),
            });
        }
        let inset = 3.5 * u;
        let _ = self.list.push(DrawCommand::TexturedQuad {
            rect: Rect::new(
                rect.x + inset,
                rect.y + inset,
                side - inset * 2.0,
                side - inset * 2.0,
            ),
            texture,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        });
        let _ = self.list.push(DrawCommand::PopOpacity);
    }

    /// The row of power icons centred over the name, whose top edge is `[x, top]`.
    fn power_row(
        &mut self,
        entry: &Entry,
        icons: &[Option<TextureId>],
        [x, top]: [f32; 2],
        size: f32,
        u: f32,
        viewport: [f32; 2],
    ) {
        let side = size;
        let gap = 2.0 * u;
        let shown = entry.powers[..usize::from(entry.power_count)]
            .iter()
            .filter(|power| icons.get(usize::from(**power)).copied().flatten().is_some())
            .count();
        if shown == 0 {
            return;
        }
        let width = shown as f32 * side + (shown - 1) as f32 * gap;
        let mut left = (x - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
        let y = top - gap - side;
        let _ = self.list.push(DrawCommand::PushOpacity(entry.proximity));
        for power in &entry.powers[..usize::from(entry.power_count)] {
            let Some(texture) = icons.get(usize::from(*power)).copied().flatten() else {
                continue;
            };
            let cell = Rect::new(left, y, side, side);
            let _ = self.list.push(DrawCommand::RoundedRect {
                rect: cell,
                radius: 3.0 * u,
                color: ICON_BACKDROP,
            });
            let inset = u.max(1.0);
            let _ = self.list.push(DrawCommand::TexturedQuad {
                rect: Rect::new(
                    left + inset,
                    y + inset,
                    side - inset * 2.0,
                    side - inset * 2.0,
                ),
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
            left += side + gap;
        }
        let _ = self.list.push(DrawCommand::PopOpacity);
    }

    /// The framed plate and its bars.
    fn plate(&mut self, entry: &Entry, stack: &Stack, u: f32) {
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect: stack.frame,
            radius: 4.0 * u,
            color: Color::new(0.03, 0.04, 0.06, 0.62),
        });
        let _ = self.list.push(DrawCommand::Border {
            rect: stack.frame,
            radius: 4.0 * u,
            width: (1.2 * u).max(1.0),
            color: entry.accent,
        });
        let colors = self.colors;
        for (bar, range, color, meter) in [
            (stack.shield, entry.shield, colors.shield, Meter::Shield),
            (stack.health, entry.health, colors.health, Meter::Health),
            (stack.force, entry.force, colors.force, Meter::Force),
        ] {
            if let (Some(rect), Some(range)) = (bar, range) {
                self.meter(rect, range, color, meter, u);
            }
        }
    }

    /// One bar: its track, the fill to the guess with the haze over the uncertain
    /// stretch, and over a full bar (overheal, overshield) a second layer in a
    /// deeper shade from the left. An empty shield is a broken grey bar; health or
    /// shield too unsure to show dims under a yellow "?".
    fn meter(&mut self, rect: Rect, range: Range, color: Color, meter: Meter, u: f32) {
        let radius = rect.height * 0.5;
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect,
            radius,
            color: Color::new(0.0, 0.0, 0.0, 0.6),
        });
        let outline = match meter {
            Meter::Force => math::lighter(color, 0.4, 0.75),
            _ => Color::new(1.0, 1.0, 1.0, 0.3),
        };
        if meter == Meter::Shield && range.high < EMPTY_SHARE {
            for (start, end) in math::broken_dashes() {
                let _ = self.list.push(DrawCommand::SolidRect {
                    rect: Rect::new(
                        rect.x + rect.width * start,
                        rect.y + rect.height * 0.2,
                        rect.width * (end - start),
                        rect.height * 0.6,
                    ),
                    color: math::EMPTY_SHIELD,
                });
            }
            let _ = self.list.push(DrawCommand::Border {
                rect,
                radius,
                width: u.max(1.0),
                color: Color::new(1.0, 1.0, 1.0, 0.15),
            });
            return;
        }
        let unsure = meter != Meter::Force && range.width() >= UNSURE_WIDTH;
        let _ = self
            .list
            .push(DrawCommand::PushOpacity(if unsure { 0.35 } else { 1.0 }));
        let layers = [
            (range.clamp(0.0, 1.0), color),
            (
                range.map(|share| (share - 1.0).clamp(0.0, 1.0)),
                math::saturated(color),
            ),
        ];
        for (index, (layer, shade)) in layers.into_iter().enumerate() {
            if index == 1 && range.high <= 1.0 {
                break;
            }
            // The second layer is an inner band, so the full bar shows round it.
            let band = if index == 0 {
                rect
            } else {
                math::overflow_band(rect)
            };
            if layer.best > 0.01 {
                let _ = self.list.push(DrawCommand::RoundedRect {
                    rect: Rect::new(
                        band.x,
                        band.y,
                        (band.width * layer.best).max(band.height),
                        band.height,
                    ),
                    radius: band.height * 0.5,
                    color: shade,
                });
            }
            self.haze(band, layer);
        }
        let _ = self.list.push(DrawCommand::PopOpacity);
        let _ = self.list.push(DrawCommand::Border {
            rect,
            radius,
            width: u.max(1.0),
            color: outline,
        });
        if unsure {
            // Sized to its bar, so the marks of two unsure bars do not meet.
            let size = rect.height * 1.3 + 2.5 * u;
            let _ = self.list.push(DrawCommand::Text {
                rect: Rect::new(
                    rect.x,
                    rect.y + (rect.height - size * 1.15) * 0.5,
                    rect.width,
                    size * 1.15,
                ),
                text: TextId(UNKNOWN_TEXT),
                size,
                color: math::UNKNOWN_MARK,
                align: TextAlign::Center,
                overflow: TextOverflow::Ellipsis,
                weight: FontWeight::Semibold,
                letter_spacing: 0.0,
            });
        }
    }

    /// The grey haze over the uncertain stretch of a bar: thickest at the guess,
    /// fading out towards either bound, so the bar's edge looks as blurred as the
    /// estimate is loose.
    fn haze(&mut self, rect: Rect, range: Range) {
        if range.width() < HAZE_MIN_WIDTH {
            return;
        }
        let at = |share: f32| rect.x + rect.width * share;
        let clear = Color::new(HAZE.r, HAZE.g, HAZE.b, 0.0);
        let _ = self.list.push(DrawCommand::PushClip(rect));
        for (from, to, start, end) in [
            (range.low, range.best, clear, HAZE),
            (range.best, range.high, HAZE, clear),
        ] {
            let width = at(to) - at(from);
            if width < 0.5 {
                continue;
            }
            let _ = self.list.push(DrawCommand::GradientRect {
                rect: Rect::new(at(from), rect.y, width, rect.height),
                radius: 0.0,
                gradient: Gradient {
                    start,
                    end,
                    vertical: false,
                },
            });
        }
        let _ = self.list.push(DrawCommand::PopClip);
    }

    /// Build the plates from the existing roster text at submission, never
    /// allocating another name store.
    pub(crate) fn append(
        &mut self,
        chat: &ChatOverlay,
        icons: &[Option<TextureId>],
        weapons: &super::icons::Icons,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        self.build(&|id| label(chat, id), icons, weapons, font, viewport);
        crate::ui_renderer::append_text_commands(
            &self.list,
            |id| label(chat, id),
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
        );
    }
}

/// The Force powers of `active` that get an icon, in display order, at most
/// [`MAX_ICONS`] of them.
fn icon_powers(active: u32) -> ([u8; MAX_ICONS], u8) {
    let mut powers = [0; MAX_ICONS];
    let mut count = 0;
    for power in ICON_POWERS {
        if active & (1 << power) != 0 && usize::from(count) < MAX_ICONS {
            powers[usize::from(count)] = power;
            count += 1;
        }
    }
    (powers, count)
}

/// Health and shield shares of `entity` (up to 2: a second bar's worth over the
/// maximum): a teammate's from the team overlay (points out of a full 100), anyone's
/// from the entity state when the server sends health there, else the `predicted`
/// estimate.
fn bar_values(
    entity: &sjk_protocol::EntityState,
    number: u16,
    teammate: bool,
    team_info: &TeamInfoTable,
    predicted: Option<&vitals_estimate::Estimator>,
) -> (Option<Range>, Option<Range>) {
    if teammate
        && let Some(row) = team_info
            .entries()
            .iter()
            .find(|row| u16::from(row.client_num) == number)
    {
        let share = |points: i32| Range::exact(points as f32).share_over(FULL_POINTS);
        return (Some(share(row.health)), Some(share(row.armor)));
    }
    let maximum = entity.max_health() as f32;
    if maximum > 0.0 {
        return (
            Some(Range::exact(entity.health() as f32).share(maximum)),
            None,
        );
    }
    let Some(vitals) = predicted else {
        return (None, None);
    };
    let full = vitals.full();
    (
        vitals.health(number).map(|range| range.share_over(full)),
        vitals.armor(number).map(|range| range.share_over(full)),
    )
}

/// A range in points for the debug log.
fn shown(range: Option<Range>) -> String {
    range.map_or_else(
        || "-".to_owned(),
        |range| format!("{:.0}[{:.0}..{:.0}]", range.best, range.low, range.high),
    )
}

/// Frame colour for a player on `team` in game type `mode`: red or blue in
/// team games, neutral otherwise.
fn team_accent(mode: i32, team: i32) -> Color {
    match (mode >= GT_TEAM, team) {
        (true, 1) => Color::new(1.0, 0.3, 0.25, 0.95),
        (true, 2) => Color::new(0.3, 0.6, 1.0, 0.95),
        _ => NEUTRAL_ACCENT,
    }
}

/// The bars' colours.
#[derive(Clone, Copy, Debug)]
struct BarColors {
    health: Color,
    shield: Color,
    force: Color,
}

impl Default for BarColors {
    fn default() -> Self {
        Self {
            health: math::HEALTH_COLOR,
            shield: math::SHIELD_COLOR,
            force: math::FORCE_COLOR,
        }
    }
}

/// Which bar [`State::meter`] draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Meter {
    Shield,
    Health,
    Force,
}

/// Text of a plate: a roster name, an NPC class name, or the "?" over a bar.
fn label(chat: &ChatOverlay, id: TextId) -> &str {
    if id.0 == UNKNOWN_TEXT {
        return "?";
    }
    match id.0.checked_sub(NPC_TEXT) {
        None => chat.player_label(id.0 as u16),
        Some(class) => super::npc_class::name(class as u8).unwrap_or(""),
    }
}

/// A plate for the off-screen snapshots (`menu_snapshot.rs`), its bars as shares of
/// a full one: `[low, guess, high]`.
#[cfg(test)]
pub(crate) struct PreviewPlate {
    pub(crate) slot: u16,
    pub(crate) point: [f32; 2],
    pub(crate) distance: f32,
    pub(crate) health: Option<[f32; 3]>,
    pub(crate) shield: Option<[f32; 3]>,
    pub(crate) force: Option<[f32; 3]>,
    pub(crate) weapon: u8,
    pub(crate) style: u8,
    pub(crate) verified: bool,
    /// The frame's colour: red or blue team, else neutral.
    pub(crate) team: Option<bool>,
}

#[cfg(test)]
impl State {
    /// Lay out `plates` as [`State::append`] does, naming slot `n` `names[n]`.
    pub(crate) fn preview(
        &mut self,
        plates: &[PreviewPlate],
        names: &[&str],
        weapons: &super::icons::Icons,
        font: &UiFont,
        vertices: &mut Vec<TextVertex>,
        viewport: [f32; 2],
    ) {
        let range = |share: [f32; 3]| Range::new(share[0], share[1], share[2]);
        self.entries.clear();
        for plate in plates {
            let proximity = math::detail(plate.distance, self.settings.near);
            self.entries.push(Entry {
                number: plate.slot,
                npc_class: 0,
                point: plate.point,
                distance: plate.distance,
                scale: math::distance_scale(plate.distance, self.settings.range, MIN_SCALE),
                alpha: 1.0,
                detail: proximity,
                proximity,
                powers: [0; MAX_ICONS],
                power_count: 0,
                health: plate.health.map(range),
                shield: plate.shield.map(range),
                force: plate.force.map(range),
                weapon: plate.weapon,
                style: plate.style,
                holstered: false,
                accent: match plate.team {
                    Some(red) => team_accent(GT_TEAM, if red { 1 } else { 2 }),
                    None => NEUTRAL_ACCENT,
                },
                icon: None,
                names_allowed: true,
                verified: plate.verified,
            });
        }
        let label = |id: TextId| {
            if id.0 == UNKNOWN_TEXT {
                "?"
            } else {
                names.get(id.0 as usize).copied().unwrap_or("")
            }
        };
        self.build(&label, &[], weapons, font, viewport);
        crate::ui_renderer::append_text_commands(
            &self.list,
            label,
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_list_the_continuous_powers_dark_side_first() {
        let active = (1 << 9) | (1 << 7) | (1 << 2);
        assert_eq!(icon_powers(active), ([7, 9, 2, 0], 3));
    }

    #[test]
    fn instant_powers_get_no_icon_and_the_row_is_capped() {
        // Jump, push, pull and the saber powers are skipped.
        let instant = (1 << 1) | (1 << 3) | (1 << 4) | (1 << 15) | (1 << 16) | (1 << 17);
        assert_eq!(icon_powers(instant).1, 0);
        let many = u32::MAX & !instant;
        let (powers, count) = icon_powers(many);
        assert_eq!(usize::from(count), MAX_ICONS);
        assert_eq!(powers, [7, 6, 13, 8]);
    }
}
