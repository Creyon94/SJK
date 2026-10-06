//! Quick wheels: hold a key, a ring of choices opens in the middle of the screen, move
//! the mouse towards one and let go to run it (SJK only).
//!
//! `+wheel <name>` opens a wheel while its key is held and `-wheel` (the key's release)
//! runs the highlighted choice; Escape, or letting go with the mouse still near the
//! middle, runs nothing. While a wheel is open the mouse moves its pointer instead of
//! the view; movement keys keep working. Choices are console commands, so a wheel does
//! exactly what typing them would. Two wheels are built in: `general` (camera,
//! nameplates, HUD, screenshot, cosmetics, AFK, menus) and `weather` (forced weather,
//! fog, clouds). A choice whose setting is in effect is marked with a dot.

use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId, TextOverflow};

/// The console commands opening and running a wheel.
pub(crate) const OPEN_COMMAND: &str = "+wheel";
pub(crate) const RUN_COMMAND: &str = "-wheel";
pub(crate) const OPEN_HELP: &str = "Hold to open a quick wheel: +wheel general or +wheel weather";
pub(crate) const RUN_HELP: &str = "Run the quick wheel's highlighted choice and close it";

/// Mouse counts from the middle before a choice is highlighted, and the farthest the
/// pointer goes: any further movement only turns it.
const DEADZONE: f32 = 28.0;
const REACH: f32 = 120.0;

/// When a choice counts as in effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    /// Never marked.
    None,
    /// The cvar holds this integer.
    Equals(&'static str, i64),
    /// The cvar is on (nonzero).
    On(&'static str),
}

/// One choice of a wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Choice {
    pub(crate) label: &'static str,
    pub(crate) command: &'static str,
    pub(crate) state: State,
}

const fn choice(label: &'static str, command: &'static str, state: State) -> Choice {
    Choice {
        label,
        command,
        state,
    }
}

/// A named wheel; its first choice sits at the top, the others follow clockwise.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Wheel {
    pub(crate) name: &'static str,
    pub(crate) title: &'static str,
    pub(crate) choices: &'static [Choice],
}

/// The built-in wheels.
pub(crate) const WHEELS: [Wheel; 2] = [
    Wheel {
        name: "general",
        title: "QUICK",
        choices: &[
            choice("THIRD PERSON", "togglecamera", State::None),
            choice("NAMEPLATES", "nameplates", State::On("cg_nameplate")),
            choice("HUD", "toggle cg_drawHud", State::On("cg_drawHud")),
            choice("SCREENSHOT", "screenshotJPEG", State::None),
            choice("COSMETICS", "cosmetics", State::None),
            choice("AFK", "afk", State::None),
            choice("GAME MENU", "togglemenu", State::None),
            choice("FIRST SETUP", "firstsetup", State::None),
        ],
    },
    Wheel {
        name: "weather",
        title: "WEATHER",
        choices: &[
            choice(
                "MAP'S OWN",
                "r_weatherForce 0",
                State::Equals("r_weatherForce", 0),
            ),
            choice(
                "DRIZZLE",
                "r_weatherForce 1",
                State::Equals("r_weatherForce", 1),
            ),
            choice(
                "RAIN",
                "r_weatherForce 2",
                State::Equals("r_weatherForce", 2),
            ),
            choice(
                "STORM",
                "r_weatherForce 3",
                State::Equals("r_weatherForce", 3),
            ),
            choice(
                "SNOW",
                "r_weatherForce 4",
                State::Equals("r_weatherForce", 4),
            ),
            choice(
                "GROUND FOG",
                "toggle r_weatherFog 1 2",
                State::Equals("r_weatherFog", 2),
            ),
            choice("CLOUDS", "toggle r_clouds", State::On("r_clouds")),
            choice("WEATHER", "toggle r_weather", State::On("r_weather")),
        ],
    },
];

/// The wheel named `name`, case-insensitively.
pub(crate) fn wheel(name: &str) -> Option<usize> {
    WHEELS
        .iter()
        .position(|wheel| wheel.name.eq_ignore_ascii_case(name))
}

