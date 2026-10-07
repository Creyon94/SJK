//! OpenJK cl_console.cpp:492-502,610-612,737-773,1038-1067.
//! notifylines/datetime extend it with TaystJK cl_console.cpp:650-668,1090.
use super::*;

/// Archived cvar naming the console style.
pub(crate) const STYLE_CVAR: &str = "con_style";
/// Internal marker of the one-time move of a saved `classic` to `auto`.
pub(crate) const STYLE_VERSION_CVAR: &str = "con_styleDefaultVersion";

/// How the console looks and behaves (`con_style`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ConsoleStyle {
    /// JKR's console: Inter text over a tinted panel with a header and hints.
    Modern,
    /// After EternalJK (`cl_console.cpp`): the `console` shader's background,
    /// a monospaced character grid, timestamps, a clock and the version line.
    /// What `auto` gives with the classic or modern menus.
    #[default]
    Classic,
    /// The SJK UI's console (`sjk`, the deck): the classic console's grid,
    /// keys and behaviour in a full-width navy panel with a header, a framed
    /// input band and a lit rail along its bottom edge, in the SJK UI's colours
    /// and type ([`super::sjk`]). What `auto` gives with the SJK UI's menus.
    Sjk,
}

impl ConsoleStyle {
    /// Values the settings screen offers: `auto` first, then each look.
    pub(crate) const NAMES: [&'static str; 4] = ["auto", "sjk", "classic", "modern"];
    /// The `con_style` value of a new profile: follow the menu style.
    pub(crate) const DEFAULT_NAME: &'static str = Self::NAMES[0];

    /// The look the cvar `value` gives; `sjk_menus` is whether the menus are
    /// the SJK UI (`ui_menuStyle sjk`). `modern` (any case) or `0` is the
    /// modern console, `classic` the classic one, `sjk` the SJK UI's; `auto`,
    /// no value or any other one follow the menus: the SJK UI's console with
    /// its menus, the classic console otherwise. `horizon` and `dock`, two
    /// retired SJK designs, are among the others.
    pub(crate) fn resolve(value: Option<&str>, sjk_menus: bool) -> Self {
        let text = value.map(str::trim).unwrap_or_default();
        let is = |name: &str| text.eq_ignore_ascii_case(name);
        if is("modern") || text == "0" {
            Self::Modern
        } else if is("classic") {
            Self::Classic
        } else if is("sjk") || sjk_menus {
            Self::Sjk
        } else {
            Self::Classic
        }
    }

    /// Whether the console is drawn on its own layer as a character grid with
    /// EternalJK's keys: the classic console and the SJK UI's.
    pub(crate) const fn is_grid(self) -> bool {
        matches!(self, Self::Classic | Self::Sjk)
    }

    /// Whether this is the SJK UI's console.
    pub(crate) const fn is_sjk(self) -> bool {
        matches!(self, Self::Sjk)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Options {
    pub style: ConsoleStyle,
    /// `con_ratioFix`: a half-height or lower classic console shows the middle
    /// of its background picture instead of squashing all of it.
    pub ratio_fix: bool,
    pub height: f32,
    pub scale: f32,
    pub opacity: f32,
    pub speed: f32,
    pub timestamps: i64,
    pub datetime: bool,
    pub notify_millis: u64,
    pub notify_lines: usize,
    /// Notify-only horizontal displacement, in virtual 640-wide coordinates.
    pub notify_x: f32,
    /// History and notify row pitch as a multiple of their text size
    /// (`con_lineSpacing`).
    pub line_spacing: f32,
    /// Extra advance after every glyph as a fraction of the text size
    /// (`ui_letterSpacing`), applied where the console lays out its text.
    pub tracking: f32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            style: ConsoleStyle::Classic,
            ratio_fix: true,
            height: 0.5,
            scale: 1.0,
            opacity: 1.0,
            speed: 3.0,
            timestamps: 0,
            datetime: false,
            notify_millis: 3000,
            notify_lines: 3,
            notify_x: 0.0,
            line_spacing: LINE_SPACING_DEFAULT,
            tracking: 0.0,
        }
    }
}

pub(super) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    for (name, value, help) in [
        ("con_notifytime", 3.0, "Notify lifetime in seconds"),
        ("con_opacity", 1.0, "Console background opacity"),
        ("con_scale", 1.0, "Console font scale"),
        (
            "con_lineSpacing",
            f64::from(LINE_SPACING_DEFAULT),
            "Console row pitch as a multiple of the text size (0.8 to 2)",
        ),
        (
            "con_height",
            0.5,
            "Open console height as a screen fraction",
        ),
        ("scr_conspeed", 3.0, "Console opening/closing speed"),
    ] {
        let flags = if matches!(name, "con_notifytime" | "scr_conspeed") {
            CvarFlags::NONE
        } else {
            CvarFlags::ARCHIVE
        };
        cvars.register(CvarDefinition::new(name, value, flags, help))?;
    }
    cvars.register(CvarDefinition::new(
        STYLE_CVAR,
        ConsoleStyle::DEFAULT_NAME,
        CvarFlags::ARCHIVE,
        "Console style: auto (the SJK UI's with its menus, else classic), sjk, classic \
         (after EternalJK) or modern",
    ))?;
    cvars.register(CvarDefinition::new(
        STYLE_VERSION_CVAR,
        0_i64,
        CvarFlags::ARCHIVE,
        "Internal migration marker for the auto console style default",
    ))?;
    for (name, value, help) in [
        ("con_notifylines", 3_i64, "Maximum visible notify lines"),
        (
            "con_timestamps",
            0,
            "Timestamps: 0 off, 1 console and notify, 2 console only (EternalJK style)",
        ),
        (
            "con_ratioFix",
            1,
            "Classic console: a console of half the screen or less shows the middle of              its background instead of squashing it; disable for custom backgrounds",
        ),
    ] {
        cvars.register(CvarDefinition::new(name, value, CvarFlags::ARCHIVE, help))?;
    }
    for (name, value, help) in [
        ("con_autoclear", true, "Clear console input when closing"),
        ("cl_noprint", false, "Suppress console output"),
        (
            "con_datetime",
            false,
            "Show current UTC date/time in the console header",
        ),
    ] {
        let flags = if name == "cl_noprint" {
            CvarFlags::NONE
        } else {
            CvarFlags::ARCHIVE
        };
        cvars.register(CvarDefinition::new(name, value, flags, help))?;
    }
    Ok(())
}

