//! The SJK UI's main page: SJK's emblem in its turning holo ring over the live
//! map, the menu on an arc round the ring's right side, and the servers the
//! player joined last in a column on the right, each joined with one click.
//!
//! The ring's gold arc points at the chosen entry. Play, SJK and Quit open
//! pages of their own on the same ring (the arc swaps to their entries, as
//! retail's pages swapped inside one frame); Character and Settings open their
//! screens. Escape leaves a page for the main one, and on the main one asks to
//! quit.
//!
//! Keys: Up and Down move along the arc, Enter takes the entry, Right (or Tab)
//! moves to the servers and Left (or Escape) back. The pointer chooses by
//! hovering and acts with a click.
//!
//! The page is laid out on a 16:9 frame of 1080-line pixels centred in the
//! window: a wider window shows more map at the sides, a narrower one scales
//! the frame down to fit its width.

use super::recent::Ago;
use super::{Frame, color, fade, fade_across, key_hint, key_hint_width, text};
use crate::menu::MainDestination;
use crate::menu::emblem::{self, EmblemLayer};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{DrawCommand, FontWeight, Rect, TextAlign};
use winit::keyboard::KeyCode;

/// The JoF community's server, suggested while the player has joined none.
pub(crate) const JOF_SERVER: &str = "135.125.145.49:29070";

/// Pointer tokens: the arc's entries, then the servers.
const ENTRY_TOKEN: u16 = 0;
const SERVER_TOKEN: u16 = 20;

/// The pages the ring shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Page {
    Main,
    Play,
    Sjk,
    Quit,
}

impl Page {
    fn entries(self) -> &'static [Entry] {
        match self {
            Self::Main => &MAIN,
            Self::Play => &PLAY,
            Self::Sjk => &SJK,
            Self::Quit => &QUIT,
        }
    }

    /// The main page's entry that opens this page.
    fn parent_entry(self) -> usize {
        match self {
            Self::Main | Self::Play => 0,
            Self::Sjk => 3,
            Self::Quit => 4,
        }
    }

    /// The page's name over its entries; none on the main page.
    fn title(self) -> Option<&'static str> {
        match self {
            Self::Main => None,
            Self::Play => Some("Play"),
            Self::Sjk => Some("SJK"),
            Self::Quit => Some("Quit"),
        }
    }
}

/// What an entry does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Step {
    /// Show another page.
    Page(Page),
    /// Leave the page for the main one.
    Back,
    /// Something the menu carries out ([`Action`]).
    Act(Action),
}

/// What the page asks the menu to do.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    /// Open a screen the main menu leads to.
    Open(MainDestination),
    /// Join server `i` of the column.
    Join(usize),
    /// Exit to the desktop.
    Quit,
}

/// One entry of the arc.
#[derive(Clone, Copy, Debug)]
struct Entry {
    label: &'static str,
    hint: &'static str,
    step: Step,
}

const fn entry(label: &'static str, hint: &'static str, step: Step) -> Entry {
    Entry { label, hint, step }
}

const fn open(destination: MainDestination) -> Step {
    Step::Act(Action::Open(destination))
}

const MAIN: [Entry; 5] = [
    entry(
        "Play",
        "Join a server, or start your own",
        Step::Page(Page::Play),
    ),
    entry(
        "Character",
        "Name, model, saber and Force",
        open(MainDestination::Player),
    ),
    entry(
        "Settings",
        "Every option and key, with search",
        open(MainDestination::Settings { tab: 0 }),
    ),
    entry(
        "SJK",
        "What's new, updates, credits, your identity",
        Step::Page(Page::Sjk),
    ),
    entry("Quit", "Leave SJK", Step::Page(Page::Quit)),
];
const PLAY: [Entry; 3] = [
    entry(
        "Join a server",
        "Every server, favourites first",
        open(MainDestination::Browser),
    ),
    entry(
        "Create a game",
        "A match with bots on this machine",
        open(MainDestination::CreateGame),
    ),
    entry("Back", "", Step::Back),
];
const SJK: [Entry; 5] = [
    entry(
        "What's new",
        "Every release and who made it",
        open(MainDestination::Changelog),
    ),
    entry(
        "Update",
        "Check for a newer SJK",
        open(MainDestination::Update),
    ),
    entry(
        "Credits",
        "The people who make SJK",
        open(MainDestination::Credits),
    ),
    entry(
        "Identity",
        "Your SJK name, bio and badge",
        open(MainDestination::Identity),
    ),
    entry("Back", "", Step::Back),
];
const QUIT: [Entry; 2] = [
    entry(
        "Quit to desktop",
        "Your settings are saved",
        Step::Act(Action::Quit),
    ),
    entry("Stay", "Back to the menu", Step::Back),
];

