//! Nameplate maths: how large, how opaque and where a plate sits, the capsule it is
//! drawn as ([`Capsule`]), and the colours and rectangles of its bars. Pure functions,
//! so the layout is testable without a window; [`super::nameplate`] collects the
//! players and draws with these.
use sjk_ui::{Color, Rect};

/// Distance, in world units, out to which a plate keeps its full size.
const FULL_SIZE_DISTANCE: f32 = 300.0;
/// Share of the range, from the camera, over which a plate is fully opaque.
const OPAQUE_SHARE: f32 = 0.75;
/// Units above the top of the player's box at which the plate is anchored.
pub(super) const HEAD_CLEARANCE: f32 = 8.0;
/// Standing box top above the origin, used when the entity sends none.
const STANDING_TOP: f32 = 40.0;
/// Milliseconds a plate takes to fade between hidden and shown.
pub(super) const FADE_MILLIS: f32 = 120.0;
/// Share of the detail distance over which the bars fade in on approach.
const DETAIL_BLEND: f32 = 0.25;

/// Size multiplier for a plate `distance` away: full size up close, shrinking
/// linearly to `minimum` at `range`.
pub(super) fn distance_scale(distance: f32, range: f32, minimum: f32) -> f32 {
    if range <= FULL_SIZE_DISTANCE {
        return 1.0;
    }
    let t = ((distance - FULL_SIZE_DISTANCE) / (range - FULL_SIZE_DISTANCE)).clamp(0.0, 1.0);
    1.0 + (minimum.clamp(0.0, 1.0) - 1.0) * t
}

/// Opacity for a plate `distance` away: opaque for the first part of the range,
/// then fading to nothing at `range`.
pub(super) fn distance_fade(distance: f32, range: f32) -> f32 {
    let start = range * OPAQUE_SHARE;
    if distance <= start || range <= start {
        return 1.0;
    }
    (1.0 - (distance - start) / (range - start)).clamp(0.0, 1.0)
}

/// How much of the plate (the bars) shows at `distance`: none beyond `near`,
/// all of it within the inner part of `near`, blending between.
pub(super) fn detail(distance: f32, near: f32) -> f32 {
    if near <= 0.0 {
        return 0.0;
    }
    ((near - distance) / (near * DETAIL_BLEND)).clamp(0.0, 1.0)
}

/// Height of the top of an entity's box above its origin, from the packed
/// `entityState_t::solid` (`SV_LinkEntity`: bits 16-23 hold top + 32). A
/// player who sends no box is taken as standing.
pub(super) fn head_height(solid: u32) -> f32 {
    let packed = (solid >> 16) & 255;
    if solid == 0 || packed == 0 {
        STANDING_TOP
    } else {
        packed as f32 - 32.0
    }
}

/// Where a player's plate hangs: the world point it is traced from and the
/// height of the box top above it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Anchor {
    pub(super) origin: [f32; 3],
    pub(super) head: f32,
}

/// A presented body: its interpolated origin and packed `entityState_t::solid`.
pub(super) type Body = ([f32; 3], u32);

/// The anchor of a player whose own body is `own` and whose vehicle's body is
/// `vehicle`, each `None` when the world does not present it.
///
/// A pilot inside a `hideRider` vehicle carries `EF_NODRAW`, so the client never
/// presents his body (`CG_AddCEntity` skips it); his plate hangs over the
/// vehicle he sits in instead. A rider whose body is presented keeps his own.
pub(super) fn anchor(own: Option<Body>, vehicle: Option<Body>) -> Option<Anchor> {
    let (origin, solid) = own.or(vehicle)?;
    Some(Anchor {
        origin,
        head: head_height(solid),
    })
}

/// `CLASS_VEHICLE` (`teams.h`).
const CLASS_VEHICLE: u8 = 53;
/// `ET_NPC` (`entityType_t`).
const ET_NPC: u8 = 13;
/// `MAX_CLIENTS`.
const MAX_CLIENTS: u16 = 32;