impl ViewerConsole {
    /// The player's `con_style`.
    pub(crate) fn console_style(&self) -> ConsoleStyle {
        let menus =
            crate::menu::style::MenuStyle::from_cvar(self.text_value(crate::menu::style::CVAR));
        ConsoleStyle::resolve(
            self.text_value(STYLE_CVAR),
            menus == crate::menu::style::MenuStyle::Sjk,
        )
    }

    pub(super) fn options(&self) -> Options {
        Options {
            style: self.console_style(),
            ratio_fix: self.integer_cvar("con_ratioFix").unwrap_or(1) != 0,
            notify_x: self.float_cvar("cl_conxoffset").unwrap_or(0.0) as f32,
            height: self.float_cvar("con_height").unwrap_or(0.5).clamp(0.0, 1.0) as f32,
            scale: self
                .float_cvar("con_scale")
                .filter(|value| *value > 0.0)
                .unwrap_or(1.0) as f32,
            opacity: self
                .float_cvar("con_opacity")
                .unwrap_or(1.0)
                .clamp(0.0, 1.0) as f32,
            speed: self
                .float_cvar("scr_conspeed")
                .unwrap_or(3.0)
                .clamp(1.0, 100.0) as f32,
            timestamps: self.integer_cvar("con_timestamps").unwrap_or(0),
            datetime: self.bool_cvar("con_datetime").unwrap_or(false),
            notify_millis: (self.float_cvar("con_notifytime").unwrap_or(3.0).max(0.0) * 1000.0)
                as u64,
            notify_lines: self
                .integer_cvar("con_notifylines")
                .unwrap_or(3)
                .clamp(0, 64) as usize,
            line_spacing: line_spacing(self.float_cvar("con_lineSpacing")),
            tracking: crate::text::TextStyle::from_cvars(
                None,
                self.float_cvar(crate::text::style::TRACKING_CVAR),
            )
            .tracking,
        }
    }
}

/// Default `con_lineSpacing`: 14 px rows 12.6 px apart at 1080p. Inter's line box
/// (ascent + descent) is 1.21 em; at 0.9 of it the deepest descender (`g`, 0.18 of
/// the box) still clears the next row's ascenders and brackets (0.64 above its
/// baseline) by 0.08 of the box, about 1 px at 1080p, and capitals fill two thirds
/// of the pitch, near stock's dense 16 px console rows.
pub(super) const LINE_SPACING_DEFAULT: f32 = 0.9;

/// Accepted `con_lineSpacing` range. At the lower end a descender meets the next
/// row's ascenders; only accented capitals can still overlap the row above.
pub(super) const LINE_SPACING_RANGE: (f32, f32) = (0.8, 2.0);

/// Height of the rectangle a console row's text is drawn and clipped in. Rows
/// closer than their line box overlap, so the text keeps its whole box and its
/// 1 px glyph shadow instead of losing the descenders to the next row's pitch.
pub(super) fn row_box(size: f32, pitch: f32) -> f32 {
    pitch.max(size + 1.0)
}

