//! The Collection's Nameplates tab: ornaments for the nameplate every player sees over
//! the player's head (`docs/client.md`, Nameplates). None exist yet, so the tab shows
//! where they will go: three empty places on the left (a crest before the name, a
//! frame round the plate, a trail under it) and, over the player's model beside the
//! page, their nameplate as a match draws it near (the capsule with their picture, and
//! the tray of bars while they share), with dashed marks at those places.
//! Under the words beside it, See and share nameplates switches `cg_nameplate`, the one
//! setting for seeing other players' nameplates and sharing the player's own health,
//! shield and Force ([`crate::hud::nameplate::see_and_share`]).

use super::view::{LEFT_X, MODEL_AREA, RIGHT_X, glow};
use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};
use std::f32::consts::{PI, TAU};

/// The empty places: their top, size and the gap between them.
const PLACES_TOP: f32 = 300.0;
const PLACE: f32 = 240.0;
const PLACE_GAP: f32 = 30.0;
/// The nameplate's capsule, at twice its design's size (its name 26 pixels): the
/// name's size, the pill's height, the picture and its inset, the gap either side of
/// the crest's place and that place, the room after the name, and the tray's width;
/// and the bars' colours (the Radial HUD's shield, health and Force, as the nameplates
/// take them).
const NAME_SIZE: f32 = 26.0;
const PILL: f32 = 52.0;
const PICTURE: f32 = 40.0;
const PICTURE_INSET: f32 = 6.0;
const PLATE_GAP: f32 = 12.0;
const CREST: f32 = 28.0;
const PLATE_END: f32 = 30.0;
const TRAY_WIDTH: f32 = 208.0;
const SHIELD: Color = Color::new(0.25, 0.82, 0.5, 1.0);
const HEALTH: Color = Color::new(0.9, 0.28, 0.3, 1.0);
const FORCE: Color = Color::new(0.29, 0.66, 1.0, 1.0);

/// The words beside the model: their card's left edge, width and foot, and the room
/// inside it; a line of the description holds this many characters at the ordinary
/// text size, a line of the other words this many.
const WORDS_X: f32 = 1_360.0;
const WORDS_WIDTH: f32 = RIGHT_X - WORDS_X;
const WORDS_BOTTOM: f32 = 990.0;
const WORDS_PAD: f32 = 26.0;
const ABOUT_CHARS: f32 = 44.0;
const NOTE_CHARS: f32 = 50.0;
const ABOUT: &str = "What every SJK player sees over your head in a match: your picture and your name, and while you share, your shield, health and Force under them.";
const ORNAMENTS: &str =
    "Ornaments will go before your name, round your plate and under it: the dashed marks.";
/// The See and share nameplates row's width, from its name to the switch's right edge:
/// the card's inside, less the ring round the row.
const SHARE_ROW: f32 = WORDS_WIDTH - WORDS_PAD * 2.0 - 28.0;
/// How long one pulse of the See and share nameplates row takes.
const SHARE_PULSE_SECONDS: f32 = 1.2;

/// How lit the See and share nameplates row is `seconds` into the page, 0 to 1 and back each
/// [`SHARE_PULSE_SECONDS`], eased at both ends.
fn share_pulse(seconds: f32) -> f32 {
    0.5 - 0.5 * (seconds * TAU / SHARE_PULSE_SECONDS).cos()
}

/// `name` without its colour codes (`^1`).
fn plain(name: &str) -> String {
    let mut plain = String::with_capacity(name.len());
    let mut characters = name.chars();
    while let Some(character) = characters.next() {
        if character == '^' {
            let _ = characters.next();
        } else {
            plain.push(character);
        }
    }
    plain
}

/// A dashed outline round `rect` (frame pixels).
fn dashed_rect(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y, width, height]: [f32; 4],
    colour: Color,
) {
    const DASH: f32 = 9.0;
    const STEP: f32 = 16.0;
    let mut along = 0.0;
    while along < width {
        let length = DASH.min(width - along);
        for top in [y, y + height - 1.2] {
            let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
                rect: frame.rect(x + along, top, length, 1.2),
                color: colour,
            });
        }
        along += STEP;
    }
    let mut down = 0.0;
    while down < height {
        let length = DASH.min(height - down);
        for left in [x, x + width - 1.2] {
            let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
                rect: frame.rect(left, y + down, 1.2, length),
                color: colour,
            });
        }
        down += STEP;
    }
}

