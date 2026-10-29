//! The lagometer (`cg_lagometer`) and JoF EJK's modes 2 and 3 of it.
//!
//! Mode 1 is the graph SJK has always drawn: frame interpolation (yellow above the
//! middle line, blue below) and snapshot latency (green, yellow when the server held
//! the snapshot back for rate, red when one was dropped) over a backdrop, where
//! `cg_lagometerX`, `cg_lagometerY` and `cg_lagometerScale` put it
//! ([`super::targeting::Policy::lagometer_rect`]).
//!
//! JoF EJK (`cg_draw.c`, `CG_DrawLagometer` and `CG_DrawSpeedGraph`) adds two modes:
//! 2 writes the average ping and interpolation of the graph's 48 columns along its top,
//! and draws a speed graph above it (56 of the graph's 48 units up) with the current
//! speed at its top left; 3 does the same without the backdrops, the speed graph three
//! times as tall, its number only while moving. EJK draws the speed graph only with
//! its `cg_speedometer` speed graph bit; SJK has no such bit and draws it with the
//! modes (Sol's decision, 29/10/2026). Everything here is in units of the graph's own
//! height over 48, EJK's size, so the numbers keep EJK's proportions at any
//! `cg_lagometerScale`.

use super::*;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, HudWidget, Rect, TextAlign, TextOverflow};
use std::fmt::Write as _;

/// The setting: 0 off, 1 the graph, 2 with numbers and the speed graph, 3 the same
/// without backdrops.
pub(crate) const CVAR: &str = "cg_lagometer";
/// The highest mode; a larger value draws it.
pub(crate) const MAX_MODE: u8 = 3;
/// The text of the average ping, the average interpolation and the speed.
pub(super) const PING_TEXT: TextId = TextId(325);
pub(super) const INTERPOLATION_TEXT: TextId = TextId(326);
pub(super) const SPEED_TEXT: TextId = TextId(327);
/// EJK's graph is 48 virtual units square, and its averages read its 48 columns (`aw`).
const UNITS: f32 = 48.0;
const AVERAGED: usize = 48;
/// Speed samples kept: as many as the widest graph draws columns.
const SPEED_SAMPLES: usize = 64;
/// `MAX_LAGOMETER_RANGE` and `MAX_LAGOMETER_PING`: the milliseconds the two graphs span.
const FRAME_RANGE: f32 = 300.0;
const PING_RANGE: f32 = 900.0;
/// `CG_DrawSpeedGraph`'s `vscale = range / 3000`: the speed the graph's height spans.
const SPEED_RANGE: f32 = 3_000.0;
/// Where the speed graph stands: 56 units above the lagometer, 48 tall; in mode 3,
/// 96 more up and three times as tall, so its bottom stays.
const SPEED_GAP: f32 = 56.0;
const TALL_SPEED_RISE: f32 = 96.0;
/// `CG_Text_Paint(..., 0.5f, ..., FONT_SMALL)`: `ocr_a`'s 21-unit line at half scale.
const TEXT_LINE: f32 = 21.0 * 0.5;
const YELLOW: Color = Color::new(1.0, 0.82, 0.20, 0.9);
const BLUE: Color = Color::new(0.25, 0.55, 1.0, 0.9);
const GREEN: Color = Color::new(0.28, 0.95, 0.48, 0.9);
const RED: Color = Color::new(1.0, 0.22, 0.18, 0.9);
const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

/// `cg_lagometer` as the HUD draws it, 0 to [`MAX_MODE`]; 0 when unset.
pub(crate) fn mode(console: Option<&ViewerConsole>) -> u8 {
    console
        .and_then(|console| console.integer_cvar(CVAR))
        .map_or(0, |mode| mode.clamp(0, i64::from(MAX_MODE)) as u8)
}

/// What modes 2 and 3 add: the numbers' retained text and the speed history.
pub(crate) struct Readout {
    pub(super) ping: String,
    pub(super) interpolation: String,
    pub(super) speed: String,
    /// The current speed is not 0, for mode 3's speed number.
    moving: bool,
    /// Horizontal speed, one sample a frame (`CG_AddSpeedGraphFrameInfo`).
    speeds: [f32; SPEED_SAMPLES],
    speed_count: u64,
}

impl Default for Readout {
    fn default() -> Self {
        Self {
            ping: String::with_capacity(16),
            interpolation: String::with_capacity(16),
            speed: String::with_capacity(16),
            moving: false,
            speeds: [0.0; SPEED_SAMPLES],
            speed_count: 0,
        }
    }
}

impl Readout {
    /// Record this frame's speed: `cg.currentSpeed`, the horizontal part of the
    /// (predicted) velocity, as the graph and EJK's number read it.
    pub(super) fn add_speed(&mut self, velocity: [f32; 3]) {
        let speed = velocity[0].hypot(velocity[1]);
        let speed = if speed.is_finite() { speed } else { 0.0 };
        self.speeds[self.speed_count as usize % SPEED_SAMPLES] = speed;
        self.speed_count += 1;
    }

