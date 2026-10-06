//! When user commands are made and when move packets leave.
//!
//! JoF EJK's `cl_cmdratecap` (`codemp/client/cl_input.cpp`
//! `CL_CreateNewCommands`) makes usercmds on a fixed 8 ms step, 125 a second,
//! however fast the client renders, so the server moves the player in the same
//! steps as a 125 FPS client. Stock makes one per rendered frame, which on a
//! `pmove_fixed` server loses input above 125 FPS: `ClientThink_real` rounds a
//! command's time up to the `pmove_msec` grid (`g_active.c`) and drops one that
//! lands in a slot already run (`msec < 1`), button presses included.
//!
//! [`CommandSchedule`] stamps every command on an 8 ms boundary of server time,
//! one per slot, so no two commands share a slot on any `pmove_msec` 8 grid. A
//! frame slower than 8 ms makes a command for each slot it passed, up to
//! [`MAX_CATCH_UP`]; after a longer hitch the older slots are skipped rather
//! than sent as a burst.
//!
//! [`PacketPacer`] is `cl_maxpackets` (`CL_ReadyToSendPacket`): commands wait
//! for a packet, which leaves at most every `1000 / cl_maxpackets` ms and
//! carries all of them.

use std::time::{Duration, Instant};

/// Milliseconds between user commands (125 a second).
pub const COMMAND_MILLIS: i32 = 8;
/// Most commands one frame makes to fill the slots it passed.
pub const MAX_CATCH_UP: usize = 4;
/// Stock `cl_maxpackets` clamp (`cl_input.cpp` `CL_ReadyToSendPacket`; JoF EJK
/// and EternalJK raise the stock 125 ceiling to 1000).
pub const MIN_PACKETS: i64 = 15;
pub const MAX_PACKETS: i64 = 1000;
/// A stamp this far behind the previous one is a new time line.
const RESTART_MILLIS: i32 = 500;

/// The server times of the commands due this frame, oldest first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DueCommands {
    stamps: [i32; MAX_CATCH_UP],
    len: usize,
}

impl DueCommands {
    pub fn as_slice(&self) -> &[i32] {
        &self.stamps[..self.len]
    }

    fn push(&mut self, stamp: i32) {
        self.stamps[self.len] = stamp;
        self.len += 1;
    }
}

/// The 125 Hz command clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct CommandSchedule {
    /// The slot of the newest command stamped on an anchored clock.
    last_slot: Option<i32>,
    /// While the clock stamps 0, the next command is due at this instant.
    unanchored_due: Option<Instant>,
}

impl CommandSchedule {
    /// Start over: the next call makes one command at once.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Commands due at `now`, given the server time the clock reads now and
    /// whether it is anchored. An unanchored clock stamps 0 (stock
    /// `cl.serverTime` during a map load); its commands still leave every
    /// 8 ms of real time, so packets keep acknowledging the server.
    pub fn due(&mut self, server_time: i32, anchored: bool, now: Instant) -> DueCommands {
        let mut due = DueCommands::default();
        if !anchored {
            self.last_slot = None;
            if self.unanchored_due.is_none_or(|at| now >= at) {
                let step = Duration::from_millis(COMMAND_MILLIS as u64);
                let next = self.unanchored_due.unwrap_or(now) + step;
                // Behind by more than a step after a hitch: start again from now.
                self.unanchored_due = Some(if next < now { now + step } else { next });
                due.push(0);
            }
            return due;
        }
        self.unanchored_due = None;
        let slot = server_time.div_euclid(COMMAND_MILLIS);
        let first = match self.last_slot {
            Some(last) if slot <= last => {
                if (last - slot) * COMMAND_MILLIS <= RESTART_MILLIS {
                    return due;
                }
                slot
            }
            Some(last) => (last + 1).max(slot - (MAX_CATCH_UP as i32 - 1)),
            None => slot,
        };
        for slot in first..=slot {
            due.push(slot * COMMAND_MILLIS);
        }
        self.last_slot = Some(slot);
        due
    }
}

/// `cl_maxpackets`: how often a move packet may leave.
#[derive(Clone, Copy, Debug, Default)]
pub struct PacketPacer {
    last_sent: Option<Instant>,
}