/// The choice the pointer points at among `count` around the ring, the first at the top
/// and the rest clockwise (screen y grows downwards); none inside the dead zone.
pub(crate) fn selection(pointer: [f32; 2], count: usize) -> Option<usize> {
    let distance = (pointer[0] * pointer[0] + pointer[1] * pointer[1]).sqrt();
    if count == 0 || distance < DEADZONE {
        return None;
    }
    // Clockwise from straight up.
    let angle = pointer[0]
        .atan2(-pointer[1])
        .rem_euclid(std::f32::consts::TAU);
    let step = std::f32::consts::TAU / count as f32;
    Some(((angle / step).round() as usize) % count)
}

/// The open wheel and its pointer.
#[derive(Clone, Copy, Debug)]
struct Open {
    wheel: usize,
    pointer: [f32; 2],
}

/// Text ids the draw list names: the title, then each choice.
const TITLE_TEXT: u32 = 0;

/// Quick wheel state and its draw list.
pub(crate) struct QuickWheel {
    open: Option<Open>,
    pub(crate) list: DrawList,
    /// Whether each choice of the open wheel is in effect, read when it opens and after
    /// each choice runs.
    marks: Vec<bool>,
}

impl Default for QuickWheel {
    fn default() -> Self {
        Self {
            open: None,
            list: DrawList::new(64),
            marks: Vec::new(),
        }
    }
}

impl QuickWheel {
    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Open wheel `wheel` with the pointer in the middle and `marks` its choices' states.
    pub(crate) fn open(&mut self, wheel: usize, marks: Vec<bool>) {
        self.open = Some(Open {
            wheel,
            pointer: [0.0; 2],
        });
        self.marks = marks;
    }

    /// Close without running anything.
    pub(crate) fn cancel(&mut self) {
        self.open = None;
        self.list.clear();
    }