    /// A speed sample, where offset zero is the newest; 0 before there was one.
    fn speed_newest(&self, offset: usize) -> f32 {
        if offset >= SPEED_SAMPLES || offset as u64 >= self.speed_count {
            return 0.0;
        }
        self.speeds[(self.speed_count - 1 - offset as u64) as usize % SPEED_SAMPLES]
    }

    /// Write the numbers `mode` shows from `samples` and the newest speed, into the
    /// retained text; modes 0 and 1 show none.
    pub(super) fn update(&mut self, samples: &sjk_client::LagometerSamples, mode: u8) {
        self.ping.clear();
        self.interpolation.clear();
        self.speed.clear();
        let speed = self.speed_newest(0);
        self.moving = speed != 0.0;
        if mode < 2 {
            return;
        }
        let (ping, interpolation) = averages(samples);
        let _ = write!(self.ping, "{ping:.0}");
        let _ = write!(self.interpolation, "{interpolation:04.1}");
        let _ = write!(self.speed, "{speed:.0}");
    }
}

/// `CG_DrawLagometer`'s two averages over the newest 48 columns: the ping, a dropped
/// snapshot counting as the highest ping seen so far in the walk (1 at least), and
/// the interpolation, the frames' offset from the latest snapshot negated (how far
/// behind it the view is drawn, in milliseconds). A column with no sample yet counts
/// as 0, as EJK's zeroed arrays do.
pub(super) fn averages(samples: &sjk_client::LagometerSamples) -> (f32, f32) {
    let mut interpolation = 0.0;
    for offset in 0..AVERAGED {
        interpolation += samples.frame_newest(offset) as f32;
    }
    let mut ping = 0.0;
    let mut highest = 1;
    for offset in 0..AVERAGED {
        let sample = samples.snapshot_newest(offset).ping;
        highest = highest.max(sample);
        if sample > 0 {
            ping += sample as f32;
        } else if sample < 0 {
            ping += highest as f32;
        }
    }
    (ping / AVERAGED as f32, -(interpolation / AVERAGED as f32))
}

/// What one frame's lagometer draws: the samples, the readout and the mode.
pub(super) struct Graph<'a> {
    pub(super) samples: &'a sjk_client::LagometerSamples,
    pub(super) readout: &'a Readout,
    pub(super) mode: u8,
    /// Columns drawn, whatever the graph's size.
    pub(super) columns: usize,
}

/// Draw the lagometer in `rect` (already placed and scaled), with what `graph.mode`
/// adds; `panel` draws a backdrop.
pub(super) fn emit(
    draw_list: &mut DrawList,
    rect: Rect,
    graph: &Graph<'_>,
    panel: &mut dyn FnMut(&mut DrawList, Rect),
) {
    let backdrop = graph.mode < 3;
    if backdrop {
        panel(draw_list, rect);
    }
    emit_lag(draw_list, rect, graph);
    if graph.mode < 2 {
        return;
    }
    let unit = rect.height / UNITS;
    // The averages along the graph's top: the ping from its left, the interpolation
    // ending at its right (`ay - 1`, `ax + 3`, `ax + aw - strW`).
    let line = TEXT_LINE * unit;
    let top = rect.y - unit;
    text(
        draw_list,
        Rect::new(rect.x + 2.0 * unit, top, rect.width, line),
        PING_TEXT,
        line,
        TextAlign::Start,
    );
    text(
        draw_list,
        Rect::new(rect.x, top, rect.width - unit, line),
        INTERPOLATION_TEXT,
        line,
        TextAlign::End,
    );
    let speed = speed_rect(rect, graph.mode);
    if backdrop {
        panel(draw_list, speed);
    }
    emit_speed(draw_list, speed, graph);
    if graph.mode == 2 || graph.readout.moving {
        text(
            draw_list,
            Rect::new(
                rect.x + 2.0 * unit,
                rect.y - SPEED_GAP * unit,
                rect.width,
                line,
            ),
            SPEED_TEXT,
            line,
            TextAlign::Start,
        );
    }
}

/// The speed graph's place above the lagometer in `rect` for `mode`.
pub(super) fn speed_rect(rect: Rect, mode: u8) -> Rect {
    let unit = rect.height / UNITS;
    if mode >= 3 {
        Rect::new(
            rect.x,
            rect.y - (SPEED_GAP + TALL_SPEED_RISE) * unit,
            rect.width,
            rect.height * 3.0,
        )
    } else {
        Rect::new(rect.x, rect.y - SPEED_GAP * unit, rect.width, rect.height)
    }
}