/// The client sealed in a vehicle whose snapshot state is given, if his own entity
/// is not in the snapshot: stock cgame names the pilot of an `ET_NPC` /
/// `CLASS_VEHICLE` entity from its `owner` while that is below `MAX_CLIENTS`
/// (`CG_DrawCrosshair`). The viewer is not named to himself, and a pilot whose
/// entity is sent (`present`) keeps the ordinary plate.
pub(super) fn hidden_pilot(
    entity_type: u8,
    npc_class: u8,
    owner: u16,
    local: u16,
    present: impl FnOnce(u16) -> bool,
) -> Option<u16> {
    (entity_type == ET_NPC
        && npc_class == CLASS_VEHICLE
        && owner < MAX_CLIENTS
        && owner != local
        && !present(owner))
    .then_some(owner)
}

/// Bar fills until the HUD in use names its own: the retail HUD's red health,
/// green shield (armour) and light blue Force.
pub(super) const HEALTH_COLOR: Color = Color::new(1.0, 0.25, 0.2, 1.0);
pub(super) const SHIELD_COLOR: Color = Color::new(0.3, 0.9, 0.35, 1.0);
pub(super) const FORCE_COLOR: Color = Color::new(0.36, 0.7, 1.0, 1.0);
/// The grey of an empty shield's broken bar.
pub(super) const EMPTY_SHIELD: Color = Color::new(0.55, 0.57, 0.6, 0.7);
/// The "?" over a bar the estimate cannot fill.
pub(super) const UNKNOWN_MARK: Color = Color::new(1.0, 0.85, 0.15, 1.0);

/// A deeper, more saturated `color`: the second layer of a bar over its maximum
/// (overheal, overshield). Each channel moves away from the mean and the whole
/// darkens, so it reads as the same colour, deeper, even for a green that is
/// already near full.
pub(super) fn saturated(color: Color) -> Color {
    let mean = (color.r + color.g + color.b) / 3.0;
    let push = |channel: f32| ((mean + (channel - mean) * 1.8) * 0.72).clamp(0.0, 1.0);
    Color::new(push(color.r), push(color.g), push(color.b), color.a)
}

/// Where a bar's second layer (over the maximum) is drawn in `bar`: an inner band,
/// so the full bar shows round it. The nameplates and the HUD's meters share it.
pub(super) fn overflow_band(bar: Rect) -> Rect {
    let inset = bar.height * 0.22;
    Rect::new(bar.x, bar.y + inset, bar.width, bar.height - inset * 2.0)
}

/// The pieces of an empty shield's broken bar, as `(start, end)` shares of its
/// width: dashes with gaps between, a crack's look.
pub(super) fn broken_dashes() -> impl Iterator<Item = (f32, f32)> {
    const DASHES: usize = 7;
    const SOLID: f32 = 0.7;
    (0..DASHES).map(|index| {
        let start = index as f32 / DASHES as f32;
        (start, start + SOLID / DASHES as f32)
    })
}

/// Which bars a plate holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Rows {
    pub(super) health: bool,
    pub(super) shield: bool,
    pub(super) force: bool,
}

impl Rows {
    pub(super) fn any(self) -> bool {
        self.health || self.shield || self.force
    }
}

