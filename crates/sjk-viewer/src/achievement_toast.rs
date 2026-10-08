//! The achievement pop-up: when the player unlocks an achievement
//! (`achievements_frame.rs`), a small card slides in at the top centre of the screen
//! with the board's medallion (`achievements/medallion.rs`), "Achievement unlocked",
//! the achievement's name, category and what it asked, and plays the single-player
//! game's secret-area sound (`audio/ui_cues.rs`, off with `cg_achievementSound 0`).
//!
//! Unlike the medal pop-up (`medal_popup.rs`) it is not modal: it takes no input and
//! pauses nothing, over play as over the menus. It enters in [`ENTER`] seconds,
//! sliding down and growing to its size while a gold ring sweeps round the medallion,
//! light bursts from it (a glow, a ring of light, sparks) and a glint crosses the
//! card; it holds [`HOLD`] seconds and fades out in [`LEAVE`]. Several unlocks queue
//! and show one after another, each with its sound.
//!
//! The top centre is free in play: the HUDs' gauges sit in the bottom corners,
//! timers at the top right, notify lines at the top left, centre prints and the
//! crosshair lower (`version_overlay.rs`), so the card covers neither the aim nor the
//! chat. It waits while the console is open or the medal pop-up shows, and draws
//! nothing, nor allocates, while no unlock waits.

use crate::achievements::Kind;
use crate::achievements::medallion::{self, Medallion, tint};
use crate::audio::ui_cues::{self, Cue};
use crate::menu::sjk::{DISPLAY_CENTRE, color, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::text::{TextStyle, TextVertex, UiFont};
use sjk_ui::{Color, DrawCommand, FontWeight, Gradient, Rect, TextAlign};
use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::time::{Duration, Instant};

/// The cvar that plays the sound with each pop-up (1, the default) or not (0).
pub(crate) const SOUND_CVAR: &str = "cg_achievementSound";

/// Seconds the card takes to come in.
pub(crate) const ENTER: f32 = 0.45;
/// Seconds it then stays.
pub(crate) const HOLD: f32 = 5.0;
/// Seconds it takes to fade out.
pub(crate) const LEAVE: f32 = 0.65;
/// Seconds one pop-up lasts.
pub(crate) const LIFETIME: f32 = ENTER + HOLD + LEAVE;
/// Pause between two pop-ups.
const GAP: Duration = Duration::from_millis(300);

/// The card, in 1080-line pixels: its size, its top's distance from the screen's
/// top and its corners' radius.
const WIDTH: f32 = 620.0;
const HEIGHT: f32 = 116.0;
const TOP: f32 = 104.0;
const RADIUS: f32 = 18.0;
/// The medallion's middle from the card's left, and its radius.
const MEDAL_X: f32 = 64.0;
const MEDAL_RADIUS: f32 = 38.0;
/// Where the text column starts and the room it has.
const TEXT_X: f32 = 126.0;
pub(crate) const TEXT_WIDTH: f32 = WIDTH - TEXT_X - 24.0;
/// The name's and the description's type sizes.
pub(crate) const NAME_SIZE: f32 = 32.0;
pub(crate) const DESCRIPTION_SIZE: f32 = 16.0;
/// The line over the name.
const KICKER: &str = "ACHIEVEMENT UNLOCKED";

/// The sparks the burst throws: direction (degrees, 0 to the right, clockwise), how
/// far each flies (pixels at 1080 lines), when it leaves (seconds) and its size.
const SPARKS: [(f32, f32, f32, f32); 14] = [
    (-90.0, 74.0, 0.20, 3.4),
    (-62.0, 58.0, 0.26, 2.6),
    (-35.0, 88.0, 0.22, 3.0),
    (-8.0, 66.0, 0.30, 2.4),
    (18.0, 92.0, 0.21, 3.2),
    (44.0, 60.0, 0.27, 2.6),
    (70.0, 80.0, 0.24, 3.0),
    (96.0, 56.0, 0.31, 2.4),
    (124.0, 86.0, 0.20, 3.4),
    (150.0, 64.0, 0.28, 2.6),
    (178.0, 94.0, 0.23, 3.0),
    (205.0, 58.0, 0.29, 2.4),
    (232.0, 82.0, 0.22, 3.2),
    (258.0, 62.0, 0.27, 2.6),
];

/// Where a pop-up is in its life.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Phase {
    Entering,
    Shown,
    Leaving,
    Gone,
}

