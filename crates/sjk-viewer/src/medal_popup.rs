//! The new medal pop-up: the first time the client sees a medal in the player's own
//! profile, or a repeatable one's count rise, it shows it once, large, with its name,
//! what it is for and the SJK team's note (`docs/identity.md`, "Medals"). Enter, Escape,
//! Space or a click closes it; several new medals show one after another.
//!
//! When: on the main menu, or when the game menu opens in a match, never over play.
//! A medal that arrives during a match is announced once with a centre print saying
//! where to see it; the pop-up waits for the game menu. While it shows, the menu under
//! it is not drawn and takes no input. What was shown is kept in `medals_seen.txt`
//! (`medals/seen.rs`), so each medal (and each new count) shows once per identity.

use crate::medals::seen::Seen;
use crate::medals::{self, Award};
use crate::menu_widgets::{ButtonStyle, MenuCanvas};
use crate::text::{TextVertex, UiFont};
use sjk_ui::{Color, DrawCommand, FontWeight, InputEvent, Rect, TextAlign, UiEventKind};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

const CLOSE_TOKEN: u16 = 970;
const GOLD: Color = Color::new(1.0, 0.82, 0.25, 1.0);

/// The pop-up's state: the medals waiting, the one showing and what was shown.
pub(crate) struct MedalPopup {
    ui: MenuCanvas,
    queue: VecDeque<Award>,
    current: Option<Award>,
    /// How many were shown before `current` since the pop-up opened, for "2 of 3".
    step: usize,
    seen: Option<Seen>,
    /// The settings folder `medals_seen.txt` is written to.
    directory: PathBuf,
    /// The key and the profile's list last offered, so an unchanged one is skipped.
    last: Option<(String, Vec<sjk_identity::Medal>)>,
    /// The centre print about the waiting medals was given.
    hinted: bool,
}

impl Default for MedalPopup {
    fn default() -> Self {
        Self {
            ui: MenuCanvas::with_capacities(24, 1_024, 96),
            queue: VecDeque::new(),
            current: None,
            step: 0,
            seen: None,
            directory: PathBuf::new(),
            last: None,
            hinted: false,
        }
    }
}