/// A server in the column, as the page shows it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ServerItem<'a> {
    /// The server's name, with its colour codes; its address when unknown.
    pub(crate) name: &'a str,
    pub(crate) map: &'a str,
    /// Players, capacity and ping, while the server list has the server.
    pub(crate) live: Option<(u16, u16, u32)>,
    /// When the player last joined it; `None` for the suggested JoF server.
    pub(crate) played: Option<Ago>,
}

/// What the page shows of the player and the servers this frame.
pub(crate) struct HomeView<'a> {
    pub(crate) name: &'a str,
    pub(crate) model: &'a str,
    pub(crate) blade_name: &'static str,
    /// The servers joined last, newest first, or the suggested JoF server.
    pub(crate) servers: &'a [ServerItem<'a>],
    /// The build's version, `2026.1007.1`.
    pub(crate) version: &'a str,
    /// A newer release the update check found.
    pub(crate) update: Option<&'a str>,
    pub(crate) seconds: f64,
}

/// Where the keyboard is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Focus {
    Arc,
    Server(usize),
}

/// The page's state: which page of the ring, its chosen entry, the keyboard's
/// place, and the gold arc's motion.
#[derive(Debug)]
pub(crate) struct Home {
    page: Page,
    entry: usize,
    focus: Focus,
    /// Where the gold arc points (radians, clockwise from the right), easing
    /// towards its target; `None` until the first frame.
    arc: Option<f32>,
    /// Menu time of the last frame, for the arc's easing.
    last: f64,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            page: Page::Main,
            entry: 0,
            focus: Focus::Arc,
            arc: None,
            last: 0.0,
        }
    }
}

impl Home {
    /// The page `page` with `entry` chosen and the keyboard on server `server`
    /// (or the arc), for the menu snapshots.
    #[cfg(test)]
    pub(crate) fn for_snapshot(page: Page, entry: usize, server: Option<usize>) -> Self {
        Self {
            page,
            entry,
            focus: server.map_or(Focus::Arc, Focus::Server),
            ..Self::default()
        }
    }

    /// Back on the main page's first entry, the keyboard on the arc.
    pub(crate) fn reset(&mut self) {
        self.page = Page::Main;
        self.entry = 0;
        self.focus = Focus::Arc;
    }