impl Phase {
    /// The phase `t` seconds after the pop-up began.
    pub(crate) fn at(t: f32) -> Self {
        if t < ENTER {
            Self::Entering
        } else if t < ENTER + HOLD {
            Self::Shown
        } else if t < LIFETIME {
            Self::Leaving
        } else {
            Self::Gone
        }
    }
}

/// How the card stands `t` seconds after it began: its opacity, how far it is above
/// its place (1080-line pixels) and its size against its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Moment {
    pub(crate) alpha: f32,
    pub(crate) lift: f32,
    pub(crate) scale: f32,
}

impl Moment {
    pub(crate) fn at(t: f32) -> Self {
        let enter = ease_out_cubic(span(t, 0.0, ENTER));
        let fade_in = ease_out_cubic(span(t, 0.0, ENTER * 0.7));
        let leave = span(t, ENTER + HOLD, LIFETIME);
        let leave_eased = leave * leave;
        Self {
            alpha: fade_in * (1.0 - leave_eased),
            lift: 30.0 * (1.0 - enter) + 18.0 * leave_eased,
            scale: (0.86 + 0.14 * ease_out_back(span(t, 0.0, ENTER))) * (1.0 - 0.04 * leave),
        }
    }
}

/// How far `t` is from `from` to `to`, from 0 to 1.
fn span(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}

fn ease_in_out_cubic(x: f32) -> f32 {
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) * 0.5
    }
}

/// Past 1 a little before settling, for a card that pops into place.
fn ease_out_back(x: f32) -> f32 {
    const C1: f32 = 1.4;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (x - 1.0).powi(3) + C1 * (x - 1.0).powi(2)
}

/// The achievement showing and since when.
#[derive(Clone, Copy)]
struct Showing {
    kind: &'static Kind,
    started: Instant,
}

/// The pop-up's state: the unlocks waiting, the one showing and its canvas.
pub(crate) struct AchievementToast {
    canvas: MenuCanvas,
    queue: VecDeque<&'static Kind>,
    current: Option<Showing>,
    /// When the last pop-up ended, for the pause before the next.
    ended: Option<Instant>,
    /// A moment held still (seconds after the start), for the off-screen shots.
    #[cfg(test)]
    held: Option<f32>,
}

impl Default for AchievementToast {
    fn default() -> Self {
        Self {
            // Four text runs and about seventy shapes a frame.
            canvas: MenuCanvas::with_capacities(8, 96, 128),
            queue: VecDeque::with_capacity(4),
            current: None,
            ended: None,
            #[cfg(test)]
            held: None,
        }
    }
}

impl AchievementToast {
    /// `kind` was unlocked: its pop-up waits for those before it.
    pub(crate) fn push(&mut self, kind: &'static Kind) {
        let showing = self
            .current
            .is_some_and(|current| current.kind.id == kind.id);
        if !showing && !self.queue.iter().any(|waiting| waiting.id == kind.id) {
            self.queue.push_back(kind);
        }
    }

    /// Unlocks waiting, the one showing included.
    pub(crate) fn pending(&self) -> usize {
        self.queue.len() + usize::from(self.current.is_some())
    }

    /// Seconds since the pop-up showing began.
    fn elapsed(&self, showing: Showing, now: Instant) -> f32 {
        #[cfg(test)]
        if let Some(held) = self.held {
            return held;
        }
        now.saturating_duration_since(showing.started).as_secs_f32()
    }

