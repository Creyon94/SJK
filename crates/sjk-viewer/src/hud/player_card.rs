//! The player card: look at a player for a moment without moving, or press the
//! `inspect` key while aiming at them, and a small card appears beside their head with
//! what the game publishes about them (name, model and its icon, saber, hat and cape,
//! duel record) and, when the SJK hub knows them, SJK's emblem, their hub name,
//! whether they are verified and the medals the SJK team gave them (their medallions;
//! a pinned card names them too). `inspect` pins the card: it shows at once, follows
//! the player wherever they go and stays until `inspect` is pressed again.
//!
//! Everything shown is already public to every client (the player's `CS_PLAYERS`
//! string), so the card gives no advantage a scoreboard glance would not. The
//! target comes from the crosshair scan (`crosshair_scan.rs`), the head position
//! from the same presented world and camera as the overhead names
//! ([`super::identification`]).

use super::identification::Camera;
use crate::{TextVertex, UiFont};
use glam::Vec3;
use sjk_client::{LegacyClientInfo, decode_legacy};
use sjk_protocol::GameState;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::TextureId;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};

/// `CS_PLAYERS`: the first player's configstring.
const CS_PLAYERS: usize = 1131;
/// A target may leave the crosshair this long without the dwell starting over, so
/// a jittering aim does not flicker the card.
const GAP_MS: i32 = 300;
/// The view may turn this far from where the dwell began.
const STEADY_DEGREES: f32 = 6.0;
const FADE_IN_MS: f32 = 180.0;
const FADE_OUT_MS: f32 = 120.0;
/// Units below the top of the player's box the card points at (the upper head).
const HEAD_INSET: f32 = 8.0;
/// World units from the head to the side of the body the card keeps clear of.
const BODY_SIDE: f32 = 22.0;
/// How far off screen (in half screens) a pinned player may be and still be followed.
const PINNED_LIMIT: f32 = 40.0;
/// Colour index of a saber's blade in `c1`/`c2`: only the six retail colours have
/// a swatch.
const RETAIL_COLOURS: i32 = 6;

/// Register the card's settings.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        "cg_playerCard",
        true,
        CvarFlags::ARCHIVE,
        "Show a card beside a player you look at without moving",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_playerCardDelay",
        1.5_f64,
        CvarFlags::ARCHIVE,
        "Seconds you must look at a player, steady, before their card shows",
    ))
}

/// Which player is being looked at, and since when.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Dwell {
    target: Option<u16>,
    since: i32,
    last_seen: i32,
    anchor: Vec3,
}

impl Dwell {
    /// Feed one frame: the player under the crosshair now (`seen`), the unit
    /// view direction and the clock in milliseconds.
    pub(crate) fn observe(&mut self, seen: Option<u16>, forward: Vec3, now: i32) {
        if self.target.is_some() && now < self.last_seen {
            // The clock restarted (a map change): start over.
            *self = Self::default();
        }
        match (seen, self.target) {
            (Some(client), Some(target)) if client == target => self.last_seen = now,
            (Some(client), _) => {
                *self = Self {
                    target: Some(client),
                    since: now,
                    last_seen: now,
                    anchor: forward,
                };
            }
            (None, Some(_)) if now - self.last_seen > GAP_MS => *self = Self::default(),
            (None, _) => {}
        }
        if self.target.is_some() && angle_degrees(forward, self.anchor) > STEADY_DEGREES {
            self.since = now;
            self.anchor = forward;
        }
    }

    /// The player looked at long enough, steadily, to show their card.
    pub(crate) fn ready(&self, now: i32, delay_ms: i32) -> Option<u16> {
        self.target.filter(|_| now - self.since >= delay_ms)
    }
}

/// The player an `inspect` press pinned the card to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Pin {
    target: Option<u16>,
    request: bool,
}

impl Pin {
    /// `inspect` was pressed: the next [`Self::resolve`] toggles the pin.
    pub(crate) fn request(&mut self) {
        self.request = true;
    }

    /// Apply a pending press and report who is pinned. A press with a pin set
    /// unpins; one without pins the player under the crosshair (`seen`), if any. The
    /// pin also drops when the player is gone (`alive` false) or the clock restarted.
    pub(crate) fn resolve(&mut self, seen: Option<u16>, alive: bool, restarted: bool) -> PinStep {
        let mut step = PinStep::default();
        if restarted || (self.target.is_some() && !alive) {
            step.released = self.target.take();
        }
        if std::mem::take(&mut self.request) {
            if let Some(client) = self.target.take() {
                step.released = Some(client);
            } else if let Some(client) = seen {
                self.target = Some(client);
            }
        }
        step.target = self.target;
        step
    }

    pub(crate) fn target(self) -> Option<u16> {
        self.target
    }
}

/// What [`Pin::resolve`] decided.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PinStep {
    /// The pinned player after the step.
    pub(crate) target: Option<u16>,
    /// The player just unpinned.
    pub(crate) released: Option<u16>,
}

fn angle_degrees(a: Vec3, b: Vec3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos().to_degrees()
}

