//! Gameplay FPS: the frame rate whose movement the player wants, apart from the
//! frames the screen draws (`cl_gameplayFps`, `sjk_client::command_rate`).
//!
//! In the original game the frame rate is also the movement: each rendered frame
//! makes one user command, and `com_maxfps` 125, 142, 250 or 333 moves the player
//! in steps of 8, 7, 4 or 3 ms, whose rounding changes jumps, strafes and
//! acceleration. Players lock those numbers for that feel and ask why SJK draws
//! 200 frames yet moves like 125. SJK makes commands on their own clock, as JoF
//! EJK's `cl_cmdratecap`, at the gameplay FPS the player picks; the FPS cap
//! (`com_maxfps`) only sets the frames drawn.
//!
//! The choice is a step of the Import card (`config_import_panel.rs`), after a
//! config from another client is read, with that config's old frame rate picked;
//! the `gameplayfps` command and the Video settings' Gameplay FPS row open the same
//! card on its own. Four presets cover what players lock; "Other" takes any value.

use sjk_client::command_rate::{
    DEFAULT_GAMEPLAY_FPS, GAMEPLAY_FPS_PRESETS, MAX_GAMEPLAY_FPS, MIN_GAMEPLAY_FPS,
    commands_per_second, gameplay_step,
};

/// The setting.
pub(crate) const CVAR: &str = "cl_gameplayFps";
/// Opens the card, or with a number sets it.
pub(crate) const COMMAND: &str = "gameplayfps";
pub(crate) const HELP: &str = "Explain and choose your gameplay FPS (the movement of 125, 142, 250, 333...); gameplayfps <fps> sets it";

/// What the card and the command say about it, one paragraph.
pub(crate) const EXPLANATION: &str = "In the original game your FPS was also your physics: at \
    125, 142, 250 or 333 FPS jumps, strafes and acceleration felt different. SJK keeps the two \
    apart. The screen draws as many frames as your FPS cap allows, and your movement runs at the \
    gameplay FPS you pick, whatever your frame rate.";

/// The note on servers that fix the movement step.
pub(crate) const FIXED_NOTE: &str =
    "Servers that fix the movement step (pmove_fixed) move everyone in their own step instead.";

/// The "Other" choice after the presets.
pub(crate) const OTHER: usize = GAMEPLAY_FPS_PRESETS.len();
/// Most digits the advanced value takes.
const DRAFT_DIGITS: usize = 4;

/// What another client's config said about its frame rate.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Origin {
    /// The config's file name.
    pub(crate) file: String,
    /// Its `com_maxfps`, when set and above 0.
    pub(crate) max_fps: Option<i64>,
    /// Its `cl_cmdratecap` was on (JoF EJK): its movement already ran at 125.
    pub(crate) cmd_rate_cap: bool,
}

impl Origin {
    /// The gameplay FPS the old client moved at: 125 under `cl_cmdratecap`, else
    /// its frame cap, clamped to the range.
    pub(crate) fn old_fps(&self) -> Option<i64> {
        if self.cmd_rate_cap {
            return Some(DEFAULT_GAMEPLAY_FPS);
        }
        self.max_fps
            .map(|fps| fps.clamp(MIN_GAMEPLAY_FPS, MAX_GAMEPLAY_FPS))
    }

    /// The line under the explanation that says what was picked and why.
    pub(crate) fn line(&self) -> Option<String> {
        if self.cmd_rate_cap {
            return Some(format!(
                "{} had cl_cmdratecap 1 (JoF EJK): your movement already ran at 125, so 125 is \
                 picked.",
                self.file
            ));
        }
        let fps = self.max_fps?;
        Some(format!(
            "{} locked com_maxfps at {fps}, so {} is picked: you keep the movement you are used to.",
            self.file,
            self.old_fps().unwrap_or(fps)
        ))
    }
}

/// The card's choice: a preset or "Other" with its typed value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Choice {
    /// A preset's index, or [`OTHER`].
    pub(crate) selected: usize,
    /// "Other"'s digits.
    pub(crate) draft: String,
    /// The setting when the card opened.
    pub(crate) current: i64,
    pub(crate) origin: Option<Origin>,
}

impl Choice {
    /// Open on the old config's frame rate when there is one, else the setting.
    pub(crate) fn new(current: i64, origin: Option<Origin>) -> Self {
        let start = origin.as_ref().and_then(Origin::old_fps).unwrap_or(current);
        let mut choice = Self {
            selected: OTHER,
            draft: String::new(),
            current,
            origin,
        };
        choice.pick(start);
        choice
    }