    /// Move the pop-ups on to `now`: end the one whose time is up, and start the next
    /// one waiting when it `may_show` (not under the console or the medal pop-up),
    /// with its `sound`. Returns whether a pop-up draws this frame.
    pub(crate) fn update(&mut self, now: Instant, may_show: bool, sound: bool) -> bool {
        if let Some(showing) = self.current
            && Phase::at(self.elapsed(showing, now)) == Phase::Gone
        {
            self.current = None;
            self.ended = Some(now);
        }
        if self.current.is_none() && may_show && !self.queue.is_empty() {
            let rested = self
                .ended
                .is_none_or(|ended| now.saturating_duration_since(ended) >= GAP);
            if rested && let Some(kind) = self.queue.pop_front() {
                self.current = Some(Showing { kind, started: now });
                if sound {
                    ui_cues::post(Cue::Achievement);
                }
            }
        }
        may_show && self.current.is_some()
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.canvas.draw_list()
    }

    /// Lay the pop-up showing out for `viewport` as it stands at `now`.
    pub(crate) fn build(&mut self, viewport: [f32; 2], now: Instant) {
        let Some(showing) = self.current else {
            return;
        };
        let t = self.elapsed(showing, now);
        draw(&mut self.canvas, showing.kind, t, viewport);
    }

    /// Append the pop-up's text: in the SJK UI's families when `fonts` has them, else
    /// in Inter (`font`).
    pub(crate) fn append_text(
        &self,
        fonts: Option<crate::game_font::SjkFonts<'_>>,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        match fonts {
            Some(fonts) => self
                .canvas
                .append_text_families(fonts, viewport, TextStyle::NEUTRAL),
            None => self
                .canvas
                .append_text_styled(vertices, font, viewport, TextStyle::NEUTRAL),
        }
    }
}

/// The card's place and size in the window `t` seconds after it began.
pub(crate) fn card_rect(viewport: [f32; 2], t: f32) -> Rect {
    let moment = Moment::at(t);
    let s = layout_scale(viewport);
    let k = s * moment.scale;
    let middle_y = (TOP + HEIGHT * 0.5 - moment.lift) * s;
    Rect::new(
        viewport[0] * 0.5 - WIDTH * k * 0.5,
        middle_y - HEIGHT * k * 0.5,
        WIDTH * k,
        HEIGHT * k,
    )
}

/// Window pixels per 1080-line pixel: the height's scale, smaller when the window is
/// too narrow for the card.
fn layout_scale(viewport: [f32; 2]) -> f32 {
    crate::ui_scale::height_scale(viewport[1]).min((viewport[0] - 32.0).max(1.0) / WIDTH)
}

