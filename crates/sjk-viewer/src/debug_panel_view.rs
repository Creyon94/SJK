//! Drawing of the debug panel: the hero header and tabs of the settings form, a list
//! with one two-line row per entry (tick box, PR, title; status and area) on the left,
//! and the selected entry's details (what changed, steps to test, notes) on a backed
//! pane on the right. Everything is formatted into the canvas' retained text slots,
//! so a steady frame allocates nothing.

use super::data::{Entry, Status};
use super::pointer::{DETAIL_TICK_TOKEN, PANE_WHEEL_TOKEN, ROW_BASE, ROW_LIMIT};
use super::pointer::{SCROLLBAR_TOKEN, TOGGLE_TOKEN, list_tokens};
use super::{Panel, TABS};
use crate::menu_widgets::{BACK_TOKEN, FormLayout, Scrim};
use crate::text::{TextVertex, UiFont};
use sjk_ui::{Color, FontWeight, Rect, TextAlign};

/// Highlight colour of notes.
const NOTE: Color = Color::new(1.0, 0.80, 0.42, 0.96);
/// Colour of a merged entry's status.
const MERGED: Color = Color::new(0.52, 0.86, 0.60, 0.95);
/// Secondary text in rows and the pane.
const SOFT: Color = Color::new(0.82, 0.88, 0.94, 0.86);

impl Panel {
    /// Draw the panel over the whole frame. Overlay text draws above every overlay's
    /// shapes, so the text other overlays appended earlier this frame is dropped
    /// rather than shown through the panel (as the console browser does).
    pub(crate) fn append(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        vertices.clear();

        let mut layout = FormLayout::new(viewport);
        let s = layout.scale;
        layout.margin = (viewport[0] * 0.06).max(48.0 * s);
        let inner = viewport[0] - layout.margin * 2.0;
        layout.column_width = (inner * 0.42).clamp(360.0 * s, 680.0 * s).min(inner * 0.6);
        layout.rows_y = layout.tabs_y() + 44.0 * s;
        layout.row_height = 52.0 * s;
        let rows_bottom = viewport[1] - 84.0 * s;
        self.rows = (((rows_bottom - layout.rows_y) / layout.row_height).floor()).max(1.0) as usize;
        self.rows = self.rows.min(ROW_LIMIT);
        self.first = self.first.min(self.visible.len().saturating_sub(self.rows));

        self.ui.begin_hero(viewport, 1.0, Scrim::Wide);
        self.ui.form_header(
            &layout,
            "SJK   /   PERSONAL BUILD",
            "TEST LIST",
            &self.summary,
        );
        self.ui.form_tabs(&layout, &TABS, self.tab);

        let shown = self.first..self.visible.len().min(self.first + self.rows);
        for (slot, position) in shown.enumerate() {
            let rect = Rect::new(
                layout.margin,
                layout.rows_y + slot as f32 * layout.row_height,
                layout.column_width,
                layout.row_height,
            );
            self.row_view(rect, slot, position, s);
        }
        if self.visible.is_empty() {
            let message = match self.tab {
                0 if !self.entries.is_empty() => "Everything in the list is ticked as tested.",
                1 => "Nothing is ticked yet.",
                _ => "The test list is empty.",
            };
            let muted = self.ui.theme().muted;
            self.ui.text(
                message,
                Rect::new(
                    layout.margin,
                    layout.rows_y + 12.0 * s,
                    layout.column_width,
                    22.0 * s,
                ),
                16.0 * s,
                muted,
                FontWeight::Regular,
                0.2 * s,
            );
        } else if self.visible.len() > self.rows {
            self.ui.scrollbar(
                SCROLLBAR_TOKEN,
                Rect::new(
                    layout.margin + layout.column_width + 12.0 * s,
                    layout.rows_y,
                    5.0 * s,
                    self.rows as f32 * layout.row_height,
                ),
                self.first,
                self.rows,
                self.visible.len(),
            );
        }

        let pane_x = layout.margin + layout.column_width + 40.0 * s;
        let pane_top = viewport[1] * 0.17 - 34.0 * s;
        let pane = Rect::new(
            pane_x,
            pane_top,
            viewport[0] - layout.margin - pane_x,
            rows_bottom - pane_top,
        );
        self.ui.scroll_region(PANE_WHEEL_TOKEN, pane);
        if let Some(index) = self.selected_entry() {
            self.detail_view(pane, index, s);
        }

        self.ui.form_footer_actions(
            &layout,
            &[
                ("SPACE", self.toggle_hint(), TOGGLE_TOKEN),
                ("TAB", "Filter", 0),
                ("ESC", "Close", BACK_TOKEN),
            ],
        );
        if !self.status.is_empty() {
            let theme = self.ui.theme();
            self.ui.text_aligned(
                &self.status,
                Rect::new(pane.x, viewport[1] - 62.0 * s, pane.width, 20.0 * s),
                14.0 * s,
                if self.status_error {
                    theme.critical
                } else {
                    theme.muted
                },
                FontWeight::Regular,
                0.2 * s,
                TextAlign::End,
            );
        }
        self.ui.end_hero();
        let selected = self.selected.saturating_sub(self.first);
        self.ui
            .finish(ROW_BASE + selected.min(ROW_LIMIT - 1) as u16);
        self.ui.append_text(vertices, font, viewport);
    }

