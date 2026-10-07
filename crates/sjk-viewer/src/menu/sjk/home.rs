//! The SJK UI's main page: SJK's emblem in its turning holo ring over the live
//! map, and the menu along a horizon line beneath it. The chosen item's part of
//! the line ignites like a saber, in the player's blade colour, and the ring's
//! gold arc turns to point at it. Under the line sit the chosen item's actions:
//! under Play, the JoF server and the last server played (when it is another)
//! to join at once, then the server browser and Create game.
//!
//! The keyboard moves along the line with Left and Right, down into the actions
//! with Down (Up comes back) and acts with Enter: on the line, Enter takes the
//! item's first action (Play joins JoF), except Quit, which only offers its
//! question. Escape leaves the actions, then moves to Quit. The pointer chooses
//! an item or an action by hovering it and acts with a click.

use super::{color, fade, fade_across, key_hint, key_hint_width, text};
use crate::menu::MainDestination;
use crate::menu::art::motion;
use crate::menu::emblem::{self, EmblemLayer};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::server_browser::ServerEntry;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign};
use std::net::SocketAddr;
use winit::keyboard::KeyCode;

/// The JoF community's server, offered first under Play.
pub(crate) const JOF_SERVER: &str = "135.125.145.49:29070";

/// Pointer tokens: the items along the line, then the actions under it.
const ITEM_TOKEN: u16 = 0;
const ACTION_TOKEN: u16 = 10;

/// The items along the line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Item {
    Play,
    Character,
    Settings,
    Sjk,
    Quit,
}

impl Item {
    pub(crate) const ALL: [Self; 5] = [
        Self::Play,
        Self::Character,
        Self::Settings,
        Self::Sjk,
        Self::Quit,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Character => "Character",
            Self::Settings => "Settings",
            Self::Sjk => "Sol JK",
            Self::Quit => "Quit",
        }
    }
}

/// What an action under an item does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    /// Join offered server `0` (JoF) or `1` (the last server played).
    Join(usize),
    /// Open a screen the main menu leads to.
    Open(MainDestination),
    /// Open First setup.
    FirstSetup,
    /// Exit to the desktop.
    Quit,
    /// Leave the quit question for Play.
    Stay,
}

/// One action as its line shows it. A server's title and detail come from the
/// server list instead ([`ServerLine`]).
#[derive(Clone, Copy, Debug)]
struct Entry {
    action: Action,
    title: &'static str,
    detail: &'static str,
}

const fn entry(action: Action, title: &'static str, detail: &'static str) -> Entry {
    Entry {
        action,
        title,
        detail,
    }
}

const BROWSE: Entry = entry(
    Action::Open(MainDestination::Browser),
    "Browse servers",
    "Every server, favourites first",
);
const CREATE: Entry = entry(
    Action::Open(MainDestination::CreateGame),
    "Create a game",
    "Bots on this machine",
);
const PLAY: [Entry; 3] = [entry(Action::Join(0), "", ""), BROWSE, CREATE];
const PLAY_WITH_LAST: [Entry; 4] = [
    entry(Action::Join(0), "", ""),
    entry(Action::Join(1), "", ""),
    BROWSE,
    CREATE,
];
const CHARACTER: [Entry; 2] = [
    entry(
        Action::Open(MainDestination::Player),
        "Change character",
        "Name, model, saber and Force",
    ),
    entry(
        Action::Open(MainDestination::Identity),
        "Identity",
        "Your SJK name, bio and badge",
    ),
];
const SETTINGS: [Entry; 4] = [
    entry(
        Action::Open(MainDestination::Settings { tab: 0 }),
        "All settings",
        "Every option, with search",
    ),
    entry(
        Action::Open(MainDestination::Keybinds { category: 0 }),
        "Key bindings",
        "Every key in one list",
    ),
    entry(
        Action::Open(MainDestination::Renderer),
        "Graphics",
        "Image, lighting, shadows, weather",
    ),
    entry(
        Action::FirstSetup,
        "First setup",
        "The settings worth a first look",
    ),
];
const SJK: [Entry; 3] = [
    entry(
        Action::Open(MainDestination::Changelog),
        "What's new",
        "Every release and who made it",
    ),
    entry(
        Action::Open(MainDestination::Update),
        "Update",
        "Check for a newer Sol JK",
    ),
    entry(
        Action::Open(MainDestination::Credits),
        "Credits",
        "The people who make Sol JK",
    ),
];
const QUIT: [Entry; 2] = [
    entry(Action::Quit, "Quit to desktop", "Your settings are saved"),
    entry(Action::Stay, "Stay", "Back to Play"),
];