/// Draw `kind`'s pop-up on `canvas` as it stands `t` seconds after it began.
fn draw(canvas: &mut MenuCanvas, kind: &'static Kind, t: f32, viewport: [f32; 2]) {
    let moment = Moment::at(t);
    let card = card_rect(viewport, t);
    let k = card.width / WIDTH;
    let at = |x: f32, y: f32| [card.x + x * k, card.y + y * k];
    let hue = tint(kind.category);
    canvas.begin_transparent(viewport);
    canvas.push_opacity(moment.alpha);
    let list = canvas.draw_list_mut();

    // A soft shadow under the card.
    for (grow, alpha) in [(14.0, 0.10), (6.0, 0.16)] {
        let _ = list.push(DrawCommand::RoundedRect {
            rect: grown(card, grow * k, 6.0 * k),
            radius: (RADIUS + grow) * k,
            color: Color::new(0.0, 0.0, 0.0, alpha),
        });
    }
    // A gold outline that leaves the card's edge as it lands, and fades.
    let pulse = span(t, 0.12, 1.0);
    if pulse > 0.0 && pulse < 1.0 {
        let grow = (4.0 + 26.0 * ease_out_cubic(pulse)) * k;
        let _ = list.push(DrawCommand::Border {
            rect: grown(card, grow, 0.0),
            radius: RADIUS * k + grow,
            width: (2.0 * (1.0 - pulse) + 0.5) * k,
            color: color::alpha(color::GOLD_BRIGHT, 0.6 * (1.0 - pulse).powi(3)),
        });
    }
    // The card: deep navy washed with the category's colour from the left.
    let _ = list.push(DrawCommand::RoundedRect {
        rect: card,
        radius: RADIUS * k,
        color: Color::new(0.05, 0.065, 0.125, 0.94),
    });
    let _ = list.push(DrawCommand::GradientRect {
        rect: card,
        radius: RADIUS * k,
        gradient: Gradient {
            start: color::alpha(hue, 0.13),
            end: color::alpha(hue, 0.0),
            vertical: false,
        },
    });
    // The border, lit as the card lands.
    let flash = 1.0 - span(t, 0.2, 1.2);
    let _ = list.push(DrawCommand::Border {
        rect: card,
        radius: RADIUS * k,
        width: 1.5 * k,
        color: color::alpha(color::GOLD, 0.55 + 0.4 * flash),
    });
    // A gold rule along the card's top, brightest at its middle.
    let rule = Rect::new(
        card.x + 40.0 * k,
        card.y,
        card.width * 0.5 - 40.0 * k,
        k.max(1.0),
    );
    for (half, start, end) in [
        (
            rule,
            color::alpha(color::GOLD_BRIGHT, 0.0),
            color::GOLD_BRIGHT,
        ),
        (
            Rect::new(rule.right(), rule.y, rule.width, rule.height),
            color::GOLD_BRIGHT,
            color::alpha(color::GOLD_BRIGHT, 0.0),
        ),
    ] {
        let _ = list.push(DrawCommand::GradientRect {
            rect: half,
            radius: 0.0,
            gradient: Gradient {
                start,
                end,
                vertical: false,
            },
        });
    }

    let centre = at(MEDAL_X, HEIGHT * 0.5);
    let medal = MEDAL_RADIUS * k;
    // The light behind the medallion: a burst as the card lands, then a steady glow.
    let burst = burst_envelope(t);
    for (factor, alpha) in [(2.1, 0.05), (1.65, 0.09), (1.3, 0.14)] {
        let radius = medal * (factor * (0.85 + 0.25 * burst));
        let _ = list.push(DrawCommand::RoundedRect {
            rect: Rect::new(
                centre[0] - radius,
                centre[1] - radius,
                radius * 2.0,
                radius * 2.0,
            ),
            radius,
            color: color::alpha(color::GOLD_BRIGHT, alpha * (0.45 + 1.4 * burst)),
        });
    }
    // A ring of light running out from the medallion.
    let wave = span(t, 0.18, 1.0);
    if wave > 0.0 && wave < 1.0 {
        let _ = list.push(DrawCommand::Arc {
            center: centre,
            radius: medal * (1.0 + 1.5 * ease_out_cubic(wave)),
            width: (5.0 * (1.0 - wave) + 1.0) * k,
            start: 0.0,
            sweep: TAU,
            color: color::alpha(color::GOLD_BRIGHT, 0.85 * (1.0 - wave).powf(1.5)),
            knockout: None,
        });
    }
    // Sparks thrown out from it, each a bright head and a fading tail.
    for (degrees, travel, delay, size) in SPARKS {
        let life = span(t, delay, delay + 0.8);
        if life <= 0.0 || life >= 1.0 {
            continue;
        }
        let (sin, cos) = degrees.to_radians().sin_cos();
        let distance = medal * 0.9 + travel * k * ease_out_cubic(life);
        let fade = (1.0 - life).powf(1.4);
        for (back, shrink, dim) in [(0.0, 1.0, 1.0), (7.0, 0.7, 0.55), (13.0, 0.45, 0.25)] {
            let reach = (distance - back * k * (1.0 - life)).max(medal * 0.9);
            let radius = size * shrink * k * (1.0 - 0.5 * life);
            let point = [centre[0] + cos * reach, centre[1] + sin * reach];
            let _ = list.push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    point[0] - radius,
                    point[1] - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                radius,
                color: color::alpha(
                    if back == 0.0 {
                        Color::new(1.0, 0.96, 0.85, 1.0)
                    } else {
                        color::GOLD_BRIGHT
                    },
                    fade * dim,
                ),
            });
        }
    }

    // The medallion, its gold ring sweeping round as the card comes in.
    let sweep = ease_in_out_cubic(span(t, 0.08, 0.7));
    medallion::draw(
        canvas,
        Medallion {
            kind,
            centre,
            radius: medal,
            fraction: sweep,
            done: true,
        },
    );
    let list = canvas.draw_list_mut();
    if sweep > 0.0 && sweep < 1.0 {
        // The sweep's bright head.
        let angle = -FRAC_PI_2 + TAU * sweep;
        let point = [
            centre[0] + medal * angle.cos(),
            centre[1] + medal * angle.sin(),
        ];
        for (radius, alpha) in [(10.0, 0.25), (4.5, 1.0)] {
            let radius = radius * k;
            let _ = list.push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    point[0] - radius,
                    point[1] - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                radius,
                color: Color::new(1.0, 0.97, 0.88, alpha),
            });
        }
    }
    // A flash over the medallion as the ring closes.
    let closed = span(t, 0.7, 1.05);
    if closed > 0.0 && closed < 1.0 {
        let radius = medal * (1.0 + 0.25 * closed);
        let _ = list.push(DrawCommand::RoundedRect {
            rect: Rect::new(
                centre[0] - radius,
                centre[1] - radius,
                radius * 2.0,
                radius * 2.0,
            ),
            radius,
            color: color::alpha(color::GOLD_BRIGHT, 0.45 * (1.0 - closed).powi(2)),
        });
    }
    // A glint crossing the card once the ring has closed.
    let glint = span(t, 0.55, 1.2);
    if glint > 0.0 && glint < 1.0 {
        let band = 70.0 * k;
        let x = card.x - band * 2.0 + (card.width + band * 2.0) * ease_in_out_cubic(glint);
        let strength = 0.16 * (PI * glint).sin();
        let _ = list.push(DrawCommand::PushClip(card));
        for (rect, start, end) in [
            (
                Rect::new(x, card.y, band, card.height),
                Color::new(1.0, 1.0, 1.0, 0.0),
                Color::new(1.0, 0.95, 0.85, strength),
            ),
            (
                Rect::new(x + band, card.y, band, card.height),
                Color::new(1.0, 0.95, 0.85, strength),
                Color::new(1.0, 1.0, 1.0, 0.0),
            ),
        ] {
            let _ = list.push(DrawCommand::GradientRect {
                rect,
                radius: 0.0,
                gradient: Gradient {
                    start,
                    end,
                    vertical: false,
                },
            });
        }
        let _ = list.push(DrawCommand::PopClip);
    }

    // The words: the kicker and the category, the name, what it asked.
    let column = |y: f32, height: f32| {
        let [x, y] = at(TEXT_X, y);
        Rect::new(x, y, TEXT_WIDTH * k, height * k)
    };
    spaced(
        canvas,
        format_args!("{KICKER}"),
        column(12.0, 24.0),
        15.0 * k,
        color::GOLD,
        2.6 * k,
    );
    text(
        canvas,
        TextFamily::Display,
        format_args!("{}", kind.category.name()),
        column(12.0, 24.0),
        15.0 * k,
        color::alpha(hue, 0.9),
        FontWeight::Regular,
        TextAlign::End,
    );
    // The name settles in a moment after the card.
    canvas.push_opacity(ease_out_cubic(span(t, 0.12, 0.5)));
    text(
        canvas,
        TextFamily::Display,
        format_args!("{}", kind.name),
        column(36.0, 40.0),
        NAME_SIZE * k,
        color::GOLD_BRIGHT,
        FontWeight::Semibold,
        TextAlign::Start,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!("{}", kind.description),
        column(78.0, 24.0),
        DESCRIPTION_SIZE * k,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    canvas.pop_opacity();
    canvas.pop_opacity();
    canvas.finish(0);
}

