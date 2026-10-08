//! The quick wheel: hold a key, a ring of choices opens in the middle of the
//! screen, move the mouse towards one and let go to run it (SJK only).
//!
//! The wheel has pages ([`pages`]: General and Weather unless the player changed
//! them in Settings > Quick wheel, kept in `wheel.json`). `+wheel <page>` opens it
//! on that page (named by its id or its name) while its key is held; a bare
//! `+wheel` opens it on the page shown last (the first one in a new run).
//! `-wheel` (the key's release) runs the highlighted choice. While it is open the
//! mouse wheel and buttons change page, wrapping round: scrolling up or a left
//! click goes back a page, scrolling down or a right click on; meanwhile they do
//! nothing else (no attack, no weapon change). Escape, or letting go with the
//! mouse still near the middle, runs nothing; the console, chat or a menu close
//! the wheel. While it is open the mouse moves its pointer instead of the view;
//! movement keys keep working. Choices are console commands, so a choice does
//! exactly what typing it would; a dot marks those in effect. The ring is drawn
//! in the SJK UI's look ([`ring`]).
//!
//! With [`SOUNDS_CVAR`] on, the wheel plays the game's own interface sounds
//! ([`crate::audio::ui_cues`]): one as it turns to another page, one as the
//! highlight moves to another choice (not for every mouse movement), and one as
//! the chosen choice runs. Letting go on nothing, Escape and closing it play
//! none: nothing ran.

pub(crate) mod catalog;
pub(crate) mod pages;
pub(crate) mod ring;

pub(crate) use catalog::ICONS;

use crate::audio::ui_cues::{self, Cue};
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextStyle, TextVertex, UiFont};
use catalog::State;
use pages::MAX_CHOICES;
use sjk_ui::DrawList;
use std::time::{Duration, Instant};

/// The console commands opening and running the wheel.
pub(crate) const OPEN_COMMAND: &str = "+wheel";
pub(crate) const RUN_COMMAND: &str = "-wheel";
pub(crate) const OPEN_HELP: &str = "Hold to open the quick wheel: +wheel <page> (general, weather...), or +wheel for the last page";
pub(crate) const RUN_HELP: &str = "Run the quick wheel's highlighted choice and close it";
/// Whether the wheel plays its sounds (archived, on by default).
pub(crate) const SOUNDS_CVAR: &str = "cg_wheelSounds";

/// Mouse counts from the middle before a choice is highlighted, and the farthest the
/// pointer goes: any further movement only turns it.
const DEADZONE: f32 = 28.0;
const REACH: f32 = 120.0;
/// Scroll of one mouse-wheel notch, as the pointer input measures it.
const NOTCH: f32 = 40.0;
/// How long a page takes to arrive after a change of page.
const ARRIVAL: Duration = Duration::from_millis(170);

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

/// A page as the open wheel shows it: copied from the pages when the wheel opens,
/// with whether each choice is in effect.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShownPage {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) choices: Vec<ShownChoice>,
}

/// A choice as the open wheel shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShownChoice {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) icon: Option<usize>,
    pub(crate) on: bool,
}

/// The open wheel: its page, its pointer and when that page arrived.
#[derive(Clone, Copy, Debug)]
struct Open {
    page: usize,
    pointer: [f32; 2],
    /// When the page last changed and which way (-1 back, 1 on).
    arrived: Option<(Instant, f32)>,
    /// The choice highlighted when the pointer last moved or the page changed:
    /// the move cue plays only when another one is.
    highlighted: Option<usize>,
}

/// The mouse buttons that change page while the wheel is open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Button {
    /// Back a page.
    Left,
    /// On a page.
    Right,
}

/// Quick wheel state and its draw list.
pub(crate) struct QuickWheel {
    open: Option<Open>,
    pages: Vec<ShownPage>,
    /// The id of the page shown last, which a bare `+wheel` opens on.
    last: String,
    /// Scroll not yet a whole notch.
    scroll: f32,
    /// Buttons whose press changed page: their release is the wheel's too.
    held: [bool; 2],
    /// Whether the wheel plays its sounds ([`SOUNDS_CVAR`], read as it opens).
    sounds: bool,
    canvas: MenuCanvas,
    /// Drawn without a game (world shots).
    #[cfg(test)]
    pub(crate) shot: bool,
}

