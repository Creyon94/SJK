//! The classic console (`con_style classic`, SJK's default), after EternalJK's
//! `cl_console.cpp` (`Con_DrawSolidConsole`, `Con_DrawInput`, `Con_DrawNotify`,
//! `CL_ConsolePrint`) and `cl_keys.cpp` (`Console_Key`, `Field_Draw`).
//!
//! Text sits on a grid of character cells. A cell is 8 by 16 pixels on a
//! 1080-line screen at `con_scale 1` and grows with the window height like the
//! rest of SJK's UI ([`Grid`]); EternalJK's cells stay 8 by 16 screen pixels
//! times `con_scale`, so `con_scale 0.5` at 2160 lines matches its size at 4K.
//! Column `c` of a row is drawn one cell in from the left (`(c + 1) * width`),
//! and a row holds `screen width / cell width - 2` columns. Rows start with the
//! time the line was written in grey (`HH:MM:SS `, nine columns) when
//! `con_timestamps` is set, and words wrap at the row's end ([`wrap`]).
//!
//! The view draws the console's background ([`crate::console_backdrop`]) down
//! to the open fraction, a bar in `console_color` under it, the version line and
//! the local date and time in the bottom-right corner, the scrollback from the
//! bottom up to the top of the screen (a row of `^` when scrolled back), and the
//! input row: a green clock, `]` and the raw input with a blinking cursor (an
//! underscore, or a block in overstrike mode). Closed, it draws the notify lines
//! at the top left while a game is running and no menu has focus. A
//! disconnected client without a menu shows the console full screen.

use super::ViewerConsole;
use super::line_edit::token_at;
use super::selection::{Gesture, Mark, PromptPointer, floor_boundary};
use crate::console_backdrop::{Background, ConsoleFrame, MAX_TEXT_VERTICES, SolidQuad, TextAtlas};
use crate::text::{UiFont, append_cell, glyph_byte_at, quake_color};
use sjk_shell::local_time::LocalTime;
use sjk_shell::{ConsoleLine, ConsoleLineKind};
use sjk_ui::{Rect, Vec2};

/// `console_color`: the bar under the console, the version line, the corner
/// clock and the scrollback arrows.
pub(crate) const BAR_COLOR: [f32; 4] = [0.509, 0.609, 0.847, 1.0];
/// `TIMESTAMP_LENGTH`: columns of a row's `HH:MM:SS ` stamp.
pub(super) const STAMP_COLUMNS: usize = 9;
/// Retail's virtual screen height, in which the background and bar are placed.
const VIRTUAL_HEIGHT: f32 = 480.0;
/// `SMALLCHAR_WIDTH` on a 1080-line screen at `con_scale 1`.
const CELL_WIDTH: f32 = 8.0;
/// Smallest UI scale the console uses (720 lines and below draw 6 by 12 cells).
const MIN_UI_SCALE: f32 = 0.75;
/// Grid column of the input row's `]`, after the clock and a space.
const PROMPT_COLUMN: usize = STAMP_COLUMNS;
/// Grid column the input text starts at.
const INPUT_COLUMN: usize = PROMPT_COLUMN + 1;
/// The text cursor is hidden for every other 256 ms (`cls.realtime >> 8 & 1`).
const BLINK_SHIFT: u32 = 8;
/// Character-set cells of the insert and overstrike cursors (`Field_Draw`).
const INSERT_CURSOR: u8 = 10;
const OVERSTRIKE_CURSOR: u8 = 11;
/// Selected text: the bar colour, translucent so the text stays readable.
const HIGHLIGHT: [f32; 4] = [0.509, 0.609, 0.847, 0.4];
/// Start colour of an error line, as the modern console draws it.
const ERROR_COLOR: [f32; 4] = [1.0, 0.55, 0.52, 1.0];
/// Scrollback rows per Page Up or Page Down, or per wheel step (`Con_PageUp`).
pub(super) const PAGE_ROWS: usize = 2;
/// Ctrl multiplies a page step (`Console_Key`).
pub(super) const FAST_PAGES: usize = 5;
/// Commands kept for Up and Down (`COMMAND_HISTORY`).
pub(super) const HISTORY: usize = 32;
/// Open fraction of Ctrl and Shift with the console key (`CL_KeyDownEvent`).
const FULL_HEIGHT: f32 = 1.0;
const QUARTER_HEIGHT: f32 = 0.25;

/// The character grid for a viewport and `con_scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Grid {
    /// Cell width in pixels, whole so characters land on pixels.
    pub(super) width: f32,
    /// Cell height: twice the width (`SMALLCHAR_HEIGHT`).
    pub(super) height: f32,
    /// Columns of a row (`con.linewidth`).
    pub(super) columns: usize,
}

