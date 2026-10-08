//! Floating message typography and deterministic motion on the shared canvas.

mod composer;
mod draft;
mod name;
mod player_menu;

use super::*;
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextFace, visible_text_width_face};
use layout::Geometry;
use sjk_ui::{Color, DrawCommand, Easing, FontWeight, Rect, TextAlign};

pub(super) fn tint(channel: Channel, alpha: f32) -> Color {
    match channel {
        Channel::Global => Color::new(0.70, 0.88, 0.98, alpha),
        Channel::Team => Color::new(0.40, 0.86, 0.86, alpha),
        Channel::Whisper => Color::new(0.84, 0.68, 1.0, alpha),
    }
}

pub(super) fn visibility(age: u64, active: bool) -> f32 {
    let entering = Tween::new(0.0, 1.0, 0, ENTER_MS, Easing::EaseOutCubic).sample(age);
    let leaving = if active {
        1.0
    } else {
        Tween::new(1.0, 0.0, HOLD_MS, FADE_MS as u32, Easing::SmoothStep).sample(age)
    };
    entering * leaving
}

impl ChatOverlay {
    pub(super) fn build(&mut self, draw_feed: bool, font: &UiFont, viewport: [f32; 2], ms: u64) {
        if self.layout_viewport != viewport {
            for line in &mut self.lines {
                line.y = None;
            }
            self.layout_viewport = viewport;
            self.pressed_action = None;
        }
        self.ui.begin_transparent(viewport);
        self.visible_targets.fill(None);
        let mut g = self.options.geometry(viewport);
        if self.scoreboard_layout {
            let layout = crate::scoreboard::layout::Layout::new(viewport);
            g.scale = layout.scale * self.options.font.min(1.5);
            g.left = layout.chat_left;
            g.width = layout.chat_width;
            g.top = 150.0 * layout.scale;
            g.bottom = viewport[1] - 200.0 * g.scale;
            g.font = 18.0 * g.scale;
            g.row = g.font * layout::ROW_PITCH;
        }
        if self.is_typing() || (draw_feed && self.options.lifetime != 0) {
            self.build_feed(font, &g, ms);
        }
        let center_start = self.ui.next_text_id();
        if let Some((text, received)) = &self.center {
            let age = ms.saturating_sub(*received);
            if age < self.options.center_time && !self.scoreboard_layout {
                // CG_DrawCenterString (cg_draw.c:4488): every `\n` row is its own
                // centred draw with the base colour, so colour codes never bleed
                // into the next row; the block sits around cg.centerPrintY (0.30).
                let base = self.ui.theme().foreground;
                let fade = self.options.center_time.saturating_sub(age).min(200) as f32 / 200.0;
                let color = Color::new(base.r, base.g, base.b, base.a * fade);
                let size = self.options.center_size;
                let center_scale = crate::ui_scale::height_scale(viewport[1]);
                let row = 30.0 * center_scale * size;
                let rows = center_rows(text).count() as f32;
                let height = if self.options.center_height == 0.0 {
                    self.combat.height.unwrap_or(0.0)
                } else {
                    self.options.center_height
                };
                let mut y = viewport[1] * if height == 0.0 { 0.30 } else { height / 480.0 }
                    - rows * row * 0.5;
                for line in center_rows(text) {
                    self.ui.text_aligned(
                        line,
                        Rect::new(32.0, y, viewport[0] - 64.0, row),
                        22.0 * center_scale * size,
                        color,
                        FontWeight::Semibold,
                        0.0,
                        TextAlign::Center,
                    );
                    y += row;
                }
            } else if age >= self.options.center_time {
                self.combat.spare = self.center.take().unwrap().0;
                self.combat.height = None;
            }
        }
        self.center_text = (center_start, self.ui.next_text_id());
        if !self.scoreboard_layout {
            self.draw_plums();
        }
        if self.is_typing() {
            self.build_composer(font, &g, ms);
            self.build_player_menu(&g, viewport);
        }
        self.ui.finish(
            self.player_menu
                .and_then(|menu| menu.selected)
                .unwrap_or(u16::MAX),
        );
    }

