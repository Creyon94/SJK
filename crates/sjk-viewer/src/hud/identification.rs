//! World-projected player labels: MMO-style nametags above each player (and,
//! optionally, NPC). Names belong to the existing chat roster; the layout maths
//! is in [`super::nametag`].
use super::nametag::{self, Plate};
use crate::{TextVertex, UiFont, chat::ChatOverlay, console::ViewerConsole};
use glam::Vec3;
use sjk_bsp::{Aabb, Bsp, TraceScratch};
use sjk_protocol::{GameState, Snapshot};
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};

/// Text ids below this are player slots; above it, `NPC_TEXT + class_t`.
const NPC_TEXT: u32 = 1024;
/// `ET_PLAYER` and `ET_NPC` entity types, and `EF_DEAD`.
const ET_PLAYER: u8 = 1;
const ET_NPC: u8 = 13;
const EF_DEAD: u32 = 2;
/// Tags drawn at most: every client slot, plus a bounded number of NPCs.
const MAX_PLAYER_TAGS: usize = 32;
const MAX_NPC_TAGS: usize = 16;
const MAX_TAGS: usize = MAX_PLAYER_TAGS + MAX_NPC_TAGS;
/// Entity numbers are ten bits on the wire.
const ENTITY_SLOTS: usize = 1024;
/// A tag unseen this long fades in again instead of resuming.
const STALE_MILLIS: i64 = 250;
/// Opacity of a tag behind a wall when `cg_nametagWalls` is on.
const WALL_OPACITY: f32 = 0.35;
/// Allowed overshoot of the screen (in half-screens) before a tag is dropped,
/// so a plate slides off the edge instead of vanishing early.
const SCREEN_MARGIN: f32 = 1.15;
/// Frame colour of tags that follow no team.
const NEUTRAL_ACCENT: Color = Color::new(0.78, 0.82, 0.9, 0.9);
/// Frame colour of NPC tags.
const NPC_ACCENT: Color = Color::new(0.95, 0.8, 0.35, 0.9);

/// Register only the completed overhead capabilities.
pub(super) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        "cg_drawPlayerNames",
        1_i64,
        CvarFlags::ARCHIVE,
        "Overhead names: 0 off, 1 names, 2 adds health bars",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_drawPlayerNamesScale",
        0.5_f64,
        CvarFlags::ARCHIVE,
        "Overhead name scale",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_drawFriend",
        true,
        CvarFlags::ARCHIVE,
        "Overhead ally markers",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nametagPlate",
        true,
        CvarFlags::ARCHIVE,
        "Overhead names on a team-coloured plate; off draws plain text",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nametagRange",
        3000_i64,
        CvarFlags::ARCHIVE,
        "Distance in units out to which overhead tags show",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nametagShrink",
        true,
        CvarFlags::ARCHIVE,
        "Overhead tags shrink with distance",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nametagMinScale",
        0.6_f64,
        CvarFlags::ARCHIVE,
        "Smallest size, as a share of full, a distant tag shrinks to",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nametagWalls",
        false,
        CvarFlags::ARCHIVE,
        "Show overhead tags dimmed through walls",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nametagNpcs",
        false,
        CvarFlags::ARCHIVE,
        "Overhead tags on NPCs: their class and health",
    ))?;
    Ok(())
}

/// Camera basis shared by projection tests and the production label pass.
#[derive(Clone, Copy)]
pub(crate) struct Camera {
    /// Actual rendered eye position.
    pub(crate) eye: Vec3,
    /// Actual rendered view target.
    pub(crate) target: Vec3,
    /// Rendered up vector.
    pub(crate) up: Vec3,
    /// Vertical field of view in degrees.
    pub(crate) fov: f32,
    /// Physical viewport dimensions.
    pub(crate) viewport: [f32; 2],
}

impl Camera {
    /// Reject behind-camera/off-screen positions before emitting an overhead HUD element.
    pub(crate) fn project(self, point: Vec3) -> Option<[f32; 2]> {
        self.project_within(point, 1.0)
    }