impl Grid {
    pub(super) fn new(viewport: [f32; 2], scale: f32) -> Self {
        // The console's 0.75 floor on the UI scale, as the modern console.
        let ui = crate::ui_scale::height_scale(viewport[1]).max(MIN_UI_SCALE);
        let width = (CELL_WIDTH * scale * ui).round().max(1.0);
        Self {
            width,
            height: width * 2.0,
            columns: ((viewport[0] / width) as usize).saturating_sub(2).max(1),
        }
    }

    /// Left edge of `column`: one cell in from the screen's edge.
    pub(super) fn x(&self, column: usize) -> f32 {
        (column + 1) as f32 * self.width
    }

    /// Columns a row's text wraps at: all of them, or those after the stamp.
    pub(super) fn text_columns(&self, stamps: bool) -> usize {
        if stamps {
            self.columns.saturating_sub(STAMP_COLUMNS).max(1)
        } else {
            self.columns
        }
    }
}

/// Where the background ends and the bar sits, in pixels: `frac * 480 - 2`
/// virtual units, none below one unit, and a bar two units tall
/// (`Con_DrawSolidConsole`). Returns `(background height, bar height)`.
pub(super) fn background_extent(fraction: f32, viewport_height: f32) -> (f32, f32) {
    let unit = viewport_height / VIRTUAL_HEIGHT;
    let virtual_y = fraction * VIRTUAL_HEIGHT - 2.0;
    let y = if virtual_y < 1.0 { 0.0 } else { virtual_y };
    (y * unit, 2.0 * unit)
}

/// Texture `t` of the background's top and bottom (`con_ratioFix`): a console
/// of half the screen or less on a screen wider than 4:3 shows the middle of
/// the picture, from `1 - k` to `k` with `k` retail's `widthRatioCoef`
/// (`4/3` over the screen's aspect), instead of squashing all of it.
pub(super) fn ratio_fix_range(fraction: f32, viewport: [f32; 2], enabled: bool) -> [f32; 2] {
    let coefficient = (4.0 / 3.0) / (viewport[0] / viewport[1].max(1.0));
    if enabled && fraction <= 0.5 && coefficient < 1.0 {
        [1.0 - coefficient, coefficient]
    } else {
        [0.0, 1.0]
    }
}

/// Open fraction for a console key: Ctrl opens it full screen, Shift a quarter,
/// otherwise `base` (`con_height`, 0.5 like EternalJK); Shift+Escape always
/// opens `base`.
pub(super) fn open_height(escape: bool, control: bool, shift: bool, base: f32) -> f32 {
    match (escape, control, shift) {
        (true, ..) => base,
        (false, true, _) => FULL_HEIGHT,
        (false, false, true) => QUARTER_HEIGHT,
        _ => base,
    }
}

/// Scrollback rows of one Page Up/Down or wheel step, with or without Ctrl.
pub(super) fn page_rows(control: bool) -> usize {
    if control {
        PAGE_ROWS * FAST_PAGES
    } else {
        PAGE_ROWS
    }
}

/// One row of a line: bytes `start..end` of its text, and the colour code in
/// force where it starts (`None`: the line's own colour).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Span {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) colour: Option<u8>,
}

/// A colour code at byte `index` of `bytes`: its digit.
fn colour_code(bytes: &[u8], index: usize) -> Option<u8> {
    (bytes[index] == b'^')
        .then(|| bytes.get(index + 1).copied())
        .flatten()
        .filter(u8::is_ascii_digit)
        .map(|digit| digit - b'0')
}

/// Whether `character` takes no cell: Windows-1252's typographic characters
/// (bytes 0x80..=0x9E: `€`, `’`, `‘`, `…`, `™`, dashes), which neither the retail
/// console character set nor EternalJK's has, are left out like colour codes, so
/// the row closes up as EternalJK's console does where it drops them. The text
/// itself keeps them, so chat and names still show them.
pub(super) fn hidden(character: char) -> bool {
    matches!(
        sjk_protocol::windows_1252_byte(character),
        Some(0x80..=0x9e)
    )
}

/// Cells `text` takes: its characters other than [`hidden`] ones.
fn cell_count(text: &str) -> usize {
    text.chars().filter(|&character| !hidden(character)).count()
}