    fn entries(&self) -> &'static [Entry] {
        self.page.entries()
    }

    /// Show `page`, its entry `entry` chosen.
    fn show(&mut self, page: Page, entry: usize) {
        self.page = page;
        self.entry = entry.min(page.entries().len() - 1);
        self.focus = Focus::Arc;
    }

    /// Take the chosen entry.
    fn take(&mut self) -> Option<Action> {
        match self.entries().get(self.entry)?.step {
            // Quit's page opens on Stay, so a stray Enter does not quit.
            Step::Page(Page::Quit) => self.show(Page::Quit, 1),
            Step::Page(page) => self.show(page, 0),
            Step::Back => self.back(),
            Step::Act(action) => return Some(action),
        }
        None
    }

    /// Leave a page for the main one, on the entry that opened it.
    fn back(&mut self) {
        let entry = self.page.parent_entry();
        self.show(Page::Main, entry);
    }

    /// A key on the page, with `servers` in the column; the action it takes.
    pub(crate) fn key(&mut self, key: KeyCode, servers: usize) -> Option<Action> {
        let count = self.entries().len();
        match (key, self.focus) {
            (KeyCode::ArrowUp | KeyCode::KeyW, Focus::Arc) => {
                self.entry = (self.entry + count - 1) % count;
            }
            (KeyCode::ArrowDown | KeyCode::KeyS, Focus::Arc) => {
                self.entry = (self.entry + 1) % count;
            }
            (KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::Tab, Focus::Arc) if servers > 0 => {
                self.focus = Focus::Server(0);
            }
            (KeyCode::ArrowUp | KeyCode::KeyW, Focus::Server(index)) => {
                self.focus = Focus::Server((index + servers - 1) % servers);
            }
            (KeyCode::ArrowDown | KeyCode::KeyS, Focus::Server(index)) => {
                self.focus = Focus::Server((index + 1) % servers);
            }
            (
                KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::Tab | KeyCode::Escape,
                Focus::Server(_),
            ) => {
                self.focus = Focus::Arc;
            }
            (KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space, Focus::Server(index)) => {
                return (index < servers).then_some(Action::Join(index));
            }
            (KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space, Focus::Arc) => {
                return self.take();
            }
            (KeyCode::Escape | KeyCode::Backspace, Focus::Arc) if self.page != Page::Main => {
                self.back();
            }
            (KeyCode::Escape, Focus::Arc) => self.show(Page::Quit, 1),
            _ => {}
        }
        None
    }

    /// The pointer over (or clicking) `token`, with `servers` in the column:
    /// hovering chooses, a click acts.
    pub(crate) fn pointer(&mut self, token: u16, activate: bool, servers: usize) -> Option<Action> {
        let entries = ENTRY_TOKEN..ENTRY_TOKEN + self.entries().len() as u16;
        if entries.contains(&token) {
            self.entry = usize::from(token - ENTRY_TOKEN);
            self.focus = Focus::Arc;
            return if activate { self.take() } else { None };
        }
        let index = usize::from(token.checked_sub(SERVER_TOKEN)?);
        if index >= servers {
            return None;
        }
        self.focus = Focus::Server(index);
        activate.then_some(Action::Join(index))
    }

    /// Where the gold arc points this frame, eased towards `target` since the
    /// last frame at menu time `seconds`.
    fn arc_towards(&mut self, target: f32, seconds: f64) -> f32 {
        let elapsed = (seconds - self.last).clamp(0.0, 0.1) as f32;
        self.last = seconds;
        let current = self.arc.unwrap_or(target);
        let mut off = (target - current).rem_euclid(std::f32::consts::TAU);
        if off > std::f32::consts::PI {
            off -= std::f32::consts::TAU;
        }
        let next = current + off * (1.0 - (-elapsed * 9.0).exp());
        self.arc = Some(next);
        next
    }
}

/// The ring's centre, its radius and the arc's radius, in frame pixels.
const RING: [f32; 2] = [620.0, 540.0];
const RING_RADIUS: f32 = 312.0;
const ARC_RADIUS: f32 = 440.0;
/// Degrees between entries along the arc.
const ARC_STEP: f32 = 16.0;
/// The servers' column: its rule's x, its text's x and width, its top and the
/// height of one server.
const COLUMN_RULE: f32 = 1470.0;
const COLUMN_X: f32 = 1500.0;
const COLUMN_WIDTH: f32 = 324.0;
const COLUMN_TOP: f32 = 236.0;
const SERVER_HEIGHT: f32 = 96.0;

/// Angle (radians) of entry `index` of `count` round the ring.
fn entry_angle(index: usize, count: usize) -> f32 {
    ((index as f32 - (count as f32 - 1.0) * 0.5) * ARC_STEP).to_radians()
}

/// Where entry `index` of `count` starts, in frame pixels: on the arc, at its
/// vertical middle.
fn entry_point(index: usize, count: usize) -> [f32; 2] {
    let angle = entry_angle(index, count);
    [
        RING[0] + ARC_RADIUS * angle.cos(),
        RING[1] + ARC_RADIUS * angle.sin(),
    ]
}