    /// Like [`Camera::project`], but keeps points up to `limit` half-screens
    /// from the centre (1.0 is the screen edge).
    pub(crate) fn project_within(self, point: Vec3, limit: f32) -> Option<[f32; 2]> {
        let forward = (self.target - self.eye).normalize_or_zero();
        let right = forward.cross(self.up).normalize_or_zero();
        let up = right.cross(forward);
        let delta = point - self.eye;
        let depth = delta.dot(forward);
        if depth <= 0.01 {
            return None;
        }
        let half = (self.fov.to_radians() * 0.5).tan() * depth;
        let x = delta.dot(right) / (half * self.viewport[0] / self.viewport[1]);
        let y = delta.dot(up) / half;
        if !x.is_finite() || !y.is_finite() || x.abs() > limit || y.abs() > limit {
            return None;
        }
        Some([
            (x + 1.0) * 0.5 * self.viewport[0],
            (1.0 - y) * 0.5 * self.viewport[1],
        ])
    }
}

/// Player settings, sampled once per frame.
#[derive(Clone, Copy)]
struct Settings {
    names: i64,
    scale: f32,
    friends: bool,
    plate: bool,
    range: f32,
    shrink: bool,
    min_scale: f32,
    walls: bool,
    npcs: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            names: 1,
            scale: 0.5,
            friends: true,
            plate: true,
            range: 3000.0,
            shrink: true,
            min_scale: 0.6,
            walls: false,
            npcs: false,
        }
    }
}

/// One visible tag, collected in `update` and drawn in `append`.
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
    health: Option<f32>,
    accent: Color,
    icon: Option<Color>,
    names_allowed: bool,
}

/// Smoothed opacity of one entity's tag.
#[derive(Clone, Copy, Default)]
struct Fade {
    alpha: f32,
    seen: i64,
}

/// Fixed-capacity tag state: at most 32 clients and 16 NPCs, no owned name strings.
pub(crate) struct State {
    /// Shapes and text IDs submitted through the normal HUD renderer.
    pub(crate) list: DrawList,
    settings: Settings,
    entries: Vec<Entry>,
    fades: Box<[Fade; ENTITY_SLOTS]>,
    last_update: i64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            list: DrawList::new(MAX_TAGS * 10 + 8),
            settings: Settings::default(),
            entries: Vec::with_capacity(MAX_TAGS),
            fades: Box::new([Fade::default(); ENTITY_SLOTS]),
            last_update: 0,
        }
    }
}

impl State {
    /// Sample settings once, outside text emission.
    pub(crate) fn sample(&mut self, console: Option<&ViewerConsole>) {
        let flag =
            |name: &str, default: bool| console.and_then(|c| c.bool_cvar(name)).unwrap_or(default);
        self.settings = Settings {
            names: console
                .and_then(|c| c.integer_cvar("cg_drawplayernames"))
                .unwrap_or(1),
            scale: crate::cgame_options::scalar(console, "cg_drawplayernamesscale", 0.5)
                .clamp(0.1, 3.0),
            friends: flag("cg_drawfriend", true),
            plate: flag("cg_nametagplate", true),
            range: console
                .and_then(|c| c.integer_cvar("cg_nametagrange"))
                .map_or(3000.0, |range| range as f32)
                .clamp(500.0, 10_000.0),
            shrink: flag("cg_nametagshrink", true),
            min_scale: crate::cgame_options::scalar(console, "cg_nametagminscale", 0.6)
                .clamp(0.25, 1.0),
            walls: flag("cg_nametagwalls", false),
            npcs: flag("cg_nametagnpcs", false),
        };
    }