/// Byte offset of cell `index` of `text`, or its length past the last cell.
fn byte_of_cell(text: &str, index: usize) -> usize {
    text.char_indices()
        .filter(|&(_, character)| !hidden(character))
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

/// Visible characters of the word starting at the beginning of `text` (up to a
/// space or control character), colour codes not counted, at most `cap`.
fn word_length(text: &str, cap: usize) -> usize {
    let bytes = text.as_bytes();
    let (mut index, mut length) = (0, 0);
    while index < bytes.len() && length < cap {
        if colour_code(bytes, index).is_some() {
            index += 2;
            continue;
        }
        let character = text[index..].chars().next().unwrap_or(' ');
        if character <= ' ' {
            break;
        }
        if !hidden(character) {
            length += 1;
        }
        index += character.len_utf8();
    }
    length
}

/// Split `text` into rows of at most `width` characters, as `CL_ConsolePrint`
/// fills its buffer: a word that fits on a row but not in what is left of the
/// current one (or would end exactly at its edge) starts the next row, a word
/// longer than a row breaks at the edge, and `\n` ends a row. Colour codes take
/// no room; each row carries the colour in force where it starts. An empty text
/// is one empty row.
pub(super) fn wrap(text: &str, width: usize, mut row: impl FnMut(Span)) {
    let width = width.max(1);
    let bytes = text.as_bytes();
    let (mut start, mut column, mut index) = (0, 0, 0);
    let mut colour = None;
    let mut start_colour = None;
    let mut word_start = true;
    let mut emitted = false;
    let mut emit = |start: usize, end: usize, colour: Option<u8>| {
        row(Span { start, end, colour });
    };
    while index < bytes.len() {
        if let Some(code) = colour_code(bytes, index) {
            colour = Some(code);
            index += 2;
            continue;
        }
        let character = text[index..].chars().next().unwrap_or(' ');
        if hidden(character) {
            index += character.len_utf8();
            continue;
        }
        if character == '\n' {
            emit(start, index, start_colour);
            emitted = true;
            index += 1;
            (start, column, start_colour, word_start) = (index, 0, colour, true);
            continue;
        }
        if word_start && character > ' ' {
            let length = word_length(&text[index..], width);
            if length < width && column + length >= width {
                emit(start, index, start_colour);
                emitted = true;
                (start, column, start_colour) = (index, 0, colour);
            }
        }
        word_start = character <= ' ';
        index += character.len_utf8();
        column += 1;
        if column >= width {
            emit(start, index, start_colour);
            emitted = true;
            (start, column, start_colour) = (index, 0, colour);
        }
    }
    if start < bytes.len() || !emitted {
        emit(start, bytes.len(), start_colour);
    }
}

/// Visible characters of `text[..byte]`: colour codes take no column.
fn column_of(text: &str, byte: usize) -> usize {
    let bytes = text.as_bytes();
    let byte = floor_boundary(text, byte);
    let (mut index, mut column) = (0, 0);
    while index < byte {
        if colour_code(bytes, index).is_some() {
            index += 2;
            continue;
        }
        let character = text[index..].chars().next().unwrap_or(' ');
        index += character.len_utf8();
        if !hidden(character) {
            column += 1;
        }
    }
    column
}

/// Byte offset of visible column `column` of `text` (its end past the last),
/// with the colour codes before that character.
fn byte_of_column(text: &str, column: usize) -> usize {
    let bytes = text.as_bytes();
    let (mut index, mut seen, mut unit) = (0, 0, 0);
    while index < bytes.len() {
        if colour_code(bytes, index).is_some() {
            index += 2;
            continue;
        }
        let character = text[index..].chars().next().unwrap_or(' ');
        if hidden(character) {
            index += character.len_utf8();
            continue;
        }
        if seen == column {
            return unit;
        }
        index += text[index..].chars().next().map_or(1, char::len_utf8);
        seen += 1;
        unit = index;
    }
    bytes.len()
}

/// One drawn row of scrollback.
#[derive(Clone, Copy, Debug)]
struct Row {
    /// Line number ([`sjk_shell::Shell::lines_written`] counting).
    line: u64,
    span: Span,
}

/// Retained storage and clocks of the classic view.
#[derive(Default)]
pub(super) struct State {
    /// Every scrollback row, oldest first.
    rows: Vec<Row>,
    /// One past the newest line drawn last frame, so new rows can keep a
    /// scrolled-back view where it is.
    lines_end: u64,
    /// The time the corner clock and the input clock were formatted at.
    clock_time: Option<LocalTime>,
    corner: String,
    clock: [u8; 8],
}

impl State {
    /// Format the clocks again when the second changed.
    fn tick_clocks(&mut self) {
        let now = LocalTime::now();
        if self.clock_time != Some(now) {
            self.clock_time = Some(now);
            self.clock = now.clock();
            self.corner.clear();
            now.push_corner_clock(&mut self.corner);
        }
    }
}

/// What the frame knows about the rest of the client.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassicEnv {
    /// A game or demo is running (`CA_ACTIVE`).
    pub(crate) in_game: bool,
    /// A menu has keyboard focus (`KEYCATCH_UI | KEYCATCH_CGAME`).
    pub(crate) menu_focus: bool,
    /// Disconnected with the client's menu closed: the console fills the
    /// screen (`Con_DrawConsole`).
    pub(crate) full_screen: bool,
}

/// Cells drawn into a frame with one font.
struct Painter<'a> {
    font: &'a UiFont,
    grid: Grid,
    viewport: [f32; 2],
}