/// What the hub knows about the player.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HubInfo {
    /// Hub display name, empty if they have none yet.
    pub(crate) name: String,
    pub(crate) verified: bool,
    /// The medals the SJK team gave them.
    pub(crate) medals: crate::medals::Medals,
}

/// Text slots of a card: ids are indices into [`Card::texts`].
const T_NAME: usize = 0;
const T_MODEL: usize = 1;
const T_SABER: usize = 2;
const T_EXTRA: usize = 3;
const T_HUB: usize = 4;
const T_VERIFIED: usize = 5;
const T_WORN: usize = 6;
const T_MEDALS: usize = 7;
const T_MEDALS_MORE: usize = 8;
/// Characters a line of medal names holds on the card (Inter at 13).
const MEDAL_LINE_CHARS: usize = 40;

/// A player's card: the texts and values drawn, built when the target or the
/// hub's roster changes and drawn every frame without allocating.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Card {
    pub(crate) texts: [String; 9],
    /// 0 free, 1 red, 2 blue, 3 spectator.
    team: u8,
    /// Blade colours of the one or two sabers, if retail colours.
    swatches: [Option<[u8; 3]>; 2],
    hub: Option<HubInfo>,
    /// The model's icon, once resolved.
    icon: Option<TextureId>,
}

impl Card {
    fn has_extra(&self) -> bool {
        !self.texts[T_EXTRA].is_empty()
    }

    fn has_worn(&self) -> bool {
        !self.texts[T_WORN].is_empty()
    }

    /// Lines of medal names a pinned card shows.
    fn medal_lines(&self) -> usize {
        [T_MEDALS, T_MEDALS_MORE]
            .iter()
            .filter(|id| !self.texts[**id].is_empty())
            .count()
    }
}

/// `atoi`: the number at the start of `text` (JoF EJK appends a cosmetic's name
/// after the colour digit).
fn atoi(text: &str) -> i32 {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

fn blade(info: LegacyClientInfo<'_>, key: &str) -> Option<[u8; 3]> {
    let index = atoi(info.text(key)?);
    (0..RETAIL_COLOURS)
        .contains(&index)
        .then(|| crate::saber::Color::ALL[index as usize].blade_rgb())
}

/// The cosmetic named in a `c1`/`c2` value (JoF EJK appends it after the colour digits).
fn cosmetic(value: Option<&str>) -> Option<String> {
    let (_, name) = sjk_client::split_color_value(value?);
    name.map(crate::cosmetics::display_name)
}

/// "Hat Santa hat  /  Cape Jedi" for what the player wears, empty when nothing.
fn worn_line(info: LegacyClientInfo<'_>) -> String {
    let hat = cosmetic(info.text("c1"));
    let cape = cosmetic(info.text("c2"));
    match (hat, cape) {
        (Some(hat), Some(cape)) => format!("Hat {hat}  /  Cape {cape}"),
        (Some(hat), None) => format!("Hat {hat}"),
        (None, Some(cape)) => format!("Cape {cape}"),
        (None, None) => String::new(),
    }
}

/// The card for the player whose configstring is `info`.
pub(crate) fn card_from(info: &[u8], hub: Option<HubInfo>) -> Card {
    let info = LegacyClientInfo::new(info);
    let text = |key: &str| {
        info.bytes(key)
            .map(|bytes| decode_legacy(bytes).into_owned())
            .unwrap_or_default()
    };
    let mut card = Card {
        team: u8::try_from(atoi(&text("t"))).unwrap_or(0),
        ..Card::default()
    };
    card.texts[T_NAME] = {
        let name = text("n");
        if name.is_empty() { text("name") } else { name }
    };
    card.texts[T_MODEL] = text("model").replace('/', " / ");
    let (first, second) = (text("st"), text("st2"));
    let dual = !second.is_empty() && !second.eq_ignore_ascii_case("none");
    card.texts[T_SABER] = match (first.is_empty(), dual) {
        (true, _) => String::new(),
        (false, false) => first,
        (false, true) => format!("{first} + {second}"),
    };
    card.swatches = [
        blade(info, "c1"),
        if dual { blade(info, "c2") } else { None },
    ];
    card.texts[T_WORN] = worn_line(info);
    card.texts[T_EXTRA] = if let Some(skill) = info.text("skill").filter(|skill| !skill.is_empty())
    {
        format!("Bot, skill {skill}")
    } else if let (Some(wins), Some(losses)) = (info.text("w"), info.text("l")) {
        format!("Duel  {wins} W / {losses} L")
    } else {
        String::new()
    };
    if let Some(hub) = &hub {
        card.texts[T_HUB] = if hub.name.is_empty() {
            "SJK player".to_owned()
        } else {
            hub.name.clone()
        };
        if hub.verified {
            card.texts[T_VERIFIED] = "VERIFIED".to_owned();
        }
        // The names on one line, or two when they are long.
        for (medal, count) in hub.medals.iter() {
            let label = medal.label(count);
            let line = if card.texts[T_MEDALS_MORE].is_empty()
                && card.texts[T_MEDALS].len() + label.len() + 2 <= MEDAL_LINE_CHARS
            {
                T_MEDALS
            } else {
                T_MEDALS_MORE
            };
            if !card.texts[line].is_empty() {
                card.texts[line].push_str(", ");
            }
            card.texts[line].push_str(&label);
        }
    }
    card.hub = hub;
    card
}

/// Where the card goes and how big it is, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub(crate) card: Rect,
    /// Whether the card is to the right of the head point.
    pub(crate) right: bool,
}

