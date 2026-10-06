//! The Identity page: the player's key, their profile at the SJK hub, and the
//! players the hub knows on the server they are on (state and work in
//! `player_identity.rs`; the words that edit it are in `identity_command.rs`).
//!
//! Opened by the in-game SJK menu or the `identity` console command. Like the
//! Update page it lives in the console and is drawn in place of it.

use crate::menu_widgets::{BACK_TOKEN, FormLayout, MenuCanvas, Scrim};
use crate::text::{TextVertex, UiFont};
use sjk_identity::{Snapshot, Status};
use sjk_ui::{Color, FontWeight, InputEvent, Rect, TextAlign, UiEventKind};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Most known players the page lists.
const PLAYERS_SHOWN: usize = 8;
/// Most bio lines the page shows.
const BIO_LINES: usize = 4;

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
}

pub(crate) struct Panel {
    open: bool,
    /// The page opened the console, so closing the page closes it too.
    owns_console: bool,
    ui: MenuCanvas,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

/// What the page knows about the settings and the service.
pub(crate) struct Inputs<'a> {
    pub(crate) enabled: bool,
    pub(crate) hub_url: &'a str,
    pub(crate) key_error: Option<&'a str>,
    pub(crate) snapshot: Option<&'a Snapshot>,
}

/// One known player.
#[derive(Debug, Eq, PartialEq)]
struct PlayerLine {
    slot: u8,
    name: String,
    verified: bool,
}

/// What the page says.
#[derive(Debug, Eq, PartialEq)]
struct View {
    headline: String,
    lines: Vec<String>,
    players: Vec<PlayerLine>,
}

fn plain(headline: &str, lines: &[&str]) -> View {
    View {
        headline: headline.to_owned(),
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        players: Vec::new(),
    }
}

fn view(inputs: &Inputs<'_>) -> View {
    if let Some(error) = inputs.key_error {
        return View {
            lines: vec![
                error.to_owned(),
                "Restore identity.key from a backup, or move it away to start a new identity, then restart SJK."
                    .to_owned(),
            ],
            ..plain("The identity key cannot be used", &[])
        };
    }
    if !inputs.enabled {
        return plain(
            "Identity is off",
            &[
                "No key is made and nothing is sent anywhere.",
                "Turn cl_identity on (Settings > Network) to use it.",
            ],
        );
    }
    let Some(snapshot) = inputs.snapshot else {
        return plain("Starting...", &["Preparing the identity key."]);
    };
    let key = format!("Key id: {}", snapshot.key_id);
    let mut view = match &snapshot.status {
        Status::Disabled => plain("Identity is off", &["The settings were just changed."]),
        Status::NoHub => View {
            lines: vec![
                key,
                "No hub is set (cl_hubUrl), so nothing is sent. The key stays on this PC."
                    .to_owned(),
            ],
            ..plain("Your identity key is ready", &[])
        },
        Status::Registering => View {
            lines: vec![key, format!("Contacting {}", inputs.hub_url)],
            ..plain("Contacting the hub...", &[])
        },
        Status::Failed(error) => View {
            lines: vec![key, error.clone(), "Retrying automatically.".to_owned()],
            ..plain("Cannot reach the hub", &[])
        },
        Status::Online => online(snapshot, key),
    };
    if let Some(notice) = &snapshot.notice {
        view.lines.push(format!("Last change: {notice}"));
    }
    view.players = snapshot
        .players
        .iter()
        .take(PLAYERS_SHOWN)
        .map(|player| PlayerLine {
            slot: player.slot,
            name: if player.name.is_empty() {
                player.claimed_name.clone()
            } else {
                player.name.clone()
            },
            verified: player.verified,
        })
        .collect();
    view
}

