//! The words in the world over the mystery boxes (and the box hunt's mark): what a box is
//! and costs, that it rolls, then what came out, for every SJK player who sees it. Drawn
//! as the drone's pilot names are: projected over the point, hidden behind walls and far
//! away.

use super::{Here, Opening, ROLL};
use crate::GpuState;
use glam::Vec3;
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};
use std::time::Instant;

/// How high over a box's base its words stand (over what floats out of it).
const RISE: f32 = 70.0;
/// Farthest a label is read from, and where it starts to fade.
const RANGE: f32 = 1_400.0;
const FADE_FROM: f32 = 1_000.0;
/// Most labels a frame: the boxes in view and the mark.
const MOST: usize = 16;

/// How a label's headline is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tone {
    /// A box waiting, or rolling.
    Box,
    /// What came out paid.
    Win,
    /// The jackpot.
    Jackpot,
    /// Nothing came out.
    Empty,
    /// The box hunt's mark.
    Mark,
}

impl Tone {
    fn colour(self, alpha: f32) -> Color {
        match self {
            Self::Box => Color::new(1.0, 0.84, 0.42, alpha),
            Self::Win => Color::new(0.55, 0.95, 0.6, alpha),
            Self::Jackpot => Color::new(1.0, 0.45, 0.3, alpha),
            Self::Empty => Color::new(0.72, 0.72, 0.72, alpha),
            Self::Mark => Color::new(1.0, 0.3, 0.25, alpha),
        }
    }
}

/// One label: two lines over a point.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Label {
    pub(super) headline: String,
    pub(super) line: String,
    pub(super) tone: Tone,
}

/// This frame's labels and their draw list.
pub(crate) struct Labels {
    pub(crate) list: DrawList,
    lines: Vec<String>,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            list: DrawList::new(MOST * 4),
            lines: Vec::new(),
        }
    }
}

/// What a box's label says: its name and price while it waits, that it rolls, then what
/// came out (the player's own winnings, or what that outcome pays by `pays`).
pub(super) fn box_label(
    here: &Here,
    price: u64,
    free: bool,
    pays: impl Fn(&str) -> Option<u64>,
    now: Instant,
) -> Label {
    let Some(opening) = &here.opening else {
        return Label {
            headline: "MYSTERY BOX".to_owned(),
            line: if free {
                "Free roll: press Use".to_owned()
            } else {
                format!("{price} credits")
            },
            tone: Tone::Box,
        };
    };
    if now.saturating_duration_since(opening.started) < ROLL {
        return Label {
            headline: "MYSTERY BOX".to_owned(),
            line: "Rolling...".to_owned(),
            tone: Tone::Box,
        };
    }
    came_out(opening, pays)
}

fn came_out(opening: &Opening, pays: impl Fn(&str) -> Option<u64>) -> Label {
    let won = opening
        .mine
        .as_ref()
        .map(|prize| prize.won)
        .or_else(|| pays(&opening.outcome))
        .unwrap_or(0);
    let (headline, tone) = match opening.outcome.as_str() {
        "jackpot" => ("JACKPOT!", Tone::Jackpot),
        "double" => ("DOUBLE!", Tone::Win),
        "refund" => ("CREDITS BACK", Tone::Win),
        "empty" => ("EMPTY", Tone::Empty),
        _ if won > 0 => ("PRIZE", Tone::Win),
        _ => ("EMPTY", Tone::Empty),
    };
    let mut line = if won > 0 {
        format!("+{won} credits")
    } else {
        "Nothing inside".to_owned()
    };
    if opening.mine.is_some() {
        line.push_str(" - press Use to take it");
    }
    Label {
        headline: headline.to_owned(),
        line,
        tone,
    }
}

