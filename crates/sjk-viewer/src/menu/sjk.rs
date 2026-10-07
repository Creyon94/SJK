//! The SJK UI (`ui_menuStyle sjk`): SJK's own menus, drawn over the live map in
//! SJK's own type, as SJK's site looks (`site/assets/style.css`). It keeps the
//! feeling of retail's menus (gold for what you choose, holo blue line-work, the
//! turning ring round the emblem) and drops their window frames and boxes:
//! screens are laid out on alignment, thin lines and dark fades at the screen's
//! edges. Design: `docs/sjk-ui.md`.
//!
//! The UI is built screen by screen and becomes the default once done. So far
//! it has its main page ([`home`]); every other screen opens in its classic
//! version ([`super::style::MenuStyle::classic_screens`]).
//!
//! Text is set in two families ([`TextFamily`]): Rajdhani for navigation,
//! titles and numbers, Exo 2 for the rest; both are bundled vector fonts
//! (`crate::text::DISPLAY`, `crate::text::BODY`). Layouts are authored in pixels
//! of a 1080-line screen and scale with the window's height
//! ([`crate::ui_scale::height_scale`]).

pub(crate) mod home;

use super::{ClientMenu, MenuAction, ReturnTarget};
use crate::console::ViewerConsole;
use crate::game_font::SjkFonts;
use crate::menu_widgets::{MenuCanvas, TextFamily};
use crate::text::{TextStyle, TextVertex, UiFont};
use sjk_ui::{Color, DrawCommand, FontWeight, Gradient, Rect, TextAlign};

/// Where an SJK UI screen's text goes this frame.
pub(crate) enum TextTarget<'a> {
    /// The UI's own families, in the player's menu text style.
    Families(SjkFonts<'a>, TextStyle),
    /// Inter, while the families are not loaded (or failed to load).
    Inter(&'a mut Vec<TextVertex>, &'a UiFont),
}

impl TextTarget<'_> {
    fn append(self, canvas: &MenuCanvas, viewport: [f32; 2]) {
        match self {
            Self::Families(fonts, style) => canvas.append_text_families(fonts, viewport, style),
            Self::Inter(vertices, font) => canvas.append_text(vertices, font, viewport),
        }
    }
}

impl ClientMenu {
    /// Draw the SJK UI's main page, with the player and servers it shows read
    /// from `console` and the server list.
    pub(crate) fn append_sjk_home(
        &mut self,
        target: TextTarget<'_>,
        console: Option<&ViewerConsole>,
        viewport: [f32; 2],
    ) {
        let reveal = self.screen_reveal();
        let entries = self.browser.entries();
        let reconnect = console.and_then(|console| console.text_value("cl_reconnectArgs"));
        let last = home::last_server(reconnect);
        let name = console
            .and_then(|console| console.text_value("name"))
            .unwrap_or("Padawan");
        let model = console
            .and_then(|console| console.text_value("model"))
            .and_then(|model| model.split('/').next())
            .filter(|model| !model.is_empty())
            .unwrap_or("kyle");
        let version = super::main_view::version_line();
        let view = home::HomeView {
            name,
            model,
            blade_name: console.map_or("blue", blade_name),
            blade: console.map_or(color::HOLO, blade_color),
            servers: [
                Some(home::ServerLine::find(home::JOF_SERVER, entries)),
                last.map(|address| home::ServerLine::find(address, entries)),
            ],
            refreshing: self.browser.is_refreshing(),
            version: &version,
            seconds: super::art::motion::seconds(),
        };
        home::build(&mut self.ui, viewport, &mut self.home, &view, reveal);
        target.append(&self.ui, viewport);
    }

    /// Whether a last server other than JoF is offered under Play.
    fn sjk_last_server(console: &ViewerConsole) -> bool {
        home::last_server(console.text_value("cl_reconnectArgs")).is_some()
    }

    /// A key on the SJK UI's main page.
    pub(super) fn sjk_home_key(
        &mut self,
        key: winit::keyboard::KeyCode,
        console: &mut ViewerConsole,
    ) -> MenuAction {
        let last = Self::sjk_last_server(console);
        match self.home.key(key, last) {
            Some(action) => self.sjk_home_act(action, console),
            None => MenuAction::None,
        }
    }

    /// The pointer over (or clicking) `token` on the SJK UI's main page.
    pub(super) fn sjk_home_pointer(
        &mut self,
        token: u16,
        activate: bool,
        console: &mut ViewerConsole,
    ) -> MenuAction {
        let last = Self::sjk_last_server(console);
        match self.home.pointer(token, activate, last) {
            Some(action) => self.sjk_home_act(action, console),
            None => MenuAction::None,
        }
    }

