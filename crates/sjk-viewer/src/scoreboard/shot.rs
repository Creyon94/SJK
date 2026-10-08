//! Made-up matches for the off-screen world shots of the scoreboard
//! (`world_shot::tests::duel6_sjk_scoreboard`, `duel6_scoreboard_medals`): rows and
//! match facts set directly, so the board draws over a map without a server, in the
//! look `cg_scoreboardStyle` picks.

use super::classic::{FlagIcons, LocalStatus};
use super::sjk::{self, Duelists, Limits, Measure, SjkHeader};
use super::{ScoreRow, Scoreboard};
use crate::player_identity::Tag;
use crate::text::TextStyle;

/// A match to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Match {
    /// Free for all: fourteen players (you third, three bots, two tied), two
    /// watching.
    Free,
    /// Free for all on a full server: thirty playing, two watching, and you
    /// just killed.
    Crowd,
    /// Capture the flag, six a side, each side's flag carried.
    Capture,
    /// A duel: you against one, five waiting.
    Duel,
    /// A power duel: you alone against a pair, three waiting.
    PowerDuel,
}

/// The match facts behind a shot's rows.
pub(crate) struct Shot {
    gametype: i32,
    limits: Limits,
    elapsed: i32,
    team_scores: [i32; 2],
    local: LocalStatus,
    killer: Option<&'static str>,
    duel: Duelists,
}

/// You, in every shot.
const YOU: u8 = 5;
const YOUR_NAME: &str = "^1Sol^7Fox";

const NAMES: [&str; 31] = [
    "^1Kael^7Varn",
    "^3Mira",
    "^5Tor^7ren",
    "Ashla",
    "^2Vex",
    "^6Nyx",
    "^4Daro",
    "Kyle",
    "Tavion",
    "^1Rax",
    "Jaden",
    "^3Ilum^7Fox",
    "Bastila",
    "^5Quin",
    "^2Sable",
    "Rosh",
    "^6Lumen",
    "^4Orin",
    "^3Talon",
    "Desann",
    "^1Ember",
    "^5Hollow",
    "Juno",
    "^2Kestrel",
    "^6Wren",
    "^4Corvin",
    "Alora",
    "^3Brask",
    "^1Siv",
    "^5Tamsin",
    "^6Pell",
];

/// A row for `client` named `name` on `team` with `score`.
fn row(client: u8, name: &str, team: u8, score: i32) -> ScoreRow {
    let bot = matches!(name, "Kyle" | "Tavion" | "Rosh" | "Desann");
    ScoreRow {
        client_num: client,
        name: name.to_owned(),
        team,
        score,
        deaths: None,
        ping: if bot {
            0
        } else {
            [18, 34, 52, 71, 96, 124, 160, 230][usize::from(client) % 8]
        },
        time: 4 + i32::from(client * 3 % 17),
        has_score: true,
        bot,
        record: None,
        powerups: 0,
        defends: i32::from(client % 4),
        assists: i32::from(client % 3),
        captures: i32::from(client % 2),
        // A few players the hub knows, two of them vouched for, three with medals.
        identity: match client {
            0 => Some(Tag {
                verified: true,
                medals: medals(&["early_tester", "early_contributor", "bug_hunter"], 3),
            }),
            11 => Some(Tag {
                verified: true,
                medals: medals(&["jof_clan"], 1),
            }),
            3 => Some(Tag {
                verified: false,
                medals: medals(&["early_tester", "bug_hunter", "jof_clan"], 2),
            }),
            _ => None,
        },
    }
}

/// Made-up medals as the hub's presence list gives them, a repeatable one `count` times.
fn medals(ids: &[&str], count: u32) -> crate::medals::Medals {
    let list: Vec<sjk_identity::Medal> = ids
        .iter()
        .map(|id| sjk_identity::Medal {
            id: (*id).to_owned(),
            count,
            awarded: 0,
            note: String::new(),
        })
        .collect();
    crate::medals::Medals::from_wire(&list)
}

/// The client slot of the `index`th made-up player, keeping yours free.
fn slot(index: usize) -> u8 {
    let slot = index as u8;
    if slot >= YOU { slot + 1 } else { slot }
}

