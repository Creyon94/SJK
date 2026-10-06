//! The radial status HUD: ratio meters drawn as arcs around the crosshair.
//!
//! A layout document ([`assets/hud/radial.json`](../../assets/hud/radial.json)) places
//! [`HudWidgetKind::Arc`] widgets; this module turns each into smooth stroked arcs.
//! The strokes are anti-aliased in the shader, so they stay clean at any resolution.

use super::widgets::EmitContext;
use crate::menu_hud::frame::{AMMO_INDEX, AMMO_MAX};
use sjk_ui::{Color, DrawCommand, DrawList, HudDataSource, HudWidget, Rect, Theme, arc_segments};

/// How full the current weapon's ammunition is, `0..=1`; zero for weapons without any.
///
/// `ammoData[].max` (`bg_weapons.c`) doubles with the Double Ammo (`EF_DOUBLE_AMMO`) rune.
pub(super) fn ammo_ratio(weapon: u8, ammo: Option<i32>, double_ammo: bool) -> f32 {
    let Some(ammo) = ammo else { return 0.0 };
    let index = AMMO_INDEX.get(usize::from(weapon)).copied().unwrap_or(0);
    let maximum = AMMO_MAX[index] * if double_ammo { 2 } else { 1 };
    if maximum <= 0 {
        return 0.0;
    }
    (ammo as f32 / maximum as f32).clamp(0.0, 1.0)
}

/// Stroke geometry shared by the track, the fill and the glow of one arc widget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Ring {
    pub(super) center: [f32; 2],
    pub(super) radius: f32,
    pub(super) width: f32,
}

impl Ring {
    /// The circle inscribed in `rect`, stroked `width` pixels wide inside it.
    pub(super) fn inscribed(rect: Rect, width: f32) -> Self {
        let width = width.max(1.0);
        Self {
            center: [rect.x + rect.width * 0.5, rect.y + rect.height * 0.5],
            radius: (rect.width.min(rect.height) * 0.5 - width * 0.5).max(0.0),
            width,
        }
    }

    /// Angle that keeps a round cap inside its segment.
    pub(super) fn cap_inset(self) -> f32 {
        if self.radius > 0.0 {
            self.width * 0.5 / self.radius
        } else {
            0.0
        }
    }
}

/// Alpha of the soft halo drawn behind the filled part of an arc.
const GLOW_ALPHA: f32 = 0.16;
/// How much wider than the stroke the halo is.
const GLOW_WIDTH: f32 = 2.4;

pub(super) fn emit(
    draw_list: &mut DrawList,
    theme: Theme,
    widget: &HudWidget,
    rect: Rect,
    context: &EmitContext<'_>,
) {
    let Some(style) = widget.style.arc else {
        return;
    };
    let binding = widget.binding.as_deref().unwrap_or_default();
    let ratio = context.data.number(binding).unwrap_or(0.0);
    let ring = Ring::inscribed(rect, style.width * context.dpi_scale);
    let mut fill = widget.style.foreground.unwrap_or(theme.foreground);
    if binding == "health_ratio" && context.low_health {
        fill = theme.critical;
        fill.a *= context.pulse;
    }
    let track = widget
        .style
        .background
        .unwrap_or(Color::new(1.0, 1.0, 1.0, 0.16));
    let stroke = |draw_list: &mut DrawList, start, sweep, width, color| {
        let _ = draw_list.push(DrawCommand::Arc {
            center: ring.center,
            radius: ring.radius,
            width,
            start,
            sweep,
            color,
        });
    };
    for segment in arc_segments(&style, ratio, ring.cap_inset()) {
        stroke(draw_list, segment.start, segment.sweep, ring.width, track);
        if segment.amount > 0.0 {
            let mut glow = fill;
            glow.a *= GLOW_ALPHA;
            stroke(
                draw_list,
                segment.fill_start,
                segment.fill_sweep,
                ring.width * GLOW_WIDTH,
                glow,
            );
            stroke(
                draw_list,
                segment.fill_start,
                segment.fill_sweep,
                ring.width,
                fill,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ammo_fills_against_the_weapons_pool_maximum() {
        // The blaster (weapon 5) draws from the 300-round pool.
        assert!((ammo_ratio(5, Some(150), false) - 0.5).abs() < 1e-6);
        assert!((ammo_ratio(5, Some(150), true) - 0.25).abs() < 1e-6);
        assert_eq!(ammo_ratio(5, Some(900), false), 1.0);
        assert_eq!(ammo_ratio(5, Some(-1), false), 0.0);
    }

    #[test]
    fn no_ammo_or_no_pool_is_empty() {
        assert_eq!(ammo_ratio(5, None, false), 0.0);
        // The saber and melee draw no ammunition.
        assert_eq!(ammo_ratio(3, Some(50), false), 0.0);
        assert_eq!(ammo_ratio(200, Some(50), false), 0.0);
    }

    #[test]
    fn the_ring_sits_inside_its_rect_with_caps_inside_the_ends() {
        let ring = Ring::inscribed(Rect::new(10.0, 20.0, 200.0, 200.0), 8.0);
        assert_eq!(ring.center, [110.0, 120.0]);
        assert_eq!(ring.radius, 96.0);
        assert!((ring.cap_inset() - 4.0 / 96.0).abs() < 1e-6);
        // A degenerate rect cannot divide by zero.
        let flat = Ring::inscribed(Rect::new(0.0, 0.0, 0.0, 0.0), 8.0);
        assert_eq!(flat.radius, 0.0);
        assert_eq!(flat.cap_inset(), 0.0);
    }
}
