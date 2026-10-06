//! The credits page: who makes Sol JK, from `assets/credits.txt` (built in,
//! parsed once; see `credits_data.rs`). Opened by the main menu's Credits entry,
//! the in-game SJK menu or the `credits` console command. Like the changelog it
//! lives in the console and is drawn in its place, so it opens over the menus and
//! in a match, and closes the console with itself when it opened it.
//!
//! The page is meant to shine: slow light beams drift across a deep backdrop,
//! sparks rise through it, SJK's emblem breathes above a title with a passing
//! glint, and each section's people sit on glass cards whose edges glow in turn.
//! With the classic menus the palette is retail's gold and blue and the text is
//! drawn in the menus' retail font. Sections come from the file, so a later
//! "Supporters" section needs no code.

use crate::menu::art::motion;
use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use crate::text::{TextFace, TextVertex, UiFont, visible_text_width_style};
use sjk_ui::{Color, DrawCommand, FontWeight, Gradient, InputEvent, Rect, TextAlign, UiEventKind};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "credits_data.rs"]
mod data;

/// Console command that toggles the page.
pub(crate) const COMMAND: &str = "credits";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str = "Show who makes Sol JK";

/// Wheel target over the page.
const PAGE_TOKEN: u16 = 906;
/// Pixels (at 1080 lines) one wheel notch or arrow key scrolls.
const STEP: f32 = 90.0;
/// The closing line, as in CREDITS.md.
const NOTICE: &str = "Star Wars, Jedi Knight and Jedi Academy are trademarks of their respective owners. Sol JK is a fan project, not affiliated with or endorsed by Lucasfilm, Disney, Raven Software or Activision.";
/// Sparks rising through the backdrop.
const SPARKS: usize = 64;

/// The page's colours: the modern theme's, or retail's for the classic menus.
#[derive(Clone, Copy)]
struct Palette {
    accent: Color,
    /// The glint and the brightest edges.
    shine: Color,
    title: Color,
    text: Color,
    muted: Color,
    /// Backdrop gradient, top and bottom.
    deep: [Color; 2],
    card: Color,
}

impl Palette {
    fn modern(accent: Color, foreground: Color, muted: Color) -> Self {
        Self {
            accent,
            shine: Color::new(1.0, 0.96, 0.88, 1.0),
            title: foreground,
            text: Color::new(0.916, 0.945, 0.973, 0.94),
            muted,
            deep: [
                Color::new(0.020, 0.030, 0.055, 0.97),
                Color::new(0.004, 0.006, 0.012, 0.99),
            ],
            card: Color::new(0.06, 0.08, 0.12, 0.72),
        }
    }

    /// Retail gold (`1 .682 0`), title blue (`.549 .854 1`) and list lilac.
    fn classic() -> Self {
        Self {
            accent: Color::new(1.0, 0.682, 0.0, 1.0),
            shine: Color::new(1.0, 0.95, 0.75, 1.0),
            title: Color::new(0.549, 0.854, 1.0, 1.0),
            text: Color::new(0.75, 0.75, 1.0, 1.0),
            muted: Color::new(0.615, 0.615, 0.956, 0.9),
            deep: [
                Color::new(0.010, 0.020, 0.090, 0.97),
                Color::new(0.002, 0.004, 0.025, 0.99),
            ],
            card: Color::new(0.04, 0.05, 0.20, 0.62),
        }
    }
}

fn alpha(color: Color, a: f32) -> Color {
    Color::new(color.r, color.g, color.b, color.a * a)
}

/// One laid-out piece of the scrolling content, at a scroll of zero.
enum Piece {
    Heading {
        y: f32,
        section: usize,
    },
    Card {
        rect: Rect,
        section: usize,
        card: usize,
        /// The contributions wrapped to the card: text, and whether it starts one.
        lines: Vec<(String, bool)>,
    },
}

