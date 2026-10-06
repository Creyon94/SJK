//! The changelog page: every SJK release with its changes and their credits,
//! read from `CHANGELOG.md` (built in, parsed once; see `changelog_data.rs`).
//!
//! Opened by the main menu's Changelog entry (modern and classic) or the
//! `changelog` console command. Like the `debug_panel` test list it lives in the
//! console and is drawn in place of it, so it opens over the menus and in a
//! match; opened with the console closed, it closes the console again with
//! itself. Releases are listed newest first on the left; the selected one's
//! introduction and changes, each followed by its credit, are wrapped on a
//! scrolling pane on the right. With `ui_menuStyle classic` the page takes the
//! classic+ look of the command browser's pop-up (`changelog_classic.rs`).

use crate::menu::art::ArtSet;
use crate::menu_widgets::{BACK_TOKEN, FormLayout, MenuCanvas, Scrim};
use crate::text::{TextFace, TextVertex, UiFont, visible_text_width_style};
use sjk_ui::{Color, FontWeight, InputEvent, Rect, TextAlign, UiEventKind};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "changelog_classic.rs"]
mod classic;
#[path = "changelog_data.rs"]
mod data;

/// Console command that toggles the page; keys bound to it toggle it too.
pub(crate) const COMMAND: &str = "changelog";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Show every SJK release's changes and credits";

/// Row `slot` on screen answers to `ROW_BASE + slot`.
const ROW_BASE: u16 = 1_000;
/// Most release rows drawn at once.
const ROW_LIMIT: usize = 100;
/// Wheel target over the release pane.
const PANE_TOKEN: u16 = 906;
/// Draggable thumb beside the pane's lines.
const PANE_BAR_TOKEN: u16 = 910;
/// Pane lines one wheel notch scrolls.
const WHEEL_LINES: usize = 3;

/// Text sizes on the pane, at a scale of 1.
const BODY_SIZE: f32 = 16.0;
const BODY_LINE: f32 = 24.0;
const CREDIT_SIZE: f32 = 12.0;
const CREDIT_LINE: f32 = 22.0;
const CREDIT_SPACING: f32 = 1.6;
/// Gap after a paragraph or a change's credit.
const GAP: f32 = 12.0;
/// Indent of a change's text after its bullet.
const BULLET_INDENT: f32 = 20.0;

/// Secondary text in rows and the pane.
const SOFT: Color = Color::new(0.916, 0.945, 0.973, 0.936);

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LineKind {
    Intro,
    /// The first line of a change, drawn with a bullet.
    Change,
    /// A change's later lines.
    More,
    Credit,
    Gap,
}

struct Line {
    kind: LineKind,
    text: String,
}

impl LineKind {
    fn height(self) -> f32 {
        match self {
            Self::Intro | Self::Change | Self::More => BODY_LINE,
            Self::Credit => CREDIT_LINE,
            Self::Gap => GAP,
        }
    }
}