    /// Carry out an action of the main page.
    fn sjk_home_act(&mut self, action: home::Action, console: &mut ViewerConsole) -> MenuAction {
        match action {
            home::Action::Join(server) => {
                let address = match server {
                    0 => Some(home::JOF_SERVER.to_owned()),
                    _ => {
                        home::last_server(console.text_value("cl_reconnectArgs")).map(str::to_owned)
                    }
                };
                match address {
                    Some(address) => self.join_address(address),
                    None => MenuAction::None,
                }
            }
            home::Action::Open(destination) => self.open_main_destination(destination, console),
            home::Action::FirstSetup => {
                self.open_quick_setup(console, ReturnTarget::MainMenu);
                MenuAction::None
            }
            home::Action::Quit => MenuAction::Quit,
            home::Action::Stay => MenuAction::None,
        }
    }

    /// Join `address` from the main page: through the password prompt when the
    /// server list says the server has one.
    fn join_address(&mut self, address: String) -> MenuAction {
        let entry = address
            .parse::<std::net::SocketAddr>()
            .ok()
            .and_then(|parsed| {
                self.browser
                    .entries()
                    .iter()
                    .find(|entry| entry.address == parsed)
            });
        self.destination_map = entry.map(|entry| entry.map.clone());
        if entry.is_some_and(|entry| entry.password) {
            // The browser draws the password prompt.
            self.open_browser();
            self.password.clear();
            self.password_target = Some(address);
            return MenuAction::None;
        }
        self.state.connecting(address.clone());
        MenuAction::Connect(address)
    }
}

/// The SJK UI's colours, as on SJK's site. They are display values, like all
/// 2D colours (`crate::ui_target`).
pub(crate) mod color {
    use sjk_ui::Color;

    /// The ground: deep space navy, never pure black.
    pub(crate) const SPACE: Color = Color::new(0.024, 0.039, 0.078, 1.0);
    /// Line-work: rules, ticks, outlines.
    pub(crate) const HOLO: Color = Color::new(0.659, 0.812, 1.0, 1.0);
    /// What you choose: the selection, the one main action of a screen.
    pub(crate) const GOLD: Color = Color::new(0.91, 0.722, 0.29, 1.0);
    /// Gold lit: the chosen item's text and marks.
    pub(crate) const GOLD_BRIGHT: Color = Color::new(1.0, 0.851, 0.478, 1.0);
    /// Text.
    pub(crate) const TEXT: Color = Color::new(0.886, 0.914, 0.965, 1.0);
    /// Secondary text.
    pub(crate) const MUTED: Color = Color::new(0.663, 0.71, 0.796, 1.0);
    /// Quieter still: an item that ends the session.
    pub(crate) const QUIET: Color = Color::new(0.53, 0.573, 0.675, 1.0);
    /// Only for leaving: Quit.
    pub(crate) const EMBER: Color = Color::new(1.0, 0.478, 0.239, 1.0);

    /// `color` at `alpha`.
    pub(crate) const fn alpha(color: Color, alpha: f32) -> Color {
        Color::new(color.r, color.g, color.b, alpha)
    }
}

/// A vertical fade over `rect` from `top` to `bottom`.
pub(crate) fn fade(canvas: &mut MenuCanvas, rect: Rect, top: Color, bottom: Color) {
    let _ = canvas.draw_list_mut().push(DrawCommand::GradientRect {
        rect,
        radius: 0.0,
        gradient: Gradient {
            start: top,
            end: bottom,
            vertical: true,
        },
    });
}

/// A horizontal fade over `rect` from `left` to `right`.
pub(crate) fn fade_across(canvas: &mut MenuCanvas, rect: Rect, left: Color, right: Color) {
    let _ = canvas.draw_list_mut().push(DrawCommand::GradientRect {
        rect,
        radius: 0.0,
        gradient: Gradient {
            start: left,
            end: right,
            vertical: false,
        },
    });
}

/// Text in `family`: the canvas's text, its family set for the run.
#[allow(clippy::too_many_arguments)]
pub(crate) fn text(
    canvas: &mut MenuCanvas,
    family: TextFamily,
    value: std::fmt::Arguments<'_>,
    rect: Rect,
    size: f32,
    color: Color,
    weight: FontWeight,
    align: TextAlign,
) {
    canvas.set_family(family);
    canvas.text_fmt_aligned(value, rect, size, color, weight, 0.0, align);
    canvas.set_family(TextFamily::Body);
}