/// The actions under `item`; `last` says whether a last server other than
/// JoF is known.
fn entries(item: Item, last: bool) -> &'static [Entry] {
    match item {
        Item::Play if last => &PLAY_WITH_LAST,
        Item::Play => &PLAY,
        Item::Character => &CHARACTER,
        Item::Settings => &SETTINGS,
        Item::Sjk => &SJK,
        Item::Quit => &QUIT,
    }
}

/// A server offered under Play: its address and, once the server list has it,
/// what the list says of it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ServerLine<'a> {
    pub(crate) address: &'a str,
    pub(crate) info: Option<&'a ServerEntry>,
}

impl<'a> ServerLine<'a> {
    /// `address` as `entries` (the server list) describe it.
    pub(crate) fn find(address: &'a str, entries: &'a [ServerEntry]) -> Self {
        let parsed = address.trim().parse::<SocketAddr>().ok();
        Self {
            address,
            info: parsed.and_then(|address| entries.iter().find(|entry| entry.address == address)),
        }
    }
}

/// The last server played (`cl_reconnectArgs`) when it is not JoF's.
pub(crate) fn last_server(reconnect: Option<&str>) -> Option<&str> {
    let address = reconnect
        .map(str::trim)
        .filter(|address| !address.is_empty())?;
    let same = |a: &str, b: &str| match (a.parse::<SocketAddr>(), b.parse::<SocketAddr>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a.eq_ignore_ascii_case(b),
    };
    (!same(address, JOF_SERVER)).then_some(address)
}

/// What the page shows of the player and the servers this frame.
pub(crate) struct HomeView<'a> {
    pub(crate) name: &'a str,
    pub(crate) model: &'a str,
    pub(crate) blade_name: &'static str,
    pub(crate) blade: Color,
    /// JoF, then the last server played when it is another.
    pub(crate) servers: [Option<ServerLine<'a>>; 2],
    /// The server list has been asked and has not answered yet.
    pub(crate) refreshing: bool,
    pub(crate) version: &'a str,
    pub(crate) seconds: f64,
}

/// The page's selection and motion.
#[derive(Debug)]
pub(crate) struct Home {
    item: usize,
    /// The action under the item that has the keyboard, when focus left the line.
    focus: Option<usize>,
    /// Where the ring's gold arc points (radians, clockwise from the right),
    /// easing towards the chosen item; `None` until the first frame.
    arc: Option<f32>,
    /// Menu time of the last frame, for the arc's easing.
    last: f64,
    /// Menu time the chosen item's blade began to ignite.
    lit_at: f64,
}

impl Default for Home {
    fn default() -> Self {
        Self {
            item: 0,
            focus: None,
            arc: None,
            last: 0.0,
            lit_at: f64::NEG_INFINITY,
        }
    }
}

/// How long a blade takes to ignite, in seconds.
const IGNITE: f64 = 0.16;

impl Home {
    /// The chosen item.
    pub(crate) fn item(&self) -> Item {
        Item::ALL[self.item.min(Item::ALL.len() - 1)]
    }

    /// The page on `item`, its action `focus` holding the keyboard, at rest,
    /// for the menu snapshots.
    #[cfg(test)]
    pub(crate) fn for_snapshot(item: Item, focus: Option<usize>) -> Self {
        Self {
            item: Item::ALL.iter().position(|each| *each == item).unwrap_or(0),
            focus,
            ..Self::default()
        }
    }

    /// Back on Play with the keyboard on the line, as at start.
    pub(crate) fn reset(&mut self) {
        self.choose(0);
        self.focus = None;
    }

    fn choose(&mut self, item: usize) {
        if item != self.item {
            self.item = item;
            self.lit_at = motion::seconds();
        }
    }