/// The capsule's measures are its design's (28/10/2026, design E), in that design's
/// pixels, whose name is this tall: a plate scales them by its name's size over it.
const DESIGN_NAME: f32 = 13.0;
/// The capsule's height, its edge's width (a duellist's is wider), and the picture's
/// inset from the outer edge (the edge and the padding round the picture).
const PILL_HEIGHT: f32 = 26.0;
pub(super) const EDGE: f32 = 1.0;
pub(super) const DUEL_EDGE: f32 = 1.5;
const PICTURE_INSET: f32 = 3.0;
/// The room before the name without a picture (an NPC's), and after the last item: a
/// duel badge's end is tighter.
const BARE_START: f32 = 11.0;
const END_PAD: f32 = 11.0;
const DUEL_END_PAD: f32 = 5.0;
/// The gap before the right end's items, and between the name and the mark after it.
const GAP: f32 = 6.0;
const MARK_GAP: f32 = 4.0;
/// The verified seal's side.
const MARK: f32 = 11.0;
/// The right end: the padding before it, the stance dot, its gap to the weapon, the
/// weapon's icon and the duel badge.
const END_LEAD: f32 = 2.0;
const DOT: f32 = 5.0;
const DOT_GAP: f32 = 3.0;
/// Wider than the design's 11: the weapons' art has room round it.
const WEAPON: f32 = 13.0;
const BADGE: f32 = 18.0;
/// The widest the name runs before its ellipsis.
const NAME_MAX: f32 = 150.0;
/// The picture's diameter as a share of the name's size, near and far: far away the
/// capsule fades and a smaller picture stays beside the name.
const NEAR_PICTURE: f32 = 20.0 / 13.0;
const FAR_PICTURE: f32 = 13.0 / 11.0;
/// The gap after the picture, near and far, as a share of the name's size.
const NEAR_GAP: f32 = 6.0 / 13.0;
const FAR_GAP: f32 = 4.0 / 11.0;
/// The tray under the capsule: its width, how far it tucks under, the room round its
/// bars (top, sides, bottom), its bottom corners, and the room the capsule keeps
/// either side of it.
const TRAY_WIDTH: f32 = 104.0;
const TRAY_TUCK: f32 = 1.0;
const TRAY_TOP: f32 = 3.0;
const TRAY_SIDE: f32 = 6.0;
const TRAY_BOTTOM: f32 = 5.0;
pub(super) const TRAY_CORNER: f32 = 6.0;
const TRAY_MARGIN: f32 = 10.0;
/// The tray's bars, top to bottom, and the gap between them.
const TRAY_SHIELD: f32 = 2.0;
const TRAY_HEALTH: f32 = 3.0;
const TRAY_FORCE: f32 = 2.0;
const TRAY_GAP: f32 = 1.5;
/// The power icons over the capsule: their chips' side, the gap between them and the
/// room under the row.
const CHIP: f32 = 15.0;
const CHIP_GAP: f32 = 2.0;
const CHIP_LIFT: f32 = 3.0;

/// What a capsule ends with, right of the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum End {
    Nothing,
    /// The held weapon's icon, after the saber stance's dot when there is one.
    Weapon {
        dot: bool,
    },
    /// The duel badge, in the weapon's place.
    Duel,
}

/// What [`Capsule::new`] lays out.
#[derive(Clone, Copy, Debug)]
pub(super) struct CapsuleSpec {
    /// The point the plate is centred on, and the bottom of the whole plate (the
    /// tray's while it shows, else the capsule's).
    pub(super) centre_x: f32,
    pub(super) bottom: f32,
    /// The name's text size and drawn width, in pixels.
    pub(super) size: f32,
    pub(super) name_width: f32,
    pub(super) picture: bool,
    /// The mark after the name (the verified seal or a trust mark).
    pub(super) mark: bool,
    pub(super) end: End,
    /// The bars on the tray; none, no tray.
    pub(super) rows: Rows,
    /// How much of the tray shows, 0 to 1 ([`detail`]): the capsule rises over it and
    /// widens to hold it as it fades in.
    pub(super) detail: f32,
    /// How close the player is, 0 (far) to 1 ([`detail`]'s ramp, bars or not).
    pub(super) proximity: f32,
    /// The thinnest a bar may be, in pixels.
    pub(super) least_bar: f32,
    pub(super) viewport: [f32; 2],
}

/// The tray under a capsule: its frame and the bars inside (shield on top, then
/// health, then Force), in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Tray {
    pub(super) frame: Rect,
    pub(super) shield: Option<Rect>,
    pub(super) health: Option<Rect>,
    pub(super) force: Option<Rect>,
}