/// Put a card of `size` beside `head`, to its right unless that leaves the
/// screen, kept inside the viewport.
pub(crate) fn place(head: [f32; 2], size: [f32; 2], offset: f32, viewport: [f32; 2]) -> Placement {
    let right = head[0] + offset + size[0] <= viewport[0] - offset * 0.25;
    let x = if right {
        head[0] + offset
    } else {
        head[0] - offset - size[0]
    };
    let margin = offset * 0.25;
    let x = x.clamp(margin, (viewport[0] - size[0] - margin).max(margin));
    let y = (head[1] - size[1] * 0.5).clamp(margin, (viewport[1] - size[1] - margin).max(margin));
    Placement {
        card: Rect::new(x, y, size[0], size[1]),
        right,
    }
}

/// Screen point of `anchor` and the pixels a body half-width (`BODY_SIDE` world units,
/// toward the camera's right) spans there, so the card clears the body at any range.
fn anchor_on_screen(camera: Camera, anchor: Vec3, limit: f32) -> Option<([f32; 2], f32)> {
    let head = camera.project_within(anchor, limit)?;
    let forward = (camera.target - camera.eye).normalize_or_zero();
    let right = forward.cross(camera.up).normalize_or_zero();
    let side = camera
        .project_within(anchor + right * BODY_SIDE, limit)
        .map_or(0.0, |point| (point[0] - head[0]).abs());
    Some((head, side))
}

/// Push a text command for the card's text slot `id`.
fn put_text(
    list: &mut DrawList,
    id: usize,
    rect: Rect,
    size: f32,
    color: Color,
    weight: FontWeight,
    align: TextAlign,
) {
    let _ = list.push(DrawCommand::Text {
        rect,
        text: TextId(id as u32),
        size,
        color,
        align,
        overflow: TextOverflow::Ellipsis,
        weight,
        letter_spacing: 0.0,
    });
}

/// Inputs of one frame.
pub(crate) struct Input<'a> {
    /// The player under the crosshair this frame.
    pub(crate) seen: Option<u16>,
    pub(crate) game: &'a GameState,
    pub(crate) world: &'a sjk_runtime::World,
    pub(crate) now: i32,
    pub(crate) camera: Camera,
    /// The scoreboard, a menu, the console or intermission is up.
    pub(crate) hidden: bool,
    /// What the hub knows about the slot, shown as the game's name.
    pub(crate) hub: &'a dyn Fn(u8, &str) -> Option<HubInfo>,
    /// Counts changes to the hub's roster, so the card is rebuilt.
    pub(crate) hub_revision: u64,
    /// The snapshot's entities, for the height of each player's box.
    pub(crate) entities: &'a [sjk_protocol::EntityState],
    /// The resolved model icon of a client slot.
    pub(crate) icon: &'a dyn Fn(u8) -> Option<TextureId>,
}

/// The card's state: settings, dwell, fade and the draw list.
pub(crate) struct State {
    enabled: bool,
    delay_ms: i32,
    dwell: Dwell,
    pin: Pin,
    /// The player whose card an `inspect` press just hid; their dwell is ignored until
    /// the crosshair leaves them, so the card does not come straight back.
    suppressed: Option<u16>,
    shown: Option<u16>,
    key: (u16, u64),
    card: Card,
    alpha: f32,
    last_frame: i32,
    /// Shapes and text ids; texts resolve through [`Card::texts`].
    pub(crate) list: DrawList,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enabled: true,
            delay_ms: 1_500,
            dwell: Dwell::default(),
            pin: Pin::default(),
            suppressed: None,
            shown: None,
            key: (0, 0),
            card: Card::default(),
            alpha: 0.0,
            last_frame: 0,
            list: DrawList::new(64),
        }
    }
}

impl State {
    /// Sample the settings once a frame, outside text emission.
    pub(crate) fn sample(&mut self, console: Option<&crate::console::ViewerConsole>) {
        self.enabled = console
            .and_then(|c| c.bool_cvar("cg_playercard"))
            .unwrap_or(true);
        let seconds = crate::cgame_options::scalar(console, "cg_playercarddelay", 1.5);
        self.delay_ms = (seconds.clamp(0.3, 10.0) * 1_000.0) as i32;
    }

    /// A card is pinned: the next `inspect` unpins it.
    pub(crate) fn pinned(&self) -> bool {
        self.pin.target().is_some()
    }

    /// `inspect`: pin the card to the player under the crosshair, or unpin it.
    pub(crate) fn inspect(&mut self) {
        self.pin.request();
    }

    /// The player whose card is up, so their model icon can be resolved.
    pub(crate) fn shown_client(&self) -> Option<u16> {
        self.shown.filter(|_| self.alpha > 0.0)
    }