/// Build the page into `canvas` at `reveal` opacity.
pub(crate) fn build(
    canvas: &mut MenuCanvas,
    viewport: [f32; 2],
    home: &mut Home,
    view: &HomeView<'_>,
    reveal: f32,
) {
    let frame = Frame::new(viewport);
    let s = frame.s;
    let seconds = view.seconds;
    canvas.begin_transparent(viewport);
    canvas.push_opacity(reveal);
    scrims(canvas, viewport, &frame);

    // The emblem in its ring, the sunburst's warmth behind it.
    let centre = frame.point(RING[0], RING[1]);
    let turn = (seconds * std::f64::consts::TAU / 240.0) as f32;
    emblem::rays(
        canvas,
        EmblemLayer::Sunburst,
        centre,
        420.0 * s,
        -turn * 0.6,
        color::alpha(color::GOLD, 0.18),
    );
    emblem::rays(
        canvas,
        EmblemLayer::Ring,
        centre,
        RING_RADIUS * s,
        turn,
        color::alpha(color::HOLO, 0.9),
    );
    emblem::draw(
        canvas,
        Rect::new(
            centre[0] - 190.0 * s,
            centre[1] - 190.0 * s,
            380.0 * s,
            380.0 * s,
        ),
        seconds,
    );

    // The gold arc points at the chosen entry, or at the servers.
    let entries = home.entries();
    let target = match home.focus {
        Focus::Arc => entry_angle(home.entry, entries.len()),
        Focus::Server(index) => {
            let y = COLUMN_TOP + 64.0 + (index as f32 + 0.5) * SERVER_HEIGHT;
            (y - RING[1]).atan2(COLUMN_X - RING[0])
        }
    };
    let arc = home.arc_towards(target, seconds);
    let sweep = 0.42;
    let _ = canvas.draw_list_mut().push(DrawCommand::Arc {
        center: centre,
        radius: (RING_RADIUS + 6.0) * s,
        width: 6.0 * s,
        start: arc - sweep * 0.5,
        sweep,
        color: color::GOLD_BRIGHT,
        knockout: None,
    });
    // A short beam from the arc towards its entry.
    if home.focus == Focus::Arc {
        let inner = RING_RADIUS + 12.0;
        let outer = ARC_RADIUS - 18.0;
        let steps = 12;
        for step in 0..steps {
            let t = step as f32 / steps as f32;
            let r = inner + (outer - inner) * t;
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    centre[0] + r * arc.cos() * s - 1.2 * s,
                    centre[1] + r * arc.sin() * s - 1.2 * s,
                    2.4 * s,
                    2.4 * s,
                ),
                radius: 1.2 * s,
                color: color::alpha(color::GOLD_BRIGHT, 0.85 * (1.0 - t)),
            });
        }
    }

    page_title(canvas, &frame, home);
    for (index, entry) in entries.iter().enumerate() {
        let chosen = index == home.entry;
        let lit = chosen && home.focus == Focus::Arc;
        let size = if chosen { 68.0 } else { 44.0 };
        let colour = match (lit, chosen, entry.step) {
            // Leaving is the one thing in ember.
            (true, _, Step::Act(Action::Quit)) => color::EMBER,
            (true, _, _) => color::GOLD_BRIGHT,
            (false, true, _) => color::TEXT,
            (false, false, Step::Back) => color::QUIET,
            (false, false, Step::Page(Page::Quit)) => color::QUIET,
            (false, false, _) => color::MUTED,
        };
        let [x, y] = entry_point(index, entries.len());
        text(
            canvas,
            TextFamily::Display,
            format_args!("{}", entry.label),
            frame.rect(x, y - size * 0.62, 640.0, size * 1.2),
            size * s,
            colour,
            FontWeight::Regular,
            TextAlign::Start,
        );
        if chosen && !entry.hint.is_empty() {
            text(
                canvas,
                TextFamily::Body,
                format_args!("{}", entry.hint),
                frame.rect(x + 2.0, y + 32.0, 520.0, 24.0),
                18.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        let pitch = ARC_RADIUS * ARC_STEP.to_radians();
        canvas.hit_region(
            ENTRY_TOKEN + index as u16,
            frame.rect(x - 16.0, y - pitch * 0.5, COLUMN_RULE - x - 40.0, pitch),
        );
    }

    servers(canvas, &frame, home, view);
    player(canvas, &frame, view);
    hints(canvas, &frame, home, !view.servers.is_empty());
    version(canvas, &frame, view);
    canvas.pop_opacity();
    let selected = match home.focus {
        Focus::Arc => ENTRY_TOKEN + home.entry as u16,
        Focus::Server(index) => SERVER_TOKEN + index as u16,
    };
    canvas.finish(selected);
}

/// Fades keeping text readable over any map: dark on the left behind the ring
/// and the arc, lighter in the middle, dark again behind the servers, and at
/// the top and bottom.
fn scrims(canvas: &mut MenuCanvas, viewport: [f32; 2], frame: &Frame) {
    let [width, height] = viewport;
    let space = |alpha| color::alpha(color::SPACE, alpha);
    // The frame's stops, stretched to the window's edges.
    let x = |frame_x: f32| frame.point(frame_x, 0.0)[0];
    let stops = [
        (0.0, 0.92),
        (x(730.0), 0.78),
        (x(1190.0), 0.35),
        (x(1400.0), 0.62),
        (width, 0.78),
    ];
    for pair in stops.windows(2) {
        let ((left, from), (right, to)) = (pair[0], pair[1]);
        if right > left {
            fade_across(
                canvas,
                Rect::new(left, 0.0, right - left, height),
                space(from),
                space(to),
            );
        }
    }
    fade(
        canvas,
        Rect::new(0.0, 0.0, width, height * 0.22),
        space(0.55),
        space(0.0),
    );
    fade(
        canvas,
        Rect::new(0.0, height * 0.72, width, height * 0.28),
        space(0.0),
        space(0.85),
    );
}

/// A sub-page's name over its first entry.
fn page_title(canvas: &mut MenuCanvas, frame: &Frame, home: &Home) {
    let Some(title) = home.page.title() else {
        return;
    };
    let count = home.entries().len();
    let angle = entry_angle(0, count) - (ARC_STEP * 1.1).to_radians();
    let x = RING[0] + ARC_RADIUS * angle.cos();
    let y = RING[1] + ARC_RADIUS * angle.sin();
    text(
        canvas,
        TextFamily::Display,
        format_args!("{title}"),
        frame.rect(x, y - 16.0, 400.0, 30.0),
        24.0 * frame.s,
        color::alpha(color::HOLO, 0.85),
        FontWeight::Semibold,
        TextAlign::Start,
    );
}

/// The servers' column: the ones joined last, or the suggested JoF server.
fn servers(canvas: &mut MenuCanvas, frame: &Frame, home: &Home, view: &HomeView<'_>) {
    let s = frame.s;
    let count = view.servers.len();
    let suggested = view
        .servers
        .first()
        .is_some_and(|server| server.played.is_none());
    let bottom = COLUMN_TOP + 64.0 + count.max(1) as f32 * SERVER_HEIGHT;
    let rule = color::alpha(color::HOLO, 0.6);
    let clear = color::alpha(color::HOLO, 0.0);
    let rule_top = COLUMN_TOP - 10.0;
    let height = bottom - rule_top;
    fade(
        canvas,
        frame.rect(COLUMN_RULE, rule_top, 2.0, height * 0.18),
        clear,
        rule,
    );
    let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
        rect: frame.rect(COLUMN_RULE, rule_top + height * 0.18, 2.0, height * 0.64),
        color: rule,
    });
    fade(
        canvas,
        frame.rect(COLUMN_RULE, rule_top + height * 0.82, 2.0, height * 0.18),
        rule,
        clear,
    );
    text(
        canvas,
        TextFamily::Display,
        format_args!(
            "{}",
            if suggested {
                "Start here"
            } else {
                "Recent servers"
            }
        ),
        frame.rect(COLUMN_X, COLUMN_TOP, COLUMN_WIDTH, 34.0),
        28.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    for (index, server) in view.servers.iter().enumerate() {
        let top = COLUMN_TOP + 64.0 + index as f32 * SERVER_HEIGHT;
        let token = SERVER_TOKEN + index as u16;
        let focused = home.focus == Focus::Server(index) || canvas.token_hovered(token);
        let target = frame.rect(
            COLUMN_X - 14.0,
            top - 8.0,
            COLUMN_WIDTH + 24.0,
            SERVER_HEIGHT - 8.0,
        );
        if focused {
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: target,
                radius: 10.0 * s,
                color: color::alpha(color::HOLO, 0.1),
            });
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: frame.rect(COLUMN_X - 14.0, top - 2.0, 3.0, SERVER_HEIGHT - 20.0),
                radius: 1.5 * s,
                color: color::GOLD_BRIGHT,
            });
        }
        text(
            canvas,
            TextFamily::Display,
            format_args!("{}", server.name),
            frame.rect(COLUMN_X, top, COLUMN_WIDTH, 32.0),
            27.0 * s,
            if focused {
                color::GOLD_BRIGHT
            } else {
                color::TEXT
            },
            FontWeight::Regular,
            TextAlign::Start,
        );
        let detail = frame.rect(COLUMN_X, top + 34.0, COLUMN_WIDTH, 22.0);
        let quiet = frame.rect(COLUMN_X, top + 56.0, COLUMN_WIDTH, 20.0);
        let body = |canvas: &mut MenuCanvas, rect, colour, value: std::fmt::Arguments<'_>| {
            text(
                canvas,
                TextFamily::Body,
                value,
                rect,
                16.0 * s,
                colour,
                FontWeight::Regular,
                TextAlign::Start,
            );
        };
        match server.live {
            Some((players, capacity, ping)) => body(
                canvas,
                detail,
                color::MUTED,
                format_args!("{players} of {capacity} on {}, {ping} ms", server.map),
            ),
            None if server.map.is_empty() => {
                body(
                    canvas,
                    detail,
                    color::MUTED,
                    format_args!("Not in the server list"),
                );
            }
            None => body(canvas, detail, color::MUTED, format_args!("{}", server.map)),
        }
        match server.played {
            Some(ago) => body(canvas, quiet, color::QUIET, format_args!("Played {ago}")),
            None => body(
                canvas,
                quiet,
                color::QUIET,
                format_args!("The JoF community's server"),
            ),
        }
        canvas.hit_region(token, target);
    }
}