/// A dashed arc round `centre` (frame pixels) from `start` over `sweep` radians.
fn dashed_arc(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    centre: [f32; 2],
    radius: f32,
    (start, sweep): (f32, f32),
    colour: Color,
) {
    let dashes = ((radius * sweep.abs()) / 14.0).ceil().max(2.0) as usize;
    let each = sweep / dashes as f32;
    for dash in 0..dashes {
        let _ = canvas.draw_list_mut().push(DrawCommand::Arc {
            center: frame.point(centre[0], centre[1]),
            radius: radius * frame.s,
            width: 1.5 * frame.s,
            start: start + each * dash as f32,
            sweep: each * 0.55,
            color: colour,
            knockout: None,
        });
    }
}

impl Panel {
    /// The Nameplates tab.
    pub(super) fn nameplates(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        self.lead(
            frame,
            "No nameplates yet",
            "Ornaments for your nameplate, the name every player sees over your head.",
        );
        let places = [
            ("Crest", "Before your name"),
            ("Frame", "Round your plate"),
            ("Trail", "Under your plate"),
        ];
        for (index, (name, place)) in places.into_iter().enumerate() {
            let x = LEFT_X + index as f32 * (PLACE + PLACE_GAP);
            self.empty_place(frame, index, x, name, place);
        }
        super::toys::room_for_more(
            &mut self.ui,
            frame,
            716.0,
            "Nothing to collect here yet: the first ones come later.",
        );
        let at = self.plate_spot(frame);
        self.nameplate(frame, inputs, at);
        self.nameplate_words(frame, inputs);
    }