impl Scoreboard {
    /// Show `game` until the shot is done, fading in as when Tab is pressed.
    pub(crate) fn show_for_shot(&mut self, game: Match) {
        self.rows.clear();
        let local = |team, rank, score| LocalStatus {
            client: u16::from(YOU),
            team,
            rank,
            score,
            ready: 0,
            intermission: false,
        };
        let limits = Limits {
            frags: 30,
            captures: 8,
            duel_wins: 0,
            minutes: 20,
        };
        let shot = match game {
            Match::Free | Match::Crowd => {
                let players = if game == Match::Free { 13 } else { 29 };
                let scores = [27, 24, 19, 19, 16, 14, 12, 11, 9, 7, 5, 3, 1];
                for (index, name) in NAMES.iter().enumerate().take(players) {
                    // You are third, after the first two.
                    if index == 2 {
                        self.rows.push(row(YOU, YOUR_NAME, 0, 21));
                    }
                    let score = scores.get(index).copied().unwrap_or(1) - (index / 13) as i32;
                    self.rows.push(row(slot(index), name, 0, score));
                }
                for (index, name) in NAMES.iter().enumerate().skip(players).take(2) {
                    self.rows.push(row(slot(index), name, 3, 0));
                }
                Shot {
                    gametype: 0,
                    limits,
                    elapsed: 7 * 60_000 + 48_000,
                    team_scores: [0, 0],
                    local: local(0, 2, 21),
                    killer: (game == Match::Crowd).then_some("^1Kael^7Varn"),
                    duel: Duelists::default(),
                }
            }
            Match::Capture => {
                for (index, name) in NAMES.iter().enumerate().take(11) {
                    let team = if index % 2 == 0 { 1 } else { 2 };
                    // In score order, as the server sends them.
                    if index == 5 {
                        let mut you = row(YOU, YOUR_NAME, 2, 14);
                        you.captures = 2;
                        you.assists = 1;
                        you.defends = 4;
                        self.rows.push(you);
                    }
                    let mut player = row(slot(index), name, team, 22 - index as i32 * 2);
                    // A red player carries the blue flag, a blue one the red.
                    player.powerups = match index {
                        2 => 1 << 5,
                        5 => 1 << 4,
                        _ => 0,
                    };
                    self.rows.push(player);
                }
                self.rows.push(row(slot(11), NAMES[11], 3, 0));
                Shot {
                    gametype: 8,
                    limits,
                    elapsed: 12 * 60_000 + 5_000,
                    team_scores: [3, 5],
                    local: local(2, 0, 14),
                    killer: None,
                    duel: Duelists::default(),
                }
            }
            Match::Duel | Match::PowerDuel => {
                let power = game == Match::PowerDuel;
                let mut you = row(YOU, YOUR_NAME, 0, 4);
                you.record = Some((3, 1));
                self.rows.push(you);
                let opponents = if power { 2 } else { 1 };
                for (index, name) in NAMES.iter().enumerate().take(opponents) {
                    let mut opponent = row(slot(index), name, 0, 2 - index as i32);
                    opponent.record = Some((5 - index as i32, 2));
                    self.rows.push(opponent);
                }
                let waiting = if power { 3 } else { 5 };
                for (index, name) in NAMES.iter().enumerate().skip(opponents).take(waiting) {
                    let mut player = row(slot(index), name, 3, 0);
                    player.record = Some((index as i32 % 3, 1 + index as i32 % 2));
                    self.rows.push(player);
                }
                Shot {
                    gametype: if power { 4 } else { 3 },
                    limits: Limits {
                        frags: 5,
                        captures: 0,
                        duel_wins: 10,
                        minutes: 0,
                    },
                    elapsed: 3 * 60_000 + 12_000,
                    team_scores: [0, 0],
                    local: local(0, 0, 4),
                    killer: None,
                    duel: Duelists {
                        clients: [Some(YOU), Some(slot(0)), power.then(|| slot(1))],
                        healths: [Some(87), Some(64)],
                        local: Some((87, 25)),
                    },
                }
            }
        };
        self.shot = Some(shot);
        self.motion = super::motion::Motion::default();
    }

    /// Whether a made-up match is on show.
    pub(crate) fn showing_shot(&self) -> bool {
        self.shot.is_some()
    }