/// The player, bottom left: a gold ring with their initial, their name, and
/// their model and blade.
fn player(canvas: &mut MenuCanvas, frame: &Frame, view: &HomeView<'_>) {
    let s = frame.s;
    let _ = canvas.draw_list_mut().push(DrawCommand::Arc {
        center: frame.point(122.0, 990.0),
        radius: 25.0 * s,
        width: 2.0 * s,
        start: 0.0,
        sweep: std::f32::consts::TAU,
        color: color::alpha(color::GOLD, 0.85),
        knockout: None,
    });
    let initial = initial(view.name);
    text(
        canvas,
        TextFamily::Display,
        format_args!("{initial}"),
        frame.rect(96.0, 972.0, 52.0, 36.0),
        30.0 * s,
        color::GOLD_BRIGHT,
        FontWeight::Semibold,
        TextAlign::Center,
    );
    text(
        canvas,
        TextFamily::Display,
        format_args!("{}", view.name),
        frame.rect(166.0, 962.0, 560.0, 30.0),
        26.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::Start,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!(
            "{}, {} saber",
            crate::menu::classic::view::Sentence(view.model),
            view.blade_name
        ),
        frame.rect(166.0, 994.0, 560.0, 22.0),
        16.0 * s,
        color::MUTED,
        FontWeight::Regular,
        TextAlign::Start,
    );
}

