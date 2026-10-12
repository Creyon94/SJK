//! The SJK UI Character page's model turn and hat and cape picker (Sol's request,
//! 10/10/2026: "rotate model with mouse click, better cosmetic picker";
//! `docs/sjk-ui.md`, Character).
//!
//! - **Turn:** opened from a game, on every page (Character, Saber and Force),
//!   dragging over the model turns it: the live preview's camera goes round it
//!   ([`PlayerMenu::sjk_model_preview`]). On the menu map's stage it does not turn
//!   (Creyon's tests, 12/10/2026: a raised blade turned end on to the stage's camera
//!   looked far too long). The turn eases to where it is sent (the picker's capes turn
//!   the model's back to the camera), holds from page to page, and faces front when
//!   the screen opens again. Under the model's feet, in the look of the screen's keys,
//!   "Drag" in a key cap says it turns ("Drag turn the model"), on every page.
//! - **Picker:** Enter or a click on the Hat or Cape row opens a picker in place of
//!   the model grid: Hats and Capes as two segments, None and every installed piece as
//!   named chips three to a line, the one worn ringed gold, scrolled by the wheel.
//!   A click (or Enter on the chip the arrows chose) wears it at once, as the rows
//!   did; Tab switches between hats and capes, Escape or Done closes it.

use super::*;
use crate::menu::sjk::{Frame, color, kit, text};
use crate::menu_widgets::TextFamily;
use sjk_client::CosmeticSlot;
use sjk_ui::{DrawCommand, FontWeight, TextAlign, UiEventKind};
use winit::keyboard::KeyCode;

/// The area over the model that turns it when dragged.
pub(super) const TURN_TOKEN: u16 = 2_400;
/// The Hats and Capes segments answer to `SLOT_BASE` and `SLOT_BASE + 1`.
const SLOT_BASE: u16 = 2_410;
/// The picker's Done button.
const DONE_TOKEN: u16 = 2_412;
/// The chips' wheel area.
const SCROLL_TOKEN: u16 = 2_413;
/// Chip `i` on show answers to `CHIP_BASE + i`.
const CHIP_BASE: u16 = 2_420;
/// The most chips on show at once.
const MAX_CHIPS: usize = 60;
/// Degrees the model turns for a window pixel dragged (a full turn is about two
/// widths of the area at 1080 lines).
const DEGREES_PER_PIXEL: f32 = 0.45;
/// How fast an eased turn closes on where it is sent (per second).
const EASE: f32 = 9.0;
/// The picker's chips: three to a line, their height and the gap between them.
const CHIP_COLUMNS: usize = 3;
const CHIP_HEIGHT: f32 = 44.0;
const CHIP_GAP: f32 = 8.0;
/// The segments' and Done's line at the picker's top.
const HEAD: f32 = 52.0;

/// How far the model is turned from facing the camera, in degrees.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Turn {
    shown: f32,
    target: f32,
    /// Menu clock seconds of the last frame, for the easing.
    at: Option<f64>,
}

impl Turn {
    /// Turn by a drag of `pixels` (window pixels, rightwards positive): at once.
    pub(super) fn drag(&mut self, pixels: f32) {
        self.target += pixels * DEGREES_PER_PIXEL;
        self.shown = self.target;
        self.keep_small();
    }

    /// Ease to `degrees` (0 faces the camera, 180 shows the back), the short way.
    pub(super) fn face(&mut self, degrees: f32) {
        self.target += (degrees - self.target + 180.0).rem_euclid(360.0) - 180.0;
    }

    /// Ease towards the target over the time since the last frame (`seconds`, the
    /// menu clock).
    pub(super) fn advance(&mut self, seconds: f64) {
        let elapsed = self.at.map_or(0.0, |at| (seconds - at).clamp(0.0, 0.25)) as f32;
        self.at = Some(seconds);
        let gap = self.target - self.shown;
        self.shown = if gap.abs() < 0.05 {
            self.target
        } else {
            self.shown + gap * (1.0 - (-EASE * elapsed).exp())
        };
        self.keep_small();
    }

