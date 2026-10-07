//! Cached authoritative scoreboard projection rendered through `sjk-ui`.

mod classic;
mod deaths;
mod icons;
mod identity_mark;
pub(crate) mod layout;
mod motion;
#[cfg(test)]
pub(crate) mod shot;
mod sjk;
pub(crate) mod style;
mod view;

use crate::game_font::{GameFonts, RetailFont};
use crate::menu_widgets::MenuCanvas;
use crate::text::{TextStyle, TextVertex, UiFont};
use sjk_client::{ClientSession, ScoreEntry};
use sjk_protocol::{GameState, InfoString};
use sjk_ui::DrawList;

const CS_PLAYERS: usize = 1_131;
/// `CS_LEVEL_START_TIME`, `CS_CLIENT_DUELISTS`, `CS_CLIENT_DUELHEALTHS`.
const CS_LEVEL_START_TIME: usize = 21;
const CS_CLIENT_DUELISTS: usize = 30;
const CS_CLIENT_DUELHEALTHS: usize = 31;
/// The board's text runs and draw commands a frame: 32 rows of up to seven
/// runs (a capture mode's, or any at intermission) and their shapes.
const TEXT_SLOTS: usize = 320;
const DRAWS: usize = 1_024;

/// Chat remains available beside the scoreboard.
pub(crate) fn chat_visible(information: bool, history: bool) -> bool {
    information || history
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ScoreRow {
    pub(super) client_num: u8,
    pub(super) name: String,
    pub(super) team: u8,
    pub(super) score: i32,
    /// Optional session-observed count; no fabricated authoritative plugin statistic.
    pub(super) deaths: Option<i32>,
    pub(super) ping: i32,
    pub(super) time: i32,
    /// The client has a `scores` row; clients still without one are listed
    /// as `N/A`, as `CG_TeamScoreboard` lists them.
    pub(super) has_score: bool,
    /// A bot (`skill` in its clientinfo).
    pub(super) bot: bool,
    /// Duel wins and losses (`w`, `l` in its clientinfo).
    pub(super) record: Option<(i32, i32)>,
    pub(super) powerups: u32,
    pub(super) defends: i32,
    pub(super) assists: i32,
    pub(super) captures: i32,
    /// The SJK hub knows this player under this name (`player_identity.rs`).
    pub(super) identity: Option<crate::player_identity::Tag>,
}

/// Cached scoreboard data and fixed retained presentation storage.
pub(crate) struct Scoreboard {
    cached_scores: Vec<ScoreEntry>,
    config_signature: u64,
    server_signature: u64,
    team_game: bool,
    map: String,
    mode: String,
    rows: Vec<ScoreRow>,
    ui: MenuCanvas,
    deaths: deaths::Deaths,
    hostname: String,
    max_clients: i32,
    gametype: i32,
    fraglimit: i32,
    /// The limits the SJK look words in its header.
    limits: sjk::Limits,
    style: style::ScoreboardStyle,
    motion: motion::Motion,
    icons: icons::HeadIcons,
    /// Who last killed the viewing player, named in `killer_name`.
    killer: Option<u16>,
    killer_name: String,
    /// The hub roster revision the rows' `identity` tags were derived from;
    /// `u64::MAX` when the rows were rebuilt and need tagging again.
    identity_revision: u64,
    /// A made-up match the world shots show without a server.
    #[cfg(test)]
    shot: Option<shot::Shot>,
}

impl Scoreboard {
    pub(crate) fn new() -> Self {
        Self {
            cached_scores: Vec::new(),
            config_signature: 0,
            server_signature: 0,
            team_game: false,
            map: String::with_capacity(64),
            mode: String::with_capacity(32),
            rows: Vec::with_capacity(32),
            ui: MenuCanvas::with_capacities(TEXT_SLOTS, 96, DRAWS),
            deaths: deaths::Deaths::default(),
            hostname: String::with_capacity(64),
            max_clients: 0,
            gametype: 0,
            fraglimit: 0,
            limits: sjk::Limits::default(),
            style: style::ScoreboardStyle::default(),
            motion: motion::Motion::default(),
            icons: icons::HeadIcons::default(),
            killer: None,
            killer_name: String::with_capacity(64),
            identity_revision: u64::MAX,
            #[cfg(test)]
            shot: None,
        }
    }

    /// Whether the scoreboard draws this frame. The modern style shows while
    /// `requested`; the classic and SJK ones also fade in and, once released,
    /// out. Nothing draws while it is not `allowed`.
    pub(crate) fn present(
        &mut self,
        console: Option<&crate::console::ViewerConsole>,
        requested: bool,
        allowed: bool,
    ) -> bool {
        self.style = style::ScoreboardStyle::from_console(console);
        #[cfg(test)]
        if self.shot.is_some() {
            return self.motion.present(true, true, std::time::Instant::now());
        }
        match self.style {
            style::ScoreboardStyle::Modern => {
                self.motion.hide();
                requested && allowed
            }
            style::ScoreboardStyle::Classic | style::ScoreboardStyle::Sjk => {
                self.motion
                    .present(requested, allowed, std::time::Instant::now())
            }
        }
    }

    pub(crate) fn draw_list(&self) -> &DrawList {
        self.ui.draw_list()
    }

    #[allow(clippy::too_many_arguments)]
    fn append(
        &mut self,
        session: &ClientSession,
        fonts: &mut GameFonts,
        options: Option<&crate::console::ViewerConsole>,
        flags: classic::FlagIcons,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
        _scale: f32,
    ) {
        self.refresh(session);
        for row in &mut self.rows {
            row.deaths = self.deaths.count(row.client_num);
        }
        // Tags follow the hub's roster, not the frame: derived again only when it
        // or the rows changed.
        let revision = crate::player_identity::revision();
        if revision != self.identity_revision {
            self.identity_revision = revision;
            for row in &mut self.rows {
                row.identity = crate::player_identity::tag(row.client_num, &row.name);
            }
        }
        let snapshot = session.latest_snapshot();
        let player = &snapshot.player;
        let local = player.client_num();
        let local_status = classic::LocalStatus {
            client: local,
            team: player.persistent[PERS_TEAM] as u8,
            rank: player.persistent[PERS_RANK],
            score: player.persistent[PERS_SCORE] as i32,
            ready: player.stats[STAT_CLIENTS_READY],
            intermission: player.movement_type() == PM_INTERMISSION,
        };
        // Named when the kill was observed, shown while you are dead.
        let killer = (self.killer.is_some() && player.health() <= 0 && !player.is_spectator())
            .then_some(self.killer_name.as_str());
        if self.style == style::ScoreboardStyle::Sjk {
            let game = session.game_state();
            let header = sjk::SjkHeader {
                map: &self.map,
                gametype: self.gametype,
                limits: self.limits,
                elapsed: level_elapsed(game, snapshot.server_time),
                team_scores: session.team_scores(),
                local: local_status,
                killer,
                duel: sjk::Duelists {
                    local: Some((player.health(), player.armor())),
                    ..duelists(game)
                },
            };
            // Measured in the families that will draw the text.
            let measure = match fonts.sjk_metrics() {
                Some((display, body)) => sjk::Measure {
                    display: Some(display),
                    body: Some(body),
                },
                None => sjk::Measure {
                    display: Some(font),
                    body: Some(font),
                },
            };
            sjk::build(
                &mut self.ui,
                &self.rows,
                &header,
                flags,
                measure,
                &mut self.motion,
                viewport,
            );
            self.ui.finish(u16::MAX);
            match fonts.sjk() {
                Some(families) => {
                    self.ui
                        .append_text_families(families, viewport, TextStyle::NEUTRAL);
                }
                None => {
                    self.ui
                        .append_text_styled(vertices, font, viewport, TextStyle::NEUTRAL);
                }
            }
            return;
        }
        if self.style == style::ScoreboardStyle::Classic {
            let header = classic::ClassicHeader {
                hostname: &self.hostname,
                max_clients: self.max_clients,
                gametype: self.gametype,
                fraglimit: self.fraglimit,
                team_scores: session.team_scores(),
                local: local_status,
                killer,
            };
            classic::build(
                &mut self.ui,
                &self.rows,
                &header,
                style::ClassicOptions::from_console(options),
                &self.icons,
                flags,
                &mut self.motion,
                viewport,
            );
            self.ui.finish(u16::MAX);
            self.ui.append_text_routed(
                fonts,
                |_, text| Some(retail_font(text)),
                vertices,
                font,
                viewport,
            );
            return;
        }
        // The modern table lists scored clients only.
        let scored = self.rows.iter().take_while(|row| row.has_score).count();
        view::build(
            &mut self.ui,
            &self.rows[..scored],
            view::MatchHeader {
                map: &self.map,
                mode: &self.mode,
                team_scores: session.team_scores(),
                team_game: self.team_game,
                local_client: local,
            },
            viewport,
        );
        self.ui.finish(u16::MAX);
        self.ui.append_text_routed(
            fonts,
            |_, text| Some(retail_font(text)),
            vertices,
            font,
            viewport,
        );
    }

    fn refresh(&mut self, session: &ClientSession) {
        let signature = config_signature(session.game_state(), session.scores());
        let server_signature = session
            .game_state()
            .config_string(0)
            .map_or(0, byte_signature);
        if self.cached_scores == session.scores()
            && self.config_signature == signature
            && self.server_signature == server_signature
        {
            return;
        }
        self.cached_scores.clear();
        self.cached_scores.extend_from_slice(session.scores());
        self.config_signature = signature;
        self.server_signature = server_signature;
        self.team_game = is_team_game(session.game_state());
        self.rows.clear();
        self.identity_revision = u64::MAX;
        let game = session.game_state();
        for score in session.scores() {
            let (name, team) = client_identity(game, score.client_num);
            let extra = client_extra(game, score.client_num);
            self.rows.push(ScoreRow {
                client_num: score.client_num,
                name,
                team,
                score: score.score,
                deaths: None,
                ping: score.ping,
                time: score.time_minutes,
                has_score: true,
                bot: extra.bot,
                record: extra.record,
                powerups: score.powerups,
                defends: score.defends,
                assists: score.assists,
                captures: score.captures,
                identity: None,
            });
        }
        // Connected clients the scores do not list yet (`CG_TeamScoreboard`'s
        // "fake" rows); only the classic style shows them.
        for client in 0..32_u8 {
            let listed = session
                .scores()
                .iter()
                .any(|score| score.client_num == client);
            let connected = game
                .config_string(CS_PLAYERS + usize::from(client))
                .is_some_and(|bytes| !bytes.is_empty());
            if listed || !connected {
                continue;
            }
            let (name, team) = client_identity(game, client);
            let extra = client_extra(game, client);
            self.rows.push(ScoreRow {
                client_num: client,
                name,
                team,
                score: 0,
                deaths: None,
                ping: 0,
                time: 0,
                has_score: false,
                bot: extra.bot,
                record: extra.record,
                powerups: 0,
                defends: 0,
                assists: 0,
                captures: 0,
                identity: None,
            });
        }
        let server = server_info(game);
        self.map.clear();
        self.map.push_str(&server.0);
        self.mode.clear();
        self.mode.push_str(server.1);
        let info = game
            .config_string(0)
            .and_then(|v| std::str::from_utf8(v).ok())
            .and_then(|v| InfoString::parse(v).ok());
        self.hostname.clear();
        self.hostname.push_str(
            info.as_ref()
                .and_then(|info| info.get("sv_hostname"))
                .unwrap_or(""),
        );
        let integer = |key: &str| info.as_ref().and_then(|info| info.get_i32(key));
        self.max_clients = integer("sv_maxclients").unwrap_or(32);
        self.gametype = integer("g_gametype").unwrap_or(0);
        self.fraglimit = integer("fraglimit").unwrap_or(0);
        self.limits = sjk::Limits {
            frags: self.fraglimit,
            captures: integer("capturelimit").unwrap_or(0),
            duel_wins: integer("duel_fraglimit").unwrap_or(0),
            minutes: integer("timelimit").unwrap_or(0),
        };
    }
}

/// A config string as text.
fn config_text(game: &GameState, index: usize) -> Option<&str> {
    game.config_string(index)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
}

/// Milliseconds since the level started (`CS_LEVEL_START_TIME`), at
/// `server_time`, as the HUD's match timer counts them.
fn level_elapsed(game: &GameState, server_time: i32) -> Option<i32> {
    config_text(game, CS_LEVEL_START_TIME)
        .and_then(|start| start.trim().parse::<i32>().ok())
        .map(|start| server_time.saturating_sub(start).max(0))
}

/// Who duels (`CS_CLIENT_DUELISTS`, `-1` for nobody) and their health when the
/// server shares it (`CS_CLIENT_DUELHEALTHS`, `g_showDuelHealths`).
fn duelists(game: &GameState) -> sjk::Duelists {
    let mut duel = sjk::Duelists::default();
    if let Some(text) = config_text(game, CS_CLIENT_DUELISTS) {
        for (slot, value) in text.split('|').take(3).enumerate() {
            duel.clients[slot] = value
                .trim()
                .parse::<u8>()
                .ok()
                .filter(|client| *client < 32);
        }
    }
    if let Some(text) = config_text(game, CS_CLIENT_DUELHEALTHS) {
        for (slot, value) in text.split('|').take(2).enumerate() {
            duel.healths[slot] = value
                .trim()
                .parse::<i32>()
                .ok()
                .filter(|health| *health >= 0);
        }
    }
    duel
}

/// `persistant[]` and `stats[]` indices (`bg_public.h`) and `PM_INTERMISSION`.
const PERS_SCORE: usize = 0;
const PERS_RANK: usize = 2;
const PERS_TEAM: usize = 3;
const STAT_CLIENTS_READY: usize = 7;
const PM_INTERMISSION: u8 = 6;

impl Scoreboard {
    /// The resolved model icon of `client`'s slot.
    pub(crate) fn head_icon(&self, client: u8) -> Option<sjk_ui::TextureId> {
        self.icons.texture(client)
    }

    /// Resolve one client's model icon for the player card, with the scoreboard closed.
    pub(crate) fn ensure_head_icon(
        &mut self,
        game: &GameState,
        client: u8,
        vfs: &sjk_vfs::VirtualFileSystem,
        shaders: &sjk_shader::ShaderCatalog,
        renderer: &crate::ui_renderer::ShapeRenderer,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        self.icons.ensure(game, client, |path, texture| {
            let Some(pixels) = crate::hud::icons::assets::decode(vfs, shaders, path) else {
                return false;
            };
            renderer.upload_icon(queue, texture, pixels.as_raw());
            true
        });
    }
}

/// Append the current server scoreboard.
pub(crate) fn append_overlay(gpu: &mut crate::GpuState, viewport: [f32; 2], scale: f32) {
    #[cfg(test)]
    if gpu.scoreboard.shot.is_some() {
        shot::append(gpu, viewport);
        return;
    }
    let Some(session) = gpu.resident.session.as_ref().or(gpu.live_session.as_ref()) else {
        return;
    };
    let options = gpu.console.as_ref();
    let classic = gpu.scoreboard.style == style::ScoreboardStyle::Classic;
    if classic
        && style::ClassicOptions::from_console(options).icons
        && let Some(vfs) = &gpu.vfs
    {
        let (shaders, renderer, queue) = (&gpu.shaders, &gpu.ui_shapes, &gpu.queue);
        gpu.scoreboard
            .icons
            .update(session.game_state(), |path, texture| {
                let Some(pixels) = crate::hud::icons::assets::decode(vfs, shaders, path) else {
                    return false;
                };
                renderer.upload_icon(queue, texture, pixels.as_raw());
                true
            });
    }
    let flags = classic::FlagIcons {
        red: gpu.hud.icons.powerup(4),
        blue: gpu.hud.icons.powerup(5),
        neutral: gpu.hud.icons.powerup(6),
    };
    gpu.scoreboard.append(
        session,
        &mut gpu.game_fonts,
        options,
        flags,
        &mut gpu.text_vertices,
        &gpu.ui_font,
        viewport,
        scale,
    );
}

/// The font retail's scoreboard draws `text` with: `CG_DrawClientScore`
/// (`cg_scoreboard.c`) paints names and headings with `FONT_MEDIUM` and the
/// score, ping and time numbers with `FONT_SMALL`.
fn retail_font(text: &str) -> RetailFont {
    let numeric = !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'-' | b'/'));
    if numeric {
        RetailFont::Small
    } else {
        RetailFont::Medium
    }
}