    /// Forget everything shown (no session).
    pub(crate) fn clear(&mut self) {
        self.list.clear();
        self.dwell = Dwell::default();
        self.pin = Pin::default();
        self.suppressed = None;
        self.shown = None;
        self.alpha = 0.0;
    }

    /// Advance the dwell and the fade and rebuild the draw list.
    pub(crate) fn update(&mut self, input: Input<'_>) {
        self.list.clear();
        let now = input.now;
        let forward = (input.camera.target - input.camera.eye).normalize_or_zero();
        // The clock going back is a map change: nothing pinned or shown carries over.
        let restarted = now < self.last_frame;
        if restarted {
            self.shown = None;
            self.alpha = 0.0;
            self.suppressed = None;
        }
        let alive = self.pin.target().is_none_or(|client| {
            input
                .world
                .entity(sjk_runtime::EntityId::new(u64::from(client) + 1))
                .is_some()
                && input
                    .game
                    .config_string(CS_PLAYERS + usize::from(client))
                    .is_some_and(|info| !info.is_empty())
        });
        let step = self.pin.resolve(input.seen, alive, restarted);
        if let Some(released) = step.released {
            self.suppressed = Some(released);
            self.dwell = Dwell::default();
        }
        if self.suppressed.is_some() && self.suppressed != input.seen {
            self.suppressed = None;
        }
        let seen = if self.enabled && !input.hidden && self.suppressed.is_none() {
            input.seen
        } else {
            None
        };
        self.dwell.observe(seen, forward, now);
        let pinned = step.target.filter(|_| self.enabled);
        let want = if input.hidden {
            None
        } else if pinned.is_some() {
            pinned
        } else if self.enabled {
            self.dwell.ready(now, self.delay_ms)
        } else {
            None
        };
        let elapsed = (now - self.last_frame).clamp(0, 100) as f32;
        self.last_frame = now;
        match want {
            Some(client) => {
                self.shown = Some(client);
                self.alpha = (self.alpha + elapsed / FADE_IN_MS).min(1.0);
            }
            None => {
                self.alpha = (self.alpha - elapsed / FADE_OUT_MS).max(0.0);
                if self.alpha <= 0.0 {
                    self.shown = None;
                }
            }
        }
        let Some(client) = self.shown else { return };
        if self.alpha <= 0.0 {
            return;
        }
        if self.key != (client, input.hub_revision) || self.card == Card::default() {
            self.rebuild(client, &input);
        }
        let Some(presented) = input
            .world
            .entity(sjk_runtime::EntityId::new(u64::from(client) + 1))
        else {
            return;
        };
        let origin = Vec3::from_array(presented.sample(i64::from(now)).translation);
        let top = input
            .entities
            .iter()
            .find(|entity| entity.number() == client)
            .map_or_else(
                || super::nameplate_math::head_height(0),
                |entity| super::nameplate_math::head_height(entity.solid()),
            );
        let limit = if pinned.is_some() { PINNED_LIMIT } else { 1.2 };
        let anchor = origin + Vec3::Z * (top - HEAD_INSET);
        let Some((head, side)) = anchor_on_screen(input.camera, anchor, limit) else {
            return;
        };
        self.card.icon = u8::try_from(client)
            .ok()
            .and_then(|slot| (input.icon)(slot));
        self.emit(head, side, input.camera.viewport, pinned.is_some());
    }

    fn rebuild(&mut self, client: u16, input: &Input<'_>) {
        self.key = (client, input.hub_revision);
        let info = input
            .game
            .config_string(CS_PLAYERS + usize::from(client))
            .unwrap_or_default();
        let name = LegacyClientInfo::new(info)
            .bytes("n")
            .map(|bytes| decode_legacy(bytes).into_owned())
            .unwrap_or_default();
        let hub = u8::try_from(client)
            .ok()
            .and_then(|slot| (input.hub)(slot, &name));
        self.card = card_from(info, hub);
    }