    /// Face the camera at once.
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(super) fn degrees(self) -> f32 {
        self.shown
    }

    /// Keep both angles within a turn of zero, together.
    fn keep_small(&mut self) {
        let turns = (self.target / 360.0).trunc();
        if turns != 0.0 {
            self.target -= turns * 360.0;
            self.shown -= turns * 360.0;
        }
    }
}

/// The open picker: the slot it lists, the chip the keys chose (0 is None, `i + 1`
/// piece `i`) and the first line on show.
#[derive(Clone, Copy, Debug)]
pub(super) struct Picker {
    pub(super) slot: CosmeticSlot,
    cursor: usize,
    first_line: usize,
    /// Scroll to the cursor on the next frame (after the keys moved it).
    follow: bool,
}

/// What the model faces with the picker on `slot`: the front for a hat, the back for
/// a cape.
fn facing(slot: CosmeticSlot) -> f32 {
    match slot {
        CosmeticSlot::Hat => 0.0,
        CosmeticSlot::Cape => 180.0,
    }
}

impl PlayerMenu {
    /// Whether the screen turns the model with the pointer: the SJK UI's, opened from a
    /// game, on each of its pages.
    pub(super) fn model_turns(&self) -> bool {
        self.is_sjk() && self.return_target == ReturnTarget::InGame
    }

    /// The model's turn where the page turns it ([`Self::model_turns`]), in degrees;
    /// 0 elsewhere.
    pub(crate) fn model_turn(&self) -> f32 {
        if self.model_turns() {
            self.turn.degrees()
        } else {
            0.0
        }
    }

    /// Open the picker on `slot`, the chip of what is worn chosen.
    pub(super) fn open_outfit(&mut self, slot: CosmeticSlot) {
        self.outfit = Some(Picker {
            slot,
            cursor: self
                .cosmetics
                .worn_position(slot)
                .map_or(0, |index| index + 1),
            first_line: 0,
            follow: true,
        });
        self.turn.face(facing(slot));
    }

    /// Close the picker, the model turned to the front again.
    pub(super) fn close_outfit(&mut self) {
        if self.outfit.take().is_some() {
            self.turn.face(0.0);
        }
    }

    /// Chips in `slot`'s picker: None and each installed piece.
    fn chip_count(&self, slot: CosmeticSlot) -> usize {
        self.cosmetics.count(slot) + 1
    }

    /// Wear chip `chip` of the picker on `slot` (0 takes the piece off).
    fn wear_chip(&mut self, console: &mut ViewerConsole, slot: CosmeticSlot, chip: usize) {
        self.cosmetics.wear(console, slot, chip.checked_sub(1));
    }