    /// Empty place `index` from `x`: a dashed niche with a drawing of where its kind
    /// goes, its name and place under it.
    fn empty_place(&mut self, frame: &Frame, index: usize, x: f32, name: &str, place: &str) {
        let s = frame.s;
        let line = color::alpha(color::HOLO, 0.55);
        let faint = color::alpha(color::HOLO, 0.28);
        dashed_rect(&mut self.ui, frame, [x, PLACES_TOP, PLACE, PLACE], faint);
        let middle = [x + PLACE * 0.5, PLACES_TOP + PLACE * 0.5];
        let bar = |canvas: &mut crate::menu_widgets::MenuCanvas, rect: [f32; 4]| {
            let [bx, by, bw, bh] = rect;
            let _ = canvas.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(bx, by, bw, bh),
                radius: 3.0 * s,
                width: 1.5 * s,
                color: faint,
            });
        };
        let outline = |canvas: &mut crate::menu_widgets::MenuCanvas,
                       rect: [f32; 4],
                       radius: f32,
                       colour: Color| {
            let [bx, by, bw, bh] = rect;
            let _ = canvas.draw_list_mut().push(DrawCommand::Border {
                rect: frame.rect(bx, by, bw, bh),
                radius: radius * s,
                width: 1.5 * s,
                color: colour,
            });
        };
        match index {
            // A crest: a ring with a mark inside, between a capsule's picture and its
            // name.
            0 => {
                let left = middle[0] - 90.0;
                outline(
                    &mut self.ui,
                    [left, middle[1] - 22.0, 180.0, 44.0],
                    22.0,
                    faint,
                );
                let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                    center: frame.point(left + 22.0, middle[1]),
                    radius: 15.0 * s,
                    width: 1.5 * s,
                    start: 0.0,
                    sweep: TAU,
                    color: faint,
                    knockout: None,
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                    center: frame.point(left + 58.0, middle[1]),
                    radius: 12.0 * s,
                    width: 2.0 * s,
                    start: 0.0,
                    sweep: TAU,
                    color: line,
                    knockout: None,
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: frame.rect(left + 54.0, middle[1] - 4.0, 8.0, 8.0),
                    radius: 2.0 * s,
                    color: line,
                });
                bar(&mut self.ui, [left + 80.0, middle[1] - 7.0, 82.0, 14.0]);
            }
            // A frame round the capsule and the tray of bars under it, with corner
            // marks.
            1 => {
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect: frame.rect(middle[0] - 72.0, middle[1] - 40.0, 144.0, 80.0),
                    radius: 10.0 * s,
                    width: 2.0 * s,
                    color: line,
                });
                for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                        rect: frame.rect(
                            middle[0] + dx * 80.0 - 3.0,
                            middle[1] + dy * 48.0 - 3.0,
                            6.0,
                            6.0,
                        ),
                        radius: 3.0 * s,
                        color: line,
                    });
                }
                outline(
                    &mut self.ui,
                    [middle[0] - 60.0, middle[1] - 30.0, 120.0, 30.0],
                    15.0,
                    faint,
                );
                outline(
                    &mut self.ui,
                    [middle[0] - 46.0, middle[1], 92.0, 24.0],
                    6.0,
                    faint,
                );
                for row in 0..3 {
                    let width = if row == 2 { 56.0 } else { 76.0 };
                    bar(
                        &mut self.ui,
                        [
                            middle[0] - 38.0,
                            middle[1] + 4.0 + row as f32 * 6.0,
                            width,
                            3.0,
                        ],
                    );
                }
            }
            // A trail sweeping under the capsule.
            _ => {
                outline(
                    &mut self.ui,
                    [middle[0] - 56.0, middle[1] - 50.0, 112.0, 28.0],
                    14.0,
                    faint,
                );
                for (radius, alpha) in [(64.0, 0.55), (48.0, 0.4)] {
                    let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                        center: frame.point(middle[0], middle[1] - 40.0),
                        radius: radius * s,
                        width: 2.0 * s,
                        start: PI * 0.25,
                        sweep: PI * 0.5,
                        color: color::alpha(color::HOLO, alpha),
                        knockout: None,
                    });
                }
            }
        }
        let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: frame.rect(x, PLACES_TOP + PLACE + 16.0, PLACE, 2.0),
            color: color::alpha(color::HOLO, 0.22),
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{name}"),
            frame.rect(x, PLACES_TOP + PLACE + 30.0, PLACE, 30.0),
            22.0 * s,
            color::MUTED,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{place}"),
            frame.rect(x, PLACES_TOP + PLACE + 62.0, PLACE, 22.0),
            15.0 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::Center,
        );
    }

    /// Where the nameplate stands, in frame pixels: over the model's head on the
    /// stage (the renderer says where), over the preview's, or high in the column.
    fn plate_spot(&self, frame: &Frame) -> [f32; 2] {
        match self.backstage {
            Backstage::World { head: Some(head) } => [
                (head[0] - frame.origin[0]) / frame.s,
                (head[1] - frame.origin[1]) / frame.s,
            ],
            Backstage::World { head: None } | Backstage::Preview { .. } => {
                let [x, y, width, _] = MODEL_AREA;
                [x + width * 0.5, y + 70.0]
            }
            // The stage's column, where the model would stand.
            Backstage::None => [1_160.0, 380.0],
        }
    }

    /// The player's nameplate as a match draws it near, its bottom at `at`: the capsule
    /// with their picture and name, gold-edged with the tray of bars under it while they
    /// share (See and share nameplates), with the dashed marks where ornaments will go:
    /// the crest between the picture and the name, the frame round the plate, the trail
    /// under it.
    fn nameplate(&mut self, frame: &Frame, inputs: &Inputs<'_>, at: [f32; 2]) {
        let s = frame.s;
        let [x, bottom] = at;
        let mark = color::alpha(color::HOLO, 0.8);
        let name = crate::profile_hub::title(inputs.name);
        let plain_name = plain(name);
        let name_width = crate::text::display_width(&plain_name, NAME_SIZE);
        let tray = inputs.share;
        let natural = PICTURE_INSET + PICTURE + PLATE_GAP * 2.0 + CREST + name_width + PLATE_END;
        let width = if tray {
            natural.max(TRAY_WIDTH + PLATE_GAP * 4.0)
        } else {
            natural
        };
        let tray_height = 2.0 * (3.0 + 2.0 + 3.0 + 2.0 + 1.5 * 2.0 + 5.0);
        let pill_bottom = if tray {
            bottom - tray_height + 2.0
        } else {
            bottom
        };
        let pill = [x - width * 0.5, pill_bottom - PILL, width, PILL];
        let middle = pill[1] + PILL * 0.5;
        let edge = if tray {
            color::GOLD
        } else {
            color::alpha(color::HOLO, 0.45)
        };
        let fill = color::alpha(color::SPACE, 0.82);
        let tray_rect = [
            x - TRAY_WIDTH * 0.5,
            pill[1] + PILL - 2.0,
            TRAY_WIDTH,
            tray_height,
        ];
        // Under the plate, the trail's place.
        let low = if tray { bottom } else { pill_bottom };
        dashed_arc(
            &mut self.ui,
            frame,
            [x, low - 30.0],
            60.0,
            (PI * 0.28, PI * 0.44),
            mark,
        );
        if tray {
            self.plate_tray(frame, tray_rect, pill[1] + PILL, fill, edge);
        }
        let pill_rect = frame.rect(pill[0], pill[1], pill[2], pill[3]);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: pill_rect,
            radius: PILL * 0.5 * s,
            color: fill,
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: pill_rect,
            radius: PILL * 0.5 * s,
            width: 2.0 * s,
            color: edge,
        });
        // The picture: theirs once loaded, else their initial on their colour.
        let picture = frame.rect(
            pill[0] + PICTURE_INSET,
            middle - PICTURE * 0.5,
            PICTURE,
            PICTURE,
        );
        let me = inputs.snapshot.and_then(|snapshot| {
            snapshot.me.as_ref().map(|me| {
                (
                    snapshot.key_id.as_str(),
                    me.avatar.as_str(),
                    me.name.as_str(),
                )
            })
        });
        let (key_id, version, hub_name) = me.unwrap_or(("", "", name));
        match crate::avatars::texture(key_id, version) {
            Some(texture) => {
                let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                    rect: picture,
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, 1.0),
                });
            }
            None => {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect: picture,
                    radius: picture.width * 0.5,
                    color: crate::profile_card::stand_in_colour(key_id, hub_name),
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{}", crate::profile_card::initial(hub_name)),
                    picture,
                    PICTURE * 0.55 * s,
                    color::TEXT,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
        }
        // Between the picture and the name, the crest's place.
        let crest_x = pill[0] + PICTURE_INSET + PICTURE + PLATE_GAP + CREST * 0.5;
        dashed_arc(
            &mut self.ui,
            frame,
            [crest_x, middle],
            CREST * 0.5,
            (0.0, TAU),
            mark,
        );
        let name_x = crest_x + CREST * 0.5 + PLATE_GAP;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{name}"),
            frame.rect(name_x, middle - 17.0, name_width + 40.0, 34.0),
            NAME_SIZE * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        // Round the plate, the frame's place.
        let foot = if tray {
            tray_rect[1] + tray_rect[3]
        } else {
            pill[1] + PILL
        };
        dashed_rect(
            &mut self.ui,
            frame,
            [
                pill[0] - 9.0,
                pill[1] - 9.0,
                pill[2] + 18.0,
                foot - pill[1] + 18.0,
            ],
            mark,
        );
    }

    /// The tray of bars under the preview's capsule, from `top` (the capsule's foot,
    /// which it tucks under): the capsule's fill and edge carried down, square at the
    /// top and round at the bottom, as the nameplates draw it.
    fn plate_tray(
        &mut self,
        frame: &Frame,
        [x, y, width, height]: [f32; 4],
        top: f32,
        fill: Color,
        edge: Color,
    ) {
        let s = frame.s;
        let corner = 12.0;
        let bottom = y + height;
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::PushRoundedClip {
            rect: frame.rect(x, top - corner, width, bottom - top + corner),
            radius: corner * s,
        });
        let _ = list.push(DrawCommand::Line {
            from: frame.point(x, (top + bottom) * 0.5),
            to: frame.point(x + width, (top + bottom) * 0.5),
            width: (bottom - top) * s,
            color: fill,
        });
        let _ = list.push(DrawCommand::PopClip);
        let line = 2.0;
        let half = line * 0.5;
        for side in [x + half, x + width - half] {
            let _ = list.push(DrawCommand::Line {
                from: frame.point(side, top),
                to: frame.point(side, bottom - corner),
                width: line * s,
                color: edge,
            });
        }
        let _ = list.push(DrawCommand::Line {
            from: frame.point(x + corner, bottom - half),
            to: frame.point(x + width - corner, bottom - half),
            width: line * s,
            color: edge,
        });
        for (centre, start) in [
            ([x + corner, bottom - corner], PI * 0.5),
            ([x + width - corner, bottom - corner], 0.0),
        ] {
            let _ = list.push(DrawCommand::Arc {
                center: frame.point(centre[0], centre[1]),
                radius: (corner - half) * s,
                width: line * s,
                start,
                sweep: PI * 0.5,
                color: edge,
                knockout: None,
            });
        }
        // Shield, health and Force, as the HUD's bars colour them.
        let mut bar_y = top + 6.0;
        for (colour, share, thick) in [(SHIELD, 0.45, 4.0), (HEALTH, 0.8, 6.0), (FORCE, 0.65, 4.0)]
        {
            let inner = width - 24.0;
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 12.0, bar_y, inner, thick),
                radius: thick * 0.5 * s,
                color: Color::new(1.0, 1.0, 1.0, 0.09),
            });
            let _ = list.push(DrawCommand::RoundedRect {
                rect: frame.rect(x + 12.0, bar_y, inner * share, thick),
                radius: thick * 0.5 * s,
                color: colour,
            });
            bar_y += thick + 3.0;
        }
    }

    /// What the nameplate is, beside the model, and the See and share nameplates switch,
    /// on a dark card at the foot of the column right of the stage (25/10/2026: words
    /// never stand on the bare map), as tall as its words need.
    fn nameplate_words(&mut self, frame: &Frame, inputs: &Inputs<'_>) {
        let s = frame.s;
        let k = crate::text::style::current().scale.max(0.5);
        let note = share_note(inputs);
        let lines = |body: &str, chars: f32| {
            crate::menu::sjk::wrap(body, (chars / k).max(12.0) as usize).count() as f32
        };
        let height = WORDS_PAD
            + 26.0
            + 62.0 * k.max(1.0)
            + 8.0
            + lines(ABOUT, ABOUT_CHARS) * 18.0 * 1.42 * k
            + 4.0
            + lines(ORNAMENTS, NOTE_CHARS) * 16.0 * 1.42 * k
            + 26.0
            + 48.0
            + lines(note, NOTE_CHARS) * 16.0 * 1.42 * k
            + WORDS_PAD;
        let top = WORDS_BOTTOM - height;
        let area = frame.rect(WORDS_X, top, WORDS_WIDTH, height);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 16.0 * s,
            color: color::alpha(color::SPACE, 0.88),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 16.0 * s,
            width: 1.2 * s,
            color: color::alpha(color::HOLO, 0.3),
        });
        let x = WORDS_X + WORDS_PAD;
        let width = WORDS_WIDTH - WORDS_PAD * 2.0;
        let mut y = top + WORDS_PAD;
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Nameplate"),
            frame.rect(x, y, width, 22.0),
            16.0 * s,
            color::HOLO,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 26.0;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Your nameplate"),
            frame.rect(x, y, width, 62.0 * k.max(1.0)),
            48.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        y += 62.0 * k.max(1.0) + 8.0;
        y = super::toys::words(
            &mut self.ui,
            frame,
            ABOUT,
            [x, y, width],
            ABOUT_CHARS,
            18.0,
            color::TEXT,
        );
        y += 4.0;
        y = super::toys::words(
            &mut self.ui,
            frame,
            ORNAMENTS,
            [x, y, width],
            NOTE_CHARS,
            16.0,
            color::MUTED,
        );
        y += 26.0;
        self.share_switch(frame, inputs, [x, y, width], note);
    }

    /// See and share nameplates: its name, the switch and what it does, from `y`. The row
    /// pulses, a gold ring round it and a glow behind the switch, so it is not missed.
    fn share_switch(
        &mut self,
        frame: &Frame,
        inputs: &Inputs<'_>,
        [x, y, width]: [f32; 3],
        note: &str,
    ) {
        let s = frame.s;
        let seconds = self.epoch.elapsed().as_secs_f32();
        #[cfg(test)]
        let seconds = self.shot_seconds.unwrap_or(seconds);
        let pulse = share_pulse(seconds);
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(x - 14.0, y - 9.0, SHARE_ROW + 28.0, 48.0),
            radius: 12.0 * s,
            width: (1.5 + pulse) * s,
            color: color::alpha(color::GOLD, 0.25 + 0.65 * pulse),
        });
        glow(
            &mut self.ui,
            frame,
            [x + SHARE_ROW - 60.0, y + 15.0],
            40.0 + 10.0 * pulse,
            color::GOLD,
            0.15 + 0.35 * pulse,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("See & share nameplates"),
            frame.rect(x, y, SHARE_ROW - 100.0, 30.0),
            22.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let right = x + SHARE_ROW;
        let left = kit::switch(&mut self.ui, frame, right, y + 15.0, inputs.share, true);
        self.ui.hit_region(
            SHARE_TOKEN,
            frame.rect(left - 4.0, y - 4.0, right - left + 8.0, 38.0),
        );
        let _ = super::toys::words(
            &mut self.ui,
            frame,
            note,
            [x, y + 48.0, width],
            NOTE_CHARS,
            16.0,
            color::QUIET,
        );
    }
}