fn online(snapshot: &Snapshot, key: String) -> View {
    let mut lines = vec![key];
    let headline = match &snapshot.me {
        Some(me) => {
            lines.push(if me.verified {
                "Verified: the hub's operator vouches for this key.".to_owned()
            } else {
                "Not verified.".to_owned()
            });
            lines.extend(me.bio.lines().take(BIO_LINES).map(str::to_owned));
            if me.name.is_empty() {
                "Registered; you have no name yet".to_owned()
            } else {
                me.name.clone()
            }
        }
        None => "Registered".to_owned(),
    };
    lines.push("Set them in the console: identity name <text>, identity bio <text>".to_owned());
    View {
        headline,
        lines,
        players: Vec::new(),
    }
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            ui: MenuCanvas::with_text_capacity(96),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page; `owns_console` when the console was closed before it.
    pub(crate) fn open(&mut self, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Escape) => PanelAction::Close,
            _ => PanelAction::None,
        }
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind == UiEventKind::Activate && event.token == Some(BACK_TOKEN) {
            PanelAction::Close
        } else {
            PanelAction::None
        }
    }

    /// Draw the page over the whole frame; text other overlays appended earlier
    /// this frame is dropped rather than shown through.
    pub(crate) fn append(
        &mut self,
        inputs: &Inputs<'_>,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        vertices.clear();
        let view = view(inputs);
        let layout = FormLayout::new(viewport);
        let s = layout.scale;
        self.ui.begin_hero(viewport, 1.0, Scrim::Wide);
        self.ui.form_header(
            &layout,
            "SJK   /   IDENTITY",
            "IDENTITY",
            crate::build_info::label(),
        );
        let theme = self.ui.theme();
        let x = layout.margin;
        let top = viewport[1] * 0.17 + 140.0 * s;
        let width = (viewport[0] - x * 2.0).min(820.0 * s);
        let pad = 28.0 * s;
        let line = 24.0 * s;
        let player_rows = view.players.len();
        let players_height = if player_rows == 0 {
            0.0
        } else {
            40.0 * s + player_rows as f32 * 26.0 * s
        };
        let card = Rect::new(
            x,
            top,
            width,
            pad * 2.0 + 52.0 * s + view.lines.len() as f32 * line + players_height,
        );
        self.ui.panel(card);
        let inner = card.width - pad * 2.0;
        self.ui.text(
            &view.headline,
            Rect::new(card.x + pad, card.y + pad, inner, 36.0 * s),
            28.0 * s,
            theme.foreground,
            FontWeight::Semibold,
            0.0,
        );
        let mut y = card.y + pad + 52.0 * s;
        for text in &view.lines {
            self.ui.text(
                text,
                Rect::new(card.x + pad, y, inner, line),
                16.0 * s,
                theme.muted,
                FontWeight::Regular,
                0.2 * s,
            );
            y += line;
        }
        if player_rows > 0 {
            y += 14.0 * s;
            self.ui.text(
                "KNOWN PLAYERS HERE",
                Rect::new(card.x + pad, y, inner, 20.0 * s),
                13.0 * s,
                theme.muted,
                FontWeight::Semibold,
                1.5 * s,
            );
            y += 26.0 * s;
            for player in &view.players {
                let color = if player.verified {
                    Color::new(1.0, 0.82, 0.25, 1.0)
                } else {
                    theme.foreground
                };
                let row = Rect::new(card.x + pad, y, inner, 24.0 * s);
                self.ui.text(
                    &format!("{:>2}   {}", player.slot, player.name),
                    row,
                    18.0 * s,
                    color,
                    FontWeight::Regular,
                    0.0,
                );
                if player.verified {
                    self.ui.text_aligned(
                        "VERIFIED",
                        row,
                        13.0 * s,
                        color,
                        FontWeight::Semibold,
                        1.0 * s,
                        TextAlign::End,
                    );
                }
                y += 26.0 * s;
            }
        }
        self.ui
            .form_footer_actions(&layout, &[("ESC", "Close", BACK_TOKEN)]);
        self.ui.end_hero();
        self.ui.finish(BACK_TOKEN);
        self.ui.append_text(vertices, font, viewport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::{Presence, Profile};
    use std::collections::HashMap;

    fn snapshot(status: Status) -> Snapshot {
        Snapshot {
            status,
            key_id: "0123456789abcdef".to_owned(),
            me: None,
            server: None,
            players: Vec::new(),
            profiles: HashMap::new(),
            notice: None,
            revision: 0,
        }
    }

    fn inputs<'a>(snapshot: Option<&'a Snapshot>) -> Inputs<'a> {
        Inputs {
            enabled: true,
            hub_url: "https://hub.example",
            key_error: None,
            snapshot,
        }
    }

    #[test]
    fn a_broken_key_beats_everything_else() {
        let shown = view(&Inputs {
            key_error: Some("damaged"),
            enabled: false,
            ..inputs(None)
        });
        assert_eq!(shown.headline, "The identity key cannot be used");
        assert_eq!(shown.lines[0], "damaged");
    }

    #[test]
    fn off_says_nothing_is_sent() {
        let shown = view(&Inputs {
            enabled: false,
            ..inputs(None)
        });
        assert_eq!(shown.headline, "Identity is off");
        assert!(shown.lines[0].contains("nothing is sent"));
    }

    #[test]
    fn without_a_hub_the_key_stays_local() {
        let state = snapshot(Status::NoHub);
        let shown = view(&inputs(Some(&state)));
        assert_eq!(shown.headline, "Your identity key is ready");
        assert!(
            shown
                .lines
                .iter()
                .any(|line| line.contains("stays on this PC"))
        );
        assert!(shown.lines[0].contains("0123456789abcdef"));
    }

    #[test]
    fn a_registered_player_sees_name_verification_bio_and_the_players_here() {
        let mut state = snapshot(Status::Online);
        state.me = Some(Profile {
            key_id: "0123456789abcdef".to_owned(),
            key: String::new(),
            name: "Sol".to_owned(),
            bio: "one\ntwo".to_owned(),
            verified: true,
            created: 0,
        });
        state.notice = Some("saved".to_owned());
        state.players = vec![
            Presence {
                slot: 4,
                claimed_name: "^1Fox".to_owned(),
                key_id: "fedcba9876543210".to_owned(),
                name: String::new(),
                verified: false,
            },
            Presence {
                slot: 7,
                claimed_name: "x".to_owned(),
                key_id: "aaaaaaaaaaaaaaaa".to_owned(),
                name: "Kit".to_owned(),
                verified: true,
            },
        ];
        let shown = view(&inputs(Some(&state)));
        assert_eq!(shown.headline, "Sol");
        assert!(shown.lines.iter().any(|line| line.starts_with("Verified")));
        assert!(shown.lines.contains(&"one".to_owned()) && shown.lines.contains(&"two".to_owned()));
        assert!(shown.lines.contains(&"Last change: saved".to_owned()));
        assert_eq!(
            shown.players,
            [
                PlayerLine {
                    slot: 4,
                    name: "^1Fox".to_owned(),
                    verified: false
                },
                PlayerLine {
                    slot: 7,
                    name: "Kit".to_owned(),
                    verified: true
                },
            ]
        );
    }

    #[test]
    fn a_failed_hub_says_why_and_that_it_retries() {
        let state = snapshot(Status::Failed("cannot reach the hub: timed out".to_owned()));
        let shown = view(&inputs(Some(&state)));
        assert_eq!(shown.headline, "Cannot reach the hub");
        assert!(shown.lines.iter().any(|line| line.contains("timed out")));
        assert!(shown.lines.contains(&"Retrying automatically.".to_owned()));
    }

    #[test]
    fn the_page_lists_a_bounded_number_of_players() {
        let mut state = snapshot(Status::Online);
        state.players = (0..20)
            .map(|slot| Presence {
                slot,
                claimed_name: format!("p{slot}"),
                key_id: format!("{slot:016x}"),
                name: format!("p{slot}"),
                verified: false,
            })
            .collect();
        assert_eq!(view(&inputs(Some(&state))).players.len(), PLAYERS_SHOWN);
    }
}