pub(crate) struct Panel {
    open: bool,
    /// The page opened the console, so closing the page closes it too.
    owns_console: bool,
    releases: Vec<data::Release>,
    /// A parse error of the built-in file, shown instead of the list.
    error: Option<String>,
    selected: usize,
    /// First release row on screen, and how many fit.
    first: usize,
    rows: usize,
    /// The selected release wrapped for the pane, and what it was wrapped for:
    /// release, pane width and text size (bits).
    lines: Vec<Line>,
    wrapped_for: Option<(usize, u32, u32)>,
    /// First pane line on screen, and the last value that still fills the pane.
    scroll: usize,
    max_scroll: usize,
    /// Pane lines the last frame showed, for Page Up / Page Down.
    page_lines: usize,
    summary: String,
    /// The classic+ look, with the retail menu art it can draw.
    classic: bool,
    art: ArtSet,
    ui: MenuCanvas,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub(crate) fn new() -> Self {
        let (releases, error) = match data::parse(data::EMBEDDED) {
            Ok(releases) => (releases, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let released = releases.iter().filter(|r| !r.date.is_empty()).count();
        let summary = format!("{released} releases, newest first   /   credits after each change");
        Self {
            open: false,
            owns_console: false,
            releases,
            error,
            selected: 0,
            first: 0,
            rows: 1,
            lines: Vec::new(),
            wrapped_for: None,
            scroll: 0,
            max_scroll: 0,
            page_lines: 1,
            summary,
            classic: false,
            art: ArtSet::default(),
            ui: MenuCanvas::with_text_capacity(256),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Choose the look: classic+ with the retail `art` it can draw, or modern.
    pub(crate) fn set_look(&mut self, classic: bool, art: ArtSet) {
        self.classic = classic;
        self.art = art;
    }

    /// Whether the classic+ look is drawn, so its text can use the retail font.
    pub(crate) fn is_classic(&self) -> bool {
        self.classic
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
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::ArrowUp => self.select(self.selected.saturating_sub(1)),
            KeyCode::ArrowDown => self.select(self.selected + 1),
            KeyCode::Home => self.select(0),
            KeyCode::End => self.select(usize::MAX),
            KeyCode::PageUp => self.scroll_pane(-(self.page_lines.max(1) as isize)),
            KeyCode::PageDown => self.scroll_pane(self.page_lines.max(1) as isize),
            _ => {}
        }
        PanelAction::None
    }

    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> PanelAction {
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind == UiEventKind::Wheel {
            let direction = event.delta.map_or(0, |delta| -delta.y.signum() as isize);
            if matches!(event.token, Some(PANE_TOKEN | PANE_BAR_TOKEN)) {
                self.scroll_pane(direction * WHEEL_LINES as isize);
            } else {
                let last_first = self.releases.len().saturating_sub(self.rows);
                self.first = self.first.saturating_add_signed(direction).min(last_first);
            }
            return PanelAction::None;
        }
        if event.kind == UiEventKind::Drag && event.token == Some(PANE_BAR_TOKEN) {
            if let (Some(point), Some(track)) = (event.position, self.ui.rect_for(PANE_BAR_TOKEN)) {
                let ratio = ((point.y - track.y) / track.height).clamp(0.0, 1.0);
                self.scroll = (ratio * self.max_scroll as f32).round() as usize;
            }
            return PanelAction::None;
        }
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        match event.token {
            Some(BACK_TOKEN) => PanelAction::Close,
            Some(token) if (ROW_BASE..ROW_BASE + ROW_LIMIT as u16).contains(&token) => {
                self.select(self.first + usize::from(token - ROW_BASE));
                PanelAction::None
            }
            _ => PanelAction::None,
        }
    }

    /// Select release `index` (clamped), from the top of its pane.
    pub(crate) fn select(&mut self, index: usize) {
        let index = index.min(self.releases.len().saturating_sub(1));
        if index != self.selected {
            self.selected = index;
            self.scroll = 0;
        }
        let rows = self.rows.max(1);
        self.first = self
            .first
            .min(self.selected)
            .max((self.selected + 1).saturating_sub(rows));
    }

    fn scroll_pane(&mut self, lines: isize) {
        self.scroll = self
            .scroll
            .saturating_add_signed(lines)
            .min(self.max_scroll);
    }

    /// Wrap the selected release for a pane `width` wide, in body text of
    /// `size` and `spacing` with changes indented by `indent` (pixels).
    fn wrap(&mut self, font: &UiFont, width: f32, size: f32, spacing: f32, indent: f32) {
        let key = (self.selected, width.to_bits(), size.to_bits());
        if self.wrapped_for == Some(key) {
            return;
        }
        self.wrapped_for = Some(key);
        self.lines.clear();
        let Some(release) = self.releases.get(self.selected) else {
            return;
        };
        let style = font.style();
        let measure = |text: &str, size: f32, spacing: f32, face: TextFace| {
            let placement = style.place(Rect::new(0.0, 0.0, 0.0, size), size, spacing);
            visible_text_width_style(
                font,
                text,
                placement.size / font.height.max(1.0),
                face,
                placement.letter_spacing,
            )
        };
        let body = |text: &str| measure(text, size, spacing, TextFace::Regular);
        for paragraph in &release.intro {
            for line in wrap_words(paragraph, width, body) {
                self.lines.push(Line {
                    kind: LineKind::Intro,
                    text: line.to_owned(),
                });
            }
            self.lines.push(Line {
                kind: LineKind::Gap,
                text: String::new(),
            });
        }
        for change in &release.changes {
            for (index, line) in wrap_words(&change.text, width - indent, body)
                .into_iter()
                .enumerate()
            {
                self.lines.push(Line {
                    kind: if index == 0 {
                        LineKind::Change
                    } else {
                        LineKind::More
                    },
                    text: line.to_owned(),
                });
            }
            self.lines.push(Line {
                kind: LineKind::Credit,
                text: change.credit.to_ascii_uppercase(),
            });
            self.lines.push(Line {
                kind: LineKind::Gap,
                text: String::new(),
            });
        }
        self.lines.pop();
    }

    /// Set the last scroll that still fills a pane `available` high, the lines
    /// being `height` high, and keep the scroll within it.
    fn fit_scroll(&mut self, available: f32, height: impl Fn(LineKind) -> f32) {
        let mut total = 0.0;
        self.max_scroll = self.lines.len();
        for (index, line) in self.lines.iter().enumerate().rev() {
            total += height(line.kind);
            if total > available {
                break;
            }
            self.max_scroll = index;
        }
        self.scroll = self.scroll.min(self.max_scroll);
    }

    /// Draw the page over the whole frame. As with the debug panel, text other
    /// overlays appended earlier this frame is dropped rather than shown through.
    pub(crate) fn append(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        if self.classic {
            self.append_classic(vertices, font, viewport);
            return;
        }
        vertices.clear();
        let mut layout = FormLayout::new(viewport);
        let s = layout.scale;
        layout.margin = (viewport[0] * 0.06).max(48.0 * s);
        let inner = viewport[0] - layout.margin * 2.0;
        layout.column_width = (inner * 0.28).clamp(280.0 * s, 440.0 * s).min(inner * 0.4);
        layout.rows_y = layout.tabs_y();
        layout.row_height = 56.0 * s;
        let rows_bottom = viewport[1] - 84.0 * s;
        self.rows = (((rows_bottom - layout.rows_y) / layout.row_height).floor()).max(1.0) as usize;
        self.rows = self.rows.min(ROW_LIMIT);
        self.first = self
            .first
            .min(self.releases.len().saturating_sub(self.rows));

        self.ui.begin_hero(viewport, 1.0, Scrim::Wide);
        self.ui
            .form_header(&layout, "SJK   /   RELEASES", "CHANGELOG", &self.summary);

        let shown = self.first..self.releases.len().min(self.first + self.rows);
        for (slot, index) in shown.enumerate() {
            let rect = Rect::new(
                layout.margin,
                layout.rows_y + slot as f32 * layout.row_height,
                layout.column_width,
                layout.row_height,
            );
            self.row_view(rect, slot, index, s);
        }

        let pane_x = layout.margin + layout.column_width + 40.0 * s;
        let pane_top = viewport[1] * 0.17 - 34.0 * s;
        let pane = Rect::new(
            pane_x,
            pane_top,
            viewport[0] - layout.margin - pane_x,
            rows_bottom - pane_top,
        );
        self.ui.scroll_region(PANE_TOKEN, pane);
        self.pane_view(pane, font, s);

        self.ui.form_footer_actions(
            &layout,
            &[
                ("UP / DOWN", "Release", 0),
                ("PAGE UP / DOWN", "Scroll", 0),
                ("ESC", "Close", BACK_TOKEN),
            ],
        );
        self.ui.end_hero();
        let selected = self.selected.saturating_sub(self.first);
        self.ui
            .finish(ROW_BASE + selected.min(ROW_LIMIT - 1) as u16);
        self.ui.append_text(vertices, font, viewport);
    }

    /// One release: its title, then its date and change count.
    fn row_view(&mut self, rect: Rect, slot: usize, index: usize, s: f32) {
        let selected = index == self.selected;
        self.ui
            .form_row_frame(rect, ROW_BASE + slot as u16, selected, s);
        let theme = self.ui.theme();
        let release = &self.releases[index];
        let x = rect.x + 16.0 * s;
        let width = rect.width - 24.0 * s;
        self.ui.text(
            &release.title,
            Rect::new(x, rect.y + 8.0 * s, width, 22.0 * s),
            17.0 * s,
            if selected { theme.foreground } else { SOFT },
            if selected {
                FontWeight::Semibold
            } else {
                FontWeight::Regular
            },
            0.2 * s,
        );
        self.ui.text(
            &release.meta,
            Rect::new(x, rect.y + 33.0 * s, width, 16.0 * s),
            11.0 * s,
            if release.date.is_empty() {
                theme.accent
            } else {
                Color::new(0.854, 0.896, 0.936, 0.864)
            },
            FontWeight::Semibold,
            1.2 * s,
        );
    }

    /// The selected release on a backed pane: title and date, then its wrapped
    /// introduction and changes from `scroll`, cut off at the pane's bottom.
    fn pane_view(&mut self, pane: Rect, font: &UiFont, s: f32) {
        self.ui.panel(pane);
        let theme = self.ui.theme();
        let pad = 28.0 * s;
        let x = pane.x + pad;
        let width = pane.width - pad * 2.0;
        let bottom = pane.bottom() - pad;
        let mut y = pane.y + pad;
        if let Some(error) = &self.error {
            self.ui.text(
                error,
                Rect::new(x, y, width, 22.0 * s),
                16.0 * s,
                theme.critical,
                FontWeight::Regular,
                0.2 * s,
            );
            return;
        }
        let Some(release) = self.releases.get(self.selected) else {
            return;
        };
        self.ui.text(
            &release.meta,
            Rect::new(x, y, width, 18.0 * s),
            13.0 * s,
            theme.accent,
            FontWeight::Semibold,
            2.0 * s,
        );
        y += 26.0 * s;
        self.ui.text(
            &release.title,
            Rect::new(x, y, width, 36.0 * s),
            28.0 * s,
            theme.foreground,
            FontWeight::Semibold,
            0.0,
        );
        y += 52.0 * s;

        self.wrap(font, width, BODY_SIZE * s, 0.2 * s, BULLET_INDENT * s);
        let available = bottom - y;
        self.fit_scroll(available, |kind| kind.height() * s);

        let indent = BULLET_INDENT * s;
        let mut shown = 0_usize;
        for line in &self.lines[self.scroll..] {
            let height = line.kind.height() * s;
            if y + height > bottom + 0.5 {
                break;
            }
            shown += 1;
            match line.kind {
                LineKind::Gap => {}
                LineKind::Intro => self.ui.text(
                    &line.text,
                    Rect::new(x, y, width, BODY_LINE * s),
                    BODY_SIZE * s,
                    theme.muted,
                    FontWeight::Regular,
                    0.2 * s,
                ),
                LineKind::Change | LineKind::More => {
                    if line.kind == LineKind::Change {
                        self.ui.accent_bar(
                            Rect::new(x + 2.0 * s, y + 9.0 * s, 6.0 * s, 6.0 * s),
                            theme.accent,
                        );
                    }
                    self.ui.text(
                        &line.text,
                        Rect::new(x + indent, y, width - indent, BODY_LINE * s),
                        BODY_SIZE * s,
                        SOFT,
                        FontWeight::Regular,
                        0.2 * s,
                    );
                }
                LineKind::Credit => self.ui.text_aligned(
                    &line.text,
                    Rect::new(x + indent, y + 2.0 * s, width - indent, 16.0 * s),
                    CREDIT_SIZE * s,
                    theme.accent,
                    FontWeight::Semibold,
                    CREDIT_SPACING * s,
                    TextAlign::Start,
                ),
            }
            y += height;
        }
        self.page_lines = shown.saturating_sub(2).max(1);
        if self.max_scroll > 0 {
            let top = bottom - available;
            self.ui.scrollbar(
                PANE_BAR_TOKEN,
                Rect::new(pane.right() - 14.0 * s, top, 5.0 * s, available),
                self.scroll,
                shown,
                self.lines.len(),
            );
        }
    }
}

/// Split `text` at spaces into lines no wider than `width`; a word wider than
/// the line stays whole (the renderer ends it in an ellipsis).
pub(crate) fn wrap_words(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut end = 0;
    for (index, _) in text.match_indices(' ').chain([(text.len(), "")]) {
        if end > start && measure(&text[start..index]) > width {
            lines.push(&text[start..end]);
            start = end + 1;
        }
        end = index;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_wrap_at_the_width() {
        let measure = |text: &str| text.len() as f32;
        assert_eq!(
            wrap_words("the quick brown fox jumps", 10.0, measure),
            ["the quick", "brown fox", "jumps"]
        );
        assert_eq!(wrap_words("short", 10.0, measure), ["short"]);
        assert_eq!(
            wrap_words("unbreakableword x", 5.0, measure),
            ["unbreakableword", "x"]
        );
        assert!(wrap_words("", 10.0, measure).is_empty());
    }

    #[test]
    fn selection_scroll_and_close_stay_in_range() {
        let mut panel = Panel::new();
        assert!(panel.error.is_none(), "{:?}", panel.error);
        panel.rows = 2;
        panel.select(usize::MAX);
        assert_eq!(panel.selected, panel.releases.len() - 1);
        assert_eq!(panel.first, panel.releases.len() - 2);
        panel.max_scroll = 4;
        panel.scroll_pane(10);
        assert_eq!(panel.scroll, 4);
        panel.select(0);
        assert_eq!((panel.selected, panel.first, panel.scroll), (0, 0, 0));
        assert!(!panel.close());
        panel.open(true);
        assert!(panel.close());
    }
}