    /// A key on the page; the action it takes, if any. `last` says whether a
    /// last server other than JoF is offered.
    pub(crate) fn key(&mut self, key: KeyCode, last: bool) -> Option<Action> {
        let count = entries(self.item(), last).len();
        match (key, self.focus) {
            (KeyCode::ArrowLeft | KeyCode::KeyA, None) => {
                self.choose((self.item + Item::ALL.len() - 1) % Item::ALL.len());
            }
            (KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::Tab, None) => {
                self.choose((self.item + 1) % Item::ALL.len());
            }
            (KeyCode::ArrowLeft | KeyCode::KeyA, Some(focus)) => {
                self.focus = Some(focus.checked_sub(1).unwrap_or(count - 1));
            }
            (KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::Tab, Some(focus)) => {
                self.focus = Some((focus + 1) % count);
            }
            (KeyCode::ArrowDown | KeyCode::KeyS, None) => self.focus = Some(0),
            (KeyCode::ArrowUp | KeyCode::KeyW, Some(_)) => self.focus = None,
            (KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space, Some(focus)) => {
                return self.act(entries(self.item(), last).get(focus)?.action);
            }
            // Quit asks first: Enter on it only offers the question.
            (KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space, None)
                if self.item() == Item::Quit =>
            {
                self.focus = Some(0);
            }
            (KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space, None) => {
                return self.act(entries(self.item(), last).first()?.action);
            }
            (KeyCode::Escape, Some(_)) => self.focus = None,
            (KeyCode::Escape, None) => self.choose(Item::ALL.len() - 1),
            _ => {}
        }
        None
    }

    /// The pointer over (or clicking) `token`: hovering chooses, a click acts.
    pub(crate) fn pointer(&mut self, token: u16, activate: bool, last: bool) -> Option<Action> {
        let items = ITEM_TOKEN..ITEM_TOKEN + Item::ALL.len() as u16;
        if items.contains(&token) {
            self.choose(usize::from(token - ITEM_TOKEN));
            self.focus = None;
            if activate && self.item() != Item::Quit {
                return self.act(entries(self.item(), last).first()?.action);
            }
            if activate {
                self.focus = Some(0);
            }
            return None;
        }
        let index = usize::from(token.checked_sub(ACTION_TOKEN)?);
        let entry = entries(self.item(), last).get(index)?;
        self.focus = Some(index);
        if activate {
            self.act(entry.action)
        } else {
            None
        }
    }