    /// A key while the picker is open; whether it took it. The arrows choose a chip
    /// (Up and Down a line), Enter or Space wears it, Tab switches hats and capes,
    /// Escape closes.
    pub(super) fn outfit_key(&mut self, key: KeyCode, console: &mut ViewerConsole) -> bool {
        let Some(picker) = self.outfit else {
            return false;
        };
        let count = self.chip_count(picker.slot);
        let step = |by: isize| {
            let mut picker = picker;
            picker.cursor = picker
                .cursor
                .saturating_add_signed(by)
                .min(count.saturating_sub(1));
            picker.follow = true;
            picker
        };
        match key {
            KeyCode::Escape => self.close_outfit(),
            KeyCode::ArrowLeft | KeyCode::KeyA => self.outfit = Some(step(-1)),
            KeyCode::ArrowRight | KeyCode::KeyD => self.outfit = Some(step(1)),
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.outfit = Some(step(-(CHIP_COLUMNS as isize)));
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.outfit = Some(step(CHIP_COLUMNS as isize));
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                self.wear_chip(console, picker.slot, picker.cursor);
            }
            KeyCode::Tab => self.open_outfit(match picker.slot {
                CosmeticSlot::Hat => CosmeticSlot::Cape,
                CosmeticSlot::Cape => CosmeticSlot::Hat,
            }),
            _ => return false,
        }
        true
    }

    /// The pointer on the turn area or the picker; `None` leaves it to the screen.
    pub(super) fn outfit_pointer(
        &mut self,
        kind: UiEventKind,
        token: u16,
        delta: Option<sjk_ui::Vec2>,
        console: &mut ViewerConsole,
    ) -> Option<PlayerMenuResult> {
        // A click on the Hat or Cape row opens the picker on it.
        if kind == UiEventKind::Activate
            && self.page == ProfilePage::Character
            && let Some(slot) = self
                .character_rows()
                .get(usize::from(token))
                .and_then(|row| row.cosmetic())
        {
            // As a click anywhere else does, it ends typing the name (keeping it) or
            // the search.
            if self.name_editing {
                self.name_editing = false;
                self.apply(console);
            }
            self.search_editing = false;
            self.selected = usize::from(token);
            self.open_outfit(slot);
            return Some(PlayerMenuResult::None);
        }
        if token == TURN_TOKEN {
            if kind == UiEventKind::Drag
                && let Some(delta) = delta
            {
                self.turn.drag(delta.x);
            }
            return Some(PlayerMenuResult::None);
        }
        let picker = self.outfit?;
        let activate = kind == UiEventKind::Activate;
        match token {
            DONE_TOKEN if activate => self.close_outfit(),
            token if (SLOT_BASE..SLOT_BASE + 2).contains(&token) => {
                if activate {
                    self.open_outfit(CosmeticSlot::ALL[usize::from(token - SLOT_BASE)]);
                }
            }
            token
                if kind == UiEventKind::Wheel
                    && (token == SCROLL_TOKEN
                        || (CHIP_BASE..CHIP_BASE + MAX_CHIPS as u16).contains(&token)) =>
            {
                let up = delta.is_some_and(|delta| delta.y > 0.0);
                let mut picker = picker;
                picker.first_line = if up {
                    picker.first_line.saturating_sub(1)
                } else {
                    picker.first_line + 1
                };
                self.outfit = Some(picker);
            }
            token if (CHIP_BASE..CHIP_BASE + MAX_CHIPS as u16).contains(&token) => {
                let chip = picker.first_line * CHIP_COLUMNS + usize::from(token - CHIP_BASE);
                if matches!(kind, UiEventKind::HoverEnter | UiEventKind::Hover) {
                    self.outfit = Some(Picker {
                        cursor: chip,
                        ..picker
                    });
                } else if activate {
                    self.outfit = Some(Picker {
                        cursor: chip,
                        ..picker
                    });
                    self.wear_chip(console, picker.slot, chip);
                } else {
                    return None;
                }
            }
            _ => return None,
        }
        Some(PlayerMenuResult::None)
    }

    /// Under the model's feet, centred on `centre_x` from `top` (frame pixels), on every
    /// page that turns it: the keys' look of the screen ([`crate::menu::sjk::key_hint`]),
    /// "Drag" in a key cap and what it does.
    pub(super) fn sjk_turn_hint(&mut self, frame: &Frame, centre_x: f32, top: f32) {
        if !self.model_turns() {
            return;
        }
        let s = frame.s;
        let (keys, action) = (&["Drag"][..], "turn the model");
        let width = crate::menu::sjk::key_hint_width(keys, action, s);
        let [centre, y] = frame.point(centre_x, top);
        crate::menu::sjk::key_hint(&mut self.canvas, keys, action, centre - width * 0.5, y, s);
    }

    /// Register the turn area over the model (before everything else, so any
    /// control drawn over it keeps the pointer).
    pub(super) fn outfit_turn_area(&mut self, frame: &Frame, area: [f32; 4]) {
        let [x, y, width, height] = area;
        self.canvas
            .hit_region(TURN_TOKEN, frame.rect(x, y, width, height));
    }

    /// The picker in the column from `x`, `width` wide, from `top` to `bottom`
    /// (frame pixels); returns its foot.
    pub(super) fn sjk_outfit_picker(
        &mut self,
        frame: &Frame,
        x: f32,
        width: f32,
        top: f32,
        bottom: f32,
    ) -> f32 {
        let Some(mut picker) = self.outfit else {
            return top;
        };
        let s = frame.s;
        let count = self.chip_count(picker.slot);
        // "Wear" and how many the slot has installed, Hats and Capes, and Done.
        let middle = top + HEAD * 0.5 - 6.0;
        let done = [x + width - 120.0, middle - 19.0, 120.0, 38.0];
        kit::button(
            &mut self.canvas,
            frame,
            done,
            "Done",
            false,
            true,
            false,
            DONE_TOKEN,
        );
        kit::segments(
            &mut self.canvas,
            frame,
            done[0] - 16.0,
            middle,
            &["Hats", "Capes"],
            picker.slot.index(),
            true,
            SLOT_BASE,
        );
        text(
            &mut self.canvas,
            TextFamily::Display,
            format_args!("Wear  ·  {} installed", count - 1),
            frame.rect(x + 22.0, middle - 15.0, 260.0, 30.0),
            22.0 * s,
            color::HOLO,
            FontWeight::Regular,
            TextAlign::Start,
        );
        // The chips, as many lines as fit above the note line.
        let chips_top = top + HEAD;
        let note = 28.0;
        let step = CHIP_HEIGHT + CHIP_GAP;
        let fit = (((bottom - note - chips_top + CHIP_GAP) / step).floor() as usize).max(1);
        let lines = count.div_ceil(CHIP_COLUMNS);
        let shown_lines = fit.min(lines).max(1);
        let most_first = lines.saturating_sub(shown_lines);
        if picker.follow {
            picker.follow = false;
            let line = picker.cursor / CHIP_COLUMNS;
            if line < picker.first_line {
                picker.first_line = line;
            } else if line >= picker.first_line + shown_lines {
                picker.first_line = line + 1 - shown_lines;
            }
        }
        picker.first_line = picker.first_line.min(most_first);
        self.outfit = Some(picker);
        let chip_width = (width - CHIP_GAP * (CHIP_COLUMNS - 1) as f32) / CHIP_COLUMNS as f32;
        let height = shown_lines as f32 * step - CHIP_GAP;
        self.canvas
            .scroll_region(SCROLL_TOKEN, frame.rect(x, chips_top, width, height));
        let worn = self
            .cosmetics
            .worn_position(picker.slot)
            .map_or(0, |index| index + 1);
        let first = picker.first_line * CHIP_COLUMNS;
        let last = (first + shown_lines * CHIP_COLUMNS)
            .min(count)
            .min(first + MAX_CHIPS);
        for chip in first..last {
            let local = chip - first;
            let token = CHIP_BASE + local as u16;
            let chip_x = x + (local % CHIP_COLUMNS) as f32 * (chip_width + CHIP_GAP);
            let chip_y = chips_top + (local / CHIP_COLUMNS) as f32 * step;
            let rect = frame.rect(chip_x, chip_y, chip_width, CHIP_HEIGHT);
            let chosen = chip == worn;
            let lit = chip == picker.cursor || self.canvas.token_hovered(token);
            let _ = self.canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                rect,
                radius: 8.0 * s,
                color: color::alpha(color::SPACE, if lit { 0.75 } else { 0.55 }),
            });
            if chosen || lit {
                let _ = self.canvas.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * s,
                    width: if chosen { 2.5 * s } else { 1.5 * s },
                    color: if chosen {
                        color::GOLD_BRIGHT
                    } else {
                        sjk_ui::Color::new(1.0, 1.0, 1.0, 0.6)
                    },
                });
            }
            let name = match chip.checked_sub(1) {
                None => "None",
                Some(index) => self.cosmetics.display(picker.slot, index).unwrap_or("?"),
            };
            text(
                &mut self.canvas,
                TextFamily::Body,
                format_args!("{name}"),
                frame.rect(
                    chip_x + 8.0,
                    chip_y + (CHIP_HEIGHT - 24.0) * 0.5,
                    chip_width - 16.0,
                    24.0,
                ),
                18.0 * s,
                if chosen {
                    color::GOLD_BRIGHT
                } else if lit {
                    color::TEXT
                } else {
                    color::alpha(color::TEXT, 0.85)
                },
                FontWeight::Regular,
                TextAlign::Center,
            );
            self.canvas.hit_region(token, rect);
        }
        if lines > shown_lines {
            self.canvas.scrollbar(
                SCROLL_TOKEN,
                frame.rect(x + width + 12.0, chips_top, 4.0, height),
                picker.first_line,
                shown_lines,
                lines,
            );
        }
        // Under the chips: why there are none, or that JoF has more.
        let line = match picker.slot {
            _ if self.cosmetics.catalog().is_none() => Some("Looking for the installed pieces..."),
            CosmeticSlot::Hat if count == 1 => {
                Some("No hats installed: JoF's cosmetics pack brings them.")
            }
            CosmeticSlot::Cape if count == 1 => {
                Some("No capes installed: JoF's cosmetics pack brings them.")
            }
            slot if self.cosmetics.has_missing(slot) => {
                Some("JoF's cosmetics pack has more than are installed.")
            }
            _ => None,
        };
        if let Some(line) = line {
            text(
                &mut self.canvas,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(x + 4.0, chips_top + height + 6.0, width, 22.0),
                16.0 * s,
                color::MUTED,
                FontWeight::Regular,
                TextAlign::Start,
            );
        }
        bottom
    }
}