/// What the line under See and share nameplates says on or off now.
fn share_note(inputs: &Inputs<'_>) -> &'static str {
    match (inputs.enabled, inputs.share) {
        (false, _) => "Needs the SJK identity (Settings > Network) to reach other players.",
        (true, true) => {
            "You see other players' nameplates, and your health, shield and Force go to the SJK players of your server."
        }
        (true, false) => "Off: you see no nameplates, and nothing of yours is sent.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pulse starts dark, is fully lit halfway through and dark again at the end.
    #[test]
    fn the_share_row_pulses_from_dark_to_lit_and_back() {
        assert!(share_pulse(0.0) < 1e-6);
        assert!((share_pulse(SHARE_PULSE_SECONDS * 0.5) - 1.0).abs() < 1e-6);
        assert!(share_pulse(SHARE_PULSE_SECONDS) < 1e-5);
    }

    /// The empty places, the nameplate and its words fit over the keys with the model
    /// on the stage, in a preview and without it, at 1080 lines, 4K, 4:3 and 21:9.
    #[test]
    fn the_places_and_the_plate_fit_over_the_keys() {
        let font = crate::text::load_modern(1.0, None).expect("Inter").font;
        for viewport in [
            [1_920.0, 1_080.0],
            [3_840.0, 2_160.0],
            [1_440.0, 1_080.0],
            [2_560.0, 1_080.0],
        ] {
            let head = Frame::new(viewport).point(1_150.0, 420.0);
            for backstage in [
                Backstage::None,
                Backstage::World { head: Some(head) },
                Backstage::Preview { ready: false },
            ] {
                let inputs = super::super::tests::inputs(unlockables::Holdings::Known(&[]), "");
                let mut panel = Panel::new();
                panel.open(Tab::Nameplates, true, true, ReturnTarget::MainMenu);
                panel.set_backstage(backstage);
                panel.build(&inputs, &font, viewport);
                assert!(!panel.ui.overflowed(), "{viewport:?}");
                let frame = Frame::new(viewport);
                let keys = frame.point(0.0, super::super::view::KEYS_Y)[1];
                let switch = panel.ui.rect_for(SHARE_TOKEN).expect("the share switch");
                assert!(switch.bottom() <= keys, "{viewport:?}");
                for command in panel.ui.draw_list().commands() {
                    if let DrawCommand::Text { rect, .. } = command {
                        assert!(rect.bottom() <= keys || rect.y >= keys, "{rect:?}");
                        assert!(rect.right() <= viewport[0] + 1.0, "{rect:?}");
                    }
                }
            }
        }
    }
}
