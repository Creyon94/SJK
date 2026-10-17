//! Capture-the-flag status on the HUD, in every HUD style: the flag icons at the left
//! (`CG_DrawFlagStatus`, `cg_draw.c`), a note when the local player carries a flag, and
//! the team scores at the top centre (`CG_DrawMiniScoreboard`'s red and blue counts).
//!
//! The flags' state is `CS_FLAGSTATUS` (config string 23, one character per team: `0` at
//! base, `1` taken, `2` dropped; `ctfFlagStatusRemap` in `cg_servercmds.c`), the team
//! scores `CS_SCORES1` and `CS_SCORES2`, and carrying is the local player state's
//! `PW_REDFLAG`, `PW_BLUEFLAG` or `PW_NEUTRALFLAG` timer, as `CG_DrawPowerupIcons` tests.
use super::icons::assets;
use super::*;
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign, TextOverflow};
use std::fmt::Write;

/// Text ids of the flag captions (two rows) and the two team score labels.
pub(super) const TEXT_FIRST: u32 = 340;
/// Last text id this module resolves.
pub(super) const TEXT_LAST: u32 = 345;

const CS_SCORES1: usize = 6;
const CS_SCORES2: usize = 7;
const CS_FLAGSTATUS: usize = 23;
/// `SCORE_NOT_PRESENT` (`bg_public.h`): a team score the server has not sent.
const SCORE_NOT_PRESENT: i32 = -9999;
/// `gametype_t`: team free for all, siege, capture the flag, capture the ysalamiri.
const GT_TEAM: i32 = 6;
const GT_SIEGE: i32 = 7;
const GT_CTF: i32 = 8;
const GT_CTY: i32 = 9;
const TEAM_RED: u8 = 1;
const TEAM_BLUE: u8 = 2;
/// `PW_REDFLAG`, `PW_BLUEFLAG`, `PW_NEUTRALFLAG`.
const FLAG_POWERUPS: [usize; 3] = [4, 5, 6];

/// Where a team's flag is (`flagStatus_t`, the states the server sends).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Flag {
    /// On its stand.
    #[default]
    AtBase,
    /// Held by a player.
    Taken,
    /// Lying where its carrier died.
    Dropped,
}

/// The red and blue flags' states from a `CS_FLAGSTATUS` string; an unknown character
/// reads as at base, as `CG_SetConfigValues` ignores it.
pub(crate) fn parse_status(status: &[u8]) -> [Flag; 2] {
    let flag = |byte: Option<&u8>| match byte {
        Some(b'1') => Flag::Taken,
        Some(b'2') => Flag::Dropped,
        _ => Flag::AtBase,
    };
    [flag(status.first()), flag(status.get(1))]
}

/// One flag icon of the left stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Row {
    /// The icon's cell ([`assets`]).
    pub(crate) slot: usize,
    /// The note beside it.
    pub(crate) caption: &'static str,
    /// The local player carries this flag: the icon stands on a gold plate.
    pub(crate) carried: bool,
}

/// The flag rows for a player of `team`: first the enemy flag when the team holds it
/// (taken, dropped, or carried by the local player whatever the status says), then the
/// team's own flag when the enemy holds it (`CG_YourTeamHasFlag`, `CG_OtherTeamHasFlag`).
pub(crate) fn rows(team: u8, flags: [Flag; 2], carried: bool, ysalamiri: bool) -> [Option<Row>; 2] {
    let (own, enemy) = match team {
        TEAM_RED => (flags[0], flags[1]),
        TEAM_BLUE => (flags[1], flags[0]),
        _ => return [None, None],
    };
    let caption = |flag| match flag {
        Flag::Dropped => "DROPPED",
        _ => "TAKEN",
    };
    let red = team == TEAM_RED;
    let enemy_slot = match (red, ysalamiri) {
        (true, false) => assets::BLUE_FLAG,
        (true, true) => assets::BLUE_FLAG_YSALAMIRI,
        (false, false) => assets::RED_FLAG,
        (false, true) => assets::RED_FLAG_YSALAMIRI,
    };
    let held = enemy != Flag::AtBase || carried;
    let first = held.then_some(Row {
        slot: enemy_slot,
        caption: if carried {
            "YOU HAVE THE FLAG"
        } else {
            caption(enemy)
        },
        carried,
    });
    let second = (own != Flag::AtBase).then_some(Row {
        slot: if red {
            assets::RED_FLAG_TAKEN
        } else {
            assets::BLUE_FLAG_TAKEN
        },
        caption: caption(own),
        carried: false,
    });
    [first, second]
}