impl Default for QuickWheel {
    fn default() -> Self {
        Self {
            open: None,
            pages: Vec::new(),
            last: String::new(),
            scroll: 0.0,
            held: [false; 2],
            sounds: true,
            canvas: MenuCanvas::with_capacities(48, 48, 256),
            #[cfg(test)]
            shot: false,
        }
    }
}

impl QuickWheel {
    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// While the wheel is open the HUD steps aside, as `cg_drawHud 0` would
    /// have it (Sol, 08/10/2026): the status, weapon, timers, crosshair and the
    /// game-data HUD hide; chat and nameplates stay.
    pub(crate) fn hides_hud(&self) -> bool {
        self.is_open()
    }

    /// Open on page `page` of `pages` with the pointer in the middle; when the
    /// wheel is already open (a second wheel key), change to that page instead.
    pub(crate) fn open(&mut self, pages: Vec<ShownPage>, page: usize, now: Instant) {
        if pages.is_empty() {
            return;
        }
        let page = page.min(pages.len() - 1);
        self.pages = pages;
        self.last.clone_from(&self.pages[page].id);
        match &mut self.open {
            Some(open) if open.page != page => {
                let direction = if page > open.page { 1.0 } else { -1.0 };
                open.page = page;
                open.arrived = Some((now, direction));
                self.page_changed();
            }
            Some(_) => {}
            None => {
                self.scroll = 0.0;
                self.open = Some(Open {
                    page,
                    pointer: [0.0; 2],
                    arrived: None,
                    highlighted: None,
                });
            }
        }
    }

    /// Whether the wheel plays its sounds ([`SOUNDS_CVAR`]).
    pub(crate) fn set_sounds(&mut self, on: bool) {
        self.sounds = on;
    }

    /// Post `cue` when the wheel plays its sounds.
    fn cue(&self, cue: Cue) {
        if self.sounds {
            ui_cues::post(cue);
        }
    }

    /// Close without running anything.
    pub(crate) fn cancel(&mut self) {
        self.open = None;
    }

    /// Close and return the highlighted choice's command (its run cue played).
    pub(crate) fn release(&mut self) -> Option<String> {
        let highlighted = self.highlighted();
        let open = self.open.take()?;
        let page = self.pages.get(open.page)?;
        let command = highlighted
            .and_then(|index| page.choices.get(index))
            .map(|choice| choice.command.clone());
        if command.is_some() {
            self.cue(Cue::WheelRun);
        }
        command
    }

