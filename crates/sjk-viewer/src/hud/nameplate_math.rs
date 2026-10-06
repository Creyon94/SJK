//! Nameplate maths: how large, how opaque and where a plate sits, and the
//! colours and rectangles of its bars. Pure functions, so the layout is
//! testable without a window; [`super::nameplate`] collects the players and
//! draws with these.
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

/// Health bar fill: green when whole, through yellow to red when low.
pub(super) fn health_color(ratio: f32) -> Color {
    let ratio = ratio.clamp(0.0, 1.0);
    let (r, g) = if ratio >= 0.5 {
        ((1.0 - ratio) * 2.0, 1.0)
    } else {
        (1.0, ratio * 2.0)
    };
    Color::new(0.15 + 0.85 * r, 0.15 + 0.8 * g, 0.1, 1.0)
}

/// Shield (armour) bar fill.
pub(super) const SHIELD_COLOR: Color = Color::new(0.35, 0.65, 1.0, 1.0);
/// Estimated Force bar fill; drawn thinner and see-through, and outlined.
pub(super) const FORCE_COLOR: Color = Color::new(0.7, 0.45, 1.0, 0.8);

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

/// The plate under a name: its frame and the bars inside, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Stack {
    pub(super) frame: Rect,
    pub(super) health: Option<Rect>,
    pub(super) shield: Option<Rect>,
    pub(super) force: Option<Rect>,
}

impl Stack {
    /// Bar heights and spacing in units of `u` pixels (the scaled pixel unit).
    const HEALTH: f32 = 7.0;
    const SHIELD: f32 = 5.0;
    const FORCE: f32 = 4.0;
    const GAP: f32 = 2.5;
    const PAD: f32 = 4.0;
    /// Plate width in units of `u`.
    pub(super) const WIDTH: f32 = 112.0;

    /// Total height of a stack holding `rows`, in pixels.
    pub(super) fn height(rows: Rows, u: f32) -> f32 {
        let bars = [
            (rows.health, Self::HEALTH),
            (rows.shield, Self::SHIELD),
            (rows.force, Self::FORCE),
        ];
        let heights: f32 = bars.iter().filter(|b| b.0).map(|b| b.1).sum();
        let count = bars.iter().filter(|b| b.0).count();
        if count == 0 {
            return 0.0;
        }
        (Self::PAD * 2.0 + heights + Self::GAP * (count - 1) as f32) * u
    }

    /// Lay a stack out centred on `centre_x` with its bottom edge on
    /// `bottom_y`, kept on screen.
    pub(super) fn new(
        centre_x: f32,
        bottom_y: f32,
        rows: Rows,
        u: f32,
        viewport: [f32; 2],
    ) -> Self {
        let width = Self::WIDTH * u;
        let height = Self::height(rows, u);
        let x = (centre_x - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
        let y = bottom_y - height;
        let inner_x = x + Self::PAD * u;
        let inner_width = width - Self::PAD * 2.0 * u;
        let mut cursor = y + Self::PAD * u;
        let mut bar = |wanted: bool, h: f32| {
            wanted.then(|| {
                let rect = Rect::new(inner_x, cursor, inner_width, h * u);
                cursor += (h + Self::GAP) * u;
                rect
            })
        };
        Self {
            frame: Rect::new(x, y, width, height),
            health: bar(rows.health, Self::HEALTH),
            shield: bar(rows.shield, Self::SHIELD),
            force: bar(rows.force, Self::FORCE),
        }
    }
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
    fn health_runs_from_green_through_yellow_to_red() {
        let full = health_color(1.0);
        let half = health_color(0.5);
        let low = health_color(0.0);
        assert!(full.g > full.r);
        assert!(half.r > 0.9 && half.g > 0.9);
        assert!(low.r > low.g);
    }

    #[test]
    fn stacks_hold_only_the_wanted_bars_inside_their_frame() {
        let viewport = [1920.0, 1080.0];
        let all = Rows {
            health: true,
            shield: true,
            force: true,
        };
        let stack = Stack::new(960.0, 500.0, all, 2.0, viewport);
        assert!((stack.frame.x + stack.frame.width * 0.5 - 960.0).abs() < 1e-3);
        assert!((stack.frame.y + stack.frame.height - 500.0).abs() < 1e-3);
        let (hp, shield, force) = (
            stack.health.unwrap(),
            stack.shield.unwrap(),
            stack.force.unwrap(),
        );
        assert!(hp.y > stack.frame.y && shield.y > hp.y + hp.height && force.y > shield.y);
        assert!(force.y + force.height < stack.frame.y + stack.frame.height);

        let only_force = Rows {
            force: true,
            ..Rows::default()
        };
        let slim = Stack::new(960.0, 500.0, only_force, 2.0, viewport);
        assert!(slim.health.is_none() && slim.shield.is_none() && slim.force.is_some());
        assert!(slim.frame.height < stack.frame.height);
        assert_eq!(Stack::height(Rows::default(), 2.0), 0.0);
    }

    #[test]
    fn stacks_stay_on_screen() {
        let viewport = [1920.0, 1080.0];
        let rows = Rows {
            health: true,
            ..Rows::default()
        };
        assert_eq!(Stack::new(5.0, 500.0, rows, 2.0, viewport).frame.x, 0.0);
        let right = Stack::new(1915.0, 500.0, rows, 2.0, viewport).frame;
        assert!((right.x + right.width - viewport[0]).abs() < 1e-3);
    }
}