    /// Footer label of the tick key for the selected entry.
    fn toggle_hint(&self) -> &'static str {
        match self.selected_entry() {
            Some(index) if self.tested[index] => "Untick",
            _ => "Tick as tested",
        }
    }

    /// One entry: tick box, PR and title; then status and area.
    fn row_view(&mut self, rect: Rect, slot: usize, position: usize, s: f32) {
        let selected = position == self.selected;
        let index = self.visible[position];
        let (row_token, tick_token) = list_tokens(slot);
        self.ui.form_row_frame(rect, row_token, selected, s);
        let tick = Rect::new(rect.x + 6.0 * s, rect.y + 15.0 * s, 20.0 * s, 20.0 * s);
        self.tick_box(tick, tick_token, self.tested[index], s);

        let theme = self.ui.theme();
        let entry = &self.entries[index];
        let text_x = tick.right() + 16.0 * s;
        // Kept for entries without a PR too, so every title starts in one column.
        let reference_width = 62.0 * s;
        self.ui.text(
            &entry.reference,
            Rect::new(text_x, rect.y + 8.0 * s, reference_width, 22.0 * s),
            16.0 * s,
            status_color(entry.status, theme.accent),
            FontWeight::Semibold,
            0.2 * s,
        );
        let title_x = text_x + reference_width;
        let dim = self.tested[index] && !selected;
        self.ui.text(
            &entry.title,
            Rect::new(title_x, rect.y + 8.0 * s, rect.right() - title_x, 22.0 * s),
            16.0 * s,
            if selected {
                theme.foreground
            } else if dim {
                Color::new(0.70, 0.76, 0.82, 0.70)
            } else {
                SOFT
            },
            if selected {
                FontWeight::Semibold
            } else {
                FontWeight::Regular
            },
            0.2 * s,
        );
        self.ui.text(
            &entry.meta,
            Rect::new(text_x, rect.y + 31.0 * s, rect.right() - text_x, 16.0 * s),
            11.0 * s,
            Color::new(0.70, 0.78, 0.86, 0.72),
            FontWeight::Semibold,
            1.2 * s,
        );
    }

    /// A square tick box: an outline, filled with the accent when ticked. The
    /// pointer target is a little larger than the box.
    fn tick_box(&mut self, rect: Rect, token: u16, ticked: bool, s: f32) {
        self.ui.text_field(rect, ticked);
        if ticked {
            let accent = self.ui.theme().accent;
            let inset = 5.0 * s;
            self.ui.accent_bar(
                Rect::new(
                    rect.x + inset,
                    rect.y + inset,
                    rect.width - inset * 2.0,
                    rect.height - inset * 2.0,
                ),
                accent,
            );
        }
        let pad = 8.0 * s;
        self.ui.hit_region(
            token,
            Rect::new(
                rect.x - pad,
                rect.y - pad,
                rect.width + pad * 2.0,
                rect.height + pad * 2.0,
            ),
        );
    }

    /// The selected entry on a backed pane: links and status, title, tick, then
    /// what changed, the steps to test and any notes, cut off at the pane's bottom.
    fn detail_view(&mut self, pane: Rect, index: usize, s: f32) {
        self.ui.panel(pane);
        let theme = self.ui.theme();
        let pad = 28.0 * s;
        let x = pane.x + pad;
        let width = pane.width - pad * 2.0;
        let bottom = pane.bottom() - pad;
        let entry: &Entry = &self.entries[index];
        let mut y = pane.y + pad;

        self.ui.text(
            &entry.links,
            Rect::new(x, y, width * 0.6, 18.0 * s),
            13.0 * s,
            theme.accent,
            FontWeight::Semibold,
            2.0 * s,
        );
        self.ui.text_aligned(
            entry.status.label(),
            Rect::new(x + width * 0.4, y, width * 0.6, 18.0 * s),
            13.0 * s,
            status_color(entry.status, theme.foreground),
            FontWeight::Semibold,
            2.0 * s,
            TextAlign::End,
        );
        y += 26.0 * s;
        self.ui.text(
            &entry.title,
            Rect::new(x, y, width, 36.0 * s),
            28.0 * s,
            theme.foreground,
            FontWeight::Semibold,
            0.0,
        );
        y += 40.0 * s;
        self.ui.text(
            &entry.area,
            Rect::new(x, y, width, 18.0 * s),
            14.0 * s,
            theme.muted,
            FontWeight::Regular,
            0.2 * s,
        );
        y += 34.0 * s;

        let ticked = self.tested[index];
        let tick = Rect::new(x, y, 22.0 * s, 22.0 * s);
        self.tick_box(tick, DETAIL_TICK_TOKEN, ticked, s);
        self.ui.text(
            if ticked {
                "TESTED   /   Space or click to untick"
            } else {
                "NOT TESTED   /   Space or click to tick"
            },
            Rect::new(
                tick.right() + 14.0 * s,
                y + 3.0 * s,
                width - 40.0 * s,
                18.0 * s,
            ),
            13.0 * s,
            if ticked { theme.accent } else { SOFT },
            FontWeight::Semibold,
            1.2 * s,
        );
        y += 46.0 * s;

        let entry: &Entry = &self.entries[index];
        let sections: [(&str, &[String], bool, Color); 3] = [
            ("WHAT CHANGED", &entry.changes, false, SOFT),
            ("TO TEST", &entry.tests, true, theme.foreground),
            ("NOTE", &entry.notes, false, NOTE),
        ];
        for (heading, lines, numbered, color) in sections {
            if lines.is_empty() || y + 40.0 * s > bottom {
                continue;
            }
            self.ui.text(
                heading,
                Rect::new(x, y, width, 16.0 * s),
                12.0 * s,
                if heading == "NOTE" {
                    NOTE
                } else {
                    theme.accent
                },
                FontWeight::Semibold,
                2.6 * s,
            );
            y += 24.0 * s;
            for (number, line) in lines.iter().enumerate() {
                if y + 22.0 * s > bottom {
                    break;
                }
                let rect = Rect::new(x, y, width, 22.0 * s);
                if numbered {
                    self.ui.text_fmt_aligned(
                        format_args!("{}.   {line}", number + 1),
                        rect,
                        16.0 * s,
                        color,
                        FontWeight::Regular,
                        0.2 * s,
                        TextAlign::Start,
                    );
                } else {
                    self.ui
                        .text(line, rect, 16.0 * s, color, FontWeight::Regular, 0.2 * s);
                }
                y += 27.0 * s;
            }
            y += 16.0 * s;
        }
    }
}

/// Colour that marks an entry's status: `open` for open PRs.
fn status_color(status: Status, open: Color) -> Color {
    match status {
        Status::Open => open,
        Status::Merged => MERGED,
        Status::Personal => NOTE,
    }
}