    /// Carry out `action` as far as the page goes: Stay is the page's own.
    fn act(&mut self, action: Action) -> Option<Action> {
        if action == Action::Stay {
            self.reset();
            return None;
        }
        Some(action)
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

    /// How far the chosen item's blade has ignited at menu time `seconds`.
    fn lit(&self, seconds: f64) -> f32 {
        let t = ((seconds - self.lit_at) / IGNITE).clamp(0.0, 1.0) as f32;
        1.0 - (1.0 - t) * (1.0 - t)
    }
}

/// The page's geometry in window pixels.
struct Layout {
    s: f32,
    width: f32,
    height: f32,
    /// Centre of the ring and the emblem.
    ring: [f32; 2],
    /// Centre x of item `i` is `item_x[i]`.
    item_x: [f32; 5],
    /// The items' baseline, and the line.
    item_bottom: f32,
    line_y: f32,
    /// Top of the actions under the line.
    actions_y: f32,
    margin: f32,
}

/// Distance between the items' centres, in 1080-line pixels.
const ITEM_PITCH: f32 = 230.0;
/// Half the length of a lit blade.
const BLADE_HALF: f32 = 84.0;
/// Widths of a server's action and of any other one, and the gap between them.
const SERVER_WIDTH: f32 = 360.0;
const ACTION_WIDTH: f32 = 260.0;
const ACTION_GAP: f32 = 36.0;

fn layout(viewport: [f32; 2]) -> Layout {
    let s = super::scale(viewport);
    let [width, height] = viewport;
    let cx = width * 0.5;
    Layout {
        s,
        width,
        height,
        ring: [cx, 392.0 * s],
        item_x: std::array::from_fn(|index| cx + (index as f32 - 2.0) * ITEM_PITCH * s),
        item_bottom: 802.0 * s,
        line_y: 834.0 * s,
        actions_y: 872.0 * s,
        margin: 96.0 * s,
    }
}

/// The actions' boxes along their row (window pixels), centred under the
/// line and narrowed together when the window is too narrow for them.
fn action_boxes(layout: &Layout, list: &[Entry]) -> ([Rect; 4], usize) {
    let s = layout.s;
    let count = list.len().min(4);
    let mut widths = [0.0_f32; 4];
    for (width, entry) in widths.iter_mut().zip(list) {
        *width = match entry.action {
            Action::Join(_) => SERVER_WIDTH,
            _ => ACTION_WIDTH,
        };
    }
    let natural = widths.iter().sum::<f32>() + ACTION_GAP * count.saturating_sub(1) as f32;
    let room = (layout.width - 2.0 * layout.margin) / s;
    let fit = (room / natural).min(1.0);
    let mut x = layout.width * 0.5 - natural * fit * s * 0.5;
    let mut boxes = [Rect::new(0.0, 0.0, 0.0, 0.0); 4];
    for (slot, width) in boxes.iter_mut().zip(&widths).take(count) {
        *slot = Rect::new(x, layout.actions_y, width * fit * s, 64.0 * s);
        x += (width + ACTION_GAP) * fit * s;
    }
    (boxes, count)
}

/// Build the page into `canvas` at `reveal` opacity.
pub(crate) fn build(
    canvas: &mut MenuCanvas,
    viewport: [f32; 2],
    home: &mut Home,
    view: &HomeView<'_>,
    reveal: f32,
) {
    let layout = layout(viewport);
    let (s, width, height) = (layout.s, layout.width, layout.height);
    let seconds = view.seconds;
    canvas.begin_transparent(viewport);
    canvas.push_opacity(reveal);
    scrims(canvas, &layout);

    // The emblem in its ring, the sunburst's warmth behind it.
    let [rx, ry] = layout.ring;
    let turn = (seconds * std::f64::consts::TAU / 240.0) as f32;
    emblem::rays(
        canvas,
        EmblemLayer::Sunburst,
        layout.ring,
        380.0 * s,
        -turn * 0.6,
        color::alpha(color::GOLD, 0.16),
    );
    emblem::rays(
        canvas,
        EmblemLayer::Ring,
        layout.ring,
        272.0 * s,
        turn,
        color::alpha(color::HOLO, 0.9),
    );
    emblem::draw(
        canvas,
        Rect::new(rx - 160.0 * s, ry - 160.0 * s, 320.0 * s, 320.0 * s),
        seconds,
    );
    let item = home.item;
    let target = (layout.item_bottom - 26.0 * s - ry).atan2(layout.item_x[item] - rx);
    let arc = home.arc_towards(target, seconds);
    let sweep = 0.42;
    let _ = canvas.draw_list_mut().push(DrawCommand::Arc {
        center: layout.ring,
        radius: 281.0 * s,
        width: 4.5 * s,
        start: arc - sweep * 0.5,
        sweep,
        color: color::GOLD_BRIGHT,
        knockout: None,
    });

    horizon(canvas, &layout);
    let lit = home.lit(seconds);
    blade(
        canvas,
        layout.item_x[item] - BLADE_HALF * s,
        layout.item_x[item] + BLADE_HALF * s,
        layout.line_y,
        s,
        view.blade,
        lit,
    );

    // The items along the line.
    for (index, entry) in Item::ALL.iter().enumerate() {
        let chosen = index == item;
        let size = if chosen { 54.0 } else { 42.0 } * s;
        let colour = match (chosen, entry) {
            (true, _) => Color::new(1.0, 1.0, 1.0, 1.0),
            (false, Item::Quit) => color::QUIET,
            (false, _) => color::MUTED,
        };
        let x = layout.item_x[index];
        let pitch = ITEM_PITCH * s;
        // Rajdhani's baseline lies 0.8 of the size below the top of its line,
        // so every item, chosen or not, stands on the same baseline.
        text(
            canvas,
            TextFamily::Display,
            format_args!("{}", entry.label()),
            Rect::new(
                x - pitch * 0.5,
                layout.item_bottom - size * 0.8,
                pitch,
                size * 1.2,
            ),
            size,
            colour,
            FontWeight::Regular,
            TextAlign::Center,
        );
        canvas.hit_region(
            ITEM_TOKEN + index as u16,
            Rect::new(
                x - pitch * 0.5,
                layout.item_bottom - 80.0 * s,
                pitch,
                104.0 * s,
            ),
        );
    }

    actions(canvas, &layout, home, view);
    player(canvas, &layout, view);
    hints(canvas, &layout, home, view.servers[1].is_some());
    text(
        canvas,
        TextFamily::Body,
        format_args!("{}", view.version),
        Rect::new(layout.margin, height - 58.0 * s, width * 0.5, 22.0 * s),
        15.0 * s,
        color::MUTED,
        FontWeight::Regular,
        TextAlign::Start,
    );
    canvas.pop_opacity();
    let selected = match home.focus {
        Some(focus) => ACTION_TOKEN + focus as u16,
        None => ITEM_TOKEN + item as u16,
    };
    canvas.finish(selected);
}

/// Fades keeping text readable over any map: a light veil over all of it,
/// darker at the top and much darker under the line.
fn scrims(canvas: &mut MenuCanvas, layout: &Layout) {
    let (s, width, height) = (layout.s, layout.width, layout.height);
    let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
        rect: Rect::new(0.0, 0.0, width, height),
        color: color::alpha(color::SPACE, 0.2),
    });
    let space = |alpha| color::alpha(color::SPACE, alpha);
    fade(
        canvas,
        Rect::new(0.0, 0.0, width, 220.0 * s),
        space(0.55),
        space(0.0),
    );
    fade(
        canvas,
        Rect::new(0.0, height * 0.5, width, height * 0.2),
        space(0.0),
        space(0.86),
    );
    fade(
        canvas,
        Rect::new(0.0, height * 0.7, width, height * 0.3),
        space(0.86),
        space(0.97),
    );
}