/// `rect` grown by `by` on every side and moved `down`.
fn grown(rect: Rect, by: f32, down: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by + down,
        rect.width + by * 2.0,
        rect.height + by * 2.0,
    )
}

/// The burst's strength `t` seconds in: up quickly as the card lands, then down
/// to nothing over a second.
fn burst_envelope(t: f32) -> f32 {
    let rise = span(t, 0.1, 0.32);
    let fall = span(t, 0.32, 1.5);
    rise * (1.0 - ease_out_cubic(fall))
}

/// Capitals in Rajdhani, spaced out, centred on `rect`'s middle line as
/// [`text`] centres its runs.
fn spaced(
    canvas: &mut MenuCanvas,
    value: std::fmt::Arguments<'_>,
    rect: Rect,
    size: f32,
    color: Color,
    spacing: f32,
) {
    let top = rect.y + rect.height * 0.5 - DISPLAY_CENTRE * size;
    canvas.set_family(TextFamily::Display);
    canvas.text_fmt_aligned(
        value,
        Rect::new(
            rect.x,
            top,
            rect.width,
            (size * 1.3).max(rect.bottom() - top),
        ),
        size,
        color,
        FontWeight::Semibold,
        spacing,
        TextAlign::Start,
    );
    canvas.set_family(TextFamily::Body);
}