impl MedalPopup {
    /// The medals the player's profile lists now, for the key `key_id`, with the settings
    /// folder: any not shown yet wait for the pop-up.
    pub(crate) fn offer(&mut self, directory: &Path, key_id: &str, list: &[sjk_identity::Medal]) {
        if self
            .last
            .as_ref()
            .is_some_and(|(key, last)| key == key_id && last == list)
        {
            return;
        }
        self.last = Some((key_id.to_owned(), list.to_vec()));
        if self
            .seen
            .as_ref()
            .is_none_or(|seen| seen.key_id() != key_id)
        {
            self.seen = Some(Seen::load(directory, key_id));
            self.queue.clear();
            self.current = None;
        }
        directory.clone_into(&mut self.directory);
        let Some(seen) = &self.seen else {
            return;
        };
        let mut added = false;
        for award in medals::awards(list) {
            if !seen.is_new(&award) {
                continue;
            }
            if self
                .current
                .as_ref()
                .is_some_and(|shown| shown.medal == award.medal && shown.count >= award.count)
            {
                continue;
            }
            // A newer count replaces one still waiting.
            match self
                .queue
                .iter_mut()
                .find(|waiting| waiting.medal == award.medal)
            {
                Some(waiting) => *waiting = award,
                None => {
                    self.queue.push_back(award);
                    added = true;
                }
            }
        }
        if added {
            self.hinted = false;
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.current.is_some()
    }

    /// A medal waits to be shown.
    pub(crate) fn pending(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Show the first medal waiting.
    pub(crate) fn open_next(&mut self) {
        if self.current.is_none() {
            self.step = 0;
            self.current = self.queue.pop_front();
        }
    }

    /// The centre print for a match, once per new medal: what came and where to see it.
    pub(crate) fn hint(&mut self) -> Option<String> {
        if self.hinted || self.queue.is_empty() {
            return None;
        }
        self.hinted = true;
        let names: Vec<&str> = self.queue.iter().map(|award| award.medal.name()).collect();
        let what = if names.len() == 1 {
            format!("New SJK medal: {}", names[0])
        } else {
            format!("New SJK medals: {}", names.join(", "))
        };
        Some(format!("{what}\nOpen the game menu to see it"))
    }

    /// Close the medal showing, count it as seen, and show the next one waiting.
    fn dismiss(&mut self) {
        if let Some(award) = self.current.take()
            && let Some(seen) = &mut self.seen
        {
            seen.mark(&award);
            if let Err(error) = seen.save(&self.directory) {
                crate::log::progress(format_args!(
                    "medals: cannot write {}: {error}",
                    crate::medals::seen::FILE
                ));
            }
        }
        self.current = self.queue.pop_front();
        self.step += 1;
    }

    /// Enter, Escape, Space or the keypad's Enter closes the medal; every other key is
    /// swallowed while it shows.
    pub(crate) fn handle_key(&mut self, event: &KeyEvent) {
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        if matches!(
            event.physical_key,
            PhysicalKey::Code(
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Escape | KeyCode::Space
            )
        ) {
            self.dismiss();
        }
    }

    /// A click anywhere closes the medal.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) {
        if self
            .ui
            .pointer(event)
            .is_some_and(|event| event.kind == UiEventKind::Activate)
        {
            self.dismiss();
        }
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Draw the medal showing over the whole frame; text other overlays appended earlier
    /// this frame is dropped rather than shown through.
    pub(crate) fn append(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let Some(award) = self.current.clone() else {
            return;
        };
        vertices.clear();
        let s = crate::ui_scale::height_scale(viewport[1]);
        let ui = &mut self.ui;
        ui.begin_transparent(viewport);
        let theme = ui.theme();
        let screen = Rect::new(0.0, 0.0, viewport[0], viewport[1]);
        let _ = ui.draw_list_mut().push(DrawCommand::SolidRect {
            rect: screen,
            color: Color::new(0.0, 0.0, 0.0, 0.62),
        });
        ui.hit_region(CLOSE_TOKEN, screen);
        let width = (viewport[0] - 32.0 * s).min(620.0 * s);
        let pad = 32.0 * s;
        let inner = width - pad * 2.0;
        let line = 24.0 * s;
        // Inter at 16 is about 8 pixels a character.
        let chars = (inner / (16.0 * s * 0.5)) as usize;
        let description: Vec<&str> =
            crate::menu::sjk::wrap(award.medal.description(), chars).collect();
        let note: Vec<&str> =
            crate::menu::sjk::wrap(&award.note, chars.saturating_sub(2)).collect();
        let given = award.given();
        let text_height = 40.0 * s
            + description.len() as f32 * line
            + if given.is_empty() { 0.0 } else { 22.0 * s }
            + if note.is_empty() {
                0.0
            } else {
                12.0 * s + note.len() as f32 * line
            };
        let fixed = pad * 2.0 + 34.0 * s + 16.0 * s + text_height + 24.0 * s + 46.0 * s;
        // The picture as large as the window leaves, up to 320.
        let art = (viewport[1] - 48.0 * s - fixed).clamp(120.0 * s, 320.0 * s);
        let height = fixed + art;
        let card = Rect::new(
            (viewport[0] - width) * 0.5,
            ((viewport[1] - height) * 0.5).max(0.0),
            width,
            height,
        );
        ui.panel(card);
        let left = card.x + pad;
        let mut y = card.y + pad;
        let total = self.step + 1 + self.queue.len();
        let kicker = if total > 1 {
            format!("NEW MEDAL   {} OF {total}", self.step + 1)
        } else {
            "NEW MEDAL".to_owned()
        };
        ui.text_aligned(
            &kicker,
            Rect::new(left, y, inner, 22.0 * s),
            14.0 * s,
            GOLD,
            FontWeight::Semibold,
            3.0 * s,
            TextAlign::Center,
        );
        y += 34.0 * s;
        let art_rect = Rect::new(card.x + (width - art) * 0.5, y, art, art);
        // A faint gold disc behind the medallion.
        let glow = art * 0.62;
        let _ = ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: Rect::new(
                art_rect.x + (art - glow) * 0.5,
                art_rect.y + art * 0.36,
                glow,
                glow,
            ),
            radius: glow * 0.5,
            color: Color::new(1.0, 0.82, 0.25, 0.08),
        });
        let _ = ui.draw_list_mut().push(DrawCommand::TexturedQuad {
            rect: art_rect,
            texture: award.medal.art(),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
        });
        y += art + 16.0 * s;
        ui.text_aligned(
            &award.label(),
            Rect::new(left, y, inner, 36.0 * s),
            30.0 * s,
            theme.foreground,
            FontWeight::Semibold,
            0.0,
            TextAlign::Center,
        );
        y += 40.0 * s;
        for text in &description {
            ui.text_aligned(
                text,
                Rect::new(left, y, inner, line),
                16.0 * s,
                theme.muted,
                FontWeight::Regular,
                0.0,
                TextAlign::Center,
            );
            y += line;
        }
        if !given.is_empty() {
            ui.text_aligned(
                &given,
                Rect::new(left, y, inner, 20.0 * s),
                13.0 * s,
                theme.muted,
                FontWeight::Regular,
                0.6 * s,
                TextAlign::Center,
            );
            y += 22.0 * s;
        }
        if !note.is_empty() {
            y += 12.0 * s;
            let last = note.len() - 1;
            for (index, text) in note.iter().enumerate() {
                let open = if index == 0 { "\"" } else { "" };
                let close = if index == last { "\"" } else { "" };
                ui.text_aligned(
                    &format!("{open}{text}{close}"),
                    Rect::new(left, y, inner, line),
                    16.0 * s,
                    theme.foreground,
                    FontWeight::Regular,
                    0.0,
                    TextAlign::Center,
                );
                y += line;
            }
        }
        y += 24.0 * s;
        let button = 180.0 * s;
        let label = if self.queue.is_empty() {
            "Close"
        } else {
            "Next"
        };
        ui.button_styled(
            CLOSE_TOKEN,
            label,
            Rect::new(card.x + (width - button) * 0.5, y, button, 46.0 * s),
            ButtonStyle {
                selected: true,
                enabled: true,
                accent: Some(GOLD),
                badge: None,
            },
        );
        ui.finish(CLOSE_TOKEN);
        ui.append_text(vertices, font, viewport);
    }
}

