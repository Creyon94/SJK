//! Capture-the-flag status on the HUD, in every HUD style: the flag icons at the left
//! (`CG_DrawFlagStatus`, `cg_draw.c`), a note when the local player carries a flag, and
//! the team scores at the top centre (`CG_DrawMiniScoreboard`'s red and blue counts).
//!
//! The flags' state is `CS_FLAGSTATUS` (config string 23, one character per team: `0` at
//! base, `1` taken, `2` dropped; `ctfFlagStatusRemap` in `cg_servercmds.c`), the team
//! scores `CS_SCORES1` and `CS_SCORES2`, and carrying is the local player state's
//! `PW_REDFLAG`, `PW_BLUEFLAG` or `PW_NEUTRALFLAG` timer, as `CG_DrawPowerupIcons` tests.
//!
//! A taken flag's caption names its carrier when the client knows who it is, from the
//! freshest source, as the scoreboard's flags do (`scoreboard/live_flags.rs`): the
//! viewed player's own state, a player entity in the snapshot, a teammate's `tinfo`
//! row, then the last carrier seen while the flag stays taken.
use super::icons::assets;
use super::*;
use sjk_client::TeamInfo;
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
/// `CS_PLAYERS`: one client-info config string per client from here.
const CS_PLAYERS: usize = 1_131;
/// `entityType_t::ET_PLAYER`.
const ET_PLAYER: u8 = 1;

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
    /// Whose flag it is: 0 red, 1 blue.
    pub(crate) side: usize,
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
    let (own_side, enemy_side) = if red { (0, 1) } else { (1, 0) };
    let enemy_slot = match (red, ysalamiri) {
        (true, false) => assets::BLUE_FLAG,
        (true, true) => assets::BLUE_FLAG_YSALAMIRI,
        (false, false) => assets::RED_FLAG,
        (false, true) => assets::RED_FLAG_YSALAMIRI,
    };
    let held = enemy != Flag::AtBase || carried;
    let first = held.then_some(Row {
        side: enemy_side,
        slot: enemy_slot,
        caption: if carried {
            "YOU HAVE THE FLAG"
        } else {
            caption(enemy)
        },
        carried,
    });
    let second = (own != Flag::AtBase).then_some(Row {
        side: own_side,
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
    /// The red and blue flags' carriers by name ([`Carrier`]).
    carriers: [Carrier; 2],
    /// A made-up CTF state for world shots.
    #[cfg(test)]
    staged: Option<Staged>,
}

/// Who carries one flag, and the name the caption shows. The name is formatted again
/// only when the carrier, their client-info config string or the colour setting
/// changes, so a steady carrier costs a byte comparison a frame and no allocation.
#[derive(Default)]
struct Carrier {
    client: Option<u16>,
    colors: bool,
    /// The client-info config string `name` was read from.
    source: Vec<u8>,
    name: String,
}

impl Carrier {
    fn set(&mut self, client: Option<u16>, game: &GameState, colors: bool) {
        let Some(number) = client else {
            self.client = None;
            self.name.clear();
            return;
        };
        let info = game
            .config_string(CS_PLAYERS + usize::from(number))
            .unwrap_or_default();
        if self.client == client && self.colors == colors && self.source == info {
            return;
        }
        self.client = client;
        self.colors = colors;
        self.source.clear();
        self.source.extend_from_slice(info);
        self.name.clear();
        if let Some((name, _)) = data_source::parse_client_info(info) {
            family::name(&mut self.name, &name, colors);
        }
    }
}

/// The client carrying flag powerup `power`, from the viewed player's state, a player
/// entity in the snapshot or a teammate's `tinfo` row.
fn find_carrier(power: usize, snapshot: &sjk_protocol::Snapshot, team: &[TeamInfo]) -> Option<u16> {
    let bit = 1_u32 << power;
    if i64::from(snapshot.player.powerups[power]) > i64::from(snapshot.server_time) {
        return Some(snapshot.player.client_num());
    }
    if let Some(state) = snapshot
        .entities
        .iter()
        .find(|state| state.entity_type() == ET_PLAYER && state.powerups() & bit != 0)
    {
        return Some(state.number());
    }
    team.iter()
        .find(|info| info.powerups as u32 & bit != 0)
        .map(|info| u16::from(info.client_num))
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

    /// Note who carries each taken flag, from `snapshot` and the teammates' `team`
    /// rows; a carrier out of sight stays named while the flag stays taken.
    pub(super) fn update_carriers(
        &mut self,
        snapshot: &sjk_protocol::Snapshot,
        game: &GameState,
        team: &[TeamInfo],
        colors: bool,
    ) {
        for (side, carrier) in self.carriers.iter_mut().enumerate() {
            let client = (self.ctf && self.flags[side] == Flag::Taken)
                .then(|| find_carrier(FLAG_POWERUPS[side], snapshot, team).or(carrier.client))
                .flatten();
            carrier.set(client, game, colors);
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

    /// The caption of the `row`th drawn icon: a taken flag's carrier by name when known.
    fn caption(&self, row: usize) -> &str {
        let Some(row) = rows(self.team, self.flags, self.carried, self.ysalamiri)
            .into_iter()
            .flatten()
            .nth(row)
        else {
            return "";
        };
        let carrier = &self.carriers[row.side].name;
        if !row.carried && self.flags[row.side] == Flag::Taken && !carrier.is_empty() {
            carrier
        } else {
            row.caption
        }
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
        // Under the status gate, as JoF's team scores in `CG_DrawHUD`: an open classic
        // board keeps the HUD but not these plates over its header.
        if self.scores_shown() && visibility.hud && visibility.status {
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

impl HudOverlay {
    /// Name the flag carriers after [`HudOverlay::update_family`] has read the flags;
    /// `team` is the teammates' `tinfo` (empty for a demo, which keeps none).
    pub(crate) fn update_flag_carriers(
        &mut self,
        snapshot: &sjk_protocol::Snapshot,
        game: &GameState,
        team: &[TeamInfo],
    ) {
        self.flag_status
            .update_carriers(snapshot, game, team, self.family.name_colors);
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
    fn a_taken_flag_names_its_carrier_and_keeps_the_name_cached() {
        let mut game = GameState::empty_local(0);
        game.replace_config_string(CS_PLAYERS + 3, b"\\n\\^1Slash\\t\\1".to_vec())
            .unwrap();
        let mut state = State {
            team: TEAM_RED,
            ctf: true,
            flags: [Flag::Taken, Flag::Taken],
            ..State::default()
        };
        state.carriers[1].set(Some(3), &game, false);
        // The blue flag (held by red) first, then red's own flag, no carrier known.
        assert_eq!(state.caption(0), "Slash");
        assert_eq!(state.caption(1), "TAKEN");
        let capacity = state.carriers[1].name.capacity();
        state.carriers[1].set(Some(3), &game, false);
        assert_eq!(state.carriers[1].name.capacity(), capacity);
        state.carriers[1].set(Some(3), &game, true);
        assert_eq!(state.caption(0), "^1Slash");
        state.carriers[1].set(None, &game, true);
        assert_eq!(state.caption(0), "TAKEN");
    }

    #[test]
    fn the_own_flag_alone_keeps_its_caption() {
        let state = State {
            team: TEAM_BLUE,
            ctf: true,
            flags: [Flag::AtBase, Flag::Dropped],
            ..State::default()
        };
        assert_eq!(state.caption(0), "DROPPED");
        assert_eq!(state.caption(1), "");
    }

    #[test]
    fn at_base_flags_show_nothing() {
        assert_eq!(
            rows(TEAM_RED, [Flag::AtBase; 2], false, false),
            [None, None]
        );
    }
}