#[cfg(test)]
mod tests {
    use super::super::grid::GRID_SCROLL_TOKEN;
    use super::*;

    #[test]
    fn a_drag_turns_at_once_and_a_face_eases_the_short_way() {
        let mut turn = Turn::default();
        turn.drag(100.0);
        assert!((turn.degrees() - 45.0).abs() < 1e-3);
        turn.drag(-100.0);
        assert!(turn.degrees().abs() < 1e-3);
        // From 170 the back (180) is ten degrees on, the front 170 back.
        turn.drag(170.0 / DEGREES_PER_PIXEL);
        turn.face(180.0);
        turn.advance(0.0);
        turn.advance(0.2);
        assert!(
            (170.0..=180.0).contains(&turn.degrees()),
            "{}",
            turn.degrees()
        );
        for frame in 1..60 {
            turn.advance(0.2 + f64::from(frame) / 60.0);
        }
        assert!((turn.degrees() - 180.0).abs() < 0.1, "{}", turn.degrees());
        // From -170 the back is -180, not 180 the long way.
        turn.reset();
        turn.drag(-170.0 / DEGREES_PER_PIXEL);
        turn.face(180.0);
        for frame in 0..120 {
            turn.advance(f64::from(frame) / 60.0);
        }
        assert!((turn.degrees() + 180.0).abs() < 0.1, "{}", turn.degrees());
    }

