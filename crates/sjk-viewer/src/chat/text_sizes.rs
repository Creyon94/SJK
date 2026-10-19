//! One-time move of saved chat and centre-print sizes to EternalJK's scale.
//!
//! On 19/10/2026 `cg_chatBoxFontSize` 1 and `cg_centerSize` 1 became EternalJK's
//! absolute sizes ([`super::layout::FONT_1080`], [`super::view::CENTER_FONT_1080`]);
//! before, they drew a smaller SJK size. A profile saved before that keeps the size
//! it showed: a value other than the old default 1 is divided by how much larger
//! the new 1 is, and clamped to the new range. A saved 1 was the default, and
//! becomes EternalJK's size like a new profile's.

use sjk_shell::{CvarRegistry, CvarValue};

/// Archived marker: 0 until the saved sizes were moved to the new scale.
pub(crate) const VERSION_CVAR: &str = "cg_textSizeEjkVersion";

/// How much larger the new chat text is at the same value: the old line box was
/// 18 px at 1080 lines.
pub(crate) const CHAT_FACTOR: f32 = super::layout::FONT_1080 / 18.0;
/// How much larger the new centre-print text is at the same value: the old line
/// box was 22 px at 1080 lines.
pub(crate) const CENTER_FACTOR: f32 = super::view::CENTER_FONT_1080 / 22.0;

/// Move a pre-19/10/2026 profile's sizes to the new scale once, then set the
/// marker. `new_profile` (no config file yet) only sets the marker.
pub(crate) fn migrate(cvars: &mut CvarRegistry, new_profile: bool) {
    let version = cvars.get(VERSION_CVAR).map(|cvar| &cvar.value);
    if !matches!(version, Some(CvarValue::Integer(0))) {
        return;
    }
    if !new_profile {
        let (chat_min, chat_max) = super::options::CHAT_SIZE;
        for (name, factor, min, max) in [
            (
                "cg_chatBoxFontSize",
                CHAT_FACTOR,
                chat_min as f32,
                chat_max as f32,
            ),
            (
                "cg_centerSize",
                CENTER_FACTOR,
                0.0,
                super::options::CENTER_SIZE_MAX,
            ),
        ] {
            let saved = match cvars.get(name).map(|cvar| &cvar.value) {
                Some(CvarValue::Float(value)) => *value as f32,
                Some(CvarValue::Integer(value)) => *value as f32,
                _ => continue,
            };
            if saved == 1.0 || !saved.is_finite() {
                continue;
            }
            let moved = (saved / factor).clamp(min, max);
            let _ = cvars.set_text(name, &format!("{moved:.3}"));
        }
    }
    let _ = cvars.set_text(VERSION_CVAR, "1");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::ViewerConsole;

    #[test]
    fn the_new_sizes_are_eternaljks() {
        // ocr_a's 21-pixel line at 0.65, ergoec's 22 at 1, 2.25 1080p pixels each.
        assert!((super::super::layout::FONT_1080 - 30.7125).abs() < 1e-4);
        assert!((super::super::view::CENTER_FONT_1080 - 49.5).abs() < 1e-4);
        assert!((super::super::view::center_row(1.0) - 58.5).abs() < 1e-4);
        assert!((CHAT_FACTOR - 1.70625).abs() < 1e-5);
        assert!((CENTER_FACTOR - 2.25).abs() < 1e-5);
    }

    /// A saved size other than the old default keeps its look, once; the old
    /// default and a new profile get EternalJK's size.
    #[test]
    fn saved_text_sizes_move_to_the_eternaljk_scale_once() {
        let directory = tempfile::tempdir().unwrap();
        let fresh = ViewerConsole::new(directory.path().join("new.cfg")).unwrap();
        assert_eq!(fresh.float_cvar("cg_chatBoxFontSize"), Some(1.0));
        assert_eq!(fresh.integer_cvar(VERSION_CVAR), Some(1));
        drop(fresh);
        for (chat, center, chat_after, center_after) in [
            ("1", "1", 1.0, 1.0),
            ("1.25", "2", 0.733, 0.889),
            ("3", "4", 1.758, 1.778),
            ("0.25", "0.5", 0.15, 0.222),
        ] {
            let path = directory.path().join(format!("old-{chat}-{center}.cfg"));
            std::fs::write(
                &path,
                format!("seta cg_chatBoxFontSize \"{chat}\"\nseta cg_centerSize \"{center}\"\n"),
            )
            .unwrap();
            let mut console = ViewerConsole::new(path.clone()).unwrap();
            let read = |console: &ViewerConsole, name| console.float_cvar(name).unwrap();
            assert!((read(&console, "cg_chatBoxFontSize") - chat_after).abs() < 1e-9);
            assert!((read(&console, "cg_centerSize") - center_after).abs() < 1e-9);
            assert!(console.set_cvar("cg_chatBoxFontSize", "1.5"));
            drop(console);
            let console = ViewerConsole::new(path).unwrap();
            assert_eq!(console.float_cvar("cg_chatBoxFontSize"), Some(1.5));
            assert!((read(&console, "cg_centerSize") - center_after).abs() < 1e-9);
        }
    }
}