/// The horizon line across the screen, fading at its ends, with its ticks.
fn horizon(canvas: &mut MenuCanvas, layout: &Layout) {
    let (s, y) = (layout.s, layout.line_y);
    let (left, right) = (layout.margin, layout.width - layout.margin);
    let span = right - left;
    let line = color::alpha(color::HOLO, 0.45);
    let clear = color::alpha(color::HOLO, 0.0);
    let thick = s.max(1.0);
    fade_across(canvas, Rect::new(left, y, span * 0.08, thick), clear, line);
    let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
        rect: Rect::new(left + span * 0.08, y, span * 0.84, thick),
        color: line,
    });
    fade_across(
        canvas,
        Rect::new(right - span * 0.08, y, span * 0.08, thick),
        line,
        clear,
    );
    let step = 48.0 * s;
    let mut x = left + step;
    while x < right - step * 0.5 {
        // Ticks fade with the line towards its ends.
        let edge = ((x - left).min(right - x) / (span * 0.08)).clamp(0.0, 1.0);
        let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
            rect: Rect::new(x, y - 6.0 * s, thick, 13.0 * s),
            color: color::alpha(color::HOLO, 0.32 * edge),
        });
        x += step;
    }
}

/// A saber over the line from `x0` to `x1` at height `y`, `lit` of the way
/// out from its hilt, in `blade`'s colour: a glow, then a white core.
fn blade(canvas: &mut MenuCanvas, x0: f32, x1: f32, y: f32, s: f32, blade: Color, lit: f32) {
    let draw = canvas.draw_list_mut();
    let hilt = Rect::new(x0 - 32.0 * s, y - 9.0 * s, 30.0 * s, 18.0 * s);
    let _ = draw.push(DrawCommand::GradientRect {
        rect: hilt,
        radius: 3.0 * s,
        gradient: sjk_ui::Gradient {
            start: Color::new(0.58, 0.61, 0.66, 1.0),
            end: Color::new(0.14, 0.15, 0.17, 1.0),
            vertical: true,
        },
    });
    let _ = draw.push(DrawCommand::SolidRect {
        rect: Rect::new(x0 - 9.0 * s, y - 9.0 * s, 3.0 * s, 18.0 * s),
        color: Color::new(0.08, 0.08, 0.09, 1.0),
    });
    let length = (x1 - x0) * lit;
    if length <= 0.0 {
        return;
    }
    let glow = |half: f32, extra: f32, alpha: f32| DrawCommand::RoundedRect {
        rect: Rect::new(x0, y - half * s, length + extra * s, half * 2.0 * s),
        radius: half * s,
        color: color::alpha(blade, alpha),
    };
    let _ = draw.push(glow(9.0, 8.0, 0.16));
    let _ = draw.push(glow(5.0, 4.0, 0.5));
    let _ = draw.push(DrawCommand::RoundedRect {
        rect: Rect::new(x0, y - 1.7 * s, length, 3.4 * s),
        radius: 1.7 * s,
        color: Color::new(0.965, 0.985, 1.0, 1.0),
    });
}