/// Sizes at a scale of 1 (1080 lines).
const NAME: f32 = 30.0;
const HANDLE: f32 = 13.0;
const ROLE: f32 = 15.0;
const BODY: f32 = 15.0;
const BODY_LINE: f32 = 21.0;
const PAD: f32 = 24.0;
const GAP: f32 = 24.0;
const BULLET: f32 = 18.0;

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    sections: Vec<data::Section>,
    error: Option<String>,
    /// Pixels scrolled, and the most there is to scroll.
    scroll: f32,
    max_scroll: f32,
    /// Window height at the last frame, for paging.
    page: f32,
    pieces: Vec<Piece>,
    /// What `pieces` was laid out for: viewport and text size (bits).
    laid_out_for: Option<(u32, u32, u32)>,
    /// Height of the laid-out content, and where its closing notice sits.
    content: f32,
    notice_y: f32,
    classic: bool,
    opened_at: f64,
    ui: MenuCanvas,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub(crate) fn new() -> Self {
        let (sections, error) = match data::parse(data::EMBEDDED) {
            Ok(sections) => (sections, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        Self {
            open: false,
            owns_console: false,
            sections,
            error,
            scroll: 0.0,
            max_scroll: 0.0,
            page: 600.0,
            pieces: Vec::new(),
            laid_out_for: None,
            content: 0.0,
            notice_y: 0.0,
            classic: false,
            opened_at: 0.0,
            ui: MenuCanvas::with_capacities(256, 16_384, 1_024),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Show the page from the top; `owns_console` when the console was closed.
    pub(crate) fn open(&mut self, owns_console: bool) {
        self.open = true;
        self.owns_console = owns_console;
        self.scroll = 0.0;
        self.opened_at = motion::seconds();
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        let owned = self.open && self.owns_console;
        self.open = false;
        self.owns_console = false;
        owned
    }

    /// Choose the palette: retail's with the classic menus, else the theme's.
    pub(crate) fn set_classic(&mut self, classic: bool) {
        self.classic = classic;
    }

    /// Whether the classic palette is drawn, so its text uses the retail font.
    pub(crate) fn is_classic(&self) -> bool {
        self.classic
    }

    /// Skip the opening animation, for snapshots.
    #[cfg(test)]
    pub(crate) fn settle(&mut self) {
        self.opened_at = f64::MIN / 2.0;
    }

    /// Scroll to `pixels`, for snapshots (clamped by the next frame).
    #[cfg(test)]
    pub(crate) fn scroll_to(&mut self, pixels: f32) {
        self.scroll = pixels;
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    pub(crate) fn handle_key(&mut self, event: &KeyEvent) -> bool {
        if event.state != ElementState::Pressed {
            return false;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return false;
        };
        let step = STEP * self.page / 1080.0;
        match key {
            KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter => return true,
            KeyCode::ArrowUp => self.scroll_by(-step),
            KeyCode::ArrowDown => self.scroll_by(step),
            KeyCode::PageUp => self.scroll_by(-self.page * 0.8),
            KeyCode::PageDown | KeyCode::Space => self.scroll_by(self.page * 0.8),
            KeyCode::Home => self.scroll = 0.0,
            KeyCode::End => self.scroll = self.max_scroll,
            _ => {}
        }
        false
    }

    /// A pointer event; returns true when it closes the page.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent) -> bool {
        let Some(event) = self.ui.pointer(event) else {
            return false;
        };
        match event.kind {
            UiEventKind::Wheel => {
                let direction = event.delta.map_or(0.0, |delta| -delta.y.signum());
                self.scroll_by(direction * STEP * self.page / 1080.0);
                false
            }
            UiEventKind::Activate => event.token == Some(BACK_TOKEN),
            _ => false,
        }
    }

    fn scroll_by(&mut self, pixels: f32) {
        self.scroll = (self.scroll + pixels).clamp(0.0, self.max_scroll);
    }

    /// Lay the sections out for `viewport`: headings, then rows of cards
    /// centred in the column, each card as tall as its row's tallest.
    fn layout(&mut self, font: &UiFont, viewport: [f32; 2]) {
        let s = crate::ui_scale::height_scale(viewport[1]);
        let style = font.style();
        let key = (
            viewport[0].to_bits(),
            viewport[1].to_bits(),
            style.scale.to_bits(),
        );
        if self.laid_out_for == Some(key) {
            return;
        }
        self.laid_out_for = Some(key);
        self.pieces.clear();
        let measure = |text: &str| {
            let size = BODY * s;
            let placement = style.place(Rect::new(0.0, 0.0, 0.0, size), size, 0.2 * s);
            visible_text_width_style(
                font,
                text,
                placement.size / font.height.max(1.0),
                TextFace::Regular,
                placement.letter_spacing,
            )
        };
        let margin = (viewport[0] * 0.07).max(40.0 * s);
        let column = viewport[0] - margin * 2.0;
        let gap = GAP * s;
        let columns = ((column + gap) / (420.0 * s + gap)).floor().clamp(1.0, 3.0) as usize;
        let mut y = header_height(s);
        for (section_index, section) in self.sections.iter().enumerate() {
            self.pieces.push(Piece::Heading {
                y,
                section: section_index,
            });
            y += 56.0 * s;
            for (row_index, row) in section.cards.chunks(columns).enumerate() {
                let count = row.len();
                let width = ((column - gap * (columns - 1) as f32) / columns as f32).min(620.0 * s);
                let row_width = width * count as f32 + gap * (count - 1) as f32;
                let left = (viewport[0] - row_width) * 0.5;
                let text_width = width - PAD * 2.0 * s - BULLET * s;
                let mut wrapped: Vec<Vec<(String, bool)>> = Vec::with_capacity(count);
                let mut tallest: f32 = 0.0;
                for card in row {
                    let mut lines = Vec::new();
                    for did in &card.did {
                        for (index, line) in
                            crate::console::changelog::wrap_words(did, text_width, measure)
                                .into_iter()
                                .enumerate()
                        {
                            lines.push((line.to_owned(), index == 0));
                        }
                    }
                    tallest = tallest.max(card_height(lines.len(), card.did.len(), s));
                    wrapped.push(lines);
                }
                for (slot, lines) in wrapped.into_iter().enumerate() {
                    self.pieces.push(Piece::Card {
                        rect: Rect::new(left + slot as f32 * (width + gap), y, width, tallest),
                        section: section_index,
                        card: row_index * columns + slot,
                        lines,
                    });
                }
                y += tallest + gap;
            }
            y += 20.0 * s;
        }
        self.notice_y = y;
        self.content = y + 70.0 * s;
    }

    /// Draw the page over the whole frame. Text other overlays appended earlier
    /// this frame is dropped rather than shown through, as for the changelog.
    pub(crate) fn append(
        &mut self,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        vertices.clear();
        let s = crate::ui_scale::height_scale(viewport[1]);
        self.page = viewport[1];
        self.layout(font, viewport);
        let footer = 64.0 * s;
        let view_height = viewport[1] - footer;
        self.max_scroll = (self.content - view_height).max(0.0);
        self.scroll = self.scroll.clamp(0.0, self.max_scroll);

        self.ui.begin_transparent(viewport);
        let theme = self.ui.theme();
        let palette = if self.classic {
            Palette::classic()
        } else {
            Palette::modern(theme.accent, theme.foreground, theme.muted)
        };
        let now = motion::seconds();
        let since = (now - self.opened_at).max(0.0) as f32;
        backdrop(&mut self.ui, viewport, palette, now as f32);

        let view = Rect::new(0.0, 0.0, viewport[0], view_height);
        self.ui.scroll_region(PAGE_TOKEN, view);
        let _ = self.ui.draw_list_mut().push(DrawCommand::PushClip(view));
        let top = -self.scroll;
        self.header(viewport, top, palette, now, s);
        if let Some(error) = &self.error {
            self.ui.text_aligned(
                error,
                Rect::new(0.0, top + header_height(s), viewport[0], 24.0 * s),
                16.0 * s,
                theme.critical,
                FontWeight::Regular,
                0.2 * s,
                TextAlign::Center,
            );
        }
        for index in 0..self.pieces.len() {
            self.piece(
                index,
                viewport,
                top,
                view_height,
                palette,
                since,
                now as f32,
                s,
            );
        }
        self.ui.text_aligned(
            NOTICE,
            Rect::new(0.0, top + self.notice_y, viewport[0], 18.0 * s),
            11.0 * s,
            alpha(palette.muted, 0.8),
            FontWeight::Regular,
            0.3 * s,
            TextAlign::Center,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::PopClip);

        self.footer(viewport, footer, palette, s);
        self.ui.finish(BACK_TOKEN);
        self.ui.append_text(vertices, font, viewport);
    }

    /// The breathing emblem, the title with its glint, and the line under it.
    fn header(&mut self, viewport: [f32; 2], top: f32, palette: Palette, now: f64, s: f32) {
        let side = 120.0 * s;
        let emblem = Rect::new((viewport[0] - side) * 0.5, top + 36.0 * s, side, side);
        // A soft halo behind the emblem, swelling with its glow.
        let swell = 0.5 + 0.5 * (now as f32 * 1.4).sin();
        for ring in 0..3 {
            let grow = (18.0 + ring as f32 * 16.0) * s;
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: Rect::new(
                    emblem.x - grow,
                    emblem.y - grow,
                    emblem.width + grow * 2.0,
                    emblem.height + grow * 2.0,
                ),
                radius: emblem.width * 0.5 + grow,
                color: alpha(palette.accent, (0.05 + 0.04 * swell) / (ring + 1) as f32),
            });
        }
        crate::menu::emblem::draw(&mut self.ui, emblem, now);

        let title = Rect::new(0.0, emblem.bottom() + 14.0 * s, viewport[0], 84.0 * s);
        // A glint crosses the title every few seconds (text draws above it).
        let period = 4.5;
        let phase = ((now % period) / period) as f32;
        let glint_width = 260.0 * s;
        let glint_x = -glint_width + phase * (viewport[0] + glint_width * 2.0);
        let _ = self.ui.draw_list_mut().push(DrawCommand::PushClip(title));
        let band = Rect::new(glint_x, title.y, glint_width * 0.5, title.height);
        horizontal(
            &mut self.ui,
            band,
            alpha(palette.shine, 0.0),
            alpha(palette.shine, 0.22),
        );
        horizontal(
            &mut self.ui,
            Rect::new(band.right(), title.y, glint_width * 0.5, title.height),
            alpha(palette.shine, 0.22),
            alpha(palette.shine, 0.0),
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::PopClip);
        let shimmer = 0.5 + 0.5 * (now as f32 * 0.9).sin();
        let title_color = mix(palette.title, palette.shine, shimmer * 0.35);
        self.ui.text_aligned(
            "CREDITS",
            title,
            76.0 * s,
            title_color,
            FontWeight::Semibold,
            14.0 * s,
            TextAlign::Center,
        );
        self.ui.text_aligned(
            "SOL JK   /   THE PEOPLE WHO MAKE IT",
            Rect::new(0.0, title.bottom() + 6.0 * s, viewport[0], 20.0 * s),
            13.0 * s,
            palette.accent,
            FontWeight::Semibold,
            3.6 * s,
            TextAlign::Center,
        );
    }

    /// One heading or card, faded and lifted in as the page opens.
    #[allow(clippy::too_many_arguments)]
    fn piece(
        &mut self,
        index: usize,
        viewport: [f32; 2],
        top: f32,
        view_height: f32,
        palette: Palette,
        since: f32,
        now: f32,
        s: f32,
    ) {
        // Each piece arrives 0.08 s after the one before it.
        let arrival = ((since - index as f32 * 0.08) / 0.45).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - arrival).powi(3);
        let lift = (1.0 - ease) * 24.0 * s;
        match &self.pieces[index] {
            Piece::Heading { y, section } => {
                let y = top + y + lift;
                if y > view_height || y + 40.0 * s < 0.0 {
                    return;
                }
                let title = &self.sections[*section].title;
                let _ = self.ui.draw_list_mut().push(DrawCommand::PushOpacity(ease));
                let line_width = 200.0 * s;
                let middle = viewport[0] * 0.5;
                let line_y = y + 15.0 * s;
                horizontal(
                    &mut self.ui,
                    Rect::new(middle - 180.0 * s - line_width, line_y, line_width, 1.5 * s),
                    alpha(palette.accent, 0.0),
                    alpha(palette.accent, 0.8),
                );
                horizontal(
                    &mut self.ui,
                    Rect::new(middle + 180.0 * s, line_y, line_width, 1.5 * s),
                    alpha(palette.accent, 0.8),
                    alpha(palette.accent, 0.0),
                );
                self.ui.text_fmt_aligned(
                    format_args!("{}", crate::menu::classic::view::Caps(title)),
                    Rect::new(middle - 175.0 * s, y + 4.0 * s, 350.0 * s, 24.0 * s),
                    17.0 * s,
                    palette.accent,
                    FontWeight::Semibold,
                    5.0 * s,
                    TextAlign::Center,
                );
                let _ = self.ui.draw_list_mut().push(DrawCommand::PopOpacity);
            }
            Piece::Card {
                rect,
                section,
                card,
                lines,
            } => {
                let rect = Rect::new(rect.x, top + rect.y + lift, rect.width, rect.height);
                if rect.y > view_height || rect.bottom() < 0.0 {
                    return;
                }
                let featured = *section == 0;
                let person = &self.sections[*section].cards[*card];
                let canvas = &mut self.ui;
                let _ = canvas.draw_list_mut().push(DrawCommand::PushOpacity(ease));
                // The edge glows in turn around the cards.
                let glow = 0.5 + 0.5 * (now * 1.1 - index as f32 * 0.7).sin();
                let draw = canvas.draw_list_mut();
                for ring in 1..=3 {
                    let grow = ring as f32 * 3.0 * s;
                    let _ = draw.push(DrawCommand::RoundedRect {
                        rect: Rect::new(
                            rect.x - grow,
                            rect.y - grow,
                            rect.width + grow * 2.0,
                            rect.height + grow * 2.0,
                        ),
                        radius: 14.0 * s + grow,
                        color: alpha(palette.accent, (0.03 + 0.05 * glow) / ring as f32),
                    });
                }
                let _ = draw.push(DrawCommand::RoundedRect {
                    rect,
                    radius: 14.0 * s,
                    color: palette.card,
                });
                let _ = draw.push(DrawCommand::GradientRect {
                    rect: Rect::new(rect.x, rect.y, rect.width, rect.height * 0.5),
                    radius: 14.0 * s,
                    gradient: Gradient {
                        start: Color::new(1.0, 1.0, 1.0, 0.06),
                        end: Color::new(1.0, 1.0, 1.0, 0.0),
                        vertical: true,
                    },
                });
                let _ = draw.push(DrawCommand::Border {
                    rect,
                    radius: 14.0 * s,
                    width: (1.5 * s).max(1.0),
                    color: alpha(palette.accent, 0.25 + 0.45 * glow),
                });
                // A bright cap along the top edge, shining from the middle.
                let cap = Rect::new(
                    rect.x + 18.0 * s,
                    rect.y,
                    (rect.width - 36.0 * s) * 0.5,
                    2.0 * s,
                );
                horizontal(
                    canvas,
                    cap,
                    alpha(palette.shine, 0.0),
                    alpha(palette.shine, 0.7 * glow + 0.2),
                );
                horizontal(
                    canvas,
                    Rect::new(cap.right(), cap.y, cap.width, cap.height),
                    alpha(palette.shine, 0.7 * glow + 0.2),
                    alpha(palette.shine, 0.0),
                );

                let x = rect.x + PAD * s;
                let width = rect.width - PAD * 2.0 * s;
                let mut y = rect.y + 18.0 * s;
                let name_size = if featured { NAME } else { NAME * 0.8 };
                canvas.text(
                    &person.name,
                    Rect::new(x, y, width, name_size * 1.25 * s),
                    name_size * s,
                    palette.title,
                    FontWeight::Semibold,
                    0.4 * s,
                );
                if !person.github.is_empty() {
                    canvas.text_aligned(
                        &person.github,
                        Rect::new(x, y + 8.0 * s, width, 18.0 * s),
                        HANDLE * s,
                        palette.accent,
                        FontWeight::Semibold,
                        1.0 * s,
                        TextAlign::End,
                    );
                }
                y += name_size * 1.25 * s + 4.0 * s;
                canvas.text(
                    &person.role,
                    Rect::new(x, y, width, 20.0 * s),
                    ROLE * s,
                    palette.muted,
                    FontWeight::Regular,
                    0.3 * s,
                );
                y += 30.0 * s;
                if !lines.is_empty() {
                    let _ = canvas.draw_list_mut().push(DrawCommand::SolidRect {
                        rect: Rect::new(x, y - 8.0 * s, width, s.max(1.0)),
                        color: alpha(palette.accent, 0.25),
                    });
                }
                for (line, first) in lines {
                    if *first {
                        y += 4.0 * s;
                        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
                            rect: Rect::new(x + 2.0 * s, y + 8.0 * s, 6.0 * s, 6.0 * s),
                            radius: 3.0 * s,
                            color: palette.accent,
                        });
                    }
                    canvas.text(
                        line,
                        Rect::new(x + BULLET * s, y, width - BULLET * s, BODY_LINE * s),
                        BODY * s,
                        palette.text,
                        FontWeight::Regular,
                        0.2 * s,
                    );
                    y += BODY_LINE * s;
                }
                let _ = canvas.draw_list_mut().push(DrawCommand::PopOpacity);
            }
        }
    }

    /// The close cap and a scroll cue, over a dark strip.
    fn footer(&mut self, viewport: [f32; 2], height: f32, palette: Palette, s: f32) {
        let strip = Rect::new(0.0, viewport[1] - height, viewport[0], height);
        let _ = self.ui.draw_list_mut().push(DrawCommand::GradientRect {
            rect: strip,
            radius: 0.0,
            gradient: Gradient {
                start: alpha(palette.deep[1], 0.0),
                end: palette.deep[1],
                vertical: true,
            },
        });
        let hovered = self.ui.token_hovered(BACK_TOKEN);
        let close = Rect::new(
            (viewport[0] - 200.0 * s) * 0.5,
            strip.y + 14.0 * s,
            200.0 * s,
            36.0 * s,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: close,
            radius: 18.0 * s,
            width: (1.5 * s).max(1.0),
            color: alpha(palette.accent, if hovered { 1.0 } else { 0.55 }),
        });
        if hovered {
            let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                rect: close,
                radius: 18.0 * s,
                color: alpha(palette.accent, 0.18),
            });
        }
        self.ui.text_aligned(
            "CLOSE",
            Rect::new(close.x, close.y + 8.0 * s, close.width, 20.0 * s),
            15.0 * s,
            if hovered {
                palette.shine
            } else {
                palette.accent
            },
            FontWeight::Semibold,
            3.0 * s,
            TextAlign::Center,
        );
        self.ui.hit_region(BACK_TOKEN, close);
        if self.max_scroll > 0.0 && self.scroll < self.max_scroll - 1.0 {
            self.ui.text_aligned(
                "SCROLL FOR MORE",
                Rect::new(
                    close.right() + 24.0 * s,
                    close.y + 10.0 * s,
                    260.0 * s,
                    16.0 * s,
                ),
                11.0 * s,
                palette.muted,
                FontWeight::Semibold,
                2.0 * s,
                TextAlign::Start,
            );
        }
    }
}