#[cfg(test)]
impl AchievementToast {
    /// Show `kinds` one after another, the first held `at` seconds after it began,
    /// for the off-screen shots.
    pub(crate) fn preview(kinds: &[&'static Kind], at: f32) -> Self {
        let mut toast = Self::default();
        for kind in kinds {
            toast.push(kind);
        }
        toast.held = Some(at);
        let _ = toast.update(Instant::now(), true, false);
        toast
    }
}

impl crate::GpuState {
    /// Move the achievement pop-up on and lay it out over the frame unless
    /// `covered` (the console over the frame, the medal pop-up); returns whether it
    /// draws this frame.
    pub(crate) fn append_achievement_toast(&mut self, viewport: [f32; 2], covered: bool) -> bool {
        if self.achievement_toast.pending() == 0 {
            return false;
        }
        let console_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::is_open);
        let sound = self
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar(SOUND_CVAR))
            .unwrap_or(true);
        let now = Instant::now();
        if !self
            .achievement_toast
            .update(now, !covered && !console_open, sound)
        {
            return false;
        }
        self.achievement_toast.build(viewport, now);
        self.achievement_toast.append_text(
            self.game_fonts.sjk(),
            &mut self.text_vertices,
            &self.ui_font,
            viewport,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::achievements::{self, ALL};

    fn kind(id: &str) -> &'static Kind {
        achievements::find(id).expect("an achievement")
    }

    #[test]
    fn unlocks_queue_and_show_one_after_another_each_with_its_sound() {
        ui_cues::take_posted();
        let mut toast = AchievementToast::default();
        toast.push(kind("first_blood"));
        toast.push(kind("streak_5"));
        toast.push(kind("first_blood"));
        assert_eq!(toast.pending(), 2, "an unlock waiting is not queued twice");
        let start = Instant::now();
        assert!(toast.update(start, true, true));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
        toast.push(kind("first_blood"));
        assert_eq!(toast.pending(), 2, "nor the one showing");
        let after = |seconds: f32| start + Duration::from_secs_f32(seconds);
        for seconds in [0.1, 1.0, 3.0, LIFETIME - 0.01] {
            assert!(toast.update(after(seconds), true, true));
            assert_eq!(
                toast.current.map(|shown| shown.kind.id),
                Some("first_blood")
            );
        }
        assert!(ui_cues::take_posted().is_empty(), "one sound per pop-up");
        // Gone, and the next waits out the pause.
        assert!(!toast.update(after(LIFETIME), true, true));
        assert!(!toast.update(after(LIFETIME + 0.1), true, true));
        assert!(toast.update(after(LIFETIME + 0.35), true, true));
        assert_eq!(toast.current.map(|shown| shown.kind.id), Some("streak_5"));
        assert_eq!(ui_cues::take_posted(), [Cue::Achievement]);
        let end = LIFETIME * 2.0 + 0.4;
        assert!(!toast.update(after(end), true, true));
        assert_eq!(toast.pending(), 0);
    }