fn text(draw_list: &mut DrawList, rect: Rect, id: TextId, size: f32, align: TextAlign) {
    let _ = draw_list.push(DrawCommand::Text {
        rect,
        text: id,
        size,
        color: WHITE,
        align,
        overflow: TextOverflow::Clip,
        weight: FontWeight::Regular,
        letter_spacing: 0.0,
    });
}

/// The interpolation and latency bars: mode 1's graph, unchanged.
fn emit_lag(draw_list: &mut DrawList, rect: Rect, graph: &Graph<'_>) {
    let width = graph.columns;
    let bar_width = rect.width / width as f32;
    let frame_range = rect.height / 3.0;
    let middle = rect.y + frame_range;
    let ping_range = rect.height / 2.0;
    for offset in 0..width {
        let x = rect.right() - (offset + 1) as f32 * bar_width;
        let frame = graph.samples.frame_newest(offset) as f32;
        let height = (frame.abs() * frame_range / FRAME_RANGE).min(frame_range);
        if height > 0.0 {
            let (y, color) = if frame > 0.0 {
                (middle - height, YELLOW)
            } else {
                (middle, BLUE)
            };
            let _ = draw_list.push(DrawCommand::SolidRect {
                rect: Rect::new(x, y, bar_width, height),
                color,
            });
        }
        let sample = graph.samples.snapshot_newest(offset);
        let (height, color) = if sample.ping < 0 {
            (ping_range, RED)
        } else {
            let color = if sample.flags & 1 != 0 { YELLOW } else { GREEN };
            (
                (sample.ping as f32 * ping_range / PING_RANGE).min(ping_range),
                color,
            )
        };
        if height > 0.0 {
            let _ = draw_list.push(DrawCommand::SolidRect {
                rect: Rect::new(x, rect.bottom() - height, bar_width, height),
                color,
            });
        }
    }
}

/// `CG_DrawSpeedGraph`: a green column per frame from the bottom, the height's
/// worth 3000 units a second.
fn emit_speed(draw_list: &mut DrawList, rect: Rect, graph: &Graph<'_>) {
    let width = graph.columns;
    let bar_width = rect.width / width as f32;
    for offset in 0..width {
        let speed = graph.readout.speed_newest(offset);
        let height = (speed * rect.height / SPEED_RANGE).min(rect.height);
        if height > 0.0 {
            let _ = draw_list.push(DrawCommand::SolidRect {
                rect: Rect::new(
                    rect.right() - (offset + 1) as f32 * bar_width,
                    rect.bottom() - height,
                    bar_width,
                    height,
                ),
                color: GREEN,
            });
        }
    }
}