/// Clamped `con_lineSpacing`; missing or non-finite values keep the default.
pub(super) fn line_spacing(value: Option<f64>) -> f32 {
    value
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
        .map_or(LINE_SPACING_DEFAULT, |value| {
            value.clamp(LINE_SPACING_RANGE.0, LINE_SPACING_RANGE.1)
        })
}

/// Gregorian UTC date from Unix days (no OS-specific code in the console).
pub(super) fn datetime(seconds: u64) -> String {
    let days = (seconds / 86400) as i64 + 719468;
    let era = days / 146097;
    let day = days - era * 146097;
    let year = (day - day / 1460 + day / 36524 - day / 146096) / 365;
    let ordinal = day - (365 * year + year / 4 - year / 100);
    let month = (5 * ordinal + 2) / 153;
    let d = ordinal - (153 * month + 2) / 5 + 1;
    let m = month + if month < 10 { 3 } else { -9 };
    let y = year + era * 400 + i64::from(m <= 2);
    format!(
        "CONSOLE  {y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_spacing_defaults_and_clamps() {
        assert_eq!(line_spacing(None), LINE_SPACING_DEFAULT);
        assert_eq!(line_spacing(Some(f64::NAN)), LINE_SPACING_DEFAULT);
        assert_eq!(line_spacing(Some(1.25)), 1.25);
        // A value saved before the pitch became a multiple of the text size.
        assert_eq!(line_spacing(Some(0.65)), LINE_SPACING_RANGE.0);
        assert_eq!(line_spacing(Some(0.1)), LINE_SPACING_RANGE.0);
        assert_eq!(line_spacing(Some(9.0)), LINE_SPACING_RANGE.1);
    }

    #[test]
    fn auto_and_mistyped_values_follow_the_menus() {
        // `horizon` and `dock`: retired designs a profile may have saved.
        for value in [
            None,
            Some("auto"),
            Some(" AUTO "),
            Some("modren"),
            Some("horizon"),
            Some("Dock"),
        ] {
            assert_eq!(ConsoleStyle::resolve(value, false), ConsoleStyle::Classic);
            assert_eq!(ConsoleStyle::resolve(value, true), ConsoleStyle::Sjk);
        }
        assert_eq!(
            ConsoleStyle::resolve(Some(ConsoleStyle::DEFAULT_NAME), false),
            ConsoleStyle::default()
        );
    }

    #[test]
    fn a_named_look_wins_over_the_menus() {
        for menus in [false, true] {
            let resolve = |value| ConsoleStyle::resolve(Some(value), menus);
            assert_eq!(resolve("classic"), ConsoleStyle::Classic);
            assert_eq!(resolve(" Modern "), ConsoleStyle::Modern);
            assert_eq!(resolve("0"), ConsoleStyle::Modern);
            assert_eq!(resolve(" SJK "), ConsoleStyle::Sjk);
        }
    }

    #[test]
    fn offered_names_parse_to_distinct_looks() {
        let looks = ConsoleStyle::NAMES[1..]
            .iter()
            .map(|name| ConsoleStyle::resolve(Some(name), false))
            .collect::<Vec<_>>();
        for (index, look) in looks.iter().enumerate() {
            assert!(!looks[..index].contains(look), "{look:?} twice");
            assert_eq!(look.is_grid(), *look != ConsoleStyle::Modern);
        }
    }

    /// The registered default, a saved `classic` from before `auto` existed and
    /// a classic chosen afterwards.
    #[test]
    fn a_saved_classic_moves_once_to_auto() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.cfg");
        let mut console = ViewerConsole::new(path.clone()).unwrap();
        assert_eq!(console.text_value(STYLE_CVAR), Some("auto"));
        assert_eq!(console.console_style(), ConsoleStyle::Sjk);
        console.set_cvar(crate::menu::style::CVAR, "classic");
        assert_eq!(console.console_style(), ConsoleStyle::Classic);
        drop(console);
        std::fs::write(
            &path,
            "seta ui_menuStyle \"sjk\"\nseta con_style \"classic\"\n",
        )
        .unwrap();
        let mut console = ViewerConsole::new(path.clone()).unwrap();
        assert_eq!(console.text_value(STYLE_CVAR), Some("auto"));
        assert_eq!(console.console_style(), ConsoleStyle::Sjk);
        assert!(console.set_cvar(STYLE_CVAR, "classic"));
        drop(console);
        let console = ViewerConsole::new(path).unwrap();
        assert_eq!(console.text_value(STYLE_CVAR), Some("classic"));
        assert_eq!(console.console_style(), ConsoleStyle::Classic);
    }

    #[test]
    fn tight_rows_keep_their_whole_text_box() {
        assert_eq!(row_box(14.0, 12.6), 15.0);
        assert_eq!(row_box(14.0, 28.0), 28.0);
    }
}