/// A nameplate as a capsule (design E, 28/10/2026): a pill holding the picture, the
/// name with its mark, and the weapon (or the duel badge) at its right end, with the
/// power icons over it and, for a player whose bars show, the tray under it. Physical
/// pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Capsule {
    /// The design's pixel in physical ones.
    pub(super) k: f32,
    pub(super) pill: Rect,
    /// The pill's vertical middle: the name's capitals, the picture and the icons are
    /// centred on it.
    pub(super) middle: f32,
    pub(super) picture: Option<Rect>,
    /// Where the name runs: its left edge and the most it may take.
    pub(super) name: Rect,
    pub(super) mark: Option<Rect>,
    pub(super) dot: Option<Rect>,
    pub(super) weapon: Option<Rect>,
    pub(super) badge: Option<Rect>,
    pub(super) tray: Option<Tray>,
}

impl Capsule {
    pub(super) fn new(spec: &CapsuleSpec) -> Self {
        let k = spec.size / DESIGN_NAME;
        let near = spec.proximity.clamp(0.0, 1.0);
        let detail = spec.detail.clamp(0.0, 1.0);
        let lerp = |far: f32, close: f32| far + (close - far) * near;
        let diameter = spec.size * lerp(FAR_PICTURE, NEAR_PICTURE);
        let height = PILL_HEIGHT * k;
        let start = if spec.picture {
            PICTURE_INSET * k + diameter + spec.size * lerp(FAR_GAP, NEAR_GAP)
        } else {
            BARE_START * k
        };
        let name_width = spec.name_width.clamp(0.0, NAME_MAX * k);
        let mark_width = if spec.mark {
            (MARK_GAP + MARK) * k
        } else {
            0.0
        };
        let (end_width, end_pad) = match spec.end {
            End::Nothing => (0.0, END_PAD),
            End::Weapon { dot } => (
                GAP + END_LEAD + if dot { DOT + DOT_GAP } else { 0.0 } + WEAPON,
                END_PAD,
            ),
            End::Duel => (GAP + END_LEAD + BADGE, DUEL_END_PAD),
        };
        let natural = start + name_width + mark_width + (end_width + end_pad) * k;
        let tray = spec.rows.any();
        let wanted = if tray {
            natural.max((TRAY_WIDTH + TRAY_MARGIN * 2.0) * k)
        } else {
            natural
        };
        let width = (natural + (wanted - natural) * detail).min(spec.viewport[0]);
        let bar = |height: f32| (height * k).max(spec.least_bar);
        let bars = [
            (spec.rows.shield, bar(TRAY_SHIELD)),
            (spec.rows.health, bar(TRAY_HEALTH)),
            (spec.rows.force, bar(TRAY_FORCE)),
        ];
        let count = bars.iter().filter(|(wanted, _)| *wanted).count();
        let tray_height = if count == 0 {
            0.0
        } else {
            (TRAY_TOP + TRAY_BOTTOM + TRAY_GAP * (count - 1) as f32) * k
                + bars
                    .iter()
                    .filter(|(wanted, _)| *wanted)
                    .map(|(_, height)| height)
                    .sum::<f32>()
        };
        let pill_bottom = spec.bottom - (tray_height - TRAY_TUCK * k).max(0.0) * detail;
        // Far away only the picture, the name and its mark (and a duel badge) stay:
        // they, not the faded pill, are centred over the player.
        let shown_from = if spec.picture {
            PICTURE_INSET * k
        } else {
            BARE_START * k
        };
        let hidden_right = match spec.end {
            End::Duel => end_pad * k,
            End::Nothing | End::Weapon { .. } => (end_width + end_pad) * k,
        };
        let lean = (hidden_right - shown_from) * 0.5 * (1.0 - near);
        let x =
            (spec.centre_x - width * 0.5 + lean).clamp(0.0, (spec.viewport[0] - width).max(0.0));
        let pill = Rect::new(x, pill_bottom - height, width, height);
        let middle = pill.y + height * 0.5;
        let square = |left: f32, side: f32| Rect::new(left, middle - side * 0.5, side, side);
        let picture = spec
            .picture
            .then(|| square(pill.x + PICTURE_INSET * k, diameter));
        let name = Rect::new(pill.x + start, pill.y, name_width, height);
        let mark = spec
            .mark
            .then(|| square(name.right() + MARK_GAP * k, MARK * k));
        let right = pill.right() - end_pad * k;
        let (dot, weapon, badge) = match spec.end {
            End::Nothing => (None, None, None),
            End::Weapon { dot } => {
                let weapon = square(right - WEAPON * k, WEAPON * k);
                let dot = dot.then(|| square(weapon.x - (DOT_GAP + DOT) * k, DOT * k));
                (dot, Some(weapon), None)
            }
            End::Duel => (None, None, Some(square(right - BADGE * k, BADGE * k))),
        };
        let tray = tray.then(|| {
            let tray_width = (TRAY_WIDTH * k).min(pill.width);
            let frame = Rect::new(
                pill.x + (pill.width - tray_width) * 0.5,
                pill.bottom() - TRAY_TUCK * k,
                tray_width,
                tray_height,
            );
            let inner_x = frame.x + TRAY_SIDE * k;
            let inner_width = (frame.width - TRAY_SIDE * 2.0 * k).max(0.0);
            let mut cursor = frame.y + TRAY_TOP * k;
            let mut next = |(wanted, height): (bool, f32)| {
                wanted.then(|| {
                    let rect = Rect::new(inner_x, cursor, inner_width, height);
                    cursor += height + TRAY_GAP * k;
                    rect
                })
            };
            Tray {
                frame,
                shield: next(bars[0]),
                health: next(bars[1]),
                force: next(bars[2]),
            }
        });
        Self {
            k,
            pill,
            middle,
            picture,
            name,
            mark,
            dot,
            weapon,
            badge,
            tray,
        }
    }

