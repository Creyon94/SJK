//! `CG_DrawJetpackFuel` (JoF EJK `cg_draw.c:9897-9940`): a tall bar at the right edge
//! shows the jetpack's fuel (a black frame, the fuel left in dark red and the fuel
//! burnt greyed out above it).
//!
//! Stock draws it while the tank is not full (`cg_draw.c:11310-11313`); SJK draws it
//! too while the player holds a jetpack (`HI_JETPACK` in `STAT_HOLDABLE_ITEMS`), so a
//! full tank shows before the first burn.
use sjk_protocol::PlayerState;
use sjk_ui::{Color, DrawCommand, DrawList, Rect};

/// `JPFUELBAR_W`, `JPFUELBAR_H`, in the 640 by 480 screen.
const WIDTH: f32 = 20.0;
const HEIGHT: f32 = 100.0;
/// Units between the screen's right edge and the bar's (`JPFUELBAR_X`).
const RIGHT: f32 = WIDTH + 8.0;
/// `JPFUELBAR_Y`.
const TOP: f32 = 260.0;
const FRAME: Color = Color::new(0.0, 0.0, 0.0, 1.0);
const FUEL: Color = Color::new(0.5, 0.0, 0.0, 0.8);
const BURNT: Color = Color::new(0.5, 0.5, 0.5, 0.1);
/// `STAT_HOLDABLE_ITEMS`, `HI_JETPACK`.
const STAT_HOLDABLE_ITEMS: usize = 2;
const HI_JETPACK: u32 = 7;
/// `playerState_t::jetpackFuel`.
const PS_JETPACK_FUEL: usize = 39;

/// The fuel to show for `player`, `None` when the bar is not drawn: a full tank
/// without a jetpack held.
pub(super) fn shown(player: &PlayerState) -> Option<i32> {
    let fuel = player.raw_field(PS_JETPACK_FUEL)? as u8 as i32;
    let held = player.stats[STAT_HOLDABLE_ITEMS] & (1 << HI_JETPACK) != 0;
    (fuel < 100 || held).then_some(fuel)
}

/// Draw the bar for `fuel` (see [`shown`]) in a `viewport` of physical pixels.
///
/// Units follow the 480-line height, so the bar keeps its shape on wide screens as
/// `widthRatioCoef` keeps it, its right edge 8 units in from the screen's.
pub(super) fn emit(list: &mut DrawList, viewport: [f32; 2], fuel: Option<i32>) {
    let Some(fuel) = fuel else {
        return;
    };
    let filled = fuel as f32 / 100.0 * HEIGHT;
    if filled > HEIGHT {
        return;
    }
    let filled = filled.max(0.1);
    let unit = viewport[1] / 480.0;
    let x = viewport[0] - RIGHT * unit;
    let y = TOP * unit;
    let mut rect = |rx: f32, ry: f32, w: f32, h: f32, color: Color| {
        let _ = list.push(DrawCommand::SolidRect {
            rect: Rect::new(x + rx * unit, y + ry * unit, w * unit, h * unit),
            color,
        });
    };
    // `CG_DrawRect(x, y, JPFUELBAR_W, JPFUELBAR_H, 1.0f, black)`: a one-unit frame.
    rect(0.0, 0.0, WIDTH, 1.0, FRAME);
    rect(0.0, HEIGHT - 1.0, WIDTH, 1.0, FRAME);
    rect(0.0, 0.0, 1.0, HEIGHT, FRAME);
    rect(WIDTH - 1.0, 0.0, 1.0, HEIGHT, FRAME);
    // An empty tank leaves `filled` at 0.1, under the frame's unit: no fuel rect.
    if filled > 1.0 {
        rect(1.0, 1.0 + HEIGHT - filled, WIDTH - 1.0, filled - 1.0, FUEL);
    }
    rect(1.0, 1.0, WIDTH - 1.0, HEIGHT - filled, BURNT);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(fuel: Option<i32>) -> DrawList {
        let mut list = DrawList::new(8);
        emit(&mut list, [1920.0, 1080.0], fuel);
        list
    }

    fn rects(list: &DrawList) -> Vec<(Rect, Color)> {
        list.commands()
            .iter()
            .map(|command| match command {
                DrawCommand::SolidRect { rect, color } => (*rect, *color),
                other => panic!("{other:?}"),
            })
            .collect()
    }

    fn player(fuel: u32, holdables: u32) -> PlayerState {
        let mut player = PlayerState::default();
        player.set_raw_field(PS_JETPACK_FUEL, fuel);
        player.stats[STAT_HOLDABLE_ITEMS] = holdables;
        player
    }

    #[test]
    fn the_bar_shows_while_burning_or_while_a_jetpack_is_held() {
        assert_eq!(shown(&player(100, 0)), None);
        assert_eq!(shown(&player(40, 0)), Some(40));
        assert_eq!(shown(&player(100, 1 << HI_JETPACK)), Some(100));
        assert_eq!(shown(&player(0, 1 << HI_JETPACK)), Some(0));
    }

    #[test]
    fn the_bar_stands_at_the_right_edge_and_fills_from_the_bottom() {
        let rects = rects(&draw(Some(50)));
        assert_eq!(rects.len(), 6);
        // 2.25 pixels a unit: 20 units wide, 8 in from the right, 260 down.
        let frame = rects[0].0;
        assert_eq!((frame.x, frame.y, frame.width), (1857.0, 585.0, 45.0));
        let (fuel, color) = rects[4];
        assert_eq!(color, FUEL);
        assert_eq!((fuel.y, fuel.height), (585.0 + 51.0 * 2.25, 49.0 * 2.25));
        let (burnt, color) = rects[5];
        assert_eq!(color, BURNT);
        assert_eq!((burnt.y, burnt.height), (585.0 + 2.25, 50.0 * 2.25));
    }

    #[test]
    fn nothing_is_drawn_without_the_bar_or_over_a_full_tank() {
        assert!(draw(None).is_empty());
        assert!(draw(Some(101)).is_empty());
        assert!(!draw(Some(100)).is_empty());
        assert!(!draw(Some(0)).is_empty());
    }

    #[test]
    fn an_empty_tank_draws_no_fuel_and_no_rect_of_negative_height() {
        let rects = rects(&draw(Some(0)));
        assert_eq!(rects.len(), 5);
        assert!(rects.iter().all(|(rect, _)| rect.height >= 0.0));
        assert!(rects.iter().all(|(_, color)| *color != FUEL));
    }
}