    /// Back to the server's scores.
    pub(crate) fn end_shot(&mut self) {
        self.shot = None;
        self.rows.clear();
    }
}

/// Draw the shot's match in the SJK look, in the UI's families.
pub(super) fn append(gpu: &mut crate::GpuState, viewport: [f32; 2]) {
    let flags = FlagIcons {
        red: gpu.hud.icons.powerup(4),
        blue: gpu.hud.icons.powerup(5),
        neutral: gpu.hud.icons.powerup(6),
    };
    let Scoreboard {
        ui,
        rows,
        motion,
        shot,
        style,
        icons,
        ..
    } = &mut gpu.scoreboard;
    let Some(shot) = shot.as_ref() else {
        return;
    };
    match style {
        super::style::ScoreboardStyle::Classic => {
            let header = super::classic::ClassicHeader {
                hostname: "^5JoF^7 Jedi of Freedom",
                max_clients: 32,
                gametype: shot.gametype,
                fraglimit: shot.limits.frags,
                team_scores: shot.team_scores,
                local: shot.local,
                killer: shot.killer,
            };
            let options = super::style::ClassicOptions::from_console(gpu.console.as_ref());
            super::classic::build(ui, rows, &header, options, icons, flags, motion, viewport);
            ui.finish(u16::MAX);
            ui.append_text_routed(
                &mut gpu.game_fonts,
                |_, text| Some(super::retail_font(text)),
                &mut gpu.text_vertices,
                &gpu.ui_font,
                viewport,
            );
            return;
        }
        super::style::ScoreboardStyle::Modern => {
            let header = super::view::MatchHeader {
                map: "mp/duel6",
                mode: "FFA",
                team_scores: shot.team_scores,
                team_game: false,
                local_client: shot.local.client,
            };
            super::view::build(ui, rows, header, viewport);
            ui.finish(u16::MAX);
            ui.append_text_routed(
                &mut gpu.game_fonts,
                |_, text| Some(super::retail_font(text)),
                &mut gpu.text_vertices,
                &gpu.ui_font,
                viewport,
            );
            return;
        }
        super::style::ScoreboardStyle::Sjk => {}
    }
    let header = SjkHeader {
        map: "mp/duel6",
        gametype: shot.gametype,
        limits: shot.limits,
        elapsed: Some(shot.elapsed),
        team_scores: shot.team_scores,
        local: shot.local,
        killer: shot.killer,
        duel: shot.duel,
    };
    let measure = match gpu.game_fonts.sjk_metrics() {
        Some((display, body)) => Measure {
            display: Some(display),
            body: Some(body),
        },
        None => Measure {
            display: Some(&gpu.ui_font),
            body: Some(&gpu.ui_font),
        },
    };
    sjk::build(ui, rows, &header, flags, measure, motion, viewport);
    ui.finish(u16::MAX);
    match gpu.game_fonts.sjk() {
        Some(families) => ui.append_text_families(families, viewport, TextStyle::NEUTRAL),
        None => ui.append_text_styled(
            &mut gpu.text_vertices,
            &gpu.ui_font,
            viewport,
            TextStyle::NEUTRAL,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every made-up match fits the board's canvas at 1080 lines and 4K.
    #[test]
    fn every_shot_fits_the_canvas() {
        for game in [
            Match::Free,
            Match::Crowd,
            Match::Capture,
            Match::Duel,
            Match::PowerDuel,
        ] {
            let mut board = Scoreboard::new();
            board.show_for_shot(game);
            assert!(board.rows.len() <= 32, "{game:?}");
            let shot = board.shot.as_ref().expect("a shot");
            let header = SjkHeader {
                map: "mp/duel6",
                gametype: shot.gametype,
                limits: shot.limits,
                elapsed: Some(shot.elapsed),
                team_scores: shot.team_scores,
                local: shot.local,
                killer: shot.killer,
                duel: shot.duel,
            };
            for viewport in [[1920.0, 1080.0], [3840.0, 2160.0]] {
                sjk::build(
                    &mut board.ui,
                    &board.rows,
                    &header,
                    FlagIcons::default(),
                    Measure::default(),
                    &mut board.motion,
                    viewport,
                );
                board.ui.finish(u16::MAX);
                assert!(!board.ui.overflowed(), "{game:?} at {viewport:?}");
            }
        }
    }
}