    fn draw(menu: &mut PlayerMenu) {
        let font = crate::text::load_modern(1.0, None).unwrap().font;
        let mut vertices = Vec::new();
        menu.append_sjk(
            crate::menu::sjk::TextTarget::Inter(&mut vertices, &font),
            [1920.0, 1080.0],
            1.0,
        );
    }

    /// Whether the last frame drew `wanted`.
    fn shows(menu: &PlayerMenu, wanted: &str) -> bool {
        menu.canvas.draw_list().commands().iter().any(|command| {
            matches!(command, DrawCommand::Text { text, .. }
                if menu.canvas.stored_text(*text) == wanted)
        })
    }

    fn centre(menu: &PlayerMenu, token: u16) -> sjk_ui::Vec2 {
        let rect = menu.canvas.rect_for(token).expect("the target");
        sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
    }

    fn click(menu: &mut PlayerMenu, console: &mut ViewerConsole, position: sjk_ui::Vec2) {
        let button = sjk_ui::PointerButton::Primary;
        for event in [
            sjk_ui::InputEvent::PointerMove(position),
            sjk_ui::InputEvent::PointerPress { position, button },
            sjk_ui::InputEvent::PointerRelease { position, button },
        ] {
            let _ = menu.handle_pointer(event, console);
        }
    }