/// One key cap and what it does, from `x` (window pixels) on the line whose
/// top is `y`; returns the x after it. `s` is the layout scale.
pub(crate) fn key_hint(
    canvas: &mut MenuCanvas,
    keys: &[&str],
    action: &str,
    x: f32,
    y: f32,
    s: f32,
) -> f32 {
    let height = 24.0 * s;
    let mut x = x;
    for key in keys {
        // Rajdhani Bold at 15 is about 8 pixels a character.
        let width = (18.0 + 8.2 * key.len() as f32) * s;
        let cap = Rect::new(x, y, width, height);
        let _ = canvas.draw_list_mut().push(DrawCommand::Border {
            rect: cap,
            radius: 5.0 * s,
            width: s.max(1.0),
            color: color::alpha(color::HOLO, 0.45),
        });
        text(
            canvas,
            TextFamily::Display,
            format_args!("{key}"),
            Rect::new(x, y + 2.0 * s, width, height - 2.0 * s),
            17.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        x += width + 6.0 * s;
    }
    let action_width = (12.0 + 7.6 * action.len() as f32) * s;
    text(
        canvas,
        TextFamily::Body,
        format_args!("{action}"),
        Rect::new(x + 4.0 * s, y + 2.0 * s, action_width, height),
        15.0 * s,
        color::MUTED,
        FontWeight::Regular,
        TextAlign::Start,
    );
    x + 4.0 * s + action_width
}

/// Width [`key_hint`] takes for `keys` and `action`, to right-align a row.
pub(crate) fn key_hint_width(keys: &[&str], action: &str, s: f32) -> f32 {
    let caps: f32 = keys
        .iter()
        .map(|key| (18.0 + 8.2 * key.len() as f32 + 6.0) * s)
        .sum();
    caps + 4.0 * s + (12.0 + 7.6 * action.len() as f32) * s
}

/// The layout scale of `viewport`: 1 at 1080 lines.
pub(crate) fn scale(viewport: [f32; 2]) -> f32 {
    crate::ui_scale::height_scale(viewport[1])
}

/// The colour of the player's first saber blade (`color1`, with JA+'s
/// `cp_sbRGB1` for an RGB blade), for the UI's lit marks.
pub(crate) fn blade_color(console: &crate::console::ViewerConsole) -> Color {
    let index = console
        .text_value("color1")
        .map(str::trim)
        .and_then(|value| {
            let digits = value.bytes().take_while(u8::is_ascii_digit).count();
            value[..digits].parse::<u8>().ok()
        })
        .unwrap_or(4);
    let packed = console.integer_cvar("cp_sbRGB1").unwrap_or(0);
    let rgb = sjk_client::unpack_saber_rgb(u32::try_from(packed).unwrap_or(0));
    crate::player_menu::saber_color(index, rgb)
}

/// What a blade of `color` is called, for the player's line ("blue saber").
pub(crate) fn blade_name(console: &crate::console::ViewerConsole) -> &'static str {
    let index = console
        .text_value("color1")
        .map(str::trim)
        .and_then(|value| value.get(..1))
        .and_then(|digit| digit.parse::<u8>().ok())
        .unwrap_or(4);
    match sjk_client::SaberColor::from_index(index) {
        Ok(sjk_client::SaberColor::Red) => "red",
        Ok(sjk_client::SaberColor::Orange) => "orange",
        Ok(sjk_client::SaberColor::Yellow) => "yellow",
        Ok(sjk_client::SaberColor::Green) => "green",
        Ok(sjk_client::SaberColor::Purple) => "purple",
        Ok(sjk_client::SaberColor::Rgb) => "custom",
        _ => "blue",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_hints_measure_what_they_draw() {
        let mut canvas = MenuCanvas::new();
        canvas.begin_transparent([1920.0, 1080.0]);
        let end = key_hint(&mut canvas, &["Left", "Right"], "choose", 100.0, 900.0, 1.0);
        assert!((end - 100.0 - key_hint_width(&["Left", "Right"], "choose", 1.0)).abs() < 1e-3);
    }

    #[test]
    fn the_blade_follows_the_profile_saber() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar("color1", "1");
        assert_eq!(blade_name(&console), "orange");
        let orange = blade_color(&console);
        assert!(orange.r > orange.b);
        // A hat's suffix on the colour does not change it.
        console.set_cvar("color1", "3tophat");
        assert_eq!(blade_name(&console), "green");
        console.set_cvar("color1", "4");
        let blue = blade_color(&console);
        assert!(blue.b > blue.r);
    }
}