    /// Close and return the highlighted choice's command.
    pub(crate) fn release(&mut self) -> Option<&'static str> {
        let open = self.open.take()?;
        self.list.clear();
        let choices = WHEELS[open.wheel].choices;
        selection(open.pointer, choices.len()).map(|index| choices[index].command)
    }

    /// Mouse movement while open, in raw counts: moves the pointer, never past [`REACH`].
    pub(crate) fn moved(&mut self, delta: [f32; 2]) {
        let Some(open) = &mut self.open else {
            return;
        };
        let mut pointer = [open.pointer[0] + delta[0], open.pointer[1] + delta[1]];
        let distance = (pointer[0] * pointer[0] + pointer[1] * pointer[1]).sqrt();
        if distance > REACH {
            pointer = pointer.map(|value| value * REACH / distance);
        }
        open.pointer = pointer;
    }

    /// The open wheel's highlighted choice.
    fn highlighted(&self) -> Option<usize> {
        let open = self.open.as_ref()?;
        selection(open.pointer, WHEELS[open.wheel].choices.len())
    }

    /// Lay the wheel out in the middle of `viewport`: a dark ring, one rounded slice per
    /// choice (the highlighted one larger, in the accent colour), its label, a dot on
    /// the choices in effect, the title and a pointer mark in the middle.
    pub(crate) fn build(&mut self, viewport: [f32; 2], accent: Color) {
        self.list.clear();
        let Some(open) = self.open else {
            return;
        };
        let choices = WHEELS[open.wheel].choices;
        let highlighted = self.highlighted();
        let unit = (viewport[1] / 1080.0).max(0.5);
        let center = [viewport[0] * 0.5, viewport[1] * 0.5];
        let radius = 200.0 * unit;
        let width = 62.0 * unit;
        let step = std::f32::consts::TAU / choices.len().max(1) as f32;
        // Round caps reach half the width past each end of a slice.
        let cap = width * 0.5 / radius;
        let gap = 0.05;
        let _ = self.list.push(DrawCommand::Arc {
            center,
            radius,
            width: width + 22.0 * unit,
            start: 0.0,
            sweep: std::f32::consts::TAU,
            color: Color::new(0.02, 0.03, 0.05, 0.55),
            knockout: None,
        });
        for index in 0..choices.len() {
            let middle = -std::f32::consts::FRAC_PI_2 + index as f32 * step;
            let selected = highlighted == Some(index);
            let slice_width = if selected { width + 10.0 * unit } else { width };
            let sweep = (step - 2.0 * cap - gap).max(0.0);
            let _ = self.list.push(DrawCommand::Arc {
                center,
                radius,
                width: slice_width,
                start: middle - sweep * 0.5,
                sweep,
                color: if selected {
                    Color::new(accent.r, accent.g, accent.b, 0.92)
                } else {
                    Color::new(0.16, 0.18, 0.22, 0.85)
                },
                knockout: None,
            });
            let at = [
                center[0] + middle.cos() * radius,
                center[1] + middle.sin() * radius,
            ];
            if self.marks.get(index).copied().unwrap_or(false) {
                let out = radius + width * 0.5 + 10.0 * unit;
                let spot = [
                    center[0] + middle.cos() * out,
                    center[1] + middle.sin() * out,
                ];
                push_dot(&mut self.list, spot, 8.0 * unit, accent);
            }
            let size = 15.0 * unit;
            let label_width = 150.0 * unit;
            let _ = self.list.push(DrawCommand::Text {
                rect: Rect::new(
                    at[0] - label_width * 0.5,
                    at[1] - size * 0.65,
                    label_width,
                    size * 1.3,
                ),
                text: TextId(index as u32 + 1),
                size,
                color: Color::new(1.0, 1.0, 1.0, if selected { 1.0 } else { 0.85 }),
                align: TextAlign::Center,
                overflow: TextOverflow::Ellipsis,
                weight: if selected {
                    FontWeight::Semibold
                } else {
                    FontWeight::Regular
                },
                letter_spacing: 0.5 * unit,
            });
        }
        let size = 20.0 * unit;
        let _ = self.list.push(DrawCommand::Text {
            rect: Rect::new(
                center[0] - 100.0 * unit,
                center[1] - size * 0.65,
                200.0 * unit,
                size * 1.3,
            ),
            text: TextId(TITLE_TEXT),
            size,
            color: Color::new(1.0, 1.0, 1.0, 0.6),
            align: TextAlign::Center,
            overflow: TextOverflow::Ellipsis,
            weight: FontWeight::Semibold,
            letter_spacing: 2.0 * unit,
        });
        // The pointer: a dot drawn from the middle towards where the mouse points.
        let reach = (radius - width * 0.5 - 14.0 * unit) / REACH;
        let pointer = [
            center[0] + open.pointer[0] * reach,
            center[1] + open.pointer[1] * reach,
        ];
        push_dot(
            &mut self.list,
            pointer,
            9.0 * unit,
            Color::new(1.0, 1.0, 1.0, 0.9),
        );
    }

    /// The text of draw-list id `id`.
    pub(crate) fn text(&self, id: TextId) -> &'static str {
        let Some(open) = self.open else {
            return "";
        };
        let wheel = &WHEELS[open.wheel];
        match id.0 {
            TITLE_TEXT => wheel.title,
            index => wheel
                .choices
                .get(index as usize - 1)
                .map_or("", |choice| choice.label),
        }
    }
}

/// A round dot `size` across centred on `at`.
fn push_dot(list: &mut DrawList, at: [f32; 2], size: f32, color: Color) {
    let _ = list.push(DrawCommand::RoundedRect {
        rect: Rect::new(at[0] - size * 0.5, at[1] - size * 0.5, size, size),
        radius: size * 0.5,
        color,
    });
}

impl crate::GpuState {
    /// `+wheel <name>`: open the wheel (the general one without a name).
    pub(crate) fn open_quick_wheel(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let name = args.first().map_or("general", String::as_str);
        // A bound key adds its number and time after the name; a typed command may not.
        let index = wheel(name).or_else(|| name.parse::<u64>().ok().and(wheel("general")));
        let Some(index) = index else {
            let names: Vec<_> = WHEELS.iter().map(|wheel| wheel.name).collect();
            return Err(format!("usage: +wheel [{}]", names.join(" | ")));
        };
        let marks = self.quick_wheel_marks(index);
        self.quick_wheel.open(index, marks);
        Ok(Vec::new())
    }