/// The chosen item's actions under the line.
fn actions(canvas: &mut MenuCanvas, layout: &Layout, home: &Home, view: &HomeView<'_>) {
    let s = layout.s;
    let list = entries(home.item(), view.servers[1].is_some());
    let (boxes, count) = action_boxes(layout, list);
    for (index, (entry, rect)) in list.iter().zip(boxes).take(count).enumerate() {
        let token = ACTION_TOKEN + index as u16;
        let focused = home.focus == Some(index) || canvas.token_hovered(token);
        if focused {
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    rect.x - 14.0 * s,
                    rect.y - 8.0 * s,
                    rect.width + 20.0 * s,
                    rect.height + 8.0 * s,
                ),
                radius: 10.0 * s,
                color: color::alpha(color::HOLO, 0.1),
            });
            let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    rect.x - 14.0 * s,
                    rect.y - 2.0 * s,
                    3.0 * s,
                    rect.height - 4.0 * s,
                ),
                radius: 1.5 * s,
                color: color::GOLD_BRIGHT,
            });
        }
        // The first action is the page's one gold action; Quit's is ember.
        let title_color = match (index, entry.action) {
            (_, Action::Quit) => color::EMBER,
            (0, _) => color::GOLD_BRIGHT,
            _ if focused => color::TEXT,
            _ => color::alpha(color::TEXT, 0.82),
        };
        let title_rect = Rect::new(rect.x, rect.y, rect.width, 32.0 * s);
        let detail_rect = Rect::new(rect.x, rect.y + 34.0 * s, rect.width, 22.0 * s);
        let detail = |canvas: &mut MenuCanvas, value: std::fmt::Arguments<'_>| {
            text(
                canvas,
                TextFamily::Body,
                value,
                detail_rect,
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        };
        match entry.action {
            Action::Join(server) => {
                let line = view.servers[server];
                let verb = if server == 0 { "Join" } else { "Rejoin" };
                let name = line
                    .and_then(|line| line.info)
                    .map(|info| info.name.as_str())
                    .unwrap_or(if server == 0 {
                        "JoF"
                    } else {
                        "the last server"
                    });
                text(
                    canvas,
                    TextFamily::Display,
                    format_args!("{verb} {name}"),
                    title_rect,
                    28.0 * s,
                    title_color,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                match line.and_then(|line| line.info) {
                    Some(info) => detail(
                        canvas,
                        format_args!(
                            "{} of {} on {}, {} ms",
                            info.players, info.capacity, info.map, info.ping_millis
                        ),
                    ),
                    None if view.refreshing => {
                        detail(canvas, format_args!("Asking the server list..."));
                    }
                    None => detail(
                        canvas,
                        format_args!("{}", line.map_or(JOF_SERVER, |line| line.address)),
                    ),
                }
            }
            _ => {
                text(
                    canvas,
                    TextFamily::Display,
                    format_args!("{}", entry.title),
                    title_rect,
                    28.0 * s,
                    title_color,
                    FontWeight::Regular,
                    TextAlign::Start,
                );
                detail(canvas, format_args!("{}", entry.detail));
            }
        }
        canvas.hit_region(
            token,
            Rect::new(
                rect.x - 14.0 * s,
                rect.y - 8.0 * s,
                rect.width + 20.0 * s,
                rect.height + 8.0 * s,
            ),
        );
        // A rule between the servers and the rest of Play's actions.
        let next_is_other = matches!(entry.action, Action::Join(_))
            && list
                .get(index + 1)
                .is_some_and(|next| !matches!(next.action, Action::Join(_)));
        if next_is_other {
            let x =
                rect.right() + ACTION_GAP * 0.5 * s * (rect.width / (SERVER_WIDTH * s)).min(1.0);
            let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
                rect: Rect::new(x - 18.0 * s, rect.y + 6.0 * s, s.max(1.0), 48.0 * s),
                color: color::alpha(color::HOLO, 0.3),
            });
        }
    }
}