    /// Mouse movement while open, in raw counts: moves the pointer, never past
    /// [`REACH`]; the move cue plays when another choice is highlighted.
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
        let count = self
            .pages
            .get(open.page)
            .map_or(0, |page| page.choices.len());
        let highlighted = selection(pointer, count);
        let before = std::mem::replace(&mut open.highlighted, highlighted);
        // Back to the middle highlights nothing: no choice to sound.
        if highlighted.is_some() && highlighted != before {
            self.cue(Cue::WheelMove);
        }
    }

    /// Change page `direction` pages on (-1 back), wrapping round; the pointer
    /// stays where it is, so the same move picks on the new page.
    pub(crate) fn turn(&mut self, direction: i32, now: Instant) {
        let count = self.pages.len();
        let Some(open) = &mut self.open else {
            return;
        };
        if count < 2 || direction == 0 {
            return;
        }
        open.page = (open.page as i64 + i64::from(direction)).rem_euclid(count as i64) as usize;
        open.arrived = Some((now, direction.signum() as f32));
        self.last.clone_from(&self.pages[open.page].id);
        self.page_changed();
    }

    /// The page changed: its cue plays (and no move cue for the choice the
    /// pointer now points at on it).
    fn page_changed(&mut self) {
        let highlighted = self.highlighted();
        if let Some(open) = &mut self.open {
            open.highlighted = highlighted;
        }
        self.cue(Cue::WheelPage);
    }

    /// Mouse-wheel scroll while open (positive away from the player): a page
    /// back a notch up, on a notch down. A trackpad's small steps add up.
    pub(crate) fn scrolled(&mut self, vertical: f32, now: Instant) {
        if !self.is_open() {
            return;
        }
        if self.scroll * vertical < 0.0 {
            self.scroll = 0.0;
        }
        self.scroll += vertical;
        while self.scroll >= NOTCH {
            self.scroll -= NOTCH;
            self.turn(-1, now);
        }
        while self.scroll <= -NOTCH {
            self.scroll += NOTCH;
            self.turn(1, now);
        }
    }

    /// A left or right mouse button: while open, a press changes page and the
    /// press and its release go no further (true). A release whose press came
    /// before the wheel opened goes on, so a held attack lets go.
    pub(crate) fn button(&mut self, button: Button, pressed: bool, now: Instant) -> bool {
        let slot = button as usize;
        if !pressed {
            return std::mem::take(&mut self.held[slot]);
        }
        if !self.is_open() {
            return false;
        }
        self.held[slot] = true;
        self.turn(
            match button {
                Button::Left => -1,
                Button::Right => 1,
            },
            now,
        );
        true
    }

    /// The page on show, while open.
    #[cfg(test)]
    pub(crate) fn page(&self) -> Option<usize> {
        self.open.map(|open| open.page)
    }

    /// The id of the page shown last.
    pub(crate) fn last_page(&self) -> &str {
        &self.last
    }

    /// The open page's highlighted choice.
    fn highlighted(&self) -> Option<usize> {
        let open = self.open.as_ref()?;
        selection(open.pointer, self.pages.get(open.page)?.choices.len())
    }

    /// Lay the wheel out in the middle of `viewport` at time `now` ([`ring`]).
    pub(crate) fn build(&mut self, viewport: [f32; 2], now: Instant) {
        let highlighted = self.highlighted();
        let open = self.open;
        let Self { pages, canvas, .. } = self;
        canvas.begin_transparent(viewport);
        let shown = open.and_then(|open| Some((open, pages.get(open.page)?)));
        if let Some((open, page)) = shown {
            let mut choices = [ring::Choice {
                label: "",
                icon: None,
                on: false,
            }; MAX_CHOICES];
            let count = page.choices.len().min(MAX_CHOICES);
            for (slot, choice) in choices.iter_mut().zip(&page.choices) {
                *slot = ring::Choice {
                    label: &choice.label,
                    icon: choice.icon,
                    on: choice.on,
                };
            }
            let total = pages.len();
            let neighbours = (total > 1).then(|| {
                (
                    pages[(open.page + total - 1) % total].name.as_str(),
                    pages[(open.page + 1) % total].name.as_str(),
                )
            });
            let [x, y] = open.pointer;
            let distance = (x * x + y * y).sqrt();
            let pointer = (distance > 1.0).then(|| {
                (
                    x.atan2(-y).rem_euclid(std::f32::consts::TAU),
                    (distance / DEADZONE).min(1.0),
                )
            });
            let arrival = open.arrived.map_or((1.0, 0.0), |(at, direction)| {
                (
                    (now.saturating_duration_since(at).as_secs_f32() / ARRIVAL.as_secs_f32())
                        .min(1.0),
                    direction,
                )
            });
            ring::draw(
                canvas,
                &ring::Ring {
                    centre: [viewport[0] * 0.5, viewport[1] * 0.5],
                    unit: (viewport[1] / 1080.0).max(0.5),
                    page: &page.name,
                    choices: &choices[..count],
                    highlighted,
                    pointer,
                    pages: (open.page, total),
                    neighbours,
                    arrival,
                    hint: true,
                },
            );
        }
        canvas.finish(0);
    }

    /// The wheel's shapes, as [`Self::build`] laid them out.
    pub(crate) fn draw_list(&self) -> &DrawList {
        self.canvas.draw_list()
    }

    /// Append the wheel's text: in the SJK UI's families when they are loaded
    /// (`fonts`), else in Inter.
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

    /// Whether the wheel is drawn without a game (world shots only).
    #[cfg(test)]
    pub(crate) fn shown_without_a_game(&self) -> bool {
        self.shot
    }

    /// Whether the wheel is drawn without a game (world shots only).
    #[cfg(not(test))]
    pub(crate) fn shown_without_a_game(&self) -> bool {
        false
    }
}

/// Whether a choice in `state` is in effect in `console` now.
fn in_effect(console: &crate::console::ViewerConsole, state: State) -> bool {
    let integer = |name: &str| {
        console
            .integer_cvar(name)
            .or_else(|| console.bool_cvar(name).map(i64::from))
    };
    match state {
        State::None => false,
        State::Equals(name, value) => integer(name) == Some(value),
        State::On(name) => integer(name).is_some_and(|value| value != 0),
        State::Near(name, value) => console
            .float_cvar(name)
            .is_some_and(|current| (current - value).abs() < 0.01),
    }
}