impl PacketPacer {
    /// Whether a packet may leave at `now` with `max_packets` a second
    /// (clamped like stock). The interval is whole milliseconds, as stock's
    /// `1000 / cl_maxpackets`.
    pub fn ready(&self, now: Instant, max_packets: i64) -> bool {
        let interval = 1000 / max_packets.clamp(MIN_PACKETS, MAX_PACKETS) as u64;
        self.last_sent.is_none_or(|sent| {
            now.saturating_duration_since(sent) >= Duration::from_millis(interval)
        })
    }

    /// A packet left at `now`.
    pub fn sent(&mut self, now: Instant) {
        self.last_sent = Some(now);
    }

    /// Let the next packet leave at once.
    pub fn reset(&mut self) {
        self.last_sent = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamps(schedule: &mut CommandSchedule, server_time: i32) -> Vec<i32> {
        schedule
            .due(server_time, true, Instant::now())
            .as_slice()
            .to_vec()
    }

    #[test]
    fn one_command_per_8_ms_slot_on_the_grid() {
        let mut schedule = CommandSchedule::default();
        assert_eq!(stamps(&mut schedule, 1_003), [1_000]);
        // A fast frame in the same slot makes nothing.
        assert_eq!(stamps(&mut schedule, 1_005), [] as [i32; 0]);
        assert_eq!(stamps(&mut schedule, 1_007), [] as [i32; 0]);
        assert_eq!(stamps(&mut schedule, 1_008), [1_008]);
        // Clock drift of a millisecond either way never doubles a slot.
        assert_eq!(stamps(&mut schedule, 1_015), [] as [i32; 0]);
        assert_eq!(stamps(&mut schedule, 1_017), [1_016]);
    }

    #[test]
    fn a_slow_frame_fills_the_slots_it_passed() {
        let mut schedule = CommandSchedule::default();
        stamps(&mut schedule, 2_000);
        // 60 FPS: about two slots a frame.
        assert_eq!(stamps(&mut schedule, 2_017), [2_008, 2_016]);
        // A hitch keeps only the newest slots instead of a burst.
        assert_eq!(stamps(&mut schedule, 2_400), [2_376, 2_384, 2_392, 2_400]);
    }

    #[test]
    fn a_new_time_line_starts_again() {
        let mut schedule = CommandSchedule::default();
        stamps(&mut schedule, 90_000);
        // Slightly behind (a clock correction) waits for the old slot.
        assert_eq!(stamps(&mut schedule, 89_990), [] as [i32; 0]);
        assert_eq!(stamps(&mut schedule, 400), [400]);
        assert_eq!(stamps(&mut schedule, 408), [408]);
    }

    #[test]
    fn an_unanchored_clock_stamps_zero_every_8_ms() {
        let mut schedule = CommandSchedule::default();
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        assert_eq!(schedule.due(0, false, at(0)).as_slice(), [0]);
        assert!(schedule.due(0, false, at(5)).as_slice().is_empty());
        assert_eq!(schedule.due(0, false, at(8)).as_slice(), [0]);
        // After a long stall, one command, not a burst.
        assert_eq!(schedule.due(0, false, at(500)).as_slice(), [0]);
        assert!(schedule.due(0, false, at(503)).as_slice().is_empty());
        // Anchoring starts the grid at once.
        assert_eq!(schedule.due(12_345, true, at(504)).as_slice(), [12_344]);
    }

    #[test]
    fn packets_leave_at_most_every_interval() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut pacer = PacketPacer::default();
        assert!(pacer.ready(at(0), 63));
        pacer.sent(at(0));
        // 1000 / 63 = 15 ms.
        assert!(!pacer.ready(at(14), 63));
        assert!(pacer.ready(at(15), 63));
        // Clamped to 15..=1000: 0 means 15 a second, 5000 means 1000.
        assert!(!pacer.ready(at(65), 0));
        assert!(pacer.ready(at(66), 15));
        assert!(pacer.ready(at(1), 5_000));
        pacer.reset();
        assert!(pacer.ready(at(0), 15));
    }
}