/// The player, top right: their name, then their model and blade.
fn player(canvas: &mut MenuCanvas, layout: &Layout, view: &HomeView<'_>) {
    let s = layout.s;
    let right = layout.width - layout.margin;
    let width = 640.0 * s;
    text(
        canvas,
        TextFamily::Display,
        format_args!("{}", view.name),
        Rect::new(right - width, 64.0 * s, width, 34.0 * s),
        28.0 * s,
        color::TEXT,
        FontWeight::Regular,
        TextAlign::End,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!(
            "{}, {} saber",
            crate::menu::classic::view::Sentence(view.model),
            view.blade_name
        ),
        Rect::new(right - width, 100.0 * s, width, 22.0 * s),
        16.0 * s,
        color::MUTED,
        FontWeight::Regular,
        TextAlign::End,
    );
}

/// The keys of the page, bottom right.
fn hints(canvas: &mut MenuCanvas, layout: &Layout, home: &Home, last: bool) {
    let s = layout.s;
    let first = entries(home.item(), last)
        .first()
        .map_or("open", |entry| match entry.action {
            Action::Join(0) => "join JoF",
            _ => "open",
        });
    let rows: [(&[&str], &str); 3] = match home.focus {
        None => [
            (&["Left", "Right"], "choose"),
            (&["Down"], "actions"),
            (
                &["Enter"],
                if home.item() == Item::Quit {
                    "ask"
                } else {
                    first
                },
            ),
        ],
        Some(_) => [
            (&["Left", "Right"], "choose"),
            (&["Up"], "back"),
            (&["Enter"], "open"),
        ],
    };
    let gap = 26.0 * s;
    let total: f32 = rows
        .iter()
        .map(|(keys, action)| key_hint_width(keys, action, s))
        .sum::<f32>()
        + gap * (rows.len() - 1) as f32;
    let mut x = layout.width - layout.margin - total;
    let y = layout.height - 62.0 * s;
    for (keys, action) in rows {
        x = key_hint(canvas, keys, action, x, y, s) + gap;
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
    fn items_and_actions_fit_every_window() {
        for viewport in VIEWPORTS {
            let layout = layout(viewport);
            let s = layout.s;
            let first = layout.item_x[0] - ITEM_PITCH * s * 0.5;
            let last = layout.item_x[4] + ITEM_PITCH * s * 0.5;
            assert!(first >= 0.0 && last <= viewport[0], "{viewport:?}");
            for list in [
                &PLAY_WITH_LAST[..],
                &PLAY,
                &CHARACTER,
                &SETTINGS,
                &SJK,
                &QUIT,
            ] {
                let (boxes, count) = action_boxes(&layout, list);
                assert_eq!(count, list.len());
                assert!(boxes[0].x >= layout.margin - 0.5, "{viewport:?}");
                assert!(
                    boxes[count - 1].right() <= viewport[0] - layout.margin + 0.5,
                    "{viewport:?}"
                );
                for pair in boxes[..count].windows(2) {
                    assert!(pair[0].right() < pair[1].x, "{viewport:?}");
                }
            }
            // The ring stays above the items' text, the actions under the line.
            assert!(layout.ring[1] + 290.0 * s < layout.item_bottom - 54.0 * s);
            assert!(layout.actions_y > layout.line_y);
            assert!(layout.actions_y + 64.0 * s < viewport[1] - 70.0 * s);
        }
    }

    #[test]
    fn the_keyboard_walks_the_line_and_the_actions() {
        let mut home = Home::default();
        assert_eq!(home.item(), Item::Play);
        // Enter on Play joins JoF.
        assert_eq!(home.key(KeyCode::Enter, false), Some(Action::Join(0)));
        assert_eq!(home.key(KeyCode::ArrowLeft, false), None);
        assert_eq!(home.item(), Item::Quit);
        // Quit only asks: Enter offers its question, a second Enter answers it.
        assert_eq!(home.key(KeyCode::Enter, false), None);
        assert_eq!(home.focus, Some(0));
        assert_eq!(home.key(KeyCode::ArrowRight, false), None);
        assert_eq!(
            home.key(KeyCode::Enter, false),
            None,
            "Stay is the page's own"
        );
        assert_eq!((home.item(), home.focus), (Item::Play, None));
        // Down into Play's actions with a last server: JoF, the last one, browse, create.
        home.key(KeyCode::ArrowDown, true);
        home.key(KeyCode::ArrowRight, true);
        assert_eq!(home.key(KeyCode::Enter, true), Some(Action::Join(1)));
        home.key(KeyCode::ArrowRight, true);
        assert_eq!(
            home.key(KeyCode::Enter, true),
            Some(Action::Open(MainDestination::Browser))
        );
        // Escape leaves the actions, then goes to Quit.
        home.key(KeyCode::Escape, true);
        assert_eq!(home.focus, None);
        home.key(KeyCode::Escape, true);
        assert_eq!(home.item(), Item::Quit);
        home.key(KeyCode::Enter, true);
        assert_eq!(home.key(KeyCode::Enter, true), Some(Action::Quit));
    }

    #[test]
    fn the_pointer_chooses_by_hovering_and_acts_by_clicking() {
        let mut home = Home::default();
        assert_eq!(home.pointer(ITEM_TOKEN + 2, false, false), None);
        assert_eq!(home.item(), Item::Settings);
        assert_eq!(
            home.pointer(ACTION_TOKEN + 1, true, false),
            Some(Action::Open(MainDestination::Keybinds { category: 0 }))
        );
        assert_eq!(home.focus, Some(1));
        // A click on Quit asks; on another item it takes its first action.
        assert_eq!(home.pointer(ITEM_TOKEN + 4, true, false), None);
        assert_eq!(home.focus, Some(0));
        assert_eq!(
            home.pointer(ITEM_TOKEN + 1, true, false),
            Some(Action::Open(MainDestination::Player))
        );
        // An action past the list does nothing.
        assert_eq!(home.pointer(ACTION_TOKEN + 3, true, false), None);
    }

    #[test]
    fn the_last_server_is_offered_only_when_it_is_not_jof() {
        assert_eq!(last_server(None), None);
        assert_eq!(last_server(Some("  ")), None);
        assert_eq!(last_server(Some(JOF_SERVER)), None);
        assert_eq!(last_server(Some(" 135.125.145.49:29070 ")), None);
        assert_eq!(last_server(Some("10.0.0.2:29070")), Some("10.0.0.2:29070"));
        assert_eq!(
            last_server(Some("duel.example.org")),
            Some("duel.example.org")
        );
    }

    #[test]
    fn the_arc_eases_the_short_way_round() {
        let mut home = Home::default();
        // The first frame points straight at the item.
        assert_eq!(home.arc_towards(1.0, 10.0), 1.0);
        let next = home.arc_towards(2.0, 10.05);
        assert!(next > 1.0 && next < 2.0);
        // Across the wrap it turns the short way.
        home.arc = Some(3.0);
        let next = home.arc_towards(-3.0, 10.1);
        assert!(next > 3.0, "{next}");
    }

    #[test]
    fn a_new_item_ignites_its_blade() {
        let mut home = Home::default();
        assert_eq!(home.lit(0.0), 1.0, "the first item is lit from the start");
        home.choose(1);
        let start = home.lit_at;
        assert_eq!(home.lit(start), 0.0);
        assert!(home.lit(start + IGNITE * 0.5) > 0.5);
        assert_eq!(home.lit(start + IGNITE), 1.0);
    }
}