/// The backdrop the HUD layout gives the lagometer's widget.
pub(super) fn panel<'a>(
    theme: Theme,
    widget: &'a HudWidget,
) -> impl FnMut(&mut DrawList, Rect) + 'a {
    move |draw_list, rect| {
        super::widgets::emit_panel(
            draw_list,
            theme,
            widget,
            rect,
            widget.style.radius.unwrap_or(theme.radii.sm),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> sjk_client::LagometerSamples {
        let mut samples = sjk_client::LagometerSamples::new();
        for sample in 0..64 {
            samples.add_frame(-20 - sample % 10);
            samples.add_snapshot(60 + sample % 5, 0);
        }
        samples
    }

    fn drawn(mode: u8, readout: &Readout) -> Vec<DrawCommand> {
        let samples = samples();
        let mut draw_list = DrawList::new(512);
        let graph = Graph {
            samples: &samples,
            readout,
            mode,
            columns: 48,
        };
        emit(
            &mut draw_list,
            Rect::new(500.0, 300.0, 96.0, 96.0),
            &graph,
            &mut |draw_list, rect| {
                let _ = draw_list.push(DrawCommand::RoundedRect {
                    rect,
                    radius: 0.0,
                    color: Color::new(0.0, 0.0, 0.0, 0.5),
                });
            },
        );
        draw_list.commands().to_vec()
    }

    fn panels(commands: &[DrawCommand]) -> Vec<Rect> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    fn texts(commands: &[DrawCommand]) -> Vec<TextId> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(*text),
                _ => None,
            })
            .collect()
    }

    fn moving(speed: f32, mode: u8) -> Readout {
        let mut readout = Readout::default();
        for _ in 0..64 {
            readout.add_speed([speed * 0.6, speed * 0.8, 250.0]);
        }
        readout.update(&samples(), mode);
        readout
    }

    #[test]
    fn the_mode_reads_clamped_and_old_configs_keep_the_graph() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(mode(None), 0);
        assert_eq!(mode(Some(&console)), 0, "off by default");
        // `seta cg_lagometer "1"`, as old SJK and EJK configs save it.
        for (value, expected) in [("1", 1), ("2", 2), ("3", 3), ("9", 3), ("-1", 0), ("0", 0)] {
            assert!(console.set_cvar(CVAR, value), "{value}");
            assert_eq!(mode(Some(&console)), expected, "{value}");
        }
    }

    /// Mode 1 draws what SJK always drew: one backdrop and the bars, no text, nothing
    /// above the graph.
    #[test]
    fn mode_1_is_the_graph_alone() {
        let readout = moving(300.0, 1);
        let commands = drawn(1, &readout);
        assert_eq!(panels(&commands), [Rect::new(500.0, 300.0, 96.0, 96.0)]);
        assert!(texts(&commands).is_empty());
        assert!(readout.ping.is_empty() && readout.speed.is_empty());
        let solids = commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::SolidRect { .. }))
            .collect::<Vec<_>>();
        // A frame and a latency bar per column, all inside the graph.
        assert_eq!(solids.len(), 96);
        for command in solids {
            let DrawCommand::SolidRect { rect, .. } = command else {
                unreachable!()
            };
            assert!(rect.y >= 300.0 && rect.bottom() <= 396.0, "{rect:?}");
        }
    }

    #[test]
    fn mode_2_adds_the_numbers_and_the_speed_graph_on_backdrops() {
        let readout = moving(300.0, 2);
        let commands = drawn(2, &readout);
        // The graph's backdrop and the speed graph's, 56 units (of 96 / 48) above.
        assert_eq!(
            panels(&commands),
            [
                Rect::new(500.0, 300.0, 96.0, 96.0),
                Rect::new(500.0, 300.0 - 112.0, 96.0, 96.0)
            ]
        );
        assert_eq!(
            texts(&commands),
            [PING_TEXT, INTERPOLATION_TEXT, SPEED_TEXT]
        );
        // 300 a second fills a tenth of the speed graph's height.
        let speed_bars: Vec<Rect> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::SolidRect { rect, .. } if rect.bottom() <= 300.0 - 16.0 => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(speed_bars.len(), 48);
        assert!(speed_bars.iter().all(|bar| (bar.height - 9.6).abs() < 1e-3));
        assert_eq!(readout.speed, "300", "horizontal speed only");
    }

    #[test]
    fn mode_3_has_no_backdrop_a_tall_speed_graph_and_a_number_only_while_moving() {
        let readout = moving(300.0, 3);
        let commands = drawn(3, &readout);
        assert!(panels(&commands).is_empty());
        assert_eq!(
            texts(&commands),
            [PING_TEXT, INTERPOLATION_TEXT, SPEED_TEXT]
        );
        let tall = speed_rect(Rect::new(500.0, 300.0, 96.0, 96.0), 3);
        assert_eq!(tall, Rect::new(500.0, 300.0 - 304.0, 96.0, 288.0));
        // Its bottom is mode 2's.
        let short = speed_rect(Rect::new(500.0, 300.0, 96.0, 96.0), 2);
        assert!((tall.bottom() - short.bottom()).abs() < 1e-3);
        let still = moving(0.0, 3);
        assert_eq!(texts(&drawn(3, &still)), [PING_TEXT, INTERPOLATION_TEXT]);
        assert_eq!(texts(&drawn(2, &moving(0.0, 2))).len(), 3, "mode 2 shows 0");
    }

    /// EJK's averages: the 48 newest columns, a dropped snapshot as the highest ping
    /// seen so far, the interpolation negated, written `%.0f` and `%04.1f`.
    #[test]
    fn the_averages_follow_ejk() {
        let mut samples = sjk_client::LagometerSamples::new();
        for _ in 0..47 {
            samples.add_frame(-30);
            samples.add_snapshot(50, 0);
        }
        samples.add_frame(-6);
        samples.add_dropped();
        // The dropped one is the newest: walked first, it counts as 1.
        let (ping, interpolation) = averages(&samples);
        assert!((ping - (47.0 * 50.0 + 1.0) / 48.0).abs() < 1e-3, "{ping}");
        assert!((interpolation - (47.0 * 30.0 + 6.0) / 48.0).abs() < 1e-3);
        samples.add_snapshot(80, 0);
        // Now 80 is seen before it.
        let (ping, _) = averages(&samples);
        assert!(
            (ping - (46.0 * 50.0 + 80.0 + 80.0) / 48.0).abs() < 1e-3,
            "{ping}"
        );
        // Fewer samples than columns count as 0, as EJK's zeroed arrays.
        let mut few = sjk_client::LagometerSamples::new();
        few.add_frame(-48);
        few.add_snapshot(96, 0);
        assert_eq!(averages(&few), (2.0, 1.0));
        let mut readout = Readout::default();
        readout.update(&few, 2);
        assert_eq!(
            (readout.ping.as_str(), readout.interpolation.as_str()),
            ("2", "01.0")
        );
        readout.update(&few, 1);
        assert!(readout.ping.is_empty() && readout.interpolation.is_empty());
    }
}