    /// Select `fps`: its preset, or "Other" holding it.
    fn pick(&mut self, fps: i64) {
        match GAMEPLAY_FPS_PRESETS
            .iter()
            .position(|preset| *preset == fps)
        {
            Some(index) => self.selected = index,
            None => {
                self.selected = OTHER;
                self.draft = fps.to_string();
            }
        }
    }

    /// The chosen value when it is in range.
    pub(crate) fn value(&self) -> Option<i64> {
        let fps = match GAMEPLAY_FPS_PRESETS.get(self.selected) {
            Some(preset) => *preset,
            None => self.draft.parse().ok()?,
        };
        (MIN_GAMEPLAY_FPS..=MAX_GAMEPLAY_FPS)
            .contains(&fps)
            .then_some(fps)
    }

    /// Move the selection by `delta` choices, wrapping.
    pub(crate) fn step(&mut self, delta: isize) {
        let count = OTHER as isize + 1;
        self.selected = (self.selected as isize + delta).rem_euclid(count) as usize;
    }

    /// Select choice `index` (a click).
    pub(crate) fn select(&mut self, index: usize) {
        self.selected = index.min(OTHER);
    }

    /// A digit typed: it goes to "Other", which it selects; the first digit after
    /// selecting by another way starts a new value.
    pub(crate) fn type_digit(&mut self, digit: char) {
        if self.selected != OTHER {
            self.selected = OTHER;
            self.draft.clear();
        }
        if self.draft.len() < DRAFT_DIGITS && digit.is_ascii_digit() {
            self.draft.push(digit);
        }
    }

    pub(crate) fn backspace(&mut self) {
        if self.selected == OTHER {
            self.draft.pop();
        }
    }

    /// What a preset's tile says under its number.
    pub(crate) fn caption(&self, index: usize) -> &'static str {
        let fps = GAMEPLAY_FPS_PRESETS.get(index).copied();
        let old = self.origin.as_ref().and_then(Origin::old_fps);
        match fps {
            Some(fps) if old == Some(fps) => "your old one",
            Some(fps) if fps == self.current && old.is_none() => "in use",
            Some(DEFAULT_GAMEPLAY_FPS) => "JoF EJK",
            Some(_) => "",
            None => "any value",
        }
    }

    /// What "Other"'s field says about its value.
    pub(crate) fn draft_note(&self) -> String {
        match self.draft.parse::<i64>() {
            Ok(fps) if (MIN_GAMEPLAY_FPS..=MAX_GAMEPLAY_FPS).contains(&fps) => {
                let made = commands_per_second(fps);
                let step = gameplay_step(fps);
                if made == fps {
                    format!("{step} ms steps")
                } else {
                    format!("moves in {step} ms steps, {made} a second")
                }
            }
            _ => format!("from {MIN_GAMEPLAY_FPS} to {MAX_GAMEPLAY_FPS}"),
        }
    }
}

/// Left/Right on the setting: the next preset above (`direction` > 0) or below
/// `current`, wrapping; a typed value steps to its nearest preset that way.
pub(crate) fn step_preset(current: i64, direction: i32) -> i64 {
    let presets = GAMEPLAY_FPS_PRESETS;
    if direction > 0 {
        presets
            .into_iter()
            .find(|preset| *preset > current)
            .unwrap_or(presets[0])
    } else {
        presets
            .into_iter()
            .rev()
            .find(|preset| *preset < current)
            .unwrap_or(presets[presets.len() - 1])
    }
}

/// The setting's row value: the number, and the server's step while it fixes one.
pub(crate) fn row_text(console: &crate::console::ViewerConsole) -> String {
    let fps = console.gameplay_fps();
    match console.pmove_fixed_msec() {
        Some(step) => format!("{fps} (this server: {step} ms)"),
        None => fps.to_string(),
    }
}

/// What `fps` moves like, for the console and the card's result line.
pub(crate) fn describe(fps: i64) -> String {
    let made = commands_per_second(fps);
    let step = gameplay_step(fps);
    if made == fps {
        format!("{fps} ({step} ms steps)")
    } else {
        format!("{fps} ({step} ms steps, {made} a second)")
    }
}