    /// Draw the card beside `head`; a `pinned` card names the player's medals.
    fn emit(&mut self, head: [f32; 2], side: f32, viewport: [f32; 2], pinned: bool) {
        let unit = crate::ui_scale::height_scale(viewport[1]);
        let a = self.alpha;
        let card = &self.card;
        let pad = 14.0 * unit;
        let width = 290.0 * unit;
        let row = |height: f32| height * unit;
        let mut height = pad * 2.0 + row(30.0) + row(22.0);
        if !card.texts[T_SABER].is_empty() {
            height += row(22.0);
        }
        if card.has_worn() {
            height += row(22.0);
        }
        if card.has_extra() {
            height += row(22.0);
        }
        let medals = card.hub.as_ref().map(|hub| hub.medals).unwrap_or_default();
        if card.hub.is_some() {
            height += row(10.0) + row(26.0);
            if !medals.is_empty() {
                height += row(30.0);
                if pinned {
                    height += row(20.0) * card.medal_lines() as f32;
                }
            }
        }
        // Keep clear of the body: the world gap projected, never less than a fixed one.
        let offset = side.max(14.0 * unit) + 12.0 * unit;
        let placed = place(head, [width, height], offset, viewport);
        let rect = placed.card;
        let white = |alpha: f32| Color::new(1.0, 1.0, 1.0, alpha * a);
        let muted = Color::new(0.74, 0.78, 0.86, a);
        let accent = match card.team {
            1 => Color::new(0.95, 0.28, 0.30, a),
            2 => Color::new(0.30, 0.60, 1.0, a),
            _ => Color::new(0.55, 0.78, 0.95, a),
        };
        // The leader from the head to the card.
        let (from, to) = if placed.right {
            (head[0], rect.x)
        } else {
            (rect.x + rect.width, head[0])
        };
        if head[1] >= rect.y && head[1] <= rect.y + rect.height {
            let _ = self.list.push(DrawCommand::SolidRect {
                rect: Rect::new(
                    from,
                    head[1] - 0.75 * unit,
                    (to - from).max(0.0),
                    1.5 * unit,
                ),
                color: white(0.5),
            });
        }
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect: Rect::new(
                head[0] - 3.5 * unit,
                head[1] - 3.5 * unit,
                7.0 * unit,
                7.0 * unit,
            ),
            radius: 3.5 * unit,
            color: white(0.9),
        });
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect,
            radius: 8.0 * unit,
            color: Color::new(0.03, 0.04, 0.07, 0.82 * a),
        });
        let _ = self.list.push(DrawCommand::Border {
            rect,
            radius: 8.0 * unit,
            width: unit.max(1.0),
            color: white(0.16),
        });
        let _ = self.list.push(DrawCommand::SolidRect {
            rect: Rect::new(
                rect.x + 1.0 * unit,
                rect.y + 8.0 * unit,
                3.0 * unit,
                rect.height - 16.0 * unit,
            ),
            color: accent,
        });
        let inner = rect.width - pad * 2.0;
        let mut y = rect.y + pad;
        let left = rect.x + pad;
        // The model's icon fills the top right, beside the name and model.
        let icon_side = row(50.0);
        let head_width = if let Some(texture) = card.icon {
            let _ = self.list.push(DrawCommand::TexturedQuad {
                rect: Rect::new(
                    rect.x + rect.width - pad - icon_side,
                    y,
                    icon_side,
                    icon_side,
                ),
                texture,
                color: Color::new(1.0, 1.0, 1.0, a),
            });
            inner - icon_side - 8.0 * unit
        } else {
            inner
        };
        put_text(
            &mut self.list,
            T_NAME,
            Rect::new(left, y, head_width, row(30.0)),
            23.0 * unit,
            white(1.0),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += row(30.0);
        put_text(
            &mut self.list,
            T_MODEL,
            Rect::new(left, y, head_width, row(22.0)),
            15.0 * unit,
            muted,
            FontWeight::Regular,
            TextAlign::Start,
        );
        y += row(22.0);
        if !card.texts[T_SABER].is_empty() {
            let swatches = card.swatches.iter().flatten().count() as f32;
            put_text(
                &mut self.list,
                T_SABER,
                Rect::new(left, y, inner - swatches * 18.0 * unit, row(22.0)),
                15.0 * unit,
                muted,
                FontWeight::Regular,
                TextAlign::Start,
            );
            let mut x = rect.x + rect.width - pad - 14.0 * unit;
            for rgb in card.swatches.iter().flatten().rev() {
                let _ = self.list.push(DrawCommand::RoundedRect {
                    rect: Rect::new(x, y + 4.0 * unit, 14.0 * unit, 14.0 * unit),
                    radius: 7.0 * unit,
                    color: Color::new(
                        f32::from(rgb[0]) / 255.0,
                        f32::from(rgb[1]) / 255.0,
                        f32::from(rgb[2]) / 255.0,
                        a,
                    ),
                });
                x -= 18.0 * unit;
            }
            y += row(22.0);
        }
        if card.has_worn() {
            put_text(
                &mut self.list,
                T_WORN,
                Rect::new(left, y, inner, row(22.0)),
                15.0 * unit,
                muted,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += row(22.0);
        }
        if card.has_extra() {
            put_text(
                &mut self.list,
                T_EXTRA,
                Rect::new(left, y, inner, row(22.0)),
                15.0 * unit,
                muted,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += row(22.0);
        }
        if let Some(hub) = &card.hub {
            y += row(10.0);
            let _ = self.list.push(DrawCommand::SolidRect {
                rect: Rect::new(left, y - row(6.0), inner, unit.max(1.0)),
                color: white(0.12),
            });
            let tint = if hub.verified {
                Color::new(1.0, 0.82, 0.25, a)
            } else {
                white(0.95)
            };
            let side = row(22.0);
            let _ = self.list.push(DrawCommand::TexturedQuad {
                rect: Rect::new(left, y + row(2.0), side, side),
                texture: crate::ui_renderer::LOGO_TEXTURE,
                color: tint,
            });
            let hub_x = left + side + 8.0 * unit;
            let verified_width = if hub.verified { 92.0 * unit } else { 0.0 };
            put_text(
                &mut self.list,
                T_HUB,
                Rect::new(
                    hub_x,
                    y,
                    inner - side - 8.0 * unit - verified_width,
                    row(26.0),
                ),
                17.0 * unit,
                tint,
                FontWeight::Semibold,
                TextAlign::Start,
            );
            if hub.verified {
                put_text(
                    &mut self.list,
                    T_VERIFIED,
                    Rect::new(left, y, inner, row(26.0)),
                    12.0 * unit,
                    tint,
                    FontWeight::Semibold,
                    TextAlign::End,
                );
            }
            y += row(26.0);
            // The medals the SJK team gave them, small medallions in a row; a pinned
            // card names them under it, with how often a repeatable one was given.
            if !medals.is_empty() {
                let icon = row(26.0);
                let mut x = left;
                for (medal, _) in medals.iter() {
                    let _ = self.list.push(DrawCommand::TexturedQuad {
                        rect: Rect::new(x, y + row(2.0), icon, icon),
                        texture: medal.icon(),
                        color: Color::new(1.0, 1.0, 1.0, a),
                    });
                    x += icon + 6.0 * unit;
                }
                y += row(30.0);
                if pinned {
                    for (index, id) in [T_MEDALS, T_MEDALS_MORE]
                        .into_iter()
                        .take(card.medal_lines())
                        .enumerate()
                    {
                        put_text(
                            &mut self.list,
                            id,
                            Rect::new(
                                left,
                                y - row(2.0) + index as f32 * row(20.0),
                                inner,
                                row(20.0),
                            ),
                            13.0 * unit,
                            muted,
                            FontWeight::Regular,
                            TextAlign::Start,
                        );
                    }
                }
            }
        }
    }

    /// Resolve the card's text ids at submission.
    pub(crate) fn append(&self, vertices: &mut Vec<TextVertex>, font: &UiFont, viewport: [f32; 2]) {
        crate::ui_renderer::append_text_commands(
            &self.list,
            |id| {
                self.card
                    .texts
                    .get(id.0 as usize)
                    .map_or("", String::as_str)
            },
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
        );
    }
}

#[cfg(test)]
impl State {
    /// A card drawn as if its player had been looked at (`pinned`: with `inspect`), for
    /// the off-screen snapshots (`menu_snapshot.rs`).
    pub(crate) fn preview(card: Card, head: [f32; 2], viewport: [f32; 2], pinned: bool) -> Self {
        let mut state = Self {
            card,
            alpha: 1.0,
            ..Self::default()
        };
        state.emit(head, 24.0, viewport, pinned);
        state
    }

    /// The text a draw command's id names.
    pub(crate) fn resolve_text(&self, id: TextId) -> &str {
        self.card
            .texts
            .get(id.0 as usize)
            .map_or("", String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORWARD: Vec3 = Vec3::X;

    fn turned(degrees: f32) -> Vec3 {
        let radians = degrees.to_radians();
        Vec3::new(radians.cos(), radians.sin(), 0.0)
    }

    #[test]
    fn a_steady_look_becomes_ready_after_the_delay() {
        let mut dwell = Dwell::default();
        dwell.observe(Some(4), FORWARD, 1_000);
        assert_eq!(dwell.ready(1_000, 1_500), None);
        dwell.observe(Some(4), FORWARD, 2_000);
        assert_eq!(dwell.ready(2_000, 1_500), None);
        dwell.observe(Some(4), FORWARD, 2_500);
        assert_eq!(dwell.ready(2_500, 1_500), Some(4));
    }

    #[test]
    fn turning_the_view_starts_the_wait_again() {
        let mut dwell = Dwell::default();
        dwell.observe(Some(4), FORWARD, 0);
        dwell.observe(Some(4), FORWARD, 2_000);
        assert_eq!(dwell.ready(2_000, 1_500), Some(4));
        dwell.observe(Some(4), turned(10.0), 2_050);
        assert_eq!(dwell.ready(2_050, 1_500), None);
        // A small drift is not a turn.
        let mut steady = Dwell::default();
        steady.observe(Some(4), FORWARD, 0);
        steady.observe(Some(4), turned(3.0), 2_000);
        assert_eq!(steady.ready(2_000, 1_500), Some(4));
    }

    #[test]
    fn another_player_or_a_long_gap_starts_over_but_a_short_gap_does_not() {
        let mut dwell = Dwell::default();
        dwell.observe(Some(4), FORWARD, 0);
        dwell.observe(None, FORWARD, 200);
        dwell.observe(Some(4), FORWARD, 300);
        dwell.observe(Some(4), FORWARD, 1_600);
        assert_eq!(
            dwell.ready(1_600, 1_500),
            Some(4),
            "a 200 ms gap is forgiven"
        );
        dwell.observe(Some(7), FORWARD, 1_700);
        assert_eq!(dwell.ready(1_700, 0), Some(7));
        assert_eq!(dwell.ready(1_700, 1), None, "a new target waits again");
        dwell.observe(None, FORWARD, 1_800);
        dwell.observe(None, FORWARD, 2_300);
        assert_eq!(dwell.ready(2_300, 0), None, "a long gap forgets the target");
    }

    #[test]
    fn a_clock_that_goes_back_forgets_the_target() {
        let mut dwell = Dwell::default();
        dwell.observe(Some(4), FORWARD, 9_000);
        dwell.observe(None, FORWARD, 100);
        assert_eq!(dwell.ready(100, 0), None);
    }

    fn info(text: &str) -> Vec<u8> {
        text.replace('|', "\\").into_bytes()
    }

    #[test]
    fn a_card_reads_what_the_server_publishes() {
        let card = card_from(
            &info("n|^1Sol|t|1|model|kyle/default|ds|m|st|single_1|st2|none|c1|4|c2|0|hc|100|"),
            None,
        );
        assert_eq!(card.texts[T_NAME], "^1Sol");
        assert_eq!(card.texts[T_MODEL], "kyle / default");
        assert_eq!(card.texts[T_SABER], "single_1", "none is no second saber");
        assert_eq!(card.team, 1);
        assert_eq!(card.swatches, [Some([51, 102, 255]), None]);
        assert!(card.texts[T_EXTRA].is_empty() && card.hub.is_none());
    }

    #[test]
    fn dual_sabers_duels_and_bots_get_their_lines() {
        let dual = card_from(
            &info("n|Fox|t|3|model|jedi_hf/blue|st|dual_1|st2|dual_2|c1|0|c2|3|w|5|l|2|"),
            None,
        );
        assert_eq!(dual.texts[T_SABER], "dual_1 + dual_2");
        assert_eq!(dual.swatches, [Some([255, 51, 51]), Some([51, 255, 51])]);
        assert_eq!(dual.texts[T_EXTRA], "Duel  5 W / 2 L");
        let bot = card_from(&info("n|Kyle|t|0|model|kyle|skill|3|"), None);
        assert_eq!(bot.texts[T_EXTRA], "Bot, skill 3");
    }

    #[test]
    fn a_cosmetic_after_the_colour_digit_is_still_a_colour() {
        let card = card_from(&info("n|Sol|st|single_1|c1|4santahat|"), None);
        assert_eq!(card.swatches[0], Some([51, 102, 255]));
        let custom = card_from(&info("n|Sol|st|single_1|c1|9|"), None);
        assert_eq!(
            custom.swatches[0], None,
            "only retail colours have a swatch"
        );
    }

    #[test]
    fn the_hub_adds_the_emblem_name_and_verification() {
        let known = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: "Sol the Fox".to_owned(),
                verified: true,
                medals: crate::medals::Medals::default(),
            }),
        );
        assert_eq!(known.texts[T_HUB], "Sol the Fox");
        assert_eq!(known.texts[T_VERIFIED], "VERIFIED");
        assert!(known.texts[T_MEDALS].is_empty());
        let unnamed = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: String::new(),
                verified: false,
                medals: crate::medals::Medals::default(),
            }),
        );
        assert_eq!(unnamed.texts[T_HUB], "SJK player");
        assert!(unnamed.texts[T_VERIFIED].is_empty());
    }

    #[test]
    fn long_medal_names_take_a_second_line() {
        let medal = |id: &str| sjk_identity::Medal {
            id: id.to_owned(),
            count: 1,
            awarded: 0,
            note: String::new(),
        };
        let card = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: "Sol".to_owned(),
                verified: false,
                medals: crate::medals::Medals::from_wire(&[
                    medal("early_tester"),
                    medal("early_contributor"),
                    medal("bug_hunter"),
                    medal("jof_clan"),
                ]),
            }),
        );
        assert_eq!(card.texts[T_MEDALS], "Early Tester, Early Contributor");
        assert_eq!(card.texts[T_MEDALS_MORE], "Bug Hunter, JoF Clan");
        assert_eq!(card.medal_lines(), 2);
    }

    #[test]
    fn medals_show_as_medallions_and_a_pinned_card_names_them() {
        let medal = |id: &str, count| sjk_identity::Medal {
            id: id.to_owned(),
            count,
            awarded: 0,
            note: String::new(),
        };
        let card = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: "Sol".to_owned(),
                verified: true,
                medals: crate::medals::Medals::from_wire(&[
                    medal("bug_hunter", 2),
                    medal("early_tester", 1),
                    medal("unknown", 1),
                ]),
            }),
        );
        assert_eq!(card.texts[T_MEDALS], "Early Tester, Bug Hunter x2");
        assert!(card.texts[T_MEDALS_MORE].is_empty());
        let icons = |state: &State| {
            state
                .list
                .commands()
                .iter()
                .filter(|command| {
                    matches!(command, DrawCommand::TexturedQuad { texture, .. }
                        if *texture == crate::medals::Medal::EarlyTester.icon()
                            || *texture == crate::medals::Medal::BugHunter.icon())
                })
                .count()
        };
        let named = |state: &State| {
            state.list.commands().iter().any(|command| {
                matches!(command, DrawCommand::Text { text, .. } if text.0 as usize == T_MEDALS)
            })
        };
        let viewport = [1_920.0, 1_080.0];
        let glance = State::preview(card.clone(), [900.0, 500.0], viewport, false);
        assert_eq!(icons(&glance), 2);
        assert!(!named(&glance));
        let pinned = State::preview(card, [900.0, 500.0], viewport, true);
        assert_eq!(icons(&pinned), 2);
        assert!(named(&pinned));
    }

    #[test]
    fn the_card_stays_beside_the_head_and_inside_the_screen() {
        let viewport = [1_920.0, 1_080.0];
        let size = [290.0, 140.0];
        let beside = place([900.0, 500.0], size, 34.0, viewport);
        assert!(beside.right && beside.card.x == 934.0);
        let edge = place([1_800.0, 500.0], size, 34.0, viewport);
        assert!(!edge.right, "no room on the right: it flips to the left");
        assert!(edge.card.x + edge.card.width < 1_800.0);
        let corner = place([10.0, 5.0], size, 34.0, viewport);
        assert!(corner.card.y >= 0.0 && corner.card.x >= 0.0);
        let low = place([900.0, 1_075.0], size, 34.0, viewport);
        assert!(low.card.y + low.card.height <= viewport[1]);
    }

    #[test]
    fn inspect_pins_the_player_under_the_crosshair_and_a_second_press_unpins() {
        let mut pin = Pin::default();
        // Nothing under the crosshair: the press pins nothing.
        pin.request();
        assert_eq!(pin.resolve(None, true, false), PinStep::default());
        pin.request();
        let step = pin.resolve(Some(4), true, false);
        assert_eq!(step.target, Some(4));
        // Looking away or at someone else does not move it.
        assert_eq!(pin.resolve(None, true, false).target, Some(4));
        assert_eq!(pin.resolve(Some(9), true, false).target, Some(4));
        pin.request();
        let step = pin.resolve(Some(9), true, false);
        assert_eq!((step.target, step.released), (None, Some(4)));
    }

    #[test]
    fn the_pin_drops_when_the_player_leaves_or_the_map_changes() {
        let mut pin = Pin::default();
        pin.request();
        pin.resolve(Some(4), true, false);
        let step = pin.resolve(None, false, false);
        assert_eq!((step.target, step.released), (None, Some(4)));
        pin.request();
        pin.resolve(Some(5), true, false);
        let step = pin.resolve(Some(5), true, true);
        assert_eq!((step.target, step.released), (None, Some(5)));
    }

    #[test]
    fn the_card_reads_the_hat_and_cape_after_the_colour_digits() {
        let both = card_from(&info("n|Sol|c1|4santa_hat|c2|0red-cape|"), None);
        assert_eq!(both.texts[T_WORN], "Hat Santa hat  /  Cape Red cape");
        let hat = card_from(&info("n|Sol|c1|4santa_hat|c2|3|"), None);
        assert_eq!(hat.texts[T_WORN], "Hat Santa hat");
        let cape = card_from(&info("n|Sol|c1|4|c2|2jedi|"), None);
        assert_eq!(cape.texts[T_WORN], "Cape Jedi");
        let none = card_from(&info("n|Sol|c1|4|c2|0|"), None);
        assert!(none.texts[T_WORN].is_empty() && !none.has_worn());
    }

    fn camera() -> Camera {
        Camera {
            eye: Vec3::ZERO,
            target: Vec3::X,
            up: Vec3::Z,
            fov: 90.0,
            viewport: [1_920.0, 1_080.0],
        }
    }

    #[test]
    fn the_anchor_follows_the_player_and_clears_the_body_at_any_range() {
        let near = anchor_on_screen(camera(), Vec3::new(100.0, 0.0, 0.0), 1.2).unwrap();
        let far = anchor_on_screen(camera(), Vec3::new(400.0, 0.0, 0.0), 1.2).unwrap();
        assert!((near.0[0] - 960.0).abs() < 1.0 && (near.0[1] - 540.0).abs() < 1.0);
        assert!(near.1 > far.1 * 3.5, "a nearer body is wider on screen");
        // Higher in the world is higher on screen.
        let above = anchor_on_screen(camera(), Vec3::new(100.0, 0.0, 30.0), 1.2).unwrap();
        assert!(above.0[1] < near.0[1]);
        // Behind the camera there is no anchor, however far the limit.
        assert!(anchor_on_screen(camera(), Vec3::new(-100.0, 0.0, 0.0), 40.0).is_none());
    }

    #[test]
    fn a_pinned_player_far_off_screen_still_gets_a_card_kept_on_screen() {
        let viewport = [1_920.0, 1_080.0];
        let off = Vec3::new(100.0, 900.0, 0.0);
        assert!(anchor_on_screen(camera(), off, 1.2).is_none());
        let (head, side) = anchor_on_screen(camera(), off, PINNED_LIMIT).unwrap();
        let placed = place(head, [290.0, 140.0], side + 20.0, viewport);
        let card = placed.card;
        assert!(card.x >= 0.0 && card.x + card.width <= viewport[0]);
        assert!(card.y >= 0.0 && card.y + card.height <= viewport[1]);
    }
}