    /// `-wheel`: run the highlighted choice and close the wheel.
    pub(crate) fn release_quick_wheel(&mut self) -> Result<Vec<String>, String> {
        let Some(command) = self.quick_wheel.release() else {
            return Ok(Vec::new());
        };
        let console = self.console.as_mut().ok_or("no console")?;
        console.queue_command(command).map(|()| Vec::new())
    }

    /// Whether each choice of wheel `index` is in effect now.
    fn quick_wheel_marks(&self, index: usize) -> Vec<bool> {
        let Some(console) = &self.console else {
            return Vec::new();
        };
        let integer = |name: &str| {
            console
                .integer_cvar(name)
                .or_else(|| console.bool_cvar(name).map(i64::from))
        };
        WHEELS[index]
            .choices
            .iter()
            .map(|choice| match choice.state {
                State::None => false,
                State::Equals(name, value) => integer(name) == Some(value),
                State::On(name) => integer(name).is_some_and(|value| value != 0),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pointer_picks_the_choice_it_points_at_clockwise_from_the_top() {
        // Eight choices: up is the first, right the third, down the fifth, left the seventh.
        assert_eq!(selection([0.0, -50.0], 8), Some(0));
        assert_eq!(selection([50.0, -50.0], 8), Some(1));
        assert_eq!(selection([50.0, 0.0], 8), Some(2));
        assert_eq!(selection([0.0, 50.0], 8), Some(4));
        assert_eq!(selection([-50.0, 0.0], 8), Some(6));
        // Just left of straight up still rounds to the first.
        assert_eq!(selection([-5.0, -60.0], 8), Some(0));
        // Near the middle nothing is chosen.
        assert_eq!(selection([10.0, -10.0], 8), None);
        assert_eq!(selection([0.0, -50.0], 0), None);
    }

    #[test]
    fn releasing_runs_the_highlighted_choice_once_and_the_middle_runs_nothing() {
        let mut wheel = QuickWheel::default();
        let weather = super::wheel("WEATHER").unwrap();
        wheel.open(weather, vec![false; 8]);
        assert!(wheel.is_open());
        wheel.moved([300.0, 0.0]);
        assert_eq!(wheel.release(), Some("r_weatherForce 2"));
        assert!(!wheel.is_open());
        assert_eq!(wheel.release(), None);
        wheel.open(weather, vec![false; 8]);
        wheel.moved([5.0, 5.0]);
        assert_eq!(wheel.release(), None);
        wheel.open(weather, vec![false; 8]);
        wheel.moved([0.0, -200.0]);
        wheel.cancel();
        assert_eq!(wheel.release(), None);
    }

    #[test]
    fn the_pointer_stops_at_its_reach_and_still_turns() {
        let mut wheel = QuickWheel::default();
        wheel.open(0, Vec::new());
        wheel.moved([1000.0, 0.0]);
        wheel.moved([0.0, 1000.0]);
        let open = wheel.open.unwrap();
        let distance = (open.pointer[0].powi(2) + open.pointer[1].powi(2)).sqrt();
        assert!((distance - REACH).abs() < 0.01);
        // Pushed down after right: past the right choice, towards the bottom ones.
        assert!(matches!(wheel.highlighted(), Some(3 | 4)));
    }

    #[test]
    fn every_wheel_lays_out_its_choices_and_names_them() {
        for (index, definition) in WHEELS.iter().enumerate() {
            let mut wheel = QuickWheel::default();
            wheel.open(index, vec![true; definition.choices.len()]);
            wheel.moved([0.0, -100.0]);
            wheel.build([1920.0, 1080.0], Color::new(1.0, 0.4, 0.2, 1.0));
            let texts = wheel
                .list
                .commands()
                .iter()
                .filter(|command| matches!(command, DrawCommand::Text { .. }))
                .count();
            assert_eq!(texts, definition.choices.len() + 1, "{}", definition.name);
            assert_eq!(wheel.text(TextId(0)), definition.title);
            assert_eq!(wheel.text(TextId(1)), definition.choices[0].label);
            assert!(
                definition
                    .choices
                    .iter()
                    .all(|choice| choice.label.len() <= 16)
            );
        }
        assert_eq!(super::wheel("General"), Some(0));
        assert_eq!(super::wheel("hail"), None);
    }
}
