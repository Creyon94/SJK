//! Overhead nametag maths: how large, how opaque and where a tag sits, and the
//! colours of its plate and health bar. Pure functions, so the layout is
//! testable without a window; [`super::identification`] collects the players and
//! draws with these.
use sjk_ui::Color;

/// Distance, in world units, out to which a tag keeps its full size.
const FULL_SIZE_DISTANCE: f32 = 300.0;
/// Share of the range, from the camera, over which a tag is fully opaque.
const OPAQUE_SHARE: f32 = 0.75;
/// Units above the top of the player's box at which the tag is anchored.
pub(super) const HEAD_CLEARANCE: f32 = 8.0;
/// Standing box top above the origin, used when the entity sends none.
const STANDING_TOP: f32 = 40.0;
/// Milliseconds a tag takes to fade between hidden and shown.
pub(super) const FADE_MILLIS: f32 = 120.0;

/// Size multiplier for a tag `distance` away: full size up close, shrinking
/// linearly to `minimum` at `range`.
pub(super) fn distance_scale(distance: f32, range: f32, minimum: f32) -> f32 {
    if range <= FULL_SIZE_DISTANCE {
        return 1.0;
    }
    let t = ((distance - FULL_SIZE_DISTANCE) / (range - FULL_SIZE_DISTANCE)).clamp(0.0, 1.0);
    1.0 + (minimum.clamp(0.0, 1.0) - 1.0) * t
}

/// Opacity for a tag `distance` away: opaque for the first part of the range,
/// then fading to nothing at `range`.
pub(super) fn distance_fade(distance: f32, range: f32) -> f32 {
    let start = range * OPAQUE_SHARE;
    if distance <= start || range <= start {
        return 1.0;
    }
    (1.0 - (distance - start) / (range - start)).clamp(0.0, 1.0)
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

/// One tag's plate, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Plate {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    /// Horizontal inset of the name and the bar from the plate's edge.
    pub(super) inset: f32,
    /// Height of the name line and of the health bar (zero without one).
    pub(super) line: f32,
    pub(super) bar: f32,
    /// Vertical padding, and the gap between name line and bar.
    pad: f32,
    gap: f32,
}

impl Plate {
    /// Place a plate for text `text_width` wide at line height `size`, with its
    /// bottom edge on `anchor_y` and centred on `anchor_x`, kept on screen.
    pub(super) fn new(
        anchor: [f32; 2],
        text_width: f32,
        size: f32,
        with_bar: bool,
        viewport: [f32; 2],
    ) -> Self {
        let inset = size * 0.5;
        let pad = size * 0.18;
        let line = size * 1.15;
        let bar = if with_bar { size * 0.3 } else { 0.0 };
        let gap = if with_bar { size * 0.12 } else { 0.0 };
        let width = (text_width + inset * 2.0)
            .max(size * 3.0)
            .min(viewport[0] * 0.5);
        let height = pad * 2.0 + line + gap + bar;
        Self {
            x: (anchor[0] - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0)),
            y: anchor[1] - height,
            width,
            height,
            inset,
            line,
            bar,
            pad,
            gap,
        }
    }

    /// Top of the name line.
    pub(super) fn text_y(&self) -> f32 {
        self.y + self.pad
    }

    /// Top of the health bar.
    pub(super) fn bar_y(&self) -> f32 {
        self.y + self.pad + self.line + self.gap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_shrink_with_distance_to_the_minimum() {
        assert_eq!(distance_scale(100.0, 3000.0, 0.5), 1.0);
        assert_eq!(distance_scale(300.0, 3000.0, 0.5), 1.0);
        assert!((distance_scale(1650.0, 3000.0, 0.5) - 0.75).abs() < 1e-5);
        assert!((distance_scale(3000.0, 3000.0, 0.5) - 0.5).abs() < 1e-5);
        assert!((distance_scale(9000.0, 3000.0, 0.5) - 0.5).abs() < 1e-5);
        assert_eq!(distance_scale(1000.0, 200.0, 0.5), 1.0);
    }

    #[test]
    fn tags_fade_out_over_the_last_quarter_of_the_range() {
        assert_eq!(distance_fade(0.0, 4000.0), 1.0);
        assert_eq!(distance_fade(3000.0, 4000.0), 1.0);
        assert!((distance_fade(3500.0, 4000.0) - 0.5).abs() < 1e-5);
        assert_eq!(distance_fade(4000.0, 4000.0), 0.0);
        assert_eq!(distance_fade(9000.0, 4000.0), 0.0);
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
    fn plates_stay_on_screen_and_sit_above_the_anchor() {
        let viewport = [1920.0, 1080.0];
        let plate = Plate::new([960.0, 500.0], 120.0, 20.0, true, viewport);
        assert!((plate.x + plate.width * 0.5 - 960.0).abs() < 1e-3);
        assert!((plate.y + plate.height - 500.0).abs() < 1e-3);
        assert!(plate.text_y() >= plate.y);
        assert!(plate.bar_y() > plate.text_y());
        assert!((plate.bar_y() + plate.bar + plate.pad - (plate.y + plate.height)).abs() < 1e-3);

        let edge = Plate::new([5.0, 500.0], 120.0, 20.0, false, viewport);
        assert_eq!(edge.x, 0.0);
        let far = Plate::new([1915.0, 500.0], 120.0, 20.0, false, viewport);
        assert!((far.x + far.width - viewport[0]).abs() < 1e-3);
    }
}