    /// The power icons' chips, `count` of them, centred over the capsule: the first's
    /// rectangle and the step to the next.
    pub(super) fn chips(&self, count: usize, viewport: [f32; 2]) -> (Rect, f32) {
        let side = CHIP * self.k;
        let step = side + CHIP_GAP * self.k;
        let width = count as f32 * step - CHIP_GAP * self.k;
        let centre = self.pill.x + self.pill.width * 0.5;
        let left = (centre - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
        (
            Rect::new(left, self.pill.y - CHIP_LIFT * self.k - side, side, side),
            step,
        )
    }
}

/// The colour a team tints a capsule with: its own colour made deep, so red reads as
/// dark as the design's blue (`rgba(22, 52, 110, 0.86)`) does.
pub(super) fn team_fill(accent: Color) -> Color {
    let deep = |channel: f32| 0.04 + 0.43 * channel * channel;
    Color::new(deep(accent.r), deep(accent.g), deep(accent.b), 0.86)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plates_shrink_with_distance_to_the_minimum() {
        assert_eq!(distance_scale(100.0, 3000.0, 0.5), 1.0);
        assert_eq!(distance_scale(300.0, 3000.0, 0.5), 1.0);
        assert!((distance_scale(1650.0, 3000.0, 0.5) - 0.75).abs() < 1e-5);
        assert!((distance_scale(3000.0, 3000.0, 0.5) - 0.5).abs() < 1e-5);
        assert!((distance_scale(9000.0, 3000.0, 0.5) - 0.5).abs() < 1e-5);
        assert_eq!(distance_scale(1000.0, 200.0, 0.5), 1.0);
    }

    #[test]
    fn plates_fade_out_over_the_last_quarter_of_the_range() {
        assert_eq!(distance_fade(0.0, 4000.0), 1.0);
        assert_eq!(distance_fade(3000.0, 4000.0), 1.0);
        assert!((distance_fade(3500.0, 4000.0) - 0.5).abs() < 1e-5);
        assert_eq!(distance_fade(4000.0, 4000.0), 0.0);
        assert_eq!(distance_fade(9000.0, 4000.0), 0.0);
    }

    #[test]
    fn bars_fade_in_between_far_and_close() {
        assert_eq!(detail(1500.0, 1000.0), 0.0);
        assert_eq!(detail(1000.0, 1000.0), 0.0);
        assert!((detail(875.0, 1000.0) - 0.5).abs() < 1e-5);
        assert_eq!(detail(750.0, 1000.0), 1.0);
        assert_eq!(detail(10.0, 1000.0), 1.0);
        assert_eq!(detail(10.0, 0.0), 0.0);
    }

    #[test]
    fn head_height_unpacks_the_encoded_box() {
        // Standing player: x 15, mins z -24 (24), maxs z 40 -> 72.
        let standing = (72 << 16) | (24 << 8) | 15;
        assert_eq!(head_height(standing), 40.0);
        // Crouching: maxs z 16 -> 48.
        let crouching = (48 << 16) | (24 << 8) | 15;
        assert_eq!(head_height(crouching), 16.0);
        assert_eq!(head_height(0), 40.0);
    }

    #[test]
    fn a_hidden_pilot_is_anchored_over_his_vehicle() {
        let ship = ([100.0, 50.0, 400.0], 96 << 16);
        let body = ([1.0, 2.0, 3.0], 72 << 16);
        // Presented body wins, even inside a presented vehicle.
        let own = anchor(Some(body), Some(ship)).unwrap();
        assert_eq!((own.origin, own.head), (body.0, 40.0));
        // No body (EF_NODRAW): the vehicle's origin and box top.
        let hidden = anchor(None, Some(ship)).unwrap();
        assert_eq!((hidden.origin, hidden.head), (ship.0, 64.0));
        // A vehicle that sends no box falls back to the standing height.
        let bare = anchor(None, Some((ship.0, 0))).unwrap();
        assert_eq!(bare.head, 40.0);
        // Nothing presented, nothing to anchor to.
        assert_eq!(anchor(None, None), None);
    }

    #[test]
    fn a_vehicles_owner_below_max_clients_is_its_hidden_pilot() {
        let never = |_| false;
        assert_eq!(hidden_pilot(13, 53, 5, 0, never), Some(5));
        // Not a vehicle, an empty one (ENTITYNUM_NONE), an NPC pilot, or myself.
        assert_eq!(hidden_pilot(13, 20, 5, 0, never), None);
        assert_eq!(hidden_pilot(1, 53, 5, 0, never), None);
        assert_eq!(hidden_pilot(13, 53, 1023, 0, never), None);
        assert_eq!(hidden_pilot(13, 53, 40, 0, never), None);
        assert_eq!(hidden_pilot(13, 53, 5, 5, never), None);
        // A pilot whose own entity is in the snapshot is drawn from it.
        assert_eq!(hidden_pilot(13, 53, 5, 0, |pilot| pilot == 5), None);
    }

    #[test]
    fn the_overflow_layer_is_the_same_hue_stronger() {
        for color in [HEALTH_COLOR, SHIELD_COLOR] {
            let deep = saturated(color);
            let spread = |c: Color| c.r.max(c.g).max(c.b) - c.r.min(c.g).min(c.b);
            let strongest = |c: Color| {
                [c.r, c.g, c.b]
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(index, _)| index)
            };
            assert!(spread(deep) >= spread(color) - 0.2, "{deep:?}");
            assert_eq!(strongest(deep), strongest(color), "same hue");
            let sum = |c: Color| c.r + c.g + c.b;
            assert!(sum(deep) < sum(color), "darker");
        }
    }

