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

    /// Where the radial layout puts its pieces for a full-health player holding the
    /// blaster, with the weapon name still showing.
    struct Placed {
        center: [f32; 2],
        /// Centre line of the outer and inner rings.
        radii: [f32; 2],
        /// Stroke thickness of the track and the fill.
        stroke: f32,
        texts: Vec<(u32, Rect)>,
        pills: Vec<Rect>,
    }

    impl Placed {
        fn new(viewport: [f32; 2], user_scale: f32) -> Self {
            use crate::hud::{HudLook, HudOverlay, HudVisibility};
            let font = crate::text::load_modern(1.0, None).unwrap().font;
            let mut hud = HudOverlay::new();
            hud.preview_values(
                sjk_client::HudDataSource {
                    health: 100,
                    armor: 100,
                    force: 100,
                    weapon: 5,
                    ammo: Some(300),
                    saber_style: None,
                },
                [1.0; 4],
            );
            let visibility = HudVisibility {
                hud: true,
                status: true,
                weapon: true,
                crosshair: true,
                crosshair_names: false,
                timer: false,
                lagometer: false,
                team_overlay: false,
                ground_hud: false,
                menu_hud: false,
            };
            let _ = hud.layout(&font, HudLook::Radial, viewport, user_scale, visibility, 0);
            let mut placed = Self {
                center: [f32::NAN; 2],
                radii: [0.0, f32::MAX],
                stroke: f32::MAX,
                texts: Vec::new(),
                pills: Vec::new(),
            };
            for command in hud.draw_list().commands() {
                match *command {
                    DrawCommand::Arc {
                        center,
                        radius,
                        width,
                        ..
                    } => {
                        // Every arc is a stroke of one ring, so they share one centre
                        // (up to the rounding of rectangles of different sizes).
                        assert!(
                            placed.center[0].is_nan()
                                || (placed.center[0] - center[0]).abs() < 0.01
                                    && (placed.center[1] - center[1]).abs() < 0.01,
                            "{:?} against {center:?}",
                            placed.center
                        );
                        placed.center = center;
                        placed.radii[0] = placed.radii[0].max(radius);
                        placed.radii[1] = placed.radii[1].min(radius);
                        placed.stroke = placed.stroke.min(width);
                    }
                    DrawCommand::Text { rect, text, .. } => placed.texts.push((text.0, rect)),
                    DrawCommand::RoundedRect { rect, radius, .. } if radius > 0.0 => {
                        placed.pills.push(rect)
                    }
                    _ => {}
                }
            }
            placed
        }

        fn text(&self, id: u32) -> Rect {
            self.texts.iter().find(|(text, _)| *text == id).unwrap().1
        }

        /// Distance from the ring centre to the outer edge of the outer ring.
        fn outer_edge(&self) -> f32 {
            self.radii[0] + self.stroke * 0.5
        }
    }

    const SCREENS: [[f32; 2]; 9] = [
        [640.0, 360.0],
        [1280.0, 720.0],
        [1280.0, 1024.0],
        [1440.0, 1080.0],
        [1920.0, 1080.0],
        [1920.0, 1200.0],
        [2560.0, 1440.0],
        [3440.0, 1440.0],
        [3840.0, 2160.0],
    ];

    #[test]
    fn the_rings_keep_one_spot_of_the_screen_at_every_size_and_hud_scale() {
        // 19 % of the height below the crosshair, where TheRisqe's bars sit (their middle
        // is 92.6 of 480 lines below the centre). The 8K screen is past the UI scale's
        // upper clamp and the 360-line one under its lower one.
        let mut screens = SCREENS.to_vec();
        screens.push([7680.0, 4320.0]);
        for viewport in screens {
            for user_scale in [0.5, 1.0, 1.5] {
                let placed = Placed::new(viewport, user_scale);
                let [x, y] = placed.center;
                assert!(
                    (x - viewport[0] * 0.5).abs() < 0.5 && (y - viewport[1] * 0.69).abs() < 0.5,
                    "{viewport:?} x{user_scale}: centre {:?}",
                    placed.center
                );
            }
        }
    }

    #[test]
    fn the_numbers_and_pills_sit_beside_the_bars_at_their_middle() {
        for viewport in SCREENS {
            let placed = Placed::new(viewport, 1.0);
            let [x, y] = placed.center;
            let edge = placed.outer_edge();
            // Health and armor left of the bars, Force and ammunition right of them,
            // each on the line through the middle of the bars.
            for (id, left) in [(6, true), (8, true), (10, false), (14, false)] {
                let rect = placed.text(id);
                assert!(
                    (rect.y + rect.height * 0.5 - y).abs() < 0.5,
                    "{viewport:?} text {id}: {rect:?}"
                );
                if left {
                    assert!(rect.right() < x - edge, "{viewport:?} text {id}: {rect:?}");
                } else {
                    assert!(rect.x > x + edge, "{viewport:?} text {id}: {rect:?}");
                }
            }
            assert_eq!(placed.pills.len(), 2, "{viewport:?}");
            for pill in &placed.pills {
                assert!((pill.y + pill.height * 0.5 - y).abs() < 0.5);
                assert!(
                    pill.right() < x - edge || pill.x > x + edge,
                    "{viewport:?} pill {pill:?}"
                );
                assert!(
                    pill.x >= 0.0 && pill.right() <= viewport[0] && pill.bottom() <= viewport[1],
                    "{viewport:?} pill {pill:?} leaves the screen"
                );
            }
            // The weapon name rests between the bars, centred on the crosshair's column.
            let name = placed.text(12);
            let inner = placed.radii[1] - placed.stroke * 0.5;
            assert!((name.x + name.width * 0.5 - x).abs() < 0.5);
            assert!(
                name.x > x - inner && name.right() < x + inner,
                "{viewport:?}"
            );
        }
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