/// Every page as the wheel shows it, from `console`'s pages and cvars.
pub(crate) fn shown_pages(console: &crate::console::ViewerConsole) -> Vec<ShownPage> {
    console
        .wheel_pages
        .pages()
        .iter()
        .map(|page| ShownPage {
            id: page.id.clone(),
            name: page.name.clone(),
            choices: page
                .choices
                .iter()
                .map(|slot| ShownChoice {
                    label: slot.label().to_owned(),
                    command: slot.command().to_owned(),
                    icon: slot.icon(),
                    on: in_effect(console, slot.state()),
                })
                .collect(),
        })
        .collect()
}

/// Per frame: close the wheel while a menu, the console or chat has the
/// keyboard, and lay it out (with its text) when it is to be drawn, `visible`.
pub(crate) fn append(gpu: &mut crate::GpuState, viewport: [f32; 2], visible: bool) {
    if gpu.quick_wheel.is_open() && gpu.text_has_keyboard() {
        gpu.quick_wheel.cancel();
    }
    if !gpu.quick_wheel.is_open() {
        return;
    }
    if !(visible || gpu.quick_wheel.shown_without_a_game()) {
        // Open but not drawn (the intermission, `cg_draw2D 0`): nothing of the
        // last frame's wheel stays on screen.
        gpu.quick_wheel.canvas.begin_transparent(viewport);
        gpu.quick_wheel.canvas.finish(0);
        return;
    }
    gpu.quick_wheel.build(viewport, Instant::now());
    gpu.quick_wheel.append_text(
        gpu.game_fonts.sjk(),
        &mut gpu.text_vertices,
        &gpu.ui_font,
        viewport,
    );
}

impl crate::GpuState {
    /// `+wheel [page]`: open the wheel on that page, or on the page shown last.
    pub(crate) fn open_quick_wheel(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let console = self.console.as_ref().ok_or("no console")?;
        let store = &console.wheel_pages;
        let last = store
            .pages()
            .iter()
            .position(|page| page.id == self.quick_wheel.last_page())
            .unwrap_or(0);
        // A bound key adds its number and time after the name; a typed command does not.
        let named = args
            .first()
            .map(String::as_str)
            .filter(|name| name.parse::<u64>().is_err());
        let page = match named.map(|name| store.find(name)) {
            None => last,
            Some(Some(page)) => page,
            // A bind naming a page since removed opens the last one.
            Some(None) if args.len() >= 3 => last,
            Some(None) => {
                let ids: Vec<_> = store.pages().iter().map(|page| page.id.as_str()).collect();
                return Err(format!("usage: +wheel [{}]", ids.join(" | ")));
            }
        };
        let pages = shown_pages(console);
        self.quick_wheel
            .set_sounds(console.bool_cvar(SOUNDS_CVAR).unwrap_or(true));
        self.quick_wheel.open(pages, page, Instant::now());
        Ok(Vec::new())
    }