fn server_info(game_state: &GameState) -> (String, &'static str) {
    let info = game_state
        .config_string(0)
        .and_then(|v| std::str::from_utf8(v).ok())
        .and_then(|v| InfoString::parse(v).ok());
    let Some(info) = info else {
        return ("UNKNOWN MAP".to_owned(), "MATCH");
    };
    let map = info.get("mapname").unwrap_or("UNKNOWN MAP").to_owned();
    let mode = match info.get_i32("g_gametype") {
        Some(0) => "FFA",
        Some(3) => "DUEL",
        Some(4) => "POWER DUEL",
        Some(6) => "TEAM FFA",
        Some(7) => "SIEGE",
        Some(8) => "CTF",
        Some(9) => "CTY",
        _ => "MATCH",
    };
    (map, mode)
}

fn client_identity(game_state: &GameState, client_num: u8) -> (String, u8) {
    game_state
        .config_string(CS_PLAYERS + usize::from(client_num))
        .map_or_else(
            || (format!("Client {client_num}"), 3),
            |bytes| client_identity_from_config(bytes, client_num),
        )
}
fn client_identity_from_config(bytes: &[u8], client_num: u8) -> (String, u8) {
    let info = sjk_client::LegacyClientInfo::new(bytes);
    (
        info.bytes("n")
            .or_else(|| info.bytes("name"))
            .filter(|n| !n.is_empty())
            .map_or_else(
                || format!("Client {client_num}"),
                // Latin-1, not UTF-8: see `sjk_client::decode_legacy`.
                |v| sjk_client::decode_legacy(v).into_owned(),
            ),
        info.integer("t")
            .and_then(|v| u8::try_from(v).ok())
            .unwrap_or(3),
    )
}