    #[test]
    fn the_overflow_band_is_inside_the_bar_and_centred() {
        let bar = Rect::new(10.0, 20.0, 100.0, 10.0);
        let band = overflow_band(bar);
        assert!(band.y > bar.y && band.bottom() < bar.bottom());
        assert!((band.y + band.height * 0.5 - 25.0).abs() < 1e-4);
        assert_eq!((band.x, band.width), (bar.x, bar.width));
    }

    #[test]
    fn a_broken_bar_has_gaps_and_stays_inside() {
        let dashes: Vec<_> = broken_dashes().collect();
        assert!(dashes.len() > 3);
        for pair in dashes.windows(2) {
            assert!(pair[0].1 < pair[1].0, "a gap between dashes");
        }
        assert!(
            dashes
                .iter()
                .all(|(start, end)| 0.0 <= *start && *end <= 1.0)
        );
    }

    /// A capsule near enough for its tray: 18-pixel names (`cg_nameplateScale` 0.5 at
    /// 1080 lines), a picture, the seal and a saber.
    fn spec(rows: Rows) -> CapsuleSpec {
        CapsuleSpec {
            centre_x: 960.0,
            bottom: 500.0,
            size: 18.0,
            name_width: 60.0,
            picture: true,
            mark: true,
            end: End::Weapon { dot: true },
            rows,
            detail: 1.0,
            proximity: 1.0,
            least_bar: 2.0,
            viewport: [1920.0, 1080.0],
        }
    }