/// Sampled once per HUD update; the strings keep their storage.
#[derive(Default)]
pub(crate) struct State {
    team: u8,
    ctf: bool,
    ysalamiri: bool,
    flags: [Flag; 2],
    carried: bool,
    show_flags: bool,
    show_scores: bool,
    scores: Option<[i32; 2]>,
    labels: [String; 2],
    /// A made-up CTF state for world shots.
    #[cfg(test)]
    staged: Option<Staged>,
}

/// What a world shot stages over a recorded game.
#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) struct Staged {
    pub(crate) team: u8,
    pub(crate) flags: [Flag; 2],
    pub(crate) carried: bool,
    pub(crate) scores: [i32; 2],
    pub(crate) ysalamiri: bool,
}

impl State {
    /// Whether the team score plates are on this frame (the HUD's score line then
    /// leaves the team scores out).
    pub(super) fn scores_shown(&self) -> bool {
        self.show_scores && self.scores.is_some()
    }

    pub(super) fn update(
        &mut self,
        snapshot: &sjk_protocol::Snapshot,
        game: &GameState,
        console: Option<&ViewerConsole>,
    ) {
        self.show_flags = family::integer(console, "cg_drawFlagStatus", 1) != 0;
        self.show_scores = family::integer(console, "cg_drawTeamScores", 1) != 0;
        let gametype = game
            .config_string(0)
            .and_then(|s| sjk_client::LegacyClientInfo::new(s).integer("g_gametype"))
            .unwrap_or(0);
        self.team = snapshot.player.team();
        self.ctf = matches!(gametype, GT_CTF | GT_CTY);
        self.ysalamiri = gametype == GT_CTY;
        self.flags = if self.ctf {
            game.config_string(CS_FLAGSTATUS)
                .map_or([Flag::AtBase; 2], parse_status)
        } else {
            [Flag::AtBase; 2]
        };
        self.carried = self.ctf
            && FLAG_POWERUPS.iter().any(|&power| {
                i64::from(snapshot.player.powerups[power]) > i64::from(snapshot.server_time)
            });
        self.scores = (gametype >= GT_TEAM && gametype != GT_SIEGE).then(|| {
            [CS_SCORES1, CS_SCORES2].map(|index| {
                game.config_string(index)
                    .and_then(|s| std::str::from_utf8(s).ok())
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0)
            })
        });
        #[cfg(test)]
        if let Some(staged) = self.staged {
            self.team = staged.team;
            self.ctf = true;
            self.ysalamiri = staged.ysalamiri;
            self.flags = staged.flags;
            self.carried = staged.carried;
            self.scores = Some(staged.scores);
        }
        for (label, (name, score)) in self.labels.iter_mut().zip(
            ["RED", "BLUE"]
                .into_iter()
                .zip(self.scores.unwrap_or([0; 2])),
        ) {
            label.clear();
            if score == SCORE_NOT_PRESENT {
                let _ = write!(label, "{name} -");
            } else {
                let _ = write!(label, "{name} {score}");
            }
        }
    }

    /// Play a made-up CTF state over a recorded game, for world shots.
    #[cfg(test)]
    pub(crate) fn stage_for_shot(&mut self, staged: Staged) {
        self.staged = Some(staged);
    }

    /// The caption or label behind text id `id`.
    pub(super) fn text(&self, id: u32) -> &str {
        match id {
            TEXT_FIRST | 343 => self.caption(usize::from(id != TEXT_FIRST)),
            344 => &self.labels[0],
            345 => &self.labels[1],
            _ => "",
        }
    }

    fn caption(&self, row: usize) -> &str {
        rows(self.team, self.flags, self.carried, self.ysalamiri)[row].map_or("", |row| row.caption)
    }

    /// Append the icons, their captions and the team score plates.
    pub(super) fn emit(
        &self,
        list: &mut DrawList,
        viewport: [f32; 2],
        icons: &icons::Icons,
        visibility: HudVisibility,
    ) {
        let [w, h] = viewport;
        if self.show_flags && self.ctf && visibility.status {
            // CG_DrawFlagStatus: 32-unit icons at the left edge of the 640x480 screen,
            // the first at y 328 and the next one icon up.
            let unit = h / 480.0;
            let size = 32.0 * unit;
            let rows = rows(self.team, self.flags, self.carried, self.ysalamiri);
            for (index, row) in rows.iter().flatten().enumerate() {
                let x = 6.0 * unit;
                let y = (328.0 - 36.0 * index as f32) * unit;
                if row.carried {
                    let _ = list.push(DrawCommand::SolidRect {
                        rect: Rect::new(
                            x - 3.0 * unit,
                            y - 3.0 * unit,
                            size + 6.0 * unit,
                            size + 6.0 * unit,
                        ),
                        color: Color::new(1.0, 0.78, 0.2, 0.6),
                    });
                }
                let texture = icons.cell(row.slot).or_else(|| {
                    icons.cell(match row.slot {
                        assets::RED_FLAG_TAKEN => assets::RED_FLAG,
                        assets::BLUE_FLAG_TAKEN => assets::BLUE_FLAG,
                        other => other,
                    })
                });
                if let Some(texture) = texture {
                    let _ = list.push(DrawCommand::TexturedQuad {
                        rect: Rect::new(x, y, size, size),
                        texture,
                        color: Color::new(1.0, 1.0, 1.0, 1.0),
                    });
                }
                let _ = list.push(DrawCommand::Text {
                    rect: Rect::new(
                        x + size + 6.0 * unit,
                        y + 6.0 * unit,
                        200.0 * unit,
                        22.0 * unit,
                    ),
                    text: TextId(TEXT_FIRST + 3 * index as u32),
                    size: 13.0 * unit,
                    color: if row.carried {
                        Color::new(1.0, 0.85, 0.35, 1.0)
                    } else {
                        Color::new(1.0, 1.0, 1.0, 1.0)
                    },
                    align: TextAlign::Start,
                    overflow: TextOverflow::Clip,
                    weight: FontWeight::Semibold,
                    letter_spacing: 0.0,
                });
            }
        }
        if self.scores_shown() && visibility.hud {
            let s = crate::ui_scale::height_scale(h);
            let (plate_w, plate_h, gap) = (130.0 * s, 30.0 * s, 8.0 * s);
            let top = 76.0 * s;
            for (index, tint) in [
                Color::new(0.75, 0.12, 0.12, 0.55),
                Color::new(0.14, 0.3, 0.85, 0.55),
            ]
            .into_iter()
            .enumerate()
            {
                let x = if index == 0 {
                    w * 0.5 - gap * 0.5 - plate_w
                } else {
                    w * 0.5 + gap * 0.5
                };
                let rect = Rect::new(x, top, plate_w, plate_h);
                let _ = list.push(DrawCommand::SolidRect { rect, color: tint });
                let _ = list.push(DrawCommand::Text {
                    rect,
                    text: TextId(344 + index as u32),
                    size: 20.0 * s,
                    color: Color::new(1.0, 1.0, 1.0, 1.0),
                    align: TextAlign::Center,
                    overflow: TextOverflow::Clip,
                    weight: FontWeight::Semibold,
                    letter_spacing: 0.0,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_characters_map_to_states() {
        assert_eq!(parse_status(b"12"), [Flag::Taken, Flag::Dropped]);
        assert_eq!(parse_status(b"00"), [Flag::AtBase; 2]);
        assert_eq!(parse_status(b"*"), [Flag::AtBase; 2]);
        assert_eq!(parse_status(b""), [Flag::AtBase; 2]);
    }

    #[test]
    fn a_red_player_sees_the_blue_flag_taken_and_the_own_flag_gone() {
        let [first, second] = rows(TEAM_RED, [Flag::Taken, Flag::Taken], false, false);
        assert_eq!(first.unwrap().slot, assets::BLUE_FLAG);
        assert_eq!(first.unwrap().caption, "TAKEN");
        assert_eq!(second.unwrap().slot, assets::RED_FLAG_TAKEN);
    }

    #[test]
    fn a_blue_player_sees_the_red_flag_and_a_dropped_caption() {
        let [first, second] = rows(TEAM_BLUE, [Flag::Dropped, Flag::AtBase], false, false);
        assert_eq!(first.unwrap().slot, assets::RED_FLAG);
        assert_eq!(first.unwrap().caption, "DROPPED");
        assert!(second.is_none());
    }

    #[test]
    fn carrying_marks_the_first_row_even_if_the_status_says_at_base() {
        let [first, second] = rows(TEAM_BLUE, [Flag::AtBase; 2], true, false);
        let first = first.unwrap();
        assert!(first.carried);
        assert_eq!(first.caption, "YOU HAVE THE FLAG");
        assert!(second.is_none());
    }

    #[test]
    fn ysalamiri_games_use_the_ys_icons_and_spectators_get_nothing() {
        let [first, _] = rows(TEAM_RED, [Flag::AtBase, Flag::Taken], false, true);
        assert_eq!(first.unwrap().slot, assets::BLUE_FLAG_YSALAMIRI);
        assert_eq!(rows(3, [Flag::Taken; 2], true, false), [None, None]);
    }

    #[test]
    fn at_base_flags_show_nothing() {
        assert_eq!(
            rows(TEAM_RED, [Flag::AtBase; 2], false, false),
            [None, None]
        );
    }
}