    /// Drop every collected tag and drawn shape.
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
        now: i64,
        camera: Camera,
        bsp: &Bsp,
        scratch: &mut TraceScratch,
        hidden: bool,
    ) {
        self.clear();
        let step = (now - self.last_update).clamp(0, 100) as f32 / nametag::FADE_MILLIS;
        self.last_update = now;
        let settings = self.settings;
        if hidden || (settings.names == 0 && !settings.friends) {
            return;
        }
        let restrictions = info_number(game.config_string(0), "restricts");
        let mode = info_number(game.config_string(0), "g_gametype");
        let local = snapshot.player.client_num();
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
                && settings.names != 0
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
            let (accent, icon, names_allowed) = if player {
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
                if icon.is_none() && (settings.names == 0 || restrictions & 64 != 0) {
                    continue;
                }
                (team_accent(mode, team), icon, restrictions & 64 == 0)
            } else {
                (NPC_ACCENT, None, true)
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
                origin + Vec3::Z * (nametag::head_height(entity.solid()) + nametag::HEAD_CLEARANCE);
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
                nametag::distance_fade(distance, settings.range) * visibility,
                now,
                step,
            );
            if alpha < 0.02 {
                continue;
            }
            let health = entity.health() as f32;
            let maximum = entity.max_health() as f32;
            if npc {
                npcs += 1;
            }
            self.entries.push(Entry {
                number,
                npc_class: if npc { entity.npc_class() } else { 0 },
                point,
                distance,
                scale: if settings.shrink {
                    nametag::distance_scale(distance, settings.range, settings.min_scale)
                } else {
                    1.0
                },
                alpha,
                health: (maximum > 0.0).then_some((health / maximum).clamp(0.0, 1.0)),
                accent,
                icon,
                names_allowed,
            });
            if self.entries.len() == MAX_TAGS {
                break;
            }
        }
        // Far tags first, so a near plate covers a far one.
        self.entries
            .sort_unstable_by(|a, b| b.distance.total_cmp(&a.distance));
    }

    /// Text id of an entry's label.
    fn text_id(entry: &Entry) -> TextId {
        if entry.npc_class == 0 {
            TextId(u32::from(entry.number))
        } else {
            TextId(NPC_TEXT + u32::from(entry.npc_class))
        }
    }

    /// Lay the collected tags out as shapes and text commands.
    fn build(&mut self, chat: &ChatOverlay, font: &UiFont, viewport: [f32; 2]) {
        self.list.clear();
        let unit = crate::ui_scale::height_scale(viewport[1]);
        for index in 0..self.entries.len() {
            let entry = self.entries[index];
            let size = 36.0 * self.settings.scale * unit * entry.scale;
            let id = Self::text_id(&entry);
            let show_name = self.settings.names != 0 && entry.names_allowed;
            let health = entry
                .health
                .filter(|_| show_name && self.settings.names > 1);
            let _ = self.list.push(DrawCommand::PushOpacity(entry.alpha));
            if self.settings.plate {
                let name = if show_name { label(chat, id) } else { "" };
                self.plate(&entry, id, name, health, size, unit, font, viewport);
            } else {
                self.plain(&entry, id, show_name, health, size, unit, viewport);
            }
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
    }

    /// A rounded plate: team-coloured frame, the name, and a framed health bar.
    #[allow(clippy::too_many_arguments)]
    fn plate(
        &mut self,
        entry: &Entry,
        id: TextId,
        name: &str,
        health: Option<f32>,
        size: f32,
        unit: f32,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let marker = entry.icon.map(|color| (color, 8.0 * unit * entry.scale));
        let marker_height = marker.map_or(0.0, |(_, side)| side + 2.0 * unit);
        let [x, y] = entry.point;
        if let Some((color, side)) = marker {
            let _ = self.list.push(DrawCommand::SolidRect {
                rect: Rect::new(x - side * 0.5, y - side, side, side),
                color,
            });
        }
        if name.is_empty() {
            return;
        }
        let text_width = crate::text::visible_text_width_style(
            font,
            name,
            size / font.height.max(1.0),
            crate::text::TextFace::Regular,
            0.0,
        );
        let plate = Plate::new(
            [x, y - marker_height - 2.0 * unit],
            text_width,
            size,
            health.is_some(),
            viewport,
        );
        let frame = Rect::new(plate.x, plate.y, plate.width, plate.height);
        let radius = size * 0.35;
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect: frame,
            radius,
            color: Color::new(0.03, 0.04, 0.06, 0.58),
        });
        let _ = self.list.push(DrawCommand::Border {
            rect: frame,
            radius,
            width: (1.5 * unit).max(1.0),
            color: entry.accent,
        });
        let _ = self.list.push(DrawCommand::Text {
            rect: Rect::new(
                plate.x + plate.inset,
                plate.text_y(),
                plate.width - plate.inset * 2.0,
                plate.line,
            ),
            text: id,
            size,
            color: if entry.npc_class == 0 {
                Color::new(1.0, 1.0, 1.0, 1.0)
            } else {
                Color::new(1.0, 0.95, 0.75, 1.0)
            },
            align: TextAlign::Center,
            overflow: TextOverflow::Ellipsis,
            weight: FontWeight::Regular,
            letter_spacing: 0.0,
        });
        if let Some(ratio) = health {
            let bar = Rect::new(
                plate.x + plate.inset,
                plate.bar_y(),
                plate.width - plate.inset * 2.0,
                plate.bar,
            );
            let _ = self.list.push(DrawCommand::RoundedRect {
                rect: bar,
                radius: bar.height * 0.5,
                color: Color::new(0.0, 0.0, 0.0, 0.7),
            });
            if ratio > 0.0 {
                let _ = self.list.push(DrawCommand::RoundedRect {
                    rect: Rect::new(
                        bar.x,
                        bar.y,
                        (bar.width * ratio).max(bar.height),
                        bar.height,
                    ),
                    radius: bar.height * 0.5,
                    color: nametag::health_color(ratio),
                });
            }
            let _ = self.list.push(DrawCommand::Border {
                rect: bar,
                radius: bar.height * 0.5,
                width: unit.max(1.0),
                color: Color::new(1.0, 1.0, 1.0, 0.35),
            });
        }
    }

    /// Plain text under the head point, with a bare health strip and ally marker.
    #[allow(clippy::too_many_arguments)]
    fn plain(
        &mut self,
        entry: &Entry,
        id: TextId,
        show_name: bool,
        health: Option<f32>,
        size: f32,
        unit: f32,
        viewport: [f32; 2],
    ) {
        let [x, y] = entry.point;
        if show_name {
            let width = (320.0 * unit).min(viewport[0]);
            let left = (x - width * 0.5).clamp(0.0, viewport[0] - width);
            let _ = self.list.push(DrawCommand::Text {
                rect: Rect::new(left, y, width, size * 1.5),
                text: id,
                size,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
                align: TextAlign::Center,
                overflow: TextOverflow::Ellipsis,
                weight: FontWeight::Regular,
                letter_spacing: 0.0,
            });
            if let Some(ratio) = health {
                let _ = self.list.push(DrawCommand::SolidRect {
                    rect: Rect::new(
                        x - 25.0 * unit,
                        y - 7.0 * unit,
                        50.0 * unit * ratio,
                        3.0 * unit,
                    ),
                    color: nametag::health_color(ratio),
                });
            }
        }
        if let Some(color) = entry.icon {
            let _ = self.list.push(DrawCommand::SolidRect {
                rect: Rect::new(x - 4.0 * unit, y - 20.0 * unit, 8.0 * unit, 8.0 * unit),
                color,
            });
        }
    }

    /// Build the tags from the existing roster text at submission, never
    /// allocating another name store.
    pub(crate) fn append(
        &mut self,
        chat: &ChatOverlay,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        self.build(chat, font, viewport);
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

/// Frame colour for a player on `team` in game type `mode`: red or blue in
/// team games (`GT_TEAM` and above), neutral otherwise.
fn team_accent(mode: i32, team: i32) -> Color {
    match (mode >= 6, team) {
        (true, 1) => Color::new(1.0, 0.3, 0.25, 0.95),
        (true, 2) => Color::new(0.3, 0.6, 1.0, 0.95),
        _ => NEUTRAL_ACCENT,
    }
}

/// Text of a tag: a roster name, or an NPC class name.
fn label(chat: &ChatOverlay, id: TextId) -> &str {
    match id.0.checked_sub(NPC_TEXT) {
        None => chat.player_label(id.0 as u16),
        Some(class) => super::npc_class::name(class as u8).unwrap_or(""),
    }
}

// TaystJK cg_players.c:11074-11155: team, Power Duel, and Jedi Master are distinct.
fn friend_icon(
    mode: i32,
    local: i32,
    team: i32,
    local_duel: i32,
    duel: i32,
    master: bool,
    local_master: bool,
    target_master: bool,
) -> Option<Color> {
    let ally = if mode >= 6 {
        local == 3 || local == team
    } else if mode == 4 {
        if local == 3 {
            duel == 2
        } else {
            local_duel == duel
        }
    } else {
        mode == 2 && (local == 3 || local == team) && master && !local_master && !target_master
    };
    ally.then_some(if team == 2 {
        Color::new(0.2, 0.55, 1.0, 1.0)
    } else {
        Color::new(1.0, 0.25, 0.2, 1.0)
    })
}

fn info_number(bytes: Option<&[u8]>, key: &str) -> i32 {
    bytes
        .and_then(|b| sjk_client::LegacyClientInfo::new(b).integer(key))
        .unwrap_or(0)
}

fn unoccluded(fraction: f32, start_solid: bool, all_solid: bool) -> bool {
    fraction >= 1.0 && !start_solid && !all_solid
}