    #[test]
    fn a_covered_screen_holds_the_queue_and_the_sound_can_be_off() {
        ui_cues::take_posted();
        let mut toast = AchievementToast::default();
        toast.push(kind("maps_10"));
        let now = Instant::now();
        assert!(!toast.update(now, false, true));
        assert!(toast.current.is_none() && toast.pending() == 1);
        assert!(ui_cues::take_posted().is_empty());
        assert!(toast.update(now, true, false));
        assert!(ui_cues::take_posted().is_empty(), "cg_achievementSound 0");
    }

    #[test]
    fn nothing_is_drawn_while_idle() {
        let mut toast = AchievementToast::default();
        assert!(!toast.update(Instant::now(), true, true));
        toast.build([1920.0, 1080.0], Instant::now());
        assert!(toast.draw_list().is_empty());
    }

    #[test]
    fn the_card_comes_in_holds_and_leaves() {
        assert_eq!(Phase::at(0.0), Phase::Entering);
        assert_eq!(Phase::at(ENTER + 1.0), Phase::Shown);
        assert_eq!(Phase::at(ENTER + HOLD + 0.1), Phase::Leaving);
        assert_eq!(Phase::at(LIFETIME), Phase::Gone);
        let start = Moment::at(0.0);
        assert!(start.alpha == 0.0 && start.lift > 0.0 && start.scale < 1.0);
        // Past its size for a moment as it lands, then still.
        assert!(Moment::at(ENTER * 0.7).scale > 1.0);
        for t in [ENTER, ENTER + 2.0, ENTER + HOLD] {
            let held = Moment::at(t);
            assert!((held.alpha - 1.0).abs() < 1e-4, "{t}");
            assert!(held.lift.abs() < 1e-4 && (held.scale - 1.0).abs() < 1e-4);
        }
        let leaving = Moment::at(ENTER + HOLD + LEAVE * 0.5);
        assert!(leaving.alpha < 1.0 && leaving.alpha > 0.0);
        assert!(Moment::at(LIFETIME).alpha.abs() < 1e-4);
    }

    /// Every achievement, at every moment, fits the canvas and the screen at 1080
    /// lines, 4K, 4:3 and 21:9, its words its column in the families and in Inter.
    #[test]
    fn every_achievement_fits_the_canvas() {
        let load = |family| crate::text::load_family(family, 1.0, None).expect("a family");
        let display = load(&crate::text::DISPLAY);
        let body = load(&crate::text::BODY);
        let inter = crate::text::load_modern(1.0, None).expect("Inter");
        let width = |font: &UiFont, value: &str, size: f32, face, spacing| {
            crate::text::visible_text_width_style(font, value, size / font.height, face, spacing)
        };
        let semibold = crate::text::TextFace::Semibold;
        let regular = crate::text::TextFace::Regular;
        for (title, words) in [(&display.font, &body.font), (&inter.font, &inter.font)] {
            let kicker = width(title, KICKER, 15.0, semibold, 2.6);
            for kind in &ALL {
                let category = width(title, kind.category.name(), 15.0, regular, 0.0);
                assert!(kicker + 16.0 + category <= TEXT_WIDTH, "{}", kind.id);
                let name = width(title, kind.name, NAME_SIZE, semibold, 0.0);
                assert!(name <= TEXT_WIDTH, "{}: {name}", kind.name);
                let description = width(words, kind.description, DESCRIPTION_SIZE, regular, 0.0);
                assert!(
                    description <= TEXT_WIDTH,
                    "{}: {description}",
                    kind.description
                );
            }
        }
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
            [800.0, 600.0],
        ] {
            for kind in &ALL {
                for t in [0.05, 0.3, 0.6, 1.0, 3.0, ENTER + HOLD + 0.3] {
                    let mut toast = AchievementToast::preview(&[kind], t);
                    toast.build(viewport, Instant::now());
                    assert!(!toast.canvas.overflowed(), "{} {t} {viewport:?}", kind.id);
                    let card = card_rect(viewport, t);
                    assert!(
                        card.x >= 0.0 && card.right() <= viewport[0] && card.y >= 0.0,
                        "{viewport:?} {t}"
                    );
                    // Clear of the crosshair, in the screen's top third.
                    assert!(card.bottom() < viewport[1] / 3.0, "{viewport:?}");
                }
            }
        }
    }
}
