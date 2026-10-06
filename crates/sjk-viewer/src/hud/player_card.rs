//! The player card: look at a player for a moment without moving and a small card
//! appears beside their head with what the game publishes about them (name, model,
//! saber, duel record) and, when the SJK hub knows them, SJK's emblem, their hub
//! name and whether they are verified.
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
/// Height above the player's origin the card points at (the head).
const HEAD_HEIGHT: f32 = 70.0;
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

fn angle_degrees(a: Vec3, b: Vec3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos().to_degrees()
}

/// What the hub knows about the player.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HubInfo {
    /// Hub display name, empty if they have none yet.
    pub(crate) name: String,
    pub(crate) verified: bool,
}

/// Text slots of a card: ids are indices into [`Card::texts`].
const T_NAME: usize = 0;
const T_MODEL: usize = 1;
const T_SABER: usize = 2;
const T_EXTRA: usize = 3;
const T_HUB: usize = 4;
const T_VERIFIED: usize = 5;

/// A player's card: the texts and values drawn, built when the target or the
/// hub's roster changes and drawn every frame without allocating.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Card {
    pub(crate) texts: [String; 6],
    /// 0 free, 1 red, 2 blue, 3 spectator.
    team: u8,
    /// Blade colours of the one or two sabers, if retail colours.
    swatches: [Option<[u8; 3]>; 2],
    hub: Option<HubInfo>,
}

impl Card {
    fn has_extra(&self) -> bool {
        !self.texts[T_EXTRA].is_empty()
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
}

/// The card's state: settings, dwell, fade and the draw list.
pub(crate) struct State {
    enabled: bool,
    delay_ms: i32,
    dwell: Dwell,
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

    /// Forget everything shown (no session).
    pub(crate) fn clear(&mut self) {
        self.list.clear();
        self.dwell = Dwell::default();
        self.shown = None;
        self.alpha = 0.0;
    }

    /// Advance the dwell and the fade and rebuild the draw list.
    pub(crate) fn update(&mut self, input: Input<'_>) {
        self.list.clear();
        let now = input.now;
        let forward = (input.camera.target - input.camera.eye).normalize_or_zero();
        let seen = if self.enabled && !input.hidden {
            input.seen
        } else {
            None
        };
        self.dwell.observe(seen, forward, now);
        let want = if self.enabled && !input.hidden {
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
        let Some(head) = input
            .camera
            .project_within(origin + Vec3::Z * HEAD_HEIGHT, 1.2)
        else {
            return;
        };
        self.emit(head, input.camera.viewport);
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

    fn emit(&mut self, head: [f32; 2], viewport: [f32; 2]) {
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
        if card.has_extra() {
            height += row(22.0);
        }
        if card.hub.is_some() {
            height += row(10.0) + row(26.0);
        }
        let placed = place(head, [width, height], 34.0 * unit, viewport);
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
        put_text(
            &mut self.list,
            T_NAME,
            Rect::new(left, y, inner, row(30.0)),
            23.0 * unit,
            white(1.0),
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += row(30.0);
        put_text(
            &mut self.list,
            T_MODEL,
            Rect::new(left, y, inner, row(22.0)),
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
    /// A card drawn as if its player had been looked at, for the off-screen
    /// snapshots (`menu_snapshot.rs`).
    pub(crate) fn preview(card: Card, head: [f32; 2], viewport: [f32; 2]) -> Self {
        let mut state = Self {
            card,
            alpha: 1.0,
            ..Self::default()
        };
        state.emit(head, viewport);
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
            }),
        );
        assert_eq!(known.texts[T_HUB], "Sol the Fox");
        assert_eq!(known.texts[T_VERIFIED], "VERIFIED");
        let unnamed = card_from(
            &info("n|Sol|"),
            Some(HubInfo {
                name: String::new(),
                verified: false,
            }),
        );
        assert_eq!(unnamed.texts[T_HUB], "SJK player");
        assert!(unnamed.texts[T_VERIFIED].is_empty());
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
}
