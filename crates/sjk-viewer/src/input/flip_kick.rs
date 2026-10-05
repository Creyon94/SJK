//! JoF EJK's `flipkick` command (`codemp/cgame/cg_consolecmds.c` `CG_Flipkick_f`,
//! sequence in `cg_view.c` `CG_DoAsync`): one press starts a run of jump taps,
//! jump on one command and off the next, so a JA+ flip kick chains off a
//! player without hammering the jump key.
//!
//! EJK counts cgame frames and sends `+moveup`/`-moveup` once per frame, one user
//! command each. SJK sends a user command every 25 ms, so the run counts user
//! commands instead: each one carries the run's jump state, which is what the
//! server sees. `cg_fkDuration` 50 is therefore 1.25 s here (0.4 s at EJK's
//! 125 fps). A server forbids the bind with bit 7 of serverinfo `restricts`
//! (`RESTRICT_FLIPKICKBIND`).

/// The run's length and its first press, in user commands
/// (`cg_fkDuration`, `cg_fkFirstJumpDuration`, `cg_fkSecondJumpDelay`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Timing {
    pub(crate) duration: u32,
    /// Commands the first jump stays held.
    pub(crate) first_jump: u32,
    /// Command count before the second jump.
    pub(crate) second_jump_delay: u32,
}

impl Default for Timing {
    /// EJK's defaults (`cg_xcvar.h`): 50, 0, 0.
    fn default() -> Self {
        Self {
            duration: 50,
            first_jump: 0,
            second_jump_delay: 0,
        }
    }
}

/// The cvars behind [`Timing`], archived with EJK's defaults.
pub(crate) const DURATION_CVAR: &str = "cg_fkDuration";
pub(crate) const FIRST_JUMP_CVAR: &str = "cg_fkFirstJumpDuration";
pub(crate) const SECOND_JUMP_CVAR: &str = "cg_fkSecondJumpDelay";

/// `RESTRICT_FLIPKICKBIND` in serverinfo `restricts`.
pub(crate) const RESTRICT_BIT: i32 = 1 << 7;

/// What the next user command does with jump.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Step {
    /// No run: the command keeps the player's own keys.
    Idle,
    /// Jump held (true) or released (false) in this command.
    Jump(bool),
    /// The run ended: jump released and the jump key let go (`-moveup`).
    End,
}

/// One run's progress; `frame` 0 means no run.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FlipKick {
    frame: u32,
    jumps: u32,
    jump: bool,
}

impl FlipKick {
    /// `flipkick`: start (or restart) a run.
    pub(crate) fn start(&mut self) {
        *self = Self {
            frame: 1,
            ..Self::default()
        };
    }

    pub(crate) fn stop(&mut self) {
        *self = Self::default();
    }

    /// Advance one user command, as `CG_DoAsync` advances one frame.
    pub(crate) fn step(&mut self, timing: Timing) -> Step {
        if self.frame == 0 {
            return Step::Idle;
        }
        if self.frame > timing.duration {
            self.stop();
            return Step::End;
        }
        match self.jumps {
            1 => {
                if self.frame > timing.first_jump {
                    self.jump = false;
                    self.jumps += 1;
                }
            }
            2 => {
                if self.frame > timing.second_jump_delay {
                    self.jump = true;
                    self.jumps += 1;
                }
            }
            _ => {
                self.jump = self.frame % 2 == 1;
                self.jumps += 1;
            }
        }
        self.frame += 1;
        Step::Jump(self.jump)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(timing: Timing) -> Vec<Step> {
        let mut kick = FlipKick::default();
        assert_eq!(kick.step(timing), Step::Idle);
        kick.start();
        let mut steps = Vec::new();
        loop {
            let step = kick.step(timing);
            steps.push(step);
            if step == Step::End {
                return steps;
            }
        }
    }

    #[test]
    fn default_run_alternates_then_releases() {
        let steps = run(Timing {
            duration: 6,
            ..Timing::default()
        });
        let expected = [true, false, true, false, true, false].map(Step::Jump);
        assert_eq!(steps[..6], expected);
        assert_eq!(steps[6..], [Step::End]);
    }

    #[test]
    fn first_jump_held_then_second_jump_delayed() {
        let steps = run(Timing {
            duration: 10,
            first_jump: 3,
            second_jump_delay: 6,
        });
        // Held through command 3, released at 4, pressed again at 7.
        let jumps: Vec<bool> = steps
            .iter()
            .filter_map(|step| match step {
                Step::Jump(jump) => Some(*jump),
                _ => None,
            })
            .collect();
        assert_eq!(
            jumps,
            [
                true, true, true, false, false, false, true, false, true, false
            ]
        );
        assert_eq!(steps.last(), Some(&Step::End));
    }

    #[test]
    fn restart_begins_a_new_run() {
        let timing = Timing::default();
        let mut kick = FlipKick::default();
        kick.start();
        kick.step(timing);
        kick.step(timing);
        kick.start();
        assert_eq!(kick.step(timing), Step::Jump(true));
    }
}