impl GpuState {
    /// Lay this frame's labels out over the view and add their words to the HUD's text;
    /// nothing while `shown` is false (menus, console, scoreboard, intermission).
    pub(crate) fn append_mystery_box_labels(&mut self, viewport: [f32; 2], shown: bool) {
        self.mystery_boxes.labels.list.clear();
        self.mystery_boxes.labels.lines.clear();
        let Some(camera) = self.frame_view.filter(|_| shown) else {
            return;
        };
        let now = Instant::now();
        let boxes = &self.mystery_boxes;
        let pays = |outcome: &str| boxes.pays(outcome);
        let mut wanted: Vec<(Vec3, Vec3, Label)> = boxes
            .here
            .iter()
            .map(|here| {
                (
                    here.origin + Vec3::Z * RISE,
                    here.origin + Vec3::Z * (super::HEIGHT + 4.0),
                    box_label(here, boxes.price, boxes.free > 0, pays, now),
                )
            })
            .collect();
        wanted.extend(self.box_hunt_mark_label());
        let unit = crate::ui_scale::height_scale(viewport[1]);
        let width = (420.0 * unit).min(viewport[0]);
        for (point, sight, label) in wanted.into_iter().take(MOST) {
            let distance = point.distance(camera.eye);
            if distance > RANGE {
                continue;
            }
            let Some(at) = camera.project(point) else {
                continue;
            };
            let trace = self.bsp.trace_box_with(
                &mut self.trace_scratch,
                camera.eye.to_array(),
                sight.to_array(),
                sjk_bsp::Aabb::new([0.0; 3], [0.0; 3]).unwrap(),
                1,
            );
            if trace.fraction < 1.0 || trace.start_solid || trace.all_solid {
                continue;
            }
            let alpha = (1.0 - (distance - FADE_FROM) / (RANGE - FADE_FROM)).clamp(0.0, 1.0);
            let near = (1.0 - distance / RANGE).clamp(0.4, 1.0);
            let head = 26.0 * unit * near;
            let small = 17.0 * unit * near;
            let x = (at[0] - width * 0.5).clamp(0.0, (viewport[0] - width).max(0.0));
            let labels = &mut self.mystery_boxes.labels;
            let id = labels.lines.len() as u32;
            labels.lines.push(label.headline);
            labels.lines.push(label.line);
            let rows = [
                (
                    at[1] - head * 1.3 - small * 1.4,
                    head,
                    id,
                    label.tone.colour(alpha),
                ),
                (
                    at[1] - small * 1.4,
                    small,
                    id + 1,
                    Color::new(1.0, 1.0, 1.0, alpha),
                ),
            ];
            for (y, size, text, colour) in rows {
                // A dark shadow under each line so it reads on any wall.
                for (offset, color) in [
                    (1.5 * unit, Color::new(0.0, 0.0, 0.0, 0.75 * alpha)),
                    (0.0, colour),
                ] {
                    let _ = labels.list.push(DrawCommand::Text {
                        rect: Rect::new(x + offset, y + offset, width, size * 1.3),
                        text: TextId(text),
                        size,
                        color,
                        align: TextAlign::Center,
                        overflow: TextOverflow::Ellipsis,
                        weight: FontWeight::Semibold,
                        letter_spacing: 0.0,
                    });
                }
            }
        }
        if self.mystery_boxes.labels.list.commands().is_empty() {
            return;
        }
        let labels = &self.mystery_boxes.labels;
        crate::ui_renderer::append_text_commands(
            &labels.list,
            |id| labels.lines.get(id.0 as usize).map_or("", String::as_str),
            &mut self.text_vertices,
            &self.ui_font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Prize, prize};
    use super::*;
    use std::time::Duration;

    fn here(opening: Option<Opening>) -> Here {
        Here {
            id: 1,
            origin: Vec3::ZERO,
            yaw: 0.0,
            opening,
            lid: Default::default(),
        }
    }

    fn opening(outcome: &str, mine: Option<Prize>, started: Instant) -> Opening {
        Opening {
            started,
            outcome: outcome.to_owned(),
            prize: prize(outcome, true),
            seed: 0,
            mine,
            revealed: false,
        }
    }

    #[test]
    fn a_label_tells_the_box_the_roll_and_what_came_out() {
        let now = Instant::now();
        let pays = |outcome: &str| (outcome == "jackpot").then_some(500);
        let waiting = box_label(&here(None), 100, false, pays, now);
        assert_eq!(
            (waiting.headline.as_str(), waiting.line.as_str()),
            ("MYSTERY BOX", "100 credits")
        );
        assert_eq!(
            box_label(&here(None), 100, true, pays, now).line,
            "Free roll: press Use"
        );
        let rolling = here(Some(opening("jackpot", None, now)));
        assert_eq!(
            box_label(&rolling, 100, false, pays, now).line,
            "Rolling..."
        );
        // Another player's jackpot: what that outcome pays.
        let later = now + ROLL + Duration::from_millis(10);
        let theirs = box_label(&rolling, 100, false, pays, later);
        assert_eq!(
            (theirs.headline.as_str(), theirs.line.as_str()),
            ("JACKPOT!", "+500 credits")
        );
        assert_eq!(theirs.tone, Tone::Jackpot);
        // The player's own: their winnings, and how to take them.
        let mine = Prize {
            price: 100,
            won: 200,
            balance: 300,
        };
        let own = here(Some(opening("double", Some(mine), now)));
        assert_eq!(
            box_label(&own, 100, false, pays, later).line,
            "+200 credits - press Use to take it"
        );
        let empty = here(Some(opening("empty", None, now)));
        let empty = box_label(&empty, 100, false, pays, later);
        assert_eq!(
            (empty.headline.as_str(), empty.tone),
            ("EMPTY", Tone::Empty)
        );
    }
}