/// The first letter of `name` past its colour codes and clan tags' symbols,
/// in capitals; `S` for a name with none.
fn initial(name: &str) -> char {
    let mut characters = name.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '^' && characters.peek().is_some_and(char::is_ascii_digit) {
            characters.next();
            continue;
        }
        if character.is_alphanumeric() {
            return character.to_ascii_uppercase();
        }
    }
    'S'
}

/// The keys of the page, bottom centre.
fn hints(canvas: &mut MenuCanvas, frame: &Frame, home: &Home, servers: bool) {
    let s = frame.s;
    let arc_back = if home.page == Page::Main {
        "quit"
    } else {
        "back"
    };
    let rows: [(&[&str], &str); 3] = match home.focus {
        Focus::Arc if servers => [
            (&["Up", "Down"], "turn the ring"),
            (&["Enter"], "open"),
            (&["Right"], "servers"),
        ],
        Focus::Arc => [
            (&["Up", "Down"], "turn the ring"),
            (&["Enter"], "open"),
            (&["Esc"], arc_back),
        ],
        Focus::Server(_) => [
            (&["Up", "Down"], "choose"),
            (&["Enter"], "join"),
            (&["Left"], "back"),
        ],
    };
    let gap = 28.0 * s;
    let total: f32 = rows
        .iter()
        .map(|(keys, action)| key_hint_width(keys, action, s))
        .sum::<f32>()
        + gap * (rows.len() - 1) as f32;
    let [centre, y] = frame.point(960.0, 1004.0);
    let mut x = centre - total * 0.5;
    for (keys, action) in rows {
        x = key_hint(canvas, keys, action, x, y, s) + gap;
    }
}

