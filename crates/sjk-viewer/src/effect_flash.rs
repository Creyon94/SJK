//! An effect's `Flash` primitive (`CFlash`, OpenJK `codemp/client/FxPrimitives.cpp`):
//! a flash of the screen whose colour `CFlash::Init` scales by where the effect is from
//! the viewer when it starts. Full within a few units, fading with the square of the
//! distance to nothing at 900 units, and nothing beyond 60 degrees off the view axis
//! unless within 100 units. `demp2/altdetonate` has one (2048 units of
//! `gfx/effects/whiteFlash`): drawn at full colour wherever a DEMP2 shock went off, it
//! lit the whole screen from across the map (Slash, hub report #125).
//!
//! The flash is drawn as the effect's billboard where it went off, its colour scaled as
//! stock scales it. Stock draws it on the viewer instead (12 units in front, as wide as
//! the view), which a world billboard matches for a flash in front of the viewer; a flash
//! within 100 units behind the viewer, which stock still shows, stays out of sight here.

use glam::Vec3;

/// `CFlash::Init`'s `maxRange`.
const MAX_RANGE: f32 = 900.0;

/// `CFlash::Init`'s colour scale for a flash at `origin`, seen from `viewer`'s eye
/// looking along its unit `forward` axis; 0 draws nothing. No viewer yet: no flash.
pub(crate) fn visibility(viewer: Option<(Vec3, Vec3)>, origin: Vec3) -> f32 {
    let Some((eye, forward)) = viewer else {
        return 0.0;
    };
    let offset = origin - eye;
    let distance = offset.length();
    // `VectorNormalize` leaves a zero vector at zero length.
    let direction = if distance > 0.0 {
        offset / distance
    } else {
        Vec3::ZERO
    };
    let mut scale = direction.dot(forward);
    if distance > MAX_RANGE || (scale < 0.5 && distance > 100.0) {
        scale = 0.0;
    } else if scale < 0.5 && distance <= 100.0 {
        scale += 1.1;
    }
    scale * (1.0 - (distance * distance) / (MAX_RANGE * MAX_RANGE))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EYE: Vec3 = Vec3::new(10.0, 20.0, 64.0);

    fn at(forward: f32, side: f32) -> f32 {
        visibility(Some((EYE, Vec3::X)), EYE + Vec3::new(forward, side, 0.0))
    }

    #[test]
    fn a_flash_fades_with_distance_and_is_gone_far_away_or_off_the_view() {
        // Straight ahead: full close by, 1 - d^2 / 900^2 further.
        assert!((at(10.0, 0.0) - (1.0 - 100.0 / 810_000.0)).abs() < 1e-5);
        assert!((at(450.0, 0.0) - 0.75).abs() < 1e-5);
        // Across the map (Slash #125): nothing.
        assert_eq!(at(901.0, 0.0), 0.0);
        assert_eq!(at(3_000.0, 0.0), 0.0);
        // Off to the side past 60 degrees, or behind, beyond 100 units: nothing.
        assert_eq!(at(100.0, 200.0), 0.0);
        assert_eq!(at(-300.0, 0.0), 0.0);
        // Within 100 units off the view, stock brightens it instead.
        let near = at(0.0, 50.0);
        assert!((near - 1.1 * (1.0 - 2_500.0 / 810_000.0)).abs() < 1e-5);
        // No viewer, no flash.
        assert_eq!(visibility(None, EYE), 0.0);
    }
}