/// Clientinfo facts beyond name and team.
struct ClientExtra {
    bot: bool,
    record: Option<(i32, i32)>,
}

fn client_extra(game_state: &GameState, client_num: u8) -> ClientExtra {
    let info = game_state
        .config_string(CS_PLAYERS + usize::from(client_num))
        .map(sjk_client::LegacyClientInfo::new);
    let integer = |key: &str| info.as_ref().and_then(|info| info.integer(key));
    ClientExtra {
        bot: info
            .as_ref()
            .is_some_and(|info| info.bytes("skill").is_some()),
        record: integer("w").zip(integer("l")),
    }
}

fn is_team_game(game_state: &GameState) -> bool {
    game_state
        .config_string(0)
        .and_then(|v| std::str::from_utf8(v).ok())
        .and_then(|v| InfoString::parse(v).ok())
        .and_then(|v| v.get_i32("g_gametype"))
        .is_some_and(|v| v >= 6)
}
fn config_signature(game_state: &GameState, scores: &[ScoreEntry]) -> u64 {
    scores
        .iter()
        .fold(0xcbf29ce484222325_u64, |mut hash, score| {
            if let Some(bytes) =
                game_state.config_string(CS_PLAYERS + usize::from(score.client_num))
            {
                hash ^= byte_signature(bytes);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash
        })
}
fn byte_signature(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

/// Automatic intermission scores and the ordinary held scoreboard share one layout.
pub(crate) fn requested(gpu: &crate::GpuState, intermission: bool) -> bool {
    intermission || gpu.gameplay_input.held(crate::input::GameButton::Scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SJK look's clock and duelists come from config strings.
    #[test]
    fn the_level_clock_and_the_duelists_are_read_from_config_strings() {
        let mut game = GameState::empty_local(0);
        assert_eq!(level_elapsed(&game, 90_000), None);
        assert_eq!(duelists(&game), sjk::Duelists::default());
        let mut set = |index, value: &str| {
            game.replace_config_string(index, value.as_bytes().to_vec())
                .unwrap();
        };
        set(CS_LEVEL_START_TIME, "30000");
        set(CS_CLIENT_DUELISTS, "4|7");
        set(CS_CLIENT_DUELHEALTHS, "87|-1|!");
        assert_eq!(level_elapsed(&game, 90_000), Some(60_000));
        // A clock behind the level's start (a restart) reads zero.
        assert_eq!(level_elapsed(&game, 10_000), Some(0));
        let duel = duelists(&game);
        assert_eq!(duel.clients, [Some(4), Some(7), None]);
        assert_eq!(duel.healths, [Some(87), None]);
        // Nobody duelling.
        let mut game = GameState::empty_local(0);
        game.replace_config_string(CS_CLIENT_DUELISTS, b"-1|-1".to_vec())
            .unwrap();
        assert_eq!(duelists(&game).clients, [None; 3]);
    }

    #[test]
    fn numbers_use_the_small_font_and_names_the_medium_one() {
        for text in ["12", "-3", "7/2"] {
            assert_eq!(retail_font(text), RetailFont::Small);
        }
        for text in ["", "^1Padawan", "PING", "SCORE / DEATHS", "2fast"] {
            assert_eq!(retail_font(text), RetailFont::Medium);
        }
    }
}
