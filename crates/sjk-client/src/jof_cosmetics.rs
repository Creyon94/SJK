//! JoF EJK's free-choice cosmetics: the hat and cape a player wears, carried
//! after the saber colour in `color1` and `color2`.
//!
//! JoF EJK (`cg_local.h`, `CG_NewClientInfo`) writes the cosmetic's name
//! straight after the colour digits: `color1 "8santahat"` is saber colour 8
//! wearing the hat `santahat`, and `color2` holds the cape the same way.
//! Servers copy the keys through to the `c1`/`c2` clientinfo verbatim (cut
//! to 15 bytes by the 16-byte buffers of OpenJK and JA+ gamecode) and every
//! client reads the colour with `atoi`, which stops at the first letter, so
//! the name needs no server support and an unaware client sees only the
//! colour. A name is at most [`MAX_COSMETIC_NAME`] bytes (`MAX_COSMETIC_LENGTH`
//! 14 with its terminator) and must not start with a digit, which the
//! receiving `atoi` would swallow.

/// Longest cosmetic name, in bytes.
pub const MAX_COSMETIC_NAME: usize = 13;

/// The two places a cosmetic is worn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CosmeticSlot {
    /// A hat, on the `*head_top` bolt, carried by `color1`/`c1`.
    Hat,
    /// A cape, on the `*back` bolt, carried by `color2`/`c2`.
    Cape,
}

impl CosmeticSlot {
    /// Both slots, hat first.
    pub const ALL: [Self; 2] = [Self::Hat, Self::Cape];

    /// Position in [`Self::ALL`], which is also the saber the key colours.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The userinfo cvar carrying this slot.
    pub const fn cvar(self) -> &'static str {
        match self {
            Self::Hat => "color1",
            Self::Cape => "color2",
        }
    }

    /// The clientinfo key carrying this slot.
    pub const fn clientinfo_key(self) -> &'static str {
        match self {
            Self::Hat => "c1",
            Self::Cape => "c2",
        }
    }
}

/// Split a `color1`/`color2` (or `c1`/`c2`) value into the colour `atoi`
/// reads and the cosmetic name after the leading digits (`Q_StripDigits`
/// with `REMOVE_DIGITS_INITIAL`), or `None` when there is no valid name.
pub fn split_color_value(value: &str) -> (i64, Option<&str>) {
    let trimmed = value.trim_start();
    let (negative, unsigned) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, &trimmed[1..]),
        Some(b'+') => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };
    let digits = unsigned
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    let magnitude = unsigned[..digits].bytes().fold(0_i64, |total, byte| {
        total
            .saturating_mul(10)
            .saturating_add(i64::from(byte - b'0'))
    });
    let colour = if negative { -magnitude } else { magnitude };
    // `Q_StripDigits` drops only the digits before the first non-digit.
    let leading = value
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    let name = &value[leading..];
    (colour, valid_cosmetic_name(name).then_some(name))
}

/// Whether `name` can be worn: 1 to [`MAX_COSMETIC_NAME`] bytes of letters,
/// digits, `_` and `-`, not starting with a digit, and not `none`.
pub fn valid_cosmetic_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_COSMETIC_NAME
        && !name.as_bytes()[0].is_ascii_digit()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        && !name.eq_ignore_ascii_case("none")
}

/// What a player's clientinfo (`CS_PLAYERS + n`) says they wear in `slot`.
pub fn worn_cosmetic(clientinfo: &[u8], slot: CosmeticSlot) -> Option<&str> {
    let value = crate::LegacyClientInfo::new(clientinfo).text(slot.clientinfo_key())?;
    split_color_value(value).1
}

/// jaPRO's race-unlock hats (`JAPRO_COSMETIC_*`), by bit: the name JoF
/// EJK's `cosmetics unlocks` lists and the model in `models/players/hats/`.
/// A jaPRO-family server grants them in the player's `c5` clientinfo (their
/// `cp_cosmetics`, checked against the unlocks earned).
pub const JAPRO_HATS: [(&str, &str); 7] = [
    ("Santa hat", "santahat"),
    ("Jack-o'-lantern", "pumpkin"),
    ("Bass Pro Shops baseball cap", "cap"),
    ("Indiana Jones", "fedora"),
    ("Kane's Kringe Kap", "cringe"),
    ("Sombrero", "sombrero"),
    ("Top hat", "tophat"),
];

/// `cg_stylePlayer`'s `JAPRO_STYLE_SEASONALCOSMETICS`: seasonal hats, and
/// jaPRO's hats on JA+ and base servers too.
pub const STYLE_SEASONAL_COSMETICS: u32 = 1 << 21;

/// The `c5` cosmetic bits of a clientinfo (`atoi`).
pub fn japro_cosmetic_bits(clientinfo: &[u8]) -> u32 {
    let value = crate::LegacyClientInfo::new(clientinfo)
        .text("c5")
        .unwrap_or_default();
    u32::try_from(split_color_value(value).0).unwrap_or(0)
}

/// The hat `bits` draws: the lowest bit's, as `CG_Player`'s chain tests them.
pub fn japro_hat(bits: u32) -> Option<&'static str> {
    JAPRO_HATS
        .iter()
        .enumerate()
        .find(|(bit, _)| bits & (1 << bit) != 0)
        .map(|(_, (_, model))| *model)
}