impl Painter<'_> {
    fn glyph(&self, frame: &mut ConsoleFrame, byte: u8, x: f32, y: f32, color: [f32; 4]) {
        if y + self.grid.height <= 0.0 || y >= self.viewport[1] {
            return;
        }
        append_cell(
            &mut frame.text,
            self.font,
            byte,
            [x, y, self.grid.width, self.grid.height],
            color,
            self.viewport,
            MAX_TEXT_VERTICES,
        );
    }

    /// Draw `text` from column `column` on, every character (colour codes
    /// included) in `color`.
    fn raw(&self, frame: &mut ConsoleFrame, text: &str, column: usize, y: f32, color: [f32; 4]) {
        let visible = text.chars().filter(|&character| !hidden(character));
        for (offset, character) in visible.enumerate() {
            let byte = console_byte(character);
            self.glyph(frame, byte, self.grid.x(column + offset), y, color);
        }
    }

    /// Draw `text` raw from `x` (pixels) on, in the bar colour.
    fn right(&self, frame: &mut ConsoleFrame, text: &str, x: f32, y: f32) {
        for (offset, character) in text.chars().enumerate() {
            let byte = console_byte(character);
            let left = x + offset as f32 * self.grid.width;
            self.glyph(frame, byte, left, y, BAR_COLOR);
        }
    }

    /// Draw `text` from column `column` on with colour codes applied, starting in
    /// `color`.
    fn coloured(
        &self,
        frame: &mut ConsoleFrame,
        text: &str,
        column: usize,
        y: f32,
        mut color: [f32; 4],
    ) {
        let bytes = text.as_bytes();
        let (mut index, mut offset) = (0, 0);
        while index < bytes.len() {
            if let Some(code) = colour_code(bytes, index) {
                color = quake_color(code);
                index += 2;
                continue;
            }
            let (byte, step) = glyph_byte_at(text, index);
            index += step;
            if text[index - step..].chars().next().is_some_and(hidden) {
                continue;
            }
            self.glyph(frame, byte, self.grid.x(column + offset), y, color);
            offset += 1;
        }
    }
}

/// Colour a line's rows start in.
fn line_colour(line: &ConsoleLine) -> [f32; 4] {
    if line.kind == ConsoleLineKind::Error {
        ERROR_COLOR
    } else {
        quake_color(7)
    }
}

impl ViewerConsole {
    /// Lay out the classic console for this frame into `frame`, with `font` (the
    /// console character set when `atlas` says so).
    pub(crate) fn append_classic(
        &mut self,
        frame: &mut ConsoleFrame,
        font: &UiFont,
        atlas: TextAtlas,
        viewport: [f32; 2],
        env: ClassicEnv,
    ) {
        frame.atlas = atlas;
        self.presentation.clear(viewport);
        if self.debug_panel.is_open()
            || self.changelog.is_open()
            || self.credits.is_open()
            || self.update_panel.is_open()
        {
            // The test list is drawn alone, as over the modern console.
            return;
        }
        let options = self.options();
        let full_screen = env.full_screen;
        let fraction = if full_screen {
            self.presentation.snap(1.0)
        } else {
            let target = if self.open {
                self.open_height.unwrap_or(options.height)
            } else {
                0.0
            };
            self.presentation.slide(target, options.speed)
        };
        let grid = Grid::new(viewport, options.scale);
        let painter = Painter {
            font,
            grid,
            viewport,
        };
        let stamps = options.timestamps != 0;
        let lines = (viewport[1] * fraction).floor().min(viewport[1]);
        if lines <= 0.0 {
            if env.in_game && !env.menu_focus {
                self.classic_notify(frame, &painter, options);
            }
            return;
        }
        self.classic.tick_clocks();

        // Background and bar.
        let (height, bar) = background_extent(fraction, viewport[1]);
        if height > 0.0 {
            frame.background = Some(Background {
                height,
                opacity: if fraction < 1.0 { options.opacity } else { 1.0 },
                t_range: ratio_fix_range(fraction, viewport, options.ratio_fix),
            });
        }
        frame.quads.push(SolidQuad {
            rect: [0.0, height, viewport[0], bar],
            color: BAR_COLOR,
        });

        // Version line and date in the corner, in the bar colour.
        let padding = (grid.height / 8.0).round();
        // The version ends a cell from the right edge, the date at the edge.
        let version = crate::menu::main_view::VERSION_LINE;
        let version_y = lines - (grid.height * 2.5).floor() + padding;
        let corner_y = lines - (grid.height * 1.5).floor() + padding;
        let length = version.chars().count() as f32 + 1.0;
        painter.right(frame, version, viewport[0] - length * grid.width, version_y);
        let corner = std::mem::take(&mut self.classic.corner);
        let length = corner.chars().count() as f32;
        painter.right(frame, &corner, viewport[0] - length * grid.width, corner_y);
        self.classic.corner = corner;

        // Scrollback, bottom row first.
        let text_columns = grid.text_columns(stamps);
        let added = self.classic_rows(text_columns);
        let total = self.classic.rows.len();
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_add(added);
        }
        self.scroll_offset = self.scroll_offset.min(total.saturating_sub(1));
        let input_y = lines - grid.height * 2.0;
        let mut y = lines - grid.height * 3.0;
        if self.scroll_offset > 0 {
            for column in (0..grid.columns).step_by(4) {
                painter.glyph(frame, b'^', grid.x(column), y, BAR_COLOR);
            }
            y -= grid.height;
        }
        let rows_bottom = y + grid.height;
        let text_column = if stamps { STAMP_COLUMNS } else { 0 };
        if self.open {
            self.selection.begin_frame(
                Rect::new(0.0, 0.0, viewport[0], rows_bottom.max(0.0)),
                Rect::new(0.0, input_y, viewport[0], grid.height),
            );
        }
        self.classic_rows_draw(frame, &painter, total, y, rows_bottom, text_column, stamps);