    /// The SJK UI's Character page, with two hats and a cape installed.
    fn character_page() -> (PlayerMenu, ViewerConsole, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_memory(
            "cosmetics",
            [
                ("models/cosmetics/hats/tophat.md3", b"x".to_vec()),
                ("models/cosmetics/hats/santahat.md3", b"x".to_vec()),
                ("models/cosmetics/capes/royalcape.md3", b"x".to_vec()),
            ],
        )
        .unwrap();
        let mut menu = PlayerMenu::new();
        menu.set_sjk(true);
        menu.cosmetics
            .open(&console, Some(&std::sync::Arc::new(vfs)));
        menu.return_target = ReturnTarget::InGame;
        menu.set_page(ProfilePage::Character);
        draw(&mut menu);
        (menu, console, directory)
    }

    /// Opened from a game, dragging over the model turns it as far as the pointer went,
    /// on every page, the preview camera going round it the other way, and the hint under
    /// the model stays; on the menu map's stage nothing turns.
    #[test]
    fn dragging_over_the_model_turns_it() {
        let (mut menu, mut console, _directory) = character_page();
        let drag = |menu: &mut PlayerMenu, console: &mut ViewerConsole, pixels: f32| {
            let start = centre(menu, TURN_TOKEN);
            let end = sjk_ui::Vec2::new(start.x + pixels, start.y);
            let button = sjk_ui::PointerButton::Primary;
            for event in [
                sjk_ui::InputEvent::PointerMove(start),
                sjk_ui::InputEvent::PointerPress {
                    position: start,
                    button,
                },
                sjk_ui::InputEvent::PointerMove(end),
                sjk_ui::InputEvent::PointerRelease {
                    position: end,
                    button,
                },
            ] {
                let _ = menu.handle_pointer(event, console);
            }
        };
        assert!(shows(&menu, "turn the model"), "the hint under the model");
        drag(&mut menu, &mut console, 100.0);
        assert!(
            (menu.model_turn() - 45.0).abs() < 0.01,
            "{}",
            menu.model_turn()
        );
        assert_eq!(menu.selected, 0, "the drag chose no row");
        let angle = menu.sjk_model_preview().and_then(|preview| preview.angle);
        assert_eq!(angle, Some(-28.0 - 45.0));
        draw(&mut menu);
        assert!(shows(&menu, "turn the model"), "it stays once dragged");
        // The Saber and Force pages turn it too, from where it stands.
        for (page, pixels, turned) in [
            (ProfilePage::Saber, -40.0, 27.0),
            (ProfilePage::Force, -60.0, 0.0),
        ] {
            menu.set_page(page);
            draw(&mut menu);
            assert!(shows(&menu, "turn the model"), "{page:?}");
            drag(&mut menu, &mut console, pixels);
            assert!(
                (menu.model_turn() - turned).abs() < 0.01,
                "{page:?}: {}",
                menu.model_turn()
            );
        }
        // On the menu map's stage nothing turns, and nothing says it would.
        menu.open(&console, ReturnTarget::MainMenu);
        assert_eq!(menu.model_turn(), 0.0);
        draw(&mut menu);
        assert!(menu.canvas.rect_for(TURN_TOKEN).is_none());
        assert!(!shows(&menu, "turn the model"));
    }