    fn build_feed(&mut self, font: &UiFont, g: &Geometry, ms: u64) {
        let active = self.is_typing() || self.options.history;
        let limit = self.options.lines;
        let mut visible = [(0_usize, 0.0_f32, 0.0_f32); MAX_VISIBLE];
        let mut count = 0;
        let mut bottom = g.bottom;
        for (index, line) in self.lines.iter_mut().enumerate().rev().skip(self.scroll) {
            let age = ms.saturating_sub(line.received_ms);
            let alpha = if active {
                visibility(age, true)
            } else {
                visibility(age, true)
                    * (self.options.lifetime.saturating_sub(age).min(2000) as f32 / 2000.0)
            };
            if (!active && line.muted) || alpha <= 0.0 {
                continue;
            }
            line.wrap
                .update(&line.body, font, g.width - 12.0 * g.scale, g.font);
            let rows = if line.muted { 1 } else { line.wrap.len };
            let header = if line.name.is_empty() {
                0.0
            } else {
                layout::NAME_ADVANCE
            };
            let height = rows as f32 * g.row + (header + layout::MESSAGE_GAP) * g.scale;
            let top = bottom - height;
            if top < g.top || count == limit {
                break;
            }
            visible[count] = (index, top, alpha);
            count += 1;
            bottom = top;
        }
        for (token, (index, top, alpha)) in visible[..count].iter().copied().enumerate().rev() {
            let line = &mut self.lines[index];
            let age = ms.saturating_sub(line.received_ms);
            let y = line.y.get_or_insert_with(|| Tween::settled(top));
            if (y.target() - top).abs() > 0.1 {
                y.retarget(top, ms, ENTER_MS, Easing::EaseOutCubic);
            }
            let y = y.sample(ms);
            let slide =
                Tween::new(14.0 * g.scale, 0.0, 0, ENTER_MS, Easing::EaseOutCubic).sample(age);
            let x = g.left + slide;
            let mut body_y = y;
            if !line.name.is_empty() {
                let end = layout::fitting_end(&line.name, font, g.width * 0.65, 16.0 * g.scale);
                let name = &line.name[..end];
                let width = visible_text_width_face(
                    font,
                    name,
                    16.0 * g.scale / font.height,
                    TextFace::Semibold,
                );
                let actionable = line
                    .sender
                    .filter(|target| self.roster.name(*target).is_some());
                let friend = actionable
                    .and_then(|target| self.roster.name(target))
                    .is_some_and(|name| self.friends.contains(name));
                let name_x = x + if friend { 14.0 * g.scale } else { 0.0 };
                let text_rect = Rect::new(name_x, y, width + 1.0, 24.0 * g.scale);
                let rect = name::ink_bounds(font, name, [name_x, y], 16.0 * g.scale);
                if friend {
                    name::star(
                        &mut self.ui,
                        Rect::new(
                            x,
                            rect.y + (rect.height - 12.0 * g.scale) * 0.5,
                            12.0 * g.scale,
                            12.0 * g.scale,
                        ),
                        Color::new(0.95, 0.78, 0.35, alpha),
                    );
                }
                if active && let Some(target) = actionable {
                    self.visible_targets[token] = Some(target);
                    self.ui.hit_region(token as u16, rect);
                }
                let hover = active && self.ui.token_hovered(token as u16);
                if hover {
                    self.ui.accent_bar(rect, Color::new(0.70, 0.88, 0.98, 0.16));
                }
                self.ui.text(
                    name,
                    text_rect,
                    16.0 * g.scale,
                    if hover {
                        Color::new(1.0, 1.0, 1.0, alpha)
                    } else {
                        tint(line.channel, alpha)
                    },
                    FontWeight::Semibold,
                    0.0,
                );
                let label = if line.muted {
                    "IGNORED"
                } else {
                    match line.channel {
                        Channel::Team => "TEAM",
                        Channel::Whisper => "WHISPER",
                        Channel::Global => "",
                    }
                };
                self.ui.text(
                    label,
                    Rect::new(
                        rect.right() + 6.0 * g.scale,
                        y + 4.0 * g.scale,
                        160.0 * g.scale,
                        18.0 * g.scale,
                    ),
                    10.0 * g.scale,
                    tint(line.channel, alpha * 0.75),
                    FontWeight::Semibold,
                    1.0 * g.scale,
                );
                body_y += layout::NAME_ADVANCE * g.scale;
            }
            if line.muted {
                self.ui.text(
                    "Messages hidden on this client",
                    Rect::new(x, body_y, g.width, row_box(g)),
                    15.0 * g.scale,
                    Color::new(0.821, 0.854, 0.886, alpha * 0.853),
                    FontWeight::Regular,
                    0.0,
                );
            } else {
                for (row, range) in line.wrap.rows[..line.wrap.len].iter().enumerate() {
                    let color = Color::new(0.982, 0.987, 0.996, alpha);
                    let truncated = row + 1 == line.wrap.len && range.end < line.body.len();
                    body_row(
                        &mut self.ui,
                        BodyRow {
                            text: &line.body[range.clone()],
                            marks: &line.emojis,
                            emojis: self.options.emojis.then_some(&self.emojis),
                            truncated,
                        },
                        font,
                        Rect::new(x, body_y + row as f32 * g.row, g.width, row_box(g)),
                        g.font,
                        color,
                    );
                }
            }
        }
    }
}

