//! Encoding for player-authored text the client sends to a server.
//!
//! JKA is a pre-Unicode engine: names, chat and console commands travel as byte
//! strings that every client draws through a 256-glyph codepage font. The retail
//! client is a Win32 ANSI program, so typed text reaches it as `WM_CHAR` bytes of the
//! Windows code page (Windows-1252 on Western systems) and leaves for the server as
//! those bytes. EternalJK also sends one byte per character
//! (`ConvertUTF32ToExpectedCharset`, `shared/sdl/sdl_input.cpp`). Sending a Rust
//! string's UTF-8 instead made `ø` arrive as `Ã¸` on those clients.
//!
//! Incoming text is decoded as Latin-1 (`sjk_client::decode_legacy`), so a byte in
//! 0x80..=0x9F reaches the client as the C1 character of the same value, and the
//! renderer draws the font glyph of that byte. Those characters go back as the same
//! byte, which keeps a name or line copied from the game byte-exact.

use std::borrow::Cow;

/// Characters Windows-1252 assigns to bytes 0x80..=0x9F. The five bytes it leaves
/// undefined hold the C1 character of the same value, as `MultiByteToWideChar` maps
/// them; only characters above U+00FF are looked up here.
const WINDOWS_1252_HIGH: [char; 32] = [
    '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}', '\u{8f}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{9d}', '\u{17e}', '\u{178}',
];

/// Encode text for the wire as legacy clients read it.
///
/// Text whose every character has a Windows-1252 byte is sent as those bytes: ASCII
/// and U+0080..=U+00FF as the byte of the same value, and the 27 typographic
/// characters Windows-1252 adds (`€`, `™`, `œ`, curly quotes, ...) as their bytes
/// in 0x80..=0x9F. Any other character (Cyrillic, CJK, emoji, `♥`) has no such
/// byte, and that text is sent as UTF-8 unchanged, which SJK clients still decode.
/// ASCII, the common case, is returned borrowed without allocating.
pub fn encode_legacy_text(text: &str) -> Cow<'_, [u8]> {
    if text.is_ascii() {
        return Cow::Borrowed(text.as_bytes());
    }
    let mut bytes = Vec::with_capacity(text.len());
    for character in text.chars() {
        match windows_1252_byte(character) {
            Some(byte) => bytes.push(byte),
            None => return Cow::Borrowed(text.as_bytes()),
        }
    }
    Cow::Owned(bytes)
}

/// The Windows-1252 byte of `character`, if it has one.
///
/// The renderer uses it too: JKA's fonts hold one glyph per byte, so the character
/// a player typed (`€`) and the byte another client sent (0x80) select one glyph.
pub fn windows_1252_byte(character: char) -> Option<u8> {
    u8::try_from(u32::from(character)).ok().or_else(|| {
        WINDOWS_1252_HIGH
            .iter()
            .position(|&mapped| mapped == character)
            .map(|index| 0x80 + index as u8)
    })
}

/// The character Windows-1252 assigns to `byte`: the byte's own value, except the
/// 27 typographic characters in 0x80..=0x9F. The inverse of [`windows_1252_byte`].
pub fn windows_1252_char(byte: u8) -> char {
    match byte {
        0x80..=0x9f => WINDOWS_1252_HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_borrowed_unchanged() {
        let encoded = encode_legacy_text("say \"^1hello\"");
        assert!(matches!(encoded, Cow::Borrowed(_)));
        assert_eq!(&*encoded, b"say \"^1hello\"");
    }

    #[test]
    fn latin1_letters_are_single_bytes() {
        assert_eq!(&*encode_legacy_text("ø"), [0xf8]);
        assert_eq!(&*encode_legacy_text("say \"j^6ø^7f §·¤\""), {
            let mut expected = b"say \"j^6".to_vec();
            expected.extend_from_slice(&[0xf8]);
            expected.extend_from_slice(b"^7f ");
            expected.extend_from_slice(&[0xa7, 0xb7, 0xa4]);
            expected.push(b'"');
            expected
        });
    }

    #[test]
    fn windows_1252_typography_uses_its_high_bytes() {
        assert_eq!(&*encode_legacy_text("€"), [0x80]);
        assert_eq!(&*encode_legacy_text("™"), [0x99]);
        assert_eq!(&*encode_legacy_text("“œ”"), [0x93, 0x9c, 0x94]);
        assert_eq!(&*encode_legacy_text("Ÿ"), [0x9f]);
    }

    #[test]
    fn c1_characters_return_to_their_own_byte() {
        // What `decode_legacy` makes of the bytes 0x80..=0x9F, undefined slots included.
        for byte in 0x80..=0x9f_u8 {
            let text = char::from(byte).to_string();
            assert_eq!(&*encode_legacy_text(&text), [byte]);
        }
    }

    #[test]
    fn text_outside_windows_1252_stays_utf8() {
        for text in ["привет", "ø ♥", "say \"日本\"", "😀"] {
            let encoded = encode_legacy_text(text);
            assert!(matches!(encoded, Cow::Borrowed(_)));
            assert_eq!(&*encoded, text.as_bytes());
        }
    }

    #[test]
    fn every_table_entry_encodes_to_its_byte() {
        for (index, &character) in WINDOWS_1252_HIGH.iter().enumerate() {
            let text = character.to_string();
            assert_eq!(&*encode_legacy_text(&text), [0x80 + index as u8]);
        }
    }
}

#[cfg(test)]
mod windows_1252_tests {
    use super::{windows_1252_byte, windows_1252_char};

    #[test]
    fn every_byte_round_trips_through_its_character() {
        for byte in 0..=u8::MAX {
            assert_eq!(windows_1252_byte(windows_1252_char(byte)), Some(byte));
        }
    }

    #[test]
    fn typographic_characters_have_their_bytes() {
        for (character, byte) in [
            ('€', 0x80),
            ('’', 0x92),
            ('‘', 0x91),
            ('™', 0x99),
            ('—', 0x97),
        ] {
            assert_eq!(windows_1252_byte(character), Some(byte));
            assert_eq!(windows_1252_char(byte), character);
        }
        assert_eq!(windows_1252_byte('♥'), None);
    }
}