    /// The Hat row opens the picker: None and the hats as chips, a click wears one,
    /// Tab shows the capes with the model's back to the camera, Escape closes it.
    #[test]
    fn the_picker_wears_what_is_clicked() {
        let (mut menu, mut console, _directory) = character_page();
        let hat = menu
            .character_rows()
            .iter()
            .position(|row| row.cosmetic() == Some(CosmeticSlot::Hat))
            .expect("the Hat row");
        // Typing the name, a click on the row keeps the name and opens the picker.
        menu.activate(&mut console);
        assert!(menu.name_editing);
        let at = centre(&menu, hat as u16);
        click(&mut menu, &mut console, at);
        assert!(!menu.name_editing, "the name was kept");
        assert_eq!(
            menu.outfit.map(|picker| picker.slot),
            Some(CosmeticSlot::Hat)
        );
        draw(&mut menu);
        assert!(!menu.canvas.overflowed());
        // None, Santahat, Tophat; no fourth.
        assert!(menu.canvas.rect_for(CHIP_BASE + 2).is_some());
        assert!(menu.canvas.rect_for(CHIP_BASE + 3).is_none());
        assert!(
            menu.canvas.rect_for(GRID_SCROLL_TOKEN).is_none(),
            "the grid gives way"
        );
        let at = centre(&menu, CHIP_BASE + 2);
        click(&mut menu, &mut console, at);
        assert!(
            console
                .text_value("color1")
                .is_some_and(|value| value.ends_with("tophat")),
            "{:?}",
            console.text_value("color1")
        );
        draw(&mut menu);
        let at = centre(&menu, CHIP_BASE);
        click(&mut menu, &mut console, at);
        assert_eq!(
            console
                .text_value("color1")
                .map(|value| sjk_client::split_color_value(value).1),
            Some(None)
        );
        assert!(menu.outfit_key(KeyCode::Tab, &mut console));
        assert_eq!(
            menu.outfit.map(|picker| picker.slot),
            Some(CosmeticSlot::Cape)
        );
        for frame in 0..90 {
            menu.turn.advance(f64::from(frame) / 60.0);
        }
        assert!(
            (menu.model_turn().abs() - 180.0).abs() < 0.5,
            "{}",
            menu.model_turn()
        );
        draw(&mut menu);
        assert!(menu.canvas.rect_for(CHIP_BASE + 1).is_some());
        assert!(menu.canvas.rect_for(CHIP_BASE + 2).is_none());
        // The keys choose a chip and wear it.
        assert!(menu.outfit_key(KeyCode::ArrowRight, &mut console));
        assert!(menu.outfit_key(KeyCode::Enter, &mut console));
        assert!(
            console
                .text_value("color2")
                .is_some_and(|value| value.ends_with("royalcape"))
        );
        assert!(menu.outfit_key(KeyCode::Escape, &mut console));
        assert!(menu.outfit.is_none());
        assert!(
            !menu.outfit_key(KeyCode::Escape, &mut console),
            "the screen's again"
        );
        draw(&mut menu);
        assert!(menu.canvas.rect_for(GRID_SCROLL_TOKEN).is_some());
    }

    #[test]
    fn whole_turns_are_dropped() {
        let mut turn = Turn::default();
        turn.drag(800.0 / DEGREES_PER_PIXEL);
        assert!((turn.degrees() - 80.0).abs() < 1e-2, "{}", turn.degrees());
    }

    #[test]
    fn a_hat_faces_the_front_and_a_cape_the_back() {
        assert_eq!(facing(CosmeticSlot::Hat), 0.0);
        assert_eq!(facing(CosmeticSlot::Cape), 180.0);
    }

    // The tokens stay apart, and clear of the screen's others: the hilts end at 1,700
    // (`sjk_view.rs`), the classic pages' colour sliders below 2,000 (`saber_rgb.rs`).
    const _: () = assert!(crate::profile_hub::TOKEN + 64 <= TURN_TOKEN);
    const _: () = assert!(TURN_TOKEN < SLOT_BASE && SLOT_BASE + 2 <= DONE_TOKEN);
    const _: () = assert!(DONE_TOKEN < SCROLL_TOKEN && SCROLL_TOKEN < CHIP_BASE);
}