    const ALL: Rows = Rows {
        health: true,
        shield: true,
        force: true,
    };

    fn inside(inner: Rect, outer: Rect) -> bool {
        inner.x >= outer.x - 1e-3
            && inner.y >= outer.y - 1e-3
            && inner.right() <= outer.right() + 1e-3
            && inner.bottom() <= outer.bottom() + 1e-3
    }

    /// Left to right in the pill: the picture, the name, the seal, the stance's dot and
    /// the weapon, all centred on its middle; the tray under it holds the bars, shield
    /// on top, and the whole plate stands on its bottom.
    #[test]
    fn a_capsule_holds_its_parts_in_order_with_the_tray_under_it() {
        let capsule = Capsule::new(&spec(ALL));
        let pill = capsule.pill;
        assert!((pill.x + pill.width * 0.5 - 960.0).abs() < 1e-3, "centred");
        assert!((capsule.middle - (pill.y + pill.height * 0.5)).abs() < 1e-4);
        let picture = capsule.picture.unwrap();
        let mark = capsule.mark.unwrap();
        let dot = capsule.dot.unwrap();
        let weapon = capsule.weapon.unwrap();
        for part in [picture, mark, dot, weapon] {
            assert!(inside(part, pill), "{part:?} in {pill:?}");
            assert!((part.y + part.height * 0.5 - capsule.middle).abs() < 1e-3);
        }
        assert!(picture.right() < capsule.name.x);
        assert!(capsule.name.right() < mark.x);
        assert!(mark.right() < dot.x && dot.right() < weapon.x);
        assert!(capsule.badge.is_none());
        let tray = capsule.tray.unwrap();
        assert!((tray.frame.bottom() - 500.0).abs() < 1e-3, "on the bottom");
        assert!(tray.frame.y < pill.bottom(), "tucked under the pill");
        assert!(tray.frame.width <= pill.width);
        let (shield, health, force) = (
            tray.shield.unwrap(),
            tray.health.unwrap(),
            tray.force.unwrap(),
        );
        assert!(shield.bottom() < health.y && health.bottom() < force.y);
        for bar in [shield, health, force] {
            assert!(inside(bar, tray.frame), "{bar:?}");
            assert!(bar.y >= pill.bottom(), "under the pill");
        }
        assert!(health.height > shield.height, "health is the thickest");
    }

    /// No bars (sharing off, names only, a duel): no tray, and the pill stands on the
    /// bottom itself; the pill widens to hold a tray only as it fades in.
    #[test]
    fn without_bars_the_capsule_stands_alone() {
        let bare = Capsule::new(&spec(Rows::default()));
        assert!(bare.tray.is_none());
        assert!((bare.pill.bottom() - 500.0).abs() < 1e-3);
        let short = CapsuleSpec {
            name_width: 10.0,
            ..spec(ALL)
        };
        let hidden = Capsule::new(&CapsuleSpec {
            detail: 0.0,
            ..short
        });
        let shown = Capsule::new(&short);
        assert!((hidden.pill.bottom() - 500.0).abs() < 1e-3, "no room kept");
        assert!(shown.pill.width > hidden.pill.width, "widened for the tray");
        let tray = shown.tray.unwrap().frame;
        assert!(tray.x > shown.pill.x && tray.right() < shown.pill.right());
    }