/// `gameplayfps <fps>`: the value to set, or why not.
pub(crate) fn parse_argument(text: &str) -> Result<i64, String> {
    text.trim()
        .parse::<i64>()
        .ok()
        .filter(|fps| (MIN_GAMEPLAY_FPS..=MAX_GAMEPLAY_FPS).contains(fps))
        .ok_or_else(|| {
            format!("usage: {COMMAND} [fps], fps from {MIN_GAMEPLAY_FPS} to {MAX_GAMEPLAY_FPS}")
        })
}

impl crate::GpuState {
    /// `gameplayfps`: explain it in the console and open the card to choose;
    /// `gameplayfps <fps>` sets it. The lines go to the console.
    pub(crate) fn gameplay_fps_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let console = self.console.as_mut().ok_or("Console unavailable")?;
        let lines = match args {
            [] => {
                let lines = vec![
                    EXPLANATION.to_owned(),
                    format!(
                        "Yours is {}. {FIXED_NOTE}",
                        describe(console.gameplay_fps())
                    ),
                ];
                console.open_gameplay_fps();
                self.sync_cursor_policy();
                lines
            }
            [value] => {
                let fps = parse_argument(value)?;
                let line = console
                    .set_gameplay_fps(fps)
                    .ok_or("Could not set the gameplay FPS")?;
                vec![line]
            }
            _ => return Err(parse_argument("").unwrap_err()),
        };
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(max_fps: Option<i64>, cmd_rate_cap: bool) -> Origin {
        Origin {
            file: "eternaljk.cfg".to_owned(),
            max_fps,
            cmd_rate_cap,
        }
    }

    #[test]
    fn the_old_configs_frame_rate_is_picked() {
        let choice = Choice::new(125, Some(origin(Some(333), false)));
        assert_eq!(choice.selected, 3);
        assert_eq!(choice.value(), Some(333));
        assert_eq!(choice.caption(3), "your old one");
        // JoF EJK's cl_cmdratecap already moved at 125, whatever its FPS cap.
        let choice = Choice::new(250, Some(origin(Some(333), true)));
        assert_eq!(choice.value(), Some(125));
        // An odd cap opens on Other with it typed.
        let choice = Choice::new(125, Some(origin(Some(200), false)));
        assert_eq!((choice.selected, choice.value()), (OTHER, Some(200)));
        // No cap in the file: the setting in use.
        let choice = Choice::new(142, Some(origin(None, false)));
        assert_eq!(choice.value(), Some(142));
        assert!(
            Choice::new(142, Some(origin(None, false)))
                .origin
                .unwrap()
                .line()
                .is_none()
        );
    }

    #[test]
    fn digits_go_to_other_and_only_in_range_values_count() {
        let mut choice = Choice::new(125, None);
        assert_eq!(choice.caption(0), "in use");
        choice.type_digit('4');
        choice.type_digit('0');
        assert_eq!(choice.selected, OTHER);
        assert_eq!(choice.value(), Some(40));
        choice.type_digit('0');
        assert_eq!(choice.draft_note(), "moves in 2 ms steps, 500 a second");
        choice.type_digit('0');
        choice.type_digit('9');
        assert_eq!(choice.draft, "4000", "four digits at most");
        assert_eq!(choice.value(), None, "above 1000");
        choice.backspace();
        choice.backspace();
        choice.backspace();
        assert_eq!(choice.value(), None, "4 is below 20");
        choice.step(1);
        assert_eq!(choice.value(), Some(125), "wraps to the first preset");
        choice.step(-1);
        assert_eq!(choice.selected, OTHER);
    }

    #[test]
    fn left_and_right_step_through_the_presets() {
        assert_eq!(step_preset(125, 1), 142);
        assert_eq!(step_preset(333, 1), 125, "wraps");
        assert_eq!(step_preset(125, -1), 333, "wraps");
        assert_eq!(
            step_preset(200, 1),
            250,
            "a typed value goes to its neighbour"
        );
        assert_eq!(step_preset(200, -1), 142);
    }

    #[test]
    fn the_command_takes_a_number_in_range() {
        assert_eq!(parse_argument("333"), Ok(333));
        assert!(parse_argument("10").is_err());
        assert!(parse_argument("fast").is_err());
        assert_eq!(describe(333), "333 (3 ms steps)");
        assert_eq!(describe(142), "142 (7 ms steps)");
        assert_eq!(describe(400), "400 (2 ms steps, 500 a second)");
    }
}