/// One wrapped row of a message body and the emojis its marks stand for.
struct BodyRow<'a> {
    text: &'a str,
    marks: &'a [u16],
    /// The pictures, while `cg_chatBoxEmojis` is on; off, a mark is a blank.
    emojis: Option<&'a emoji::Emojis>,
    truncated: bool,
}

/// Draw a body row: as one text when it holds no emoji, else as its text runs
/// with each emoji picture between them, a run after a picture keeping the colour
/// the row had reached.
fn body_row(
    ui: &mut MenuCanvas,
    row: BodyRow<'_>,
    font: &UiFont,
    rect: Rect,
    size: f32,
    color: Color,
) {
    let ellipsis = if row.truncated { "..." } else { "" };
    let is_mark = |character: char| emoji::mark_index(character).is_some();
    if !row.text.contains(is_mark) {
        ui.text_fmt_aligned(
            format_args!("{}{ellipsis}", row.text),
            rect,
            size,
            color,
            FontWeight::Regular,
            0.0,
            TextAlign::Start,
        );
        return;
    }
    let mut x = rect.x;
    let mut colour = "";
    let mut rest = row.text;
    loop {
        let (run, after) = rest.split_at(rest.find(is_mark).unwrap_or(rest.len()));
        let tail = if after.is_empty() { ellipsis } else { "" };
        if !run.is_empty() || !tail.is_empty() {
            ui.text_fmt_aligned(
                format_args!("{colour}{run}{tail}"),
                Rect::new(x, rect.y, (rect.right() - x).max(1.0), rect.height),
                size,
                color,
                FontWeight::Regular,
                0.0,
                TextAlign::Start,
            );
            x += visible_text_width_face(font, run, size / font.height, TextFace::Regular);
            colour = crate::text::last_colour(run).unwrap_or(colour);
        }
        let Some(mark) = after.chars().next() else {
            break;
        };
        let picture = emoji::mark_index(mark)
            .and_then(|index| row.marks.get(index))
            .zip(row.emojis)
            .and_then(|(&emoji, emojis)| emojis.picture(emoji));
        if let Some((texture, uv)) = picture {
            let side = emoji::side(size);
            let _ = ui.draw_list_mut().push(DrawCommand::TexturedQuadUv {
                rect: Rect::new(x, rect.y + (size - side) * 0.5, side, side),
                texture,
                color: Color::new(1.0, 1.0, 1.0, color.a),
                uv,
            });
        }
        x += emoji::advance(size);
        rest = &after[mark.len_utf8()..];
    }
}

/// Height of the rectangle a body row is drawn and clipped in: its whole line box
/// and 1 px glyph shadow, whatever the row pitch.
fn row_box(g: &Geometry) -> f32 {
    g.row.max(g.font + 1.0)
}

/// CG_DrawCenterString wraps rows longer than this at their last space ([BugFix19];
/// `BG_IsWhiteSpace` counts only the space, so a tab or vertical tab in a name
/// does not break the row).
/// Retail counts bytes of its one-byte code page, so the limit counts characters:
/// `×` or `é` in a name is one byte there but two in UTF-8.
const CENTER_WRAP_CHARS: usize = 50;

/// Rows of a centre print: explicit `\n` breaks, then whitespace wrapping of
/// over-long rows, without allocating.
pub(super) fn center_rows(text: &str) -> impl Iterator<Item = &str> {
    text.trim_end_matches('\n')
        .split('\n')
        .flat_map(|row| WrapRow { rest: row })
}

struct WrapRow<'a> {
    rest: &'a str,
}

impl<'a> Iterator for WrapRow<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        // Byte offset of the first character past the limit, if the row is longer.
        // Slicing at a fixed byte count panicked when it fell inside a character.
        let cut = match self.rest.char_indices().nth(CENTER_WRAP_CHARS) {
            Some((limit, _)) => self.rest[..limit]
                .char_indices()
                .rev()
                .find(|&(_, character)| character == ' ')
                .map_or(limit, |(index, _)| index + 1),
            None => self.rest.len(),
        };
        let (head, tail) = self.rest.split_at(cut);
        self.rest = tail.trim_start_matches(' ');
        Some(head.trim_end_matches(' '))
    }
}

#[cfg(test)]
mod tests {
    use super::center_rows;
    use super::{BodyRow, body_row, emoji};
    use crate::menu_widgets::MenuCanvas;
    use sjk_ui::{Color, DrawCommand, Rect};

