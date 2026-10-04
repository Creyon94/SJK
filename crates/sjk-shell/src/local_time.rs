//! Local wall-clock time for console timestamps and clocks.
//!
//! Stock and EternalJK stamp console text with `Com_RealTime` and draw the
//! console's clocks with `localtime`: the player's own time zone, daylight
//! saving included. The time zone rules come from the operating system through
//! `chrono` (the Windows time zone API on Windows, the tz database on Unix), so
//! a clock follows a daylight saving change or a time zone change without a
//! restart. Formatting is kept apart from reading the clock so it can be
//! tested with fixed times.

use chrono::{Datelike, Local, Timelike};

/// Short English day names, as C's `asctime` prints them.
const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
/// Short English month names, as C's `asctime` prints them.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A local calendar time, to the second.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalTime {
    /// Calendar year.
    pub year: i32,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day of the month, 1 to 31.
    pub day: u8,
    /// Day of the week, 0 for Sunday to 6 for Saturday (`tm_wday`).
    pub weekday: u8,
    /// Hour, 0 to 23.
    pub hour: u8,
    /// Minute, 0 to 59.
    pub minute: u8,
    /// Second, 0 to 59 (a leap second reads as 59).
    pub second: u8,
}

impl LocalTime {
    /// The current local time.
    pub fn now() -> Self {
        let now = Local::now();
        Self {
            year: now.year(),
            month: now.month() as u8,
            day: now.day() as u8,
            weekday: now.weekday().num_days_from_sunday() as u8,
            hour: now.hour() as u8,
            minute: now.minute() as u8,
            second: now.second().min(59) as u8,
        }
    }

    /// `HH:MM:SS` on the 24-hour clock, as `Con_Linefeed` and `Con_DrawInput`
    /// format their stamps.
    pub fn clock(&self) -> [u8; 8] {
        let digits = |value: u8| [b'0' + value / 10 % 10, b'0' + value % 10];
        let [h0, h1] = digits(self.hour);
        let [m0, m1] = digits(self.minute);
        let [s0, s1] = digits(self.second);
        [h0, h1, b':', m0, m1, b':', s0, s1]
    }

    /// Append `HH:MM:SS` ([`Self::clock`]) to `out`.
    pub fn push_clock(&self, out: &mut String) {
        out.extend(self.clock().map(char::from));
    }

    /// Append the date and 12-hour time EternalJK draws in the console's corner
    /// (`Con_DrawSolidConsole`): the first 19 characters of `asctime` after the
    /// hour is moved onto the 12-hour clock, then `AM` or `PM` and a space, as
    /// in `Sun Oct  4 10:52:10 PM `.
    pub fn push_corner_clock(&self, out: &mut String) {
        use std::fmt::Write as _;
        let pm = self.hour >= 12;
        let hour = match self.hour % 12 {
            0 => 12,
            hour => hour,
        };
        let _ = write!(
            out,
            "{} {} {:2} {hour:02}:{:02}:{:02} {} ",
            DAYS[usize::from(self.weekday % 7)],
            MONTHS[usize::from(self.month.clamp(1, 12) - 1)],
            self.day,
            self.minute,
            self.second,
            if pm { "PM" } else { "AM" },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: u8, minute: u8, second: u8) -> LocalTime {
        LocalTime {
            year: 2026,
            month: 10,
            day: 4,
            weekday: 0,
            hour,
            minute,
            second,
        }
    }

    #[test]
    fn clock_is_zero_padded_24_hour_time() {
        assert_eq!(&at(22, 52, 10).clock(), b"22:52:10");
        assert_eq!(&at(0, 5, 9).clock(), b"00:05:09");
        let mut text = String::from("x");
        at(7, 0, 59).push_clock(&mut text);
        assert_eq!(text, "x07:00:59");
    }

    #[test]
    fn corner_clock_matches_ejk_asctime_layout() {
        let mut text = String::new();
        at(22, 52, 10).push_corner_clock(&mut text);
        assert_eq!(text, "Sun Oct  4 10:52:10 PM ");
        text.clear();
        // Midnight and noon on the 12-hour clock; two-digit days need no pad.
        let mut midnight = at(0, 1, 2);
        midnight.day = 25;
        midnight.weekday = 3;
        midnight.month = 12;
        midnight.push_corner_clock(&mut text);
        assert_eq!(text, "Wed Dec 25 12:01:02 AM ");
        text.clear();
        at(12, 0, 0).push_corner_clock(&mut text);
        assert_eq!(text, "Sun Oct  4 12:00:00 PM ");
    }

    #[test]
    fn now_is_a_valid_calendar_time() {
        let now = LocalTime::now();
        assert!((1..=12).contains(&now.month));
        assert!((1..=31).contains(&now.day));
        assert!(now.weekday < 7 && now.hour < 24 && now.minute < 60 && now.second < 60);
    }
}
