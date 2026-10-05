//! The copyright and licence announcement shown at launch.
//!
//! GPLv2 §2(c) asks an interactive program to announce its copyright notice
//! and the absence of warranty when it starts; the client prints these lines to
//! its log and console, so modified versions must keep showing them.

/// Copyright line: SJK's authors, and JKR's whose work it builds on. The version
/// is the build's ([`crate::build_info::VERSION`]).
pub(crate) const COPYRIGHT: &str = concat!(
    "Sol JK ",
    env!("SJK_BUILD_VERSION"),
    ", Copyright (C) 2026 Sol-Vulpes, Bishop-R and the JKR contributors"
);

/// Licence and warranty line.
pub(crate) const LICENSE: &str = "Based on JKR by Bishop. Free software under the GNU GPL v2, \
     with ABSOLUTELY NO WARRANTY; see LICENSE and CREDITS.md";

/// Both announcement lines, in order.
pub(crate) const LINES: [&str; 2] = [COPYRIGHT, LICENSE];