/// JoF EJK's seasonal hat on `month` (1 to 12) and `day`, for a player
/// with no jaPRO cosmetic (`CG_NewClientInfo`): a Santa hat from 22 November
/// to 7 January, a pumpkin on 31 October.
pub fn seasonal_hat(month: u8, day: u8) -> Option<&'static str> {
    match (month, day) {
        (11, 22..) | (12, _) | (1, ..=7) => Some("santahat"),
        (10, 31) => Some("pumpkin"),
        _ => None,
    }
}

/// jaPRO's name for movement style `style` (`IntegerToRaceName`), as the
/// unlock requirements print it.
pub fn race_style_name(style: i16) -> &'static str {
    const STYLES: [&str; 15] = [
        "siege", "jka", "qw", "cpm", "q3", "pjk", "wsw", "rjq3", "rjcpm", "swoop", "jetpack",
        "speed", "sp", "slick", "botcpm",
    ];
    usize::try_from(style)
        .ok()
        .and_then(|style| STYLES.get(style))
        .copied()
        .unwrap_or("co-op")
}

/// `color1`/`color2` for saber colour `colour` wearing `cosmetic` (when it
/// is a valid name), as JoF EJK's `UI_SetCosmetic` writes it.
pub fn join_color_value(colour: i64, cosmetic: Option<&str>) -> String {
    match cosmetic.filter(|name| valid_cosmetic_name(name)) {
        Some(name) => format!("{colour}{name}"),
        None => colour.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_and_name_split_as_atoi_and_strip_digits_do() {
        assert_eq!(split_color_value("8santahat"), (8, Some("santahat")));
        assert_eq!(split_color_value("4"), (4, None));
        assert_eq!(split_color_value("6royalcape"), (6, Some("royalcape")));
        assert_eq!(split_color_value(""), (0, None));
        assert_eq!(split_color_value("fedora"), (0, Some("fedora")));
        // Digits after the first letter belong to the name.
        assert_eq!(split_color_value("3fedora2"), (3, Some("fedora2")));
        assert_eq!(split_color_value("4none"), (4, None));
        assert_eq!(split_color_value("4a long name!"), (4, None));
        assert_eq!(split_color_value("4thisnameistoolong"), (4, None));
    }

    #[test]
    fn names_join_back_to_the_same_value() {
        assert_eq!(join_color_value(4, Some("santahat")), "4santahat");
        assert_eq!(join_color_value(4, None), "4");
        assert_eq!(join_color_value(6, Some("bad name")), "6");
        for value in ["8santahat", "4", "6predatorhelm"] {
            let (colour, name) = split_color_value(value);
            assert_eq!(join_color_value(colour, name), value);
        }
        // Two colour digits and the longest name fit OpenJK's 16-byte buffer.
        assert!(join_color_value(11, Some("predatorhelm1")).len() <= 15);
    }

    #[test]
    fn clientinfo_names_the_worn_pieces() {
        let info = br"n\Sol\t\0\c1\4santahat\c2\0";
        assert_eq!(worn_cosmetic(info, CosmeticSlot::Hat), Some("santahat"));
        assert_eq!(worn_cosmetic(info, CosmeticSlot::Cape), None);
        assert_eq!(worn_cosmetic(br"n\Sol", CosmeticSlot::Hat), None);
    }

    #[test]
    fn japro_hats_follow_the_lowest_bit_and_the_season() {
        assert_eq!(japro_hat(0), None);
        assert_eq!(japro_hat(1), Some("santahat"));
        assert_eq!(japro_hat(0b110), Some("pumpkin"));
        assert_eq!(japro_hat(1 << 6), Some("tophat"));
        assert_eq!(japro_hat(1 << 7), None);
        assert_eq!(japro_cosmetic_bits(br"n\Sol\c5\4"), 4);
        assert_eq!(japro_cosmetic_bits(br"n\Sol"), 0);
        assert_eq!(seasonal_hat(11, 21), None);
        assert_eq!(seasonal_hat(11, 22), Some("santahat"));
        assert_eq!(seasonal_hat(12, 31), Some("santahat"));
        assert_eq!(seasonal_hat(1, 7), Some("santahat"));
        assert_eq!(seasonal_hat(1, 8), None);
        assert_eq!(seasonal_hat(10, 31), Some("pumpkin"));
        assert_eq!(seasonal_hat(10, 30), None);
        assert_eq!(race_style_name(1), "jka");
        assert_eq!(race_style_name(14), "botcpm");
        assert_eq!(race_style_name(15), "co-op");
    }

    #[test]
    fn names_follow_the_userinfo_rules() {
        assert!(valid_cosmetic_name("santahat"));
        assert!(valid_cosmetic_name("fedora_2"));
        assert!(!valid_cosmetic_name("2fedora"));
        assert!(!valid_cosmetic_name("hat\\x"));
        assert!(!valid_cosmetic_name("hat;"));
        assert!(!valid_cosmetic_name("None"));
        assert!(!valid_cosmetic_name(""));
    }
}