#[cfg(test)]
impl MedalPopup {
    /// Show `awards` as if the profile had just listed them, for the off-screen snapshots.
    pub(crate) fn preview(awards: Vec<Award>) -> Self {
        let mut popup = Self {
            queue: awards.into(),
            ..Self::default()
        };
        popup.open_next();
        popup
    }
}

impl crate::GpuState {
    /// Hand the pop-up the medals of the player's own profile (twice a second, with the
    /// identity's settings).
    pub(crate) fn offer_medals(&mut self) {
        let Some((key_id, list)) = crate::player_identity::own_medals() else {
            return;
        };
        let Some(console) = self.console.as_ref() else {
            return;
        };
        self.medal_popup
            .offer(console.config_directory(), &key_id, &list);
    }

    /// Open the pop-up when a medal waits and the main menu or the game menu is up, or
    /// announce it once with a centre print during a match. Returns whether the pop-up
    /// draws this frame (not under the console).
    pub(crate) fn prepare_medal_popup(&mut self, console_covers_frame: bool) -> bool {
        let console_open = self
            .console
            .as_ref()
            .is_some_and(crate::console::ViewerConsole::is_open);
        if !self.medal_popup.is_open() && self.medal_popup.pending() {
            let main_menu = self.live_session.is_none()
                && self
                    .client_menu
                    .as_ref()
                    .is_some_and(crate::menu::ClientMenu::on_main_menu);
            let menu = self.game_menu || main_menu;
            if menu && !console_open && !self.text_dialog.is_open() {
                self.medal_popup.open_next();
            } else if !menu
                && self.live_session.is_some()
                && let Some(message) = self.medal_popup.hint()
            {
                self.chat.receive(
                    sjk_client::ServerEventKind::CenterPrint,
                    message,
                    None,
                    std::time::Instant::now(),
                );
            }
        }
        self.medal_popup.is_open() && !console_open && !console_covers_frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::medals::Medal;

    fn wire(id: &str, count: u32) -> sjk_identity::Medal {
        sjk_identity::Medal {
            id: id.to_owned(),
            count,
            awarded: 0,
            note: String::new(),
        }
    }

    #[test]
    fn new_medals_wait_show_one_by_one_and_are_remembered() {
        let directory = tempfile::tempdir().expect("a folder");
        let mut popup = MedalPopup::default();
        popup.offer(
            directory.path(),
            "aa",
            &[wire("early_tester", 1), wire("bug_hunter", 1)],
        );
        assert!(popup.pending() && !popup.is_open());
        assert!(
            popup
                .hint()
                .is_some_and(|text| text.contains("Early Tester, Bug Hunter"))
        );
        assert_eq!(popup.hint(), None, "said once");
        popup.open_next();
        assert_eq!(
            popup.current.as_ref().map(|a| a.medal),
            Some(Medal::EarlyTester)
        );
        popup.dismiss();
        assert_eq!(
            popup.current.as_ref().map(|a| a.medal),
            Some(Medal::BugHunter)
        );
        popup.dismiss();
        assert!(!popup.is_open() && !popup.pending());
        // The same list again, or after a restart, shows nothing.
        popup.offer(
            directory.path(),
            "aa",
            &[wire("early_tester", 1), wire("bug_hunter", 1)],
        );
        assert!(!popup.pending());
        let mut restarted = MedalPopup::default();
        restarted.offer(
            directory.path(),
            "aa",
            &[wire("early_tester", 1), wire("bug_hunter", 1)],
        );
        assert!(!restarted.pending());
        // A repeatable medal given again shows again; another identity sees its own.
        restarted.offer(
            directory.path(),
            "aa",
            &[wire("early_tester", 1), wire("bug_hunter", 2)],
        );
        assert!(restarted.pending());
        restarted.offer(directory.path(), "bb", &[wire("early_tester", 1)]);
        assert_eq!(restarted.queue.len(), 1);
        assert_eq!(restarted.queue[0].medal, Medal::EarlyTester);
    }

    #[test]
    fn unknown_medals_never_show() {
        let directory = tempfile::tempdir().expect("a folder");
        let mut popup = MedalPopup::default();
        popup.offer(directory.path(), "aa", &[wire("from_the_future", 1)]);
        assert!(!popup.pending());
    }
}