/// The version, bottom right, and the newer release the update check found.
fn version(canvas: &mut MenuCanvas, frame: &Frame, view: &HomeView<'_>) {
    let s = frame.s;
    text(
        canvas,
        TextFamily::Display,
        format_args!("SJK {}", view.version),
        frame.rect(1224.0, 962.0, 600.0, 28.0),
        22.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::End,
    );
    if let Some(update) = view.update {
        text(
            canvas,
            TextFamily::Body,
            format_args!("SJK {update} is ready to install"),
            frame.rect(1224.0, 994.0, 600.0, 22.0),
            16.0 * s,
            color::GOLD_BRIGHT,
            FontWeight::Regular,
            TextAlign::End,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEWPORTS: [[f32; 2]; 6] = [
        [1_920.0, 1_080.0],
        [3_840.0, 2_160.0],
        [2_560.0, 1_080.0],
        [1_440.0, 1_080.0],
        [1_280.0, 1_024.0],
        [1_024.0, 768.0],
    ];

    #[test]
    fn the_frame_fits_every_window_and_keeps_its_proportions() {
        for viewport in VIEWPORTS {
            let frame = Frame::new(viewport);
            let corner = frame.point(1920.0, 1080.0);
            assert!(
                frame.origin[0] >= -0.01 && frame.origin[1] >= -0.01,
                "{viewport:?}"
            );
            assert!(corner[0] <= viewport[0] + 0.01 && corner[1] <= viewport[1] + 0.01);
            // Centred both ways.
            assert!((frame.origin[0] - (viewport[0] - corner[0])).abs() < 0.01);
        }
        // 16:9 fills the window; 4:3 scales the frame to its width.
        assert_eq!(Frame::new([3_840.0, 2_160.0]).s, 2.0);
        assert_eq!(Frame::new([1_440.0, 1_080.0]).s, 0.75);
    }

    #[test]
    fn the_arc_clears_the_ring_and_the_column() {
        for page in [Page::Main, Page::Play, Page::Sjk, Page::Quit] {
            let count = page.entries().len();
            for index in 0..count {
                let [x, y] = entry_point(index, count);
                assert!(x > RING[0] + RING_RADIUS + 60.0, "{page:?} {index}");
                // The widest label at the chosen size ("Quit to desktop", about
                // 400 pixels of Rajdhani at 68) stays short of the column.
                assert!(x + 400.0 < COLUMN_RULE, "{page:?} {index}");
                assert!(y > 120.0 && y < 900.0, "{page:?} {index}");
            }
        }
    }

    // Four servers end above the corners' text.
    const _: () = assert!(COLUMN_TOP + 64.0 + 4.0 * SERVER_HEIGHT < 940.0);

    #[test]
    fn pages_open_and_close_on_the_ring() {
        let mut home = Home::default();
        // Play opens its page; its Back returns to Play.
        assert_eq!(home.key(KeyCode::Enter, 0), None);
        assert_eq!(home.page, Page::Play);
        assert_eq!(
            home.key(KeyCode::Enter, 0),
            Some(Action::Open(MainDestination::Browser))
        );
        home.key(KeyCode::ArrowUp, 0);
        assert_eq!(home.key(KeyCode::Enter, 0), None, "Back");
        assert_eq!((home.page, home.entry), (Page::Main, 0));
        // Escape on the main page asks to quit, on Stay; Escape again leaves.
        home.key(KeyCode::Escape, 0);
        assert_eq!((home.page, home.entry), (Page::Quit, 1));
        home.key(KeyCode::Escape, 0);
        assert_eq!((home.page, home.entry), (Page::Main, 4));
        // Quit to desktop.
        home.key(KeyCode::Enter, 0);
        home.key(KeyCode::ArrowUp, 0);
        assert_eq!(home.key(KeyCode::Enter, 0), Some(Action::Quit));
        // Settings opens its screen straight away.
        home.reset();
        home.key(KeyCode::ArrowDown, 0);
        home.key(KeyCode::ArrowDown, 0);
        assert_eq!(
            home.key(KeyCode::Enter, 0),
            Some(Action::Open(MainDestination::Settings { tab: 0 }))
        );
    }

    #[test]
    fn the_keyboard_and_pointer_reach_the_servers() {
        let mut home = Home::default();
        // No servers: Right stays on the arc.
        home.key(KeyCode::ArrowRight, 0);
        assert_eq!(home.focus, Focus::Arc);
        home.key(KeyCode::ArrowRight, 3);
        assert_eq!(home.focus, Focus::Server(0));
        home.key(KeyCode::ArrowUp, 3);
        assert_eq!(home.key(KeyCode::Enter, 3), Some(Action::Join(2)));
        home.key(KeyCode::Escape, 3);
        assert_eq!(home.focus, Focus::Arc);
        assert_eq!(
            home.page,
            Page::Main,
            "Escape left the column, not the page"
        );
        // Hovering a server chooses it, a click joins it.
        assert_eq!(home.pointer(SERVER_TOKEN + 1, false, 3), None);
        assert_eq!(home.focus, Focus::Server(1));
        assert_eq!(
            home.pointer(SERVER_TOKEN + 1, true, 3),
            Some(Action::Join(1))
        );
        assert_eq!(home.pointer(SERVER_TOKEN + 3, true, 3), None);
        // Hovering an entry chooses it; a click on SJK opens its page.
        home.pointer(ENTRY_TOKEN + 3, false, 3);
        assert_eq!((home.entry, home.focus), (3, Focus::Arc));
        assert_eq!(home.pointer(ENTRY_TOKEN + 3, true, 3), None);
        assert_eq!(home.page, Page::Sjk);
        assert_eq!(
            home.pointer(ENTRY_TOKEN + 2, true, 3),
            Some(Action::Open(MainDestination::Credits))
        );
    }

    #[test]
    fn the_arc_eases_the_short_way_round() {
        let mut home = Home::default();
        assert_eq!(home.arc_towards(1.0, 10.0), 1.0);
        let next = home.arc_towards(2.0, 10.05);
        assert!(next > 1.0 && next < 2.0);
        home.arc = Some(3.0);
        assert!(home.arc_towards(-3.0, 10.1) > 3.0);
    }

    #[test]
    fn the_initial_skips_colour_codes_and_symbols() {
        assert_eq!(initial("^5JoF^7 Jedi"), 'J');
        assert_eq!(initial("{JoF}solol"), 'J');
        assert_eq!(initial("^1^2"), 'S');
        assert_eq!(initial("sol"), 'S');
    }
}