    /// At 1080 lines with the default text size no bar is thinner than two pixels,
    /// even at the far edge of the bars' range, where the plate has shrunk.
    #[test]
    fn the_bars_are_never_under_two_pixels() {
        let shrunk = distance_scale(1000.0, 3000.0, 0.6);
        let capsule = Capsule::new(&CapsuleSpec {
            size: 36.0 * 0.5 * shrunk,
            ..spec(ALL)
        });
        let tray = capsule.tray.unwrap();
        for bar in [tray.shield, tray.health, tray.force] {
            assert!(bar.unwrap().height >= 2.0 - 1e-4, "{bar:?}");
        }
    }

    /// The duel badge takes the weapon's place at the right end; with no picture (an
    /// NPC) the name starts further in; far away the picture is smaller.
    #[test]
    fn the_right_end_and_the_picture_follow_the_plate() {
        let duel = Capsule::new(&CapsuleSpec {
            end: End::Duel,
            rows: Rows::default(),
            ..spec(ALL)
        });
        let badge = duel.badge.unwrap();
        assert!(duel.weapon.is_none() && duel.dot.is_none());
        assert!(inside(badge, duel.pill) && badge.x > duel.name.right());
        let npc = Capsule::new(&CapsuleSpec {
            picture: false,
            ..spec(ALL)
        });
        assert!(npc.picture.is_none());
        assert!(npc.name.x > npc.pill.x);
        let gun = Capsule::new(&CapsuleSpec {
            end: End::Weapon { dot: false },
            ..spec(ALL)
        });
        assert!(gun.dot.is_none() && gun.weapon.is_some());
        let near = Capsule::new(&spec(Rows::default()));
        let far = Capsule::new(&CapsuleSpec {
            proximity: 0.0,
            ..spec(Rows::default())
        });
        assert!(far.picture.unwrap().width < near.picture.unwrap().width);
        assert!(
            far.picture.unwrap().width > 18.0,
            "still taller than the name"
        );
        // Far, what stays (the picture to the mark) is centred, not the faded pill.
        let shown = (far.picture.unwrap().x + far.mark.unwrap().right()) * 0.5;
        assert!((shown - 960.0).abs() < 0.5, "{shown}");
    }

    /// A long name stops at its widest, and a plate at the screen's edge stays on it.
    #[test]
    fn capsules_stay_on_screen() {
        let long = Capsule::new(&CapsuleSpec {
            name_width: 5_000.0,
            ..spec(ALL)
        });
        assert!(long.name.width < 300.0);
        let left = Capsule::new(&CapsuleSpec {
            centre_x: 5.0,
            ..spec(ALL)
        });
        assert_eq!(left.pill.x, 0.0);
        let right = Capsule::new(&CapsuleSpec {
            centre_x: 1915.0,
            ..spec(ALL)
        });
        assert!((right.pill.right() - 1920.0).abs() < 1e-3);
        // The power icons centre over it and stand clear of it.
        let (chip, step) = right.chips(3, [1920.0, 1080.0]);
        assert!(chip.bottom() < right.pill.y && step > chip.width);
        assert!(chip.x + 2.0 * step + chip.width <= 1920.0 + 1e-3);
    }

    /// A team's tint is its colour made deep: the blue one is the design's, and red is as
    /// dark.
    #[test]
    fn team_tints_are_deep_and_alike() {
        let blue = team_fill(Color::new(0.3, 0.6, 1.0, 0.95));
        assert!((blue.r - 22.0 / 255.0).abs() < 0.02);
        assert!((blue.g - 52.0 / 255.0).abs() < 0.02);
        assert!((blue.b - 110.0 / 255.0).abs() < 0.05);
        let red = team_fill(Color::new(1.0, 0.3, 0.25, 0.95));
        assert!(red.r > red.g && red.r > red.b, "still red");
        let light = |c: Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
        assert!((light(red) - light(blue)).abs() < 0.06);
    }
}