        // Input row, while the console has the keyboard or fills the screen.
        if self.open || full_screen {
            self.classic_input(frame, &painter, input_y);
        }
        if self.open {
            self.selection.end_frame();
            self.apply_prompt_pointer();
        }
    }

    /// Recompute every scrollback row at `width` columns; returns how many rows
    /// belong to lines written since the last frame.
    fn classic_rows(&mut self, width: usize) -> usize {
        let mut rows = std::mem::take(&mut self.classic.rows);
        rows.clear();
        let end = self.shell.lines_written();
        let first = end - self.shell.lines().count() as u64;
        let mut added = 0;
        for (line, entry) in (first..).zip(self.shell.lines()) {
            let before = rows.len();
            wrap(&entry.text, width, |span| rows.push(Row { line, span }));
            if line >= self.classic.lines_end {
                added += rows.len() - before;
            }
        }
        if self.classic.lines_end == 0 {
            added = 0;
        }
        self.classic.lines_end = end;
        self.classic.rows = rows;
        added
    }

    /// Draw scrollback rows upward from `y` (the bottom row's top), resolving a
    /// pointer gesture over them and highlighting the selection.
    #[allow(clippy::too_many_arguments)]
    fn classic_rows_draw(
        &mut self,
        frame: &mut ConsoleFrame,
        painter: &Painter<'_>,
        total: usize,
        mut y: f32,
        rows_bottom: f32,
        text_column: usize,
        stamps: bool,
    ) {
        let grid = painter.grid;
        // Pointer position over the rows: row counted up from the bottom one.
        let target = (self.open && self.selection.gesture() == Gesture::Output)
            .then(|| self.selection.pending_position())
            .flatten()
            .map(|position: Vec2| {
                let row = ((rows_bottom - position.y) / grid.height).floor();
                let column = (position.x / grid.width).floor() - 1.0 - text_column as f32;
                (row.max(-1.0) as isize, column.max(0.0) as usize)
            });
        let range = self.selection.range();
        let mut resolved = None;
        let mut bottom_end = None;
        let mut top_start = None;
        let Some(newest) = total.checked_sub(1 + self.scroll_offset) else {
            return;
        };
        // Rows above the screen are only walked to resolve a pointer over them.
        let needed = target.map_or(0, |(row, _)| row.max(0) as usize);
        for (index, slot) in (0..=newest).rev().enumerate() {
            if y + grid.height <= 0.0 && index > needed {
                break;
            }
            let Row { line, span } = self.classic.rows[slot];
            let Some(entry) = self.shell.line(line) else {
                continue;
            };
            let text = &entry.text[span.start..span.end];
            let begin = Mark {
                line,
                byte: span.start,
            };
            let end = Mark {
                line,
                byte: span.end,
            };
            if index == 0 {
                bottom_end = Some(end);
            }
            top_start = Some(begin);
            if let Some((from, to)) = range
                && from < end
                && to > begin
            {
                let left = column_of(text, from.max(begin).byte - span.start);
                let mut right = column_of(text, to.min(end).byte - span.start);
                if to > end {
                    right += 1;
                }
                if right > left {
                    frame.quads.push(SolidQuad {
                        rect: [
                            grid.x(text_column + left),
                            y,
                            (right - left) as f32 * grid.width,
                            grid.height,
                        ],
                        color: HIGHLIGHT,
                    });
                }
            }
            if y + grid.height > 0.0 {
                if stamps {
                    painter.raw(frame, entry.clock(), 0, y, quake_color(9));
                }
                let color = span.colour.map_or_else(|| line_colour(entry), quake_color);
                painter.coloured(frame, text, text_column, y, color);
            }
            if let Some((row, column)) = target
                && row == index as isize
            {
                let byte = byte_of_column(text, column);
                let token = token_at(text, byte);
                resolved = Some((
                    Mark {
                        line,
                        byte: span.start + byte,
                    },
                    (
                        Mark {
                            line,
                            byte: span.start + token.start,
                        },
                        Mark {
                            line,
                            byte: span.start + token.end,
                        },
                    ),
                ));
            }
            y -= grid.height;
        }
        let resolved = resolved.or_else(|| {
            let (row, _) = target?;
            let at = if row < 0 { bottom_end } else { top_start }?;
            Some((at, (at, at)))
        });
        if let Some((at, token)) = resolved {
            match self.selection.press() {
                Some(press) => self.selection.press_output(press, at, token),
                None => self.selection.drag_output(at),
            }
        }
    }

    /// The input row at `y`: green clock, `]`, the raw input and the cursor.
    fn classic_input(&mut self, frame: &mut ConsoleFrame, painter: &Painter<'_>, y: f32) {
        let grid = painter.grid;
        let clock = self.classic.clock;
        for (column, byte) in clock.iter().enumerate() {
            painter.glyph(frame, *byte, grid.x(column), y, quake_color(2));
        }
        let white = quake_color(7);
        painter.glyph(frame, b']', grid.x(PROMPT_COLUMN), y, white);
        let cells = (painter.viewport[0] / grid.width) as usize;
        // Room for the input and the cursor after it, within the screen.
        let room = cells.saturating_sub(INPUT_COLUMN + 2).max(1);
        let cursor = self.edit.cursor(&self.input);
        let caret = cell_count(&self.input[..cursor]);
        let prestep = caret.saturating_sub(room - 1);
        let byte_at = byte_of_cell;
        if self.open
            && self.selection.gesture() == Gesture::Prompt
            && let Some(position) = self.selection.pending_position()
        {
            let column = (position.x / grid.width).floor() - 1.0 - INPUT_COLUMN as f32;
            let index = prestep + column.max(0.0) as usize;
            let byte = byte_at(&self.input, index);
            self.selection.set_prompt(match self.selection.press() {
                Some(press) if press.double => PromptPointer::Token(byte),
                Some(press) => PromptPointer::Place {
                    byte,
                    extend: press.extend,
                },
                None => PromptPointer::Place { byte, extend: true },
            });
        }
        let start = byte_at(&self.input, prestep);
        let shown_end = byte_at(&self.input, prestep + room);
        if let Some(range) = self.edit.selection(&self.input) {
            let left = cell_count(&self.input[..range.start]).saturating_sub(prestep);
            let right = cell_count(&self.input[..range.end])
                .saturating_sub(prestep)
                .min(room);
            if right > left {
                frame.quads.push(SolidQuad {
                    rect: [
                        grid.x(INPUT_COLUMN + left),
                        y,
                        (right - left) as f32 * grid.width,
                        grid.height,
                    ],
                    color: HIGHLIGHT,
                });
            }
        }
        painter.raw(frame, &self.input[start..shown_end], INPUT_COLUMN, y, white);
        let blink = (self.shell.command_clock_millis() >> BLINK_SHIFT) & 1 == 1;
        if !blink {
            let x = grid.x(INPUT_COLUMN + caret - prestep);
            if painter.font.is_modern() {
                if self.overstrike {
                    frame.quads.push(SolidQuad {
                        rect: [x, y, grid.width, grid.height],
                        color: [1.0, 1.0, 1.0, 0.6],
                    });
                } else {
                    painter.glyph(frame, b'_', x, y, white);
                }
            } else {
                let cursor = if self.overstrike {
                    OVERSTRIKE_CURSOR
                } else {
                    INSERT_CURSOR
                };
                painter.glyph(frame, cursor, x, y, white);
            }
        }
    }

    /// The notify lines (`Con_DrawNotify`): of the last `con_notifylines` rows,
    /// those written in the last `con_notifytime` seconds and not quiet, from
    /// the top of the screen, one cell plus `cl_conXOffset` pixels from the left.
    fn classic_notify(
        &mut self,
        frame: &mut ConsoleFrame,
        painter: &Painter<'_>,
        options: super::console_options::Options,
    ) {
        let grid = painter.grid;
        let wanted = options.notify_lines;
        if wanted == 0 {
            return;
        }
        let width = grid.text_columns(options.timestamps != 0);
        let mut rows = std::mem::take(&mut self.classic.rows);
        rows.clear();
        let end = self.shell.lines_written();
        // Newest lines first, each line's rows in reverse, until enough rows.
        for (line, entry) in (0..end).rev().zip(self.shell.lines().rev()) {
            let before = rows.len();
            wrap(&entry.text, width, |span| rows.push(Row { line, span }));
            rows[before..].reverse();
            if rows.len() >= wanted {
                break;
            }
        }
        rows.truncate(wanted);
        let now = self.shell.command_clock_millis();
        let left = options.notify_x.trunc() + grid.width;
        let mut y = 0.0;
        for Row { line, span } in rows.iter().rev().copied() {
            let Some(entry) = self.shell.line(line) else {
                continue;
            };
            if !entry.notify || now.saturating_sub(entry.written_millis) >= options.notify_millis {
                continue;
            }
            let mut x = left;
            if options.timestamps == 1 {
                for byte in entry.clock().bytes() {
                    painter.glyph(frame, byte, x, y, quake_color(9));
                    x += grid.width;
                }
                x = left + STAMP_COLUMNS as f32 * grid.width;
            }
            let text = &entry.text[span.start..span.end];
            let mut color = span.colour.map_or_else(|| line_colour(entry), quake_color);
            let bytes = text.as_bytes();
            let mut index = 0;
            while index < bytes.len() {
                if let Some(code) = colour_code(bytes, index) {
                    color = quake_color(code);
                    index += 2;
                    continue;
                }
                let (byte, step) = glyph_byte_at(text, index);
                index += step;
                if text[index - step..].chars().next().is_some_and(hidden) {
                    continue;
                }
                painter.glyph(frame, byte, x, y, color);
                x += grid.width;
            }
            y += grid.height;
        }
        self.classic.rows = rows;
        // The next opening counts every row as already seen.
        self.classic.lines_end = end;
    }
}