    /// `-wheel`: run the highlighted choice and close the wheel.
    pub(crate) fn release_quick_wheel(&mut self) -> Result<Vec<String>, String> {
        let Some(command) = self.quick_wheel.release() else {
            return Ok(Vec::new());
        };
        let console = self.console.as_mut().ok_or("no console")?;
        console.queue_command(&command).map(|()| Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_ui::DrawCommand;

    fn shown() -> Vec<ShownPage> {
        let directory = tempfile::tempdir().unwrap();
        let console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        shown_pages(&console)
    }

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
        let now = Instant::now();
        let mut wheel = QuickWheel::default();
        wheel.open(shown(), 1, now);
        assert!(wheel.is_open());
        wheel.moved([300.0, 0.0]);
        assert_eq!(wheel.release().as_deref(), Some("r_weatherForce 2"));
        assert!(!wheel.is_open());
        assert_eq!(wheel.release(), None);
        wheel.open(shown(), 1, now);
        wheel.moved([5.0, 5.0]);
        assert_eq!(wheel.release(), None);
        wheel.open(shown(), 1, now);
        wheel.moved([0.0, -200.0]);
        wheel.cancel();
        assert_eq!(wheel.release(), None);
    }

    #[test]
    fn the_pointer_stops_at_its_reach_and_still_turns() {
        let mut wheel = QuickWheel::default();
        wheel.open(shown(), 0, Instant::now());
        wheel.moved([1000.0, 0.0]);
        wheel.moved([0.0, 1000.0]);
        let open = wheel.open.unwrap();
        let distance = (open.pointer[0].powi(2) + open.pointer[1].powi(2)).sqrt();
        assert!((distance - REACH).abs() < 0.01);
        // Pushed down after right: past the right choice, towards the bottom ones.
        assert!(matches!(wheel.highlighted(), Some(3 | 4)));
    }

    #[test]
    fn the_hud_steps_aside_only_while_the_wheel_is_open() {
        let now = Instant::now();
        let mut wheel = QuickWheel::default();
        assert!(!wheel.hides_hud());
        wheel.open(shown(), 0, now);
        assert!(wheel.hides_hud());
        let _ = wheel.release();
        assert!(!wheel.hides_hud(), "letting go brings the HUD back");
        wheel.open(shown(), 1, now);
        wheel.cancel();
        assert!(!wheel.hides_hud(), "so does cancelling it");
    }

    #[test]
    fn scroll_and_clicks_change_page_and_wrap_keeping_the_pointer() {
        let now = Instant::now();
        let mut wheel = QuickWheel::default();
        // Closed, the buttons and the scroll are the game's.
        assert!(!wheel.button(Button::Left, true, now));
        assert!(!wheel.button(Button::Left, false, now));
        wheel.scrolled(-40.0, now);
        wheel.open(shown(), 0, now);
        wheel.moved([80.0, 0.0]);
        // A notch down goes on a page, keeping where the mouse points.
        wheel.scrolled(-40.0, now);
        assert_eq!(wheel.page(), Some(1));
        assert_eq!(wheel.last_page(), "weather");
        assert_eq!(wheel.release().as_deref(), Some("r_weatherForce 2"));
        // A notch up goes back, wrapping round from the first page.
        wheel.open(shown(), 0, now);
        wheel.scrolled(40.0, now);
        assert_eq!(wheel.page(), Some(1));
        // A trackpad's small steps add up to a notch.
        for _ in 0..3 {
            wheel.scrolled(15.0, now);
        }
        assert_eq!(wheel.page(), Some(0));
        // A left click goes back, a right click on; their releases are the wheel's.
        assert!(wheel.button(Button::Right, true, now));
        assert_eq!(wheel.page(), Some(1));
        assert!(wheel.button(Button::Left, true, now));
        assert_eq!(wheel.page(), Some(0));
        wheel.cancel();
        assert!(wheel.button(Button::Right, false, now));
        assert!(wheel.button(Button::Left, false, now));
        // A release whose press came before the wheel opened goes on.
        wheel.open(shown(), 0, now);
        assert!(!wheel.button(Button::Left, false, now));
    }

    #[test]
    fn a_second_wheel_key_changes_page_and_one_page_stays_put() {
        let now = Instant::now();
        let mut wheel = QuickWheel::default();
        wheel.open(shown(), 0, now);
        wheel.moved([0.0, -90.0]);
        wheel.open(shown(), 1, now);
        assert_eq!(wheel.page(), Some(1));
        assert_eq!(wheel.release().as_deref(), Some("r_weatherForce 0"));
        let mut single = shown();
        single.truncate(1);
        wheel.open(single, 0, now);
        wheel.turn(1, now);
        wheel.scrolled(-80.0, now);
        assert_eq!(wheel.page(), Some(0));
    }

    #[test]
    fn every_page_lays_out_its_choices_and_names_them() {
        let now = Instant::now();
        for page in 0..2 {
            let mut wheel = QuickWheel::default();
            let pages = shown();
            let pictured = pages[page]
                .choices
                .iter()
                .filter(|choice| choice.icon.is_some())
                .count();
            let count = pages[page].choices.len();
            wheel.open(pages, page, now);
            wheel.moved([0.0, -100.0]);
            wheel.build([1920.0, 1080.0], now);
            let commands = wheel.draw_list().commands();
            assert_eq!(
                commands
                    .iter()
                    .filter(|c| matches!(c, DrawCommand::TexturedQuad { .. }))
                    .count(),
                pictured
            );
            let texts: Vec<&str> = wheel.canvas.text_runs().collect();
            // The page's name, the highlighted choice's, the neighbours and their
            // buttons, the hint, and the names of choices without pictures.
            assert!(texts.contains(&if page == 0 { "General" } else { "Weather" }));
            let first = if page == 0 {
                "Third person"
            } else {
                "Map's weather"
            };
            assert!(texts.contains(&first), "{texts:?}");
            assert!(texts.contains(&"Scroll to change page"));
            assert!(texts.iter().any(|text| text.contains("Left click")));
            assert!(texts.len() >= 2 + 4 + 1 + count - pictured, "{texts:?}");
            assert!(!wheel.canvas.overflowed());
        }
    }

    #[test]
    fn each_event_posts_its_cue_and_moves_within_a_choice_post_none() {
        let now = Instant::now();
        let mut wheel = QuickWheel::default();
        ui_cues::take_posted();
        // Opening, and moving about the middle, sound nothing.
        wheel.open(shown(), 0, now);
        wheel.moved([5.0, -5.0]);
        assert!(ui_cues::take_posted().is_empty());
        // Out of the middle onto the first choice: the move cue, once.
        wheel.moved([0.0, -60.0]);
        assert_eq!(ui_cues::take_posted(), [Cue::WheelMove]);
        // Moving about within that choice: none.
        wheel.moved([3.0, -10.0]);
        wheel.moved([-6.0, 0.0]);
        wheel.moved([0.0, 40.0]);
        assert!(ui_cues::take_posted().is_empty());
        // On to the next choice (up and right, of eight): once.
        wheel.moved([48.0, -15.0]);
        assert_eq!(wheel.highlighted(), Some(1));
        assert_eq!(ui_cues::take_posted(), [Cue::WheelMove]);
        // Back to the middle sounds nothing; out again to the same choice does.
        wheel.moved([-50.0, 50.0]);
        assert_eq!(wheel.highlighted(), None);
        assert!(ui_cues::take_posted().is_empty());
        wheel.moved([50.0, -50.0]);
        assert_eq!(ui_cues::take_posted(), [Cue::WheelMove]);
        // A page change by scroll, by click or by a second wheel key: the page
        // cue alone, though the pointer now points at another page's choice.
        wheel.scrolled(-40.0, now);
        assert_eq!(ui_cues::take_posted(), [Cue::WheelPage]);
        assert!(wheel.button(Button::Left, true, now));
        assert!(wheel.button(Button::Left, false, now));
        assert_eq!(ui_cues::take_posted(), [Cue::WheelPage]);
        wheel.open(shown(), 1, now);
        assert_eq!(ui_cues::take_posted(), [Cue::WheelPage]);
        // The same page again: nothing.
        wheel.open(shown(), 1, now);
        assert!(ui_cues::take_posted().is_empty());
        // Moving within the choice it points at on the new page: none.
        wheel.moved([2.0, 2.0]);
        assert!(ui_cues::take_posted().is_empty());
        // Letting go on a choice runs it: the run cue.
        assert!(wheel.release().is_some());
        assert_eq!(ui_cues::take_posted(), [Cue::WheelRun]);
        // Letting go on nothing, Escape, or no wheel at all: nothing.
        wheel.open(shown(), 0, now);
        assert_eq!(wheel.release(), None);
        wheel.open(shown(), 0, now);
        wheel.moved([0.0, -90.0]);
        ui_cues::take_posted();
        wheel.cancel();
        assert_eq!(wheel.release(), None);
        assert!(ui_cues::take_posted().is_empty());
        // One page only: turning it sounds nothing.
        let mut single = shown();
        single.truncate(1);
        wheel.open(single, 0, now);
        wheel.scrolled(-40.0, now);
        wheel.cancel();
        assert!(ui_cues::take_posted().is_empty());
    }

    #[test]
    fn the_sounds_switch_silences_every_cue() {
        let now = Instant::now();
        let mut wheel = QuickWheel::default();
        ui_cues::take_posted();
        wheel.set_sounds(false);
        wheel.open(shown(), 0, now);
        wheel.moved([0.0, -60.0]);
        wheel.moved([80.0, 0.0]);
        wheel.scrolled(-40.0, now);
        assert!(wheel.release().is_some());
        assert!(ui_cues::take_posted().is_empty());
    }

    #[test]
    fn a_full_page_of_custom_choices_fits_the_canvas() {
        let choice = ShownChoice {
            label: "A long custom name here".to_owned(),
            command: "say hi".to_owned(),
            icon: None,
            on: true,
        };
        let pages: Vec<ShownPage> = (0..pages::MAX_PAGES)
            .map(|index| ShownPage {
                id: format!("page{index}"),
                name: "Twenty characters ok".to_owned(),
                choices: vec![choice.clone(); MAX_CHOICES],
            })
            .collect();
        let mut wheel = QuickWheel::default();
        let now = Instant::now();
        wheel.open(pages, 3, now);
        wheel.turn(1, now);
        wheel.moved([60.0, 60.0]);
        wheel.build([3840.0, 2160.0], now + Duration::from_millis(50));
        assert!(!wheel.canvas.overflowed());
        assert_eq!(wheel.page(), Some(4));
    }
}