/// Height above the first section: emblem, title and the line under it.
fn header_height(s: f32) -> f32 {
    300.0 * s
}

/// A card's height for `lines` wrapped lines from `items` contributions.
fn card_height(lines: usize, items: usize, s: f32) -> f32 {
    let head = 18.0 + NAME * 1.25 + 4.0 + 30.0;
    let body = lines as f32 * BODY_LINE + items as f32 * 4.0;
    (head + body + 22.0) * s
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn horizontal(canvas: &mut MenuCanvas, rect: Rect, start: Color, end: Color) {
    let _ = canvas.draw_list_mut().push(DrawCommand::GradientRect {
        rect,
        radius: 0.0,
        gradient: Gradient {
            start,
            end,
            vertical: false,
        },
    });
}

/// The deep backdrop, three slow light beams drifting across it and sparks
/// rising through it, all from the clock so nothing is stored per frame.
fn backdrop(canvas: &mut MenuCanvas, viewport: [f32; 2], palette: Palette, now: f32) {
    let [width, height] = viewport;
    let _ = canvas.draw_list_mut().push(DrawCommand::GradientRect {
        rect: Rect::new(0.0, 0.0, width, height),
        radius: 0.0,
        gradient: Gradient {
            start: palette.deep[0],
            end: palette.deep[1],
            vertical: true,
        },
    });
    for beam in 0..3 {
        let speed = 0.021 + beam as f32 * 0.009;
        let phase = (now * speed + beam as f32 * 0.37).fract();
        let beam_width = width * (0.22 + beam as f32 * 0.06);
        let x = -beam_width + phase * (width + beam_width * 2.0);
        let strength = 0.045 + 0.02 * (now * 0.5 + beam as f32).sin();
        horizontal(
            canvas,
            Rect::new(x, 0.0, beam_width * 0.5, height),
            alpha(palette.accent, 0.0),
            alpha(palette.accent, strength),
        );
        horizontal(
            canvas,
            Rect::new(x + beam_width * 0.5, 0.0, beam_width * 0.5, height),
            alpha(palette.accent, strength),
            alpha(palette.accent, 0.0),
        );
    }
    let s = crate::ui_scale::height_scale(height);
    for spark in 0..SPARKS {
        let seed = hash(spark as u32);
        let column = (seed & 0xffff) as f32 / 65_535.0;
        let speed = 18.0 + ((seed >> 16) & 0xff) as f32 / 255.0 * 46.0;
        let start = ((seed >> 24) & 0xff) as f32 / 255.0;
        let rise = (start + now * speed * s / height).fract();
        let y = height * (1.0 - rise);
        let sway = (now * 0.6 + spark as f32).sin() * 14.0 * s;
        let size = (1.5 + (seed % 5) as f32 * 0.6) * s;
        let twinkle = 0.25 + 0.75 * (0.5 + 0.5 * (now * 2.3 + spark as f32 * 1.7).sin());
        // Sparks fade in at the bottom and out at the top.
        let fade = (rise * 4.0).min(1.0) * ((1.0 - rise) * 3.0).min(1.0);
        let color = if spark % 3 == 0 {
            palette.shine
        } else {
            palette.accent
        };
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: Rect::new(column * width + sway, y, size, size),
            radius: size * 0.5,
            color: alpha(color, 0.55 * twinkle * fade),
        });
    }
}

/// A well-mixed 32-bit hash, so sparks spread without a random generator.
fn hash(mut value: u32) -> u32 {
    value = value.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolling_stays_in_range_and_close_reports_ownership() {
        let mut panel = Panel::new();
        assert!(panel.error.is_none(), "{:?}", panel.error);
        panel.max_scroll = 500.0;
        panel.scroll_by(10_000.0);
        assert_eq!(panel.scroll, 500.0);
        panel.scroll_by(-10_000.0);
        assert_eq!(panel.scroll, 0.0);
        assert!(!panel.close());
        panel.open(true);
        assert!(panel.close());
    }

    #[test]
    fn sparks_spread_across_the_width() {
        let columns: Vec<u32> = (0..SPARKS as u32)
            .map(|i| (hash(i) & 0xffff) / 6_554)
            .collect();
        for bucket in 0..10 {
            assert!(columns.contains(&bucket), "no spark in tenth {bucket}");
        }
    }
}