/// The character-set cell that draws `character`: its Windows-1252 byte, as
/// [`crate::text::glyph_byte_at`] picks it, so a typed `’` uses cell 0x92 like
/// the same byte from another player; `?` for a character with no such byte.
fn console_byte(character: char) -> u8 {
    sjk_protocol::windows_1252_byte(character).unwrap_or(b'?')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typographic_symbols_take_no_cell_but_stay_in_the_text() {
        // `€` and `’` (bytes 0x80, 0x92) close up; `¬` and `×` keep their cells.
        let text = "A€B’C¬D×";
        assert_eq!(cell_count(text), 6);
        assert_eq!(column_of(text, text.len()), 6);
        assert_eq!(&text[byte_of_cell(text, 1)..], "B’C¬D×");
        // Like a colour code, a hidden character before the cell belongs to it.
        assert_eq!(&text[byte_of_column(text, 2)..], "’C¬D×");
        assert_eq!(word_length(text, 20), 6);
        assert!(hidden('\u{80}') && hidden('…') && !hidden('Ÿ') && !hidden('¬'));
    }

    #[test]
    fn typed_symbols_use_their_windows_1252_cells() {
        let cells: Vec<u8> = "a×’‘€…♥".chars().map(console_byte).collect();
        assert_eq!(cells, [b'a', 0xd7, 0x92, 0x91, 0x80, 0x85, b'?']);
    }

    fn rows(text: &str, width: usize) -> Vec<String> {
        let mut out = Vec::new();
        wrap(text, width, |span| {
            out.push(text[span.start..span.end].to_owned())
        });
        out
    }

    #[test]
    fn cells_follow_con_scale_and_the_1080_line_scale() {
        let full_hd = Grid::new([1920.0, 1080.0], 1.0);
        assert_eq!(
            (full_hd.width, full_hd.height, full_hd.columns),
            (8.0, 16.0, 238)
        );
        // con_scale 0.5 at 2160 lines is EternalJK's 8 x 16 at 4K.
        let uhd = Grid::new([3840.0, 2160.0], 0.5);
        assert_eq!((uhd.width, uhd.height, uhd.columns), (8.0, 16.0, 478));
        let uhd_full = Grid::new([3840.0, 2160.0], 1.0);
        assert_eq!((uhd_full.width, uhd_full.height), (16.0, 32.0));
        // Cells stay whole pixels: 1440 lines is 10.67 rounded.
        assert_eq!(Grid::new([2560.0, 1440.0], 1.0).width, 11.0);
        // Small windows keep the console's 0.75 floor.
        assert_eq!(Grid::new([1280.0, 720.0], 1.0).width, 6.0);
        // Column c is drawn one cell in.
        assert_eq!(full_hd.x(0), 8.0);
        assert_eq!(full_hd.x(9), 80.0);
        assert_eq!(full_hd.text_columns(true), 229);
        assert_eq!(full_hd.text_columns(false), 238);
    }

    #[test]
    fn background_and_bar_follow_the_virtual_screen() {
        // Half open at 1080 lines: 238 virtual units, 2.25 px each.
        let (height, bar) = background_extent(0.5, 1080.0);
        assert!((height - 535.5).abs() < 1e-3 && (bar - 4.5).abs() < 1e-3);
        // Under one unit there is no background, only the bar at the top.
        assert_eq!(background_extent(0.004, 1080.0).0, 0.0);
        // Full screen at 2160 lines: the bar's two units are 9 px.
        let (height, bar) = background_extent(1.0, 2160.0);
        assert_eq!((height, bar), (2151.0, 9.0));
    }

    #[test]
    fn ratio_fix_shows_the_middle_of_the_picture_up_to_half_height() {
        let wide = [1920.0, 1080.0];
        assert_eq!(ratio_fix_range(0.5, wide, true), [0.25, 0.75]);
        assert_eq!(ratio_fix_range(0.25, wide, true), [0.25, 0.75]);
        assert_eq!(ratio_fix_range(1.0, wide, true), [0.0, 1.0]);
        assert_eq!(ratio_fix_range(0.5, wide, false), [0.0, 1.0]);
        // A 4:3 screen has nothing to fix.
        assert_eq!(ratio_fix_range(0.5, [1024.0, 768.0], true), [0.0, 1.0]);
    }

    #[test]
    fn console_key_heights() {
        assert_eq!(open_height(false, false, false, 0.5), 0.5);
        assert_eq!(open_height(false, true, false, 0.5), 1.0);
        assert_eq!(open_height(false, false, true, 0.5), 0.25);
        // Ctrl wins over Shift; Shift+Escape is the plain height.
        assert_eq!(open_height(false, true, true, 0.5), 1.0);
        assert_eq!(open_height(true, false, true, 0.5), 0.5);
        assert_eq!(open_height(false, false, false, 0.7), 0.7);
    }

    #[test]
    fn page_steps_are_two_rows_or_ten_with_ctrl() {
        assert_eq!(page_rows(false), 2);
        assert_eq!(page_rows(true), 10);
    }

    #[test]
    fn words_wrap_before_they_would_reach_the_edge() {
        assert_eq!(rows("hello world", 20), ["hello world"]);
        // "world" would end at column 10 of 10: it starts the next row, and the
        // space stays at the end of the first.
        assert_eq!(rows("hello world", 10), ["hello ", "world"]);
        assert_eq!(rows("abc de fgh", 8), ["abc de ", "fgh"]);
        // A word longer than a row breaks at the edge.
        assert_eq!(rows("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(rows("", 4), [""]);
        assert_eq!(rows("one\ntwo", 10), ["one", "two"]);
    }

    #[test]
    fn colour_codes_take_no_room_and_carry_into_the_next_row() {
        let text = "^1red ^2green";
        let mut spans = Vec::new();
        wrap(text, 6, |span| spans.push(span));
        assert_eq!(spans.len(), 2);
        // The code before the wrapped word is read before the word wraps, so it
        // ends the first row and the second starts in its colour.
        assert_eq!(&text[spans[0].start..spans[0].end], "^1red ^2");
        assert_eq!(spans[0].colour, None);
        assert_eq!(&text[spans[1].start..spans[1].end], "green");
        assert_eq!(spans[1].colour, Some(2));
        let mut spans = Vec::new();
        wrap("^3abcdefgh", 4, |span| spans.push(span));
        assert_eq!(spans[1].colour, Some(3));
        // A coloured word is counted by its visible characters.
        assert_eq!(rows("ab ^3cd", 6), ["ab ^3cd"]);
        // A word ending exactly at the edge wraps, as `con.x + l >= linewidth`.
        assert_eq!(rows("ab ^3cd", 5), ["ab ^3", "cd"]);
    }

    #[test]
    fn stamped_rows_wrap_at_the_columns_after_the_stamp() {
        let grid = Grid::new([160.0, 1080.0], 1.0);
        // 160 / 8 - 2 = 18 columns, 9 after the stamp.
        assert_eq!(grid.columns, 18);
        let width = grid.text_columns(true);
        assert_eq!(
            rows("connected to server", width),
            ["connected", " to ", "server"]
        );
    }

    #[test]
    fn columns_and_bytes_skip_colour_codes() {
        let text = "^1ab^2cd";
        assert_eq!(column_of(text, 0), 0);
        assert_eq!(column_of(text, 3), 1);
        assert_eq!(column_of(text, text.len()), 4);
        assert_eq!(byte_of_column(text, 0), 0);
        // The code before `c` goes with it.
        assert_eq!(byte_of_column(text, 2), 4);
        assert_eq!(byte_of_column(text, 9), text.len());
    }

    #[test]
    fn overstrike_replaces_characters_after_the_caret() {
        let mut edit = super::super::line_edit::LineEdit::default();
        let mut text = String::from("abcd");
        edit.place(&text, 1, false);
        edit.overwrite(&mut text, "XY", 512);
        assert_eq!(text, "aXYd");
        assert_eq!(edit.cursor(&text), 3);
        // Past the end the line grows.
        edit.overwrite(&mut text, "ZW", 512);
        assert_eq!(text, "aXYZW");
        // A selection is replaced as a whole, as in insert mode.
        edit.select(&text, 0..2);
        edit.overwrite(&mut text, "q", 512);
        assert_eq!(text, "qYZW");
        // The limit holds.
        edit.to_end(&text);
        edit.overwrite(&mut text, "123", 5);
        assert_eq!(text, "qYZW1");
    }
}
