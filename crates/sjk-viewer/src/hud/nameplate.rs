//! MMO-style nameplates over players (and, optionally, NPCs).
//!
//! Far away a player shows only a small, dim name. Closer, the name rises and a
//! framed plate fades in beneath it with health, shield and estimated Force bars.
//! Names come from the chat roster and are drawn in the classic HUD font with
//! their colour codes; the layout maths is in [`super::nameplate_math`] and the
//! Force estimate in [`super::force_estimate`]. The plain TaystJK names stay in
//! [`super::identification`] (`cg_drawPlayerNames`); a nameplate replaces them.
use super::force_estimate::{self, Calibration, Estimator};
use super::identification::{Camera, friend_icon, info_number, unoccluded};
use super::nameplate_math::{self as math, Rows, Stack};
use crate::{TextVertex, UiFont, chat::ChatOverlay, console::ViewerConsole};
use glam::Vec3;
use sjk_bsp::{Aabb, Bsp, TraceScratch};
use sjk_client::TeamInfoTable;
use sjk_game_jka::force_powers::FP_DRAIN;
use sjk_protocol::{GameState, Snapshot};
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};

/// Text ids below this are player slots; above it, `NPC_TEXT + class_t`.
const NPC_TEXT: u32 = 1024;
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
/// Frame colour of players who follow no team.
const NEUTRAL_ACCENT: Color = Color::new(0.78, 0.82, 0.9, 0.9);
/// Frame colour of NPC plates.
const NPC_ACCENT: Color = Color::new(0.95, 0.8, 0.35, 0.9);

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
    walls: bool,
    npcs: bool,
    friends: bool,
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
            walls: false,
            npcs: false,
            friends: true,
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
    health: Option<f32>,
    shield: Option<f32>,
    force: Option<f32>,
    accent: Color,
    icon: Option<Color>,
    names_allowed: bool,
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            list: DrawList::new(MAX_TAGS * 24 + 8),
            settings: Settings::default(),
            entries: Vec::with_capacity(MAX_TAGS),
            fades: Box::new([Fade::default(); ENTITY_SLOTS]),
            last_update: 0,
            last_debug: 0,
            force: Estimator::default(),
            calibration: Calibration::default(),
            regen_source: "default",
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
            walls: flag("cg_nameplatewalls", false),
            npcs: flag("cg_nameplatenpcs", false),
            friends: flag("cg_drawfriend", true),
            debug: flag("cg_nameplatedebug", false),
        };
    }

    /// Whether nameplates replace the plain overhead names.
    pub(crate) fn enabled(&self) -> bool {
        self.settings.enabled
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
        if settings.force && settings.bars != 0 {
            self.observe_force(snapshot, game, mode);
        }
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
                bar_values(entity, number, ally && player, team_info)
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
                health,
                shield,
                force,
                accent,
                icon,
                names_allowed,
            });
            if self.entries.len() == MAX_TAGS {
                break;
            }
        }
        // Far plates first, so a near plate covers a far one.
        self.entries
            .sort_unstable_by(|a, b| b.distance.total_cmp(&a.distance));
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
            "nameplate: regen pace {:.0} ms/point ({}; measured {:?}), server info force keys {force_keys:?}",
            self.force.regen_millis(),
            self.regen_source,
            self.calibration.millis_per_point().map(f32::round),
        );
        let local = snapshot.player.client_num();
        eprintln!(
            "nameplate: own Force actual {} estimated {:?}",
            snapshot.player.force_power(),
            self.force.ratio(local).map(|r| (r * 100.0).round()),
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
                "nameplate: client {number} health {} max {} powers {:#x} fp~{:?} tinfo {team:?}",
                entity.health(),
                entity.max_health(),
                entity.force_powers_active(),
                self.force.ratio(number).map(|r| (r * 100.0).round()),
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
        self.force.set_regen_millis(measured.or(info));
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
    fn build(&mut self, chat: &ChatOverlay, viewport: [f32; 2]) {
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
            if entry.names_allowed && !label(chat, id).is_empty() {
                // The name rises as the plate fades in beneath it.
                let line = size * 1.15;
                let width = (320.0 * unit).min(viewport[0]);
                let left = (x - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
                let top = bottom - stack_height * entry.detail - line;
                let opacity = FAR_NAME_OPACITY + (1.0 - FAR_NAME_OPACITY) * entry.detail;
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
            }
            if rows.any() && entry.detail > 0.01 {
                let stack = Stack::new(x, bottom, rows, u, viewport);
                let _ = self.list.push(DrawCommand::PushOpacity(entry.detail));
                self.plate(&entry, &stack, u);
                let _ = self.list.push(DrawCommand::PopOpacity);
            }
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
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
        for (bar, ratio, color, outline) in [
            (
                stack.health,
                entry.health,
                entry.health.map(math::health_color),
                None,
            ),
            (stack.shield, entry.shield, Some(math::SHIELD_COLOR), None),
            (
                stack.force,
                entry.force,
                Some(math::FORCE_COLOR),
                Some(Color::new(0.8, 0.6, 1.0, 0.75)),
            ),
        ] {
            let (Some(rect), Some(ratio), Some(color)) = (bar, ratio, color) else {
                continue;
            };
            let radius = rect.height * 0.5;
            let _ = self.list.push(DrawCommand::RoundedRect {
                rect,
                radius,
                color: Color::new(0.0, 0.0, 0.0, 0.6),
            });
            if ratio > 0.01 {
                let _ = self.list.push(DrawCommand::RoundedRect {
                    rect: Rect::new(
                        rect.x,
                        rect.y,
                        (rect.width * ratio).max(rect.height),
                        rect.height,
                    ),
                    radius,
                    color,
                });
            }
            let _ = self.list.push(DrawCommand::Border {
                rect,
                radius,
                width: u.max(1.0),
                color: outline.unwrap_or(Color::new(1.0, 1.0, 1.0, 0.3)),
            });
        }
    }

    /// Build the plates from the existing roster text at submission, never
    /// allocating another name store.
    pub(crate) fn append(
        &mut self,
        chat: &ChatOverlay,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        self.build(chat, viewport);
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

/// Health and shield shares of `entity`: a teammate's from the team overlay
/// (points out of a full 100), anyone's from the entity state when the server
/// sends health there.
fn bar_values(
    entity: &sjk_protocol::EntityState,
    number: u16,
    teammate: bool,
    team_info: &TeamInfoTable,
) -> (Option<f32>, Option<f32>) {
    if teammate
        && let Some(row) = team_info
            .entries()
            .iter()
            .find(|row| u16::from(row.client_num) == number)
    {
        let share = |points: i32| (points as f32 / FULL_POINTS).clamp(0.0, 1.0);
        return (Some(share(row.health)), Some(share(row.armor)));
    }
    let maximum = entity.max_health() as f32;
    let health = (maximum > 0.0).then(|| (entity.health() as f32 / maximum).clamp(0.0, 1.0));
    (health, None)
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

/// Text of a plate: a roster name, or an NPC class name.
fn label(chat: &ChatOverlay, id: TextId) -> &str {
    match id.0.checked_sub(NPC_TEXT) {
        None => chat.player_label(id.0 as u16),
        Some(class) => super::npc_class::name(class as u8).unwrap_or(""),
    }
}
