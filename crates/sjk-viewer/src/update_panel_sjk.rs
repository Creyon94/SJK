//! Update in the SJK UI (`docs/sjk-ui.md`, Sol JK's pages): the version this
//! is, then what the update check found as a headline over its detail, the
//! download's gold progress, and the page's actions as buttons (the one Enter
//! takes gold), over the map. The state and its actions are the page's own.

use super::*;
use crate::menu::sjk::{Frame, TextTarget, color, key_hint, key_hint_width, kit, text, wrap};
use crate::menu_widgets::TextFamily;
use sjk_ui::TextAlign;

/// The page's column.
const COLUMN_X: f32 = 520.0;
const COLUMN_WIDTH: f32 = 900.0;
const TOP: f32 = 220.0;
/// The keys' line.
const KEYS_Y: f32 = 992.0;

impl Panel {
    /// Draw the page in the SJK UI.
    pub(crate) fn append_sjk(&mut self, target: TextTarget<'_>, viewport: [f32; 2]) {
        let frame = Frame::new(viewport);
        let s = frame.s;
        let installed = crate::build_info::VERSION;
        let view = view(&update::state(), installed);
        self.ui.begin_transparent(viewport);
        crate::settings::sjk_view::backdrop(&mut self.ui, viewport);
        let [x, y] = frame.point(96.0, 75.0);
        let end = key_hint(&mut self.ui, &["Esc"], "Back", x, y, s);
        self.ui
            .hit_region(BACK_TOKEN, Rect::new(x, y, end - x, 24.0 * s));
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Update"),
            frame.rect((end - frame.origin[0]) / s + 22.0, 57.0, 600.0, 60.0),
            48.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("This is Sol JK {installed}"),
            frame.rect(COLUMN_X, TOP, COLUMN_WIDTH, 26.0),
            19.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Start,
        );
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", view.headline),
            frame.rect(COLUMN_X, TOP + 36.0, COLUMN_WIDTH, 70.0),
            56.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        let mut y = TOP + 120.0;
        for line in wrap(&view.detail, 80).take(4) {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!("{line}"),
                frame.rect(COLUMN_X, y, COLUMN_WIDTH, 30.0),
                20.0 * s,
                color::alpha(color::TEXT, 0.86),
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 32.0;
        }
        if let Some(progress) = view.progress {
            y += 18.0;
            let track = frame.rect(COLUMN_X, y, 640.0, 6.0);
            let _ = self
                .ui
                .draw_list_mut()
                .push(sjk_ui::DrawCommand::RoundedRect {
                    rect: track,
                    radius: track.height * 0.5,
                    color: color::alpha(color::HOLO, 0.2),
                });
            let filled = frame.rect(COLUMN_X, y, 640.0 * progress, 6.0);
            let _ = self
                .ui
                .draw_list_mut()
                .push(sjk_ui::DrawCommand::RoundedRect {
                    rect: filled,
                    radius: filled.height * 0.5,
                    color: color::GOLD,
                });
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{:.0} %", progress * 100.0),
                frame.rect(COLUMN_X + 660.0, y - 13.0, 100.0, 30.0),
                22.0 * s,
                color::GOLD_BRIGHT,
                FontWeight::Regular,
                TextAlign::Start,
            );
            y += 6.0;
        }
        y += 40.0;
        let mut x = COLUMN_X;
        let mut button = |ui: &mut MenuCanvas, label: &str, primary: bool, token: u16| {
            // Rajdhani at 20 is about 9.5 pixels a character.
            let width = (48.0 + 9.5 * label.chars().count() as f32).max(150.0);
            kit::button(
                ui,
                &frame,
                [x, y, width, 50.0],
                label,
                primary,
                true,
                primary,
                token,
            );
            x += width + 14.0;
        };
        if let Some(primary) = view.primary {
            button(&mut self.ui, primary, true, PRIMARY_TOKEN);
        }
        if view.check {
            button(&mut self.ui, "Check again", false, CHECK_TOKEN);
        }
        if view.notes {
            button(&mut self.ui, "Release notes", false, NOTES_TOKEN);
        }
        self.sjk_keys(&frame, &view);
        self.ui.finish(PRIMARY_TOKEN);
        target.append(&self.ui, viewport);
    }

    /// The page's keys, right-aligned at the bottom.
    fn sjk_keys(&mut self, frame: &Frame, view: &View) {
        let s = frame.s;
        let mut keys: Vec<(&[&str], &str)> = Vec::with_capacity(4);
        if let Some(primary) = view.primary {
            keys.push((&["Enter"], primary));
        }
        if view.check {
            keys.push((&["C"], "check again"));
        }
        if view.notes {
            keys.push((&["N"], "release notes"));
        }
        keys.push((&["Esc"], "back"));
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * keys.len().saturating_sub(1) as f32;
        let [right, y] = frame.point(1_824.0, KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}