    /// The texts and emoji rectangles drawn for `body` with `names` loaded.
    fn drawn(names: &[&str], body: &str, truncated: bool) -> (Vec<(f32, String)>, Vec<Rect>) {
        drawn_shown(names, body, truncated, true)
    }

    fn drawn_shown(
        names: &[&str],
        body: &str,
        truncated: bool,
        shown: bool,
    ) -> (Vec<(f32, String)>, Vec<Rect>) {
        let emojis = emoji::Emojis::from_names(names);
        let (text, marks) = emojis.markup(body);
        let mut ui = MenuCanvas::with_text_capacity(512);
        ui.begin_transparent([640.0, 480.0]);
        let font = crate::text::test_font();
        body_row(
            &mut ui,
            BodyRow {
                text: &text,
                marks: &marks,
                emojis: shown.then_some(&emojis),
                truncated,
            },
            &font,
            Rect::new(10.0, 20.0, 400.0, 13.0),
            12.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        let mut texts = Vec::new();
        let mut pictures = Vec::new();
        for command in ui.draw_list().commands() {
            match command {
                DrawCommand::Text { rect, text, .. } => {
                    texts.push((rect.x, ui.stored_text(*text).to_owned()));
                }
                DrawCommand::TexturedQuadUv { rect, .. } => pictures.push(*rect),
                _ => {}
            }
        }
        (texts, pictures)
    }

    #[test]
    fn a_row_without_emojis_is_one_text() {
        let (texts, pictures) = drawn(&[":x:"], "^2plain", true);
        assert_eq!(texts, [(10.0, "^2plain...".to_owned())]);
        assert!(pictures.is_empty());
    }

    #[test]
    fn an_emoji_sits_between_its_text_runs_and_the_colour_carries_on() {
        // Test glyphs advance 8 units at height 12, so "^1hi " is 24 wide.
        let (texts, pictures) = drawn(&[":x:"], "^1hi :x: there", false);
        let after = 10.0 + 24.0 + emoji::advance(12.0);
        assert_eq!(
            texts,
            [(10.0, "^1hi ".to_owned()), (after, "^1 there".to_owned())]
        );
        assert_eq!(pictures.len(), 1);
        assert_eq!(pictures[0].x, 34.0);
        assert_eq!(pictures[0].width, emoji::side(12.0));
    }

    #[test]
    fn switched_off_the_pictures_leave_their_blanks() {
        let (texts, pictures) = drawn_shown(&[":x:"], "hi :x: there", false, false);
        assert!(pictures.is_empty());
        assert_eq!(texts[1].0, 10.0 + 24.0 + emoji::advance(12.0));
    }

    #[test]
    fn a_row_ending_in_an_emoji_still_shows_the_ellipsis() {
        let (texts, pictures) = drawn(&[":x:"], "a:x:", true);
        assert_eq!(pictures.len(), 1);
        assert_eq!(texts.last().map(|(_, text)| text.as_str()), Some("..."));
    }

    #[test]
    fn center_rows_wrap_long_rows_at_a_space() {
        let text = format!("{} {}", "a".repeat(30), "b".repeat(30));
        let rows: Vec<_> = center_rows(&text).collect();
        assert_eq!(rows, ["a".repeat(30), "b".repeat(30)]);
    }

    #[test]
    fn center_rows_do_not_split_a_character_at_the_limit() {
        // 49 ASCII bytes put `×` across UTF-8 bytes 49..51: a byte limit of 50
        // sliced inside it and crashed on a duel challenge from such a name.
        let name = format!("{}×", "x".repeat(49));
        let text = format!("{name} has challenged you to a duel!");
        let rows: Vec<_> = center_rows(&text).collect();
        assert_eq!(rows, [name.as_str(), "has challenged you to a duel!"]);
    }

    #[test]
    fn center_rows_break_only_at_a_space() {
        // A name with vertical tabs (0x0B) stays on the row after the last space.
        let text = "You have challenged ^6{^0JoF^6}^7\u{b}Toxiee\u{b}^6{^0C^6}.ak";
        assert_eq!(
            center_rows(text).collect::<Vec<_>>(),
            [
                "You have challenged",
                "^6{^0JoF^6}^7\u{b}Toxiee\u{b}^6{^0C^6}.ak"
            ]
        );
    }

    #[test]
    fn center_rows_count_characters_as_retail_counts_bytes() {
        // Fifty two-byte characters fit one retail row of fifty bytes.
        let row = "é".repeat(50);
        assert_eq!(center_rows(&row).collect::<Vec<_>>(), [row.as_str()]);
        let longer = "é".repeat(51);
        assert_eq!(
            center_rows(&longer).collect::<Vec<_>>(),
            ["é".repeat(50), "é".to_owned()]
        );
    }
}
