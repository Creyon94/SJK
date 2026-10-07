//! Timed, presentation-only player monitoring, from JoF EJKSol's CG_Peek_f
//! and CG_CalcViewValues. Only players in the received snapshot are available.
use crate::{GpuState, movement_collision};
use glam::Vec3;
use sjk_protocol::GameState;
use sjk_runtime::EntityId;

const CS_PLAYERS: usize = 1131;
#[derive(Default)]
pub(crate) struct State {
    target: Option<Target>,
}
struct Target {
    client: u16,
    name: Vec<u8>,
    map: Vec<u8>,
    start: i64,
    end: i64,
}

fn name(game: &GameState, client: u16) -> Option<&[u8]> {
    sjk_client::LegacyClientInfo::new(game.config_string(CS_PLAYERS + usize::from(client))?)
        .bytes("n")
}
fn map(game: &GameState) -> &[u8] {
    game.config_string(0)
        .and_then(|info| sjk_protocol::info_value(info, b"mapname"))
        .unwrap_or_default()
}
fn clean(bytes: &[u8]) -> String {
    let bytes = sjk_game_jka::client_view::strip_colours(bytes);
    String::from_utf8(bytes.clone())
        .unwrap_or_else(|_| {
            bytes
                .into_iter()
                .map(sjk_protocol::windows_1252_char)
                .collect()
        })
        .to_lowercase()
}

/// Slot, exact colour-insensitive name, then an unambiguous name fragment.
fn resolve<'a>(players: impl Iterator<Item = (u16, &'a [u8])>, query: &str) -> Result<u16, String> {
    let players: Vec<_> = players.collect();
    if let Ok(slot) = query.parse::<u16>()
        && players.iter().any(|(client, _)| *client == slot)
    {
        return Ok(slot);
    }
    let query = clean(query.as_bytes());
    let exact: Vec<_> = players
        .iter()
        .filter(|(_, name)| clean(name) == query)
        .collect();
    let matches: Vec<_> = if exact.is_empty() {
        players
            .iter()
            .filter(|(_, name)| clean(name).contains(&query))
            .collect()
    } else {
        exact
    };
    match matches.as_slice() {
        [player] => Ok(player.0),
        [] => Err("peek: no matching player".into()),
        _ => Err("peek: name is ambiguous; use a client ID".into()),
    }
}
fn duration(text: Option<&str>) -> Result<i64, String> {
    let seconds = text
        .unwrap_or("5")
        .parse::<f64>()
        .map_err(|_| "peek: invalid duration")?;
    if !seconds.is_finite() {
        return Err("peek: duration must be finite".into());
    }
    Ok((seconds.clamp(0., 60.) * 1000.) as i64)
}

impl GpuState {
    /// `/peek [id|name] [seconds]`, `/peek off`, or the crosshair player without arguments.
    pub(crate) fn peek_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        if args.len() > 2 {
            return Err("Usage: peek [id|name] [seconds], or peek off".into());
        }
        if args
            .first()
            .is_some_and(|arg| arg.eq_ignore_ascii_case("off"))
        {
            self.peek.target = None;
            return Ok(vec!["peek: returned to your camera".into()]);
        }
        let milliseconds = duration(args.get(1).map(String::as_str))?;
        if milliseconds == 0 {
            self.peek.target = None;
            return Ok(vec!["peek: returned to your camera".into()]);
        }
        if self.local_prediction.fake_noclip() {
            return Err("peek: end local flight before watching a player".into());
        }
        let session = self.live_session.as_ref().ok_or("peek: not in a game")?;
        let game = session.game_state();
        let snapshot = session.latest_snapshot();
        if snapshot.player.movement_type() == 7 {
            return Err("peek: unavailable during intermission".into());
        }
        let client = if let Some(query) = args.first() {
            resolve(
                (0..64).filter_map(|client| Some((client, name(game, client)?))),
                query,
            )?
        } else {
            self.crosshair_scan
                .chat_client(snapshot.server_time)
                .ok_or("peek: aim at a player, or use peek <id|name> [seconds]")?
        };
        if client == snapshot.player.client_num() {
            return Err("peek: select another player".into());
        }
        if !snapshot
            .entities
            .iter()
            .any(|entity| entity.number() == client && entity.entity_type() == 1)
        {
            return Err(
                "peek: player is outside your received view; no camera data available".into(),
            );
        }
        let target_name = name(game, client).ok_or("peek: player left")?;
        self.peek.target = Some(Target {
            client,
            name: target_name.to_vec(),
            map: map(game).to_vec(),
            start: i64::from(snapshot.server_time),
            end: i64::from(snapshot.server_time) + milliseconds,
        });
        Ok(vec![format!(
            "peek: watching client {client} for {:.1}s; /peek off returns",
            milliseconds as f64 / 1000.
        )])
    }
}

/// Derive the rendered view without modifying player origin, angles or commands.
pub(crate) fn camera(state: &mut GpuState, time: i64) -> Option<(Vec3, Vec3)> {
    if state.local_prediction.fake_noclip() {
        state.peek.target = None;
        return None;
    }
    let Some(session) = state.live_session.as_ref() else {
        state.peek.target = None;
        return None;
    };
    let target = state.peek.target.as_ref()?;
    let game = session.game_state();
    let snapshot = session.snapshot_at_or_before(time as i32);
    let valid = time >= target.start - 1000
        && time < target.end
        && map(game) == target.map
        && name(game, target.client) == Some(target.name.as_slice())
        && snapshot.player.movement_type() != 7
        && time - i64::from(snapshot.server_time) < 1000
        && snapshot
            .entities
            .iter()
            .any(|entity| entity.number() == target.client && entity.entity_type() == 1);
    if !valid {
        state.peek.target = None;
        return None;
    }
    let entity = state
        .live_world
        .entity(EntityId::new(u64::from(target.client) + 1))?;
    let transform = entity.sample(time);
    let yaw = entity
        .sample_pose(time)
        .map_or(0., |pose| pose.view_angles_degrees[1])
        .to_radians();
    let focus = Vec3::from_array(transform.translation) + Vec3::Z * 26.;
    let wanted = focus - Vec3::new(yaw.cos(), yaw.sin(), 0.) * 80. + Vec3::Z * 32.;
    let position = movement_collision::camera_trace(
        &state.bsp,
        &mut state.trace_scratch,
        Some((game, snapshot)),
        time as i32,
        focus,
        wanted,
    );
    // A fully collapsed trace still has a defined look direction.
    let look = if position.distance_squared(focus) < 0.001 {
        focus + Vec3::new(yaw.cos(), yaw.sin(), 0.)
    } else {
        focus
    };
    Some((position, look))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolve_names_and_reject_ambiguous_fragments() {
        let players = [(1, b"^1Tox".as_slice()), (2, b"Toxiee".as_slice())];
        assert_eq!(resolve(players.into_iter(), "1").unwrap(), 1);
        assert_eq!(resolve(players.into_iter(), "tox").unwrap(), 1);
        assert!(resolve(players.into_iter(), "to").is_err());
        assert_eq!(resolve(players.into_iter(), "xiee").unwrap(), 2);
    }
    #[test]
    fn durations_are_bounded_and_nonfinite_values_rejected() {
        assert_eq!(duration(None).unwrap(), 5000);
        assert_eq!(duration(Some("90")).unwrap(), 60000);
        assert_eq!(duration(Some("0")).unwrap(), 0);
        assert!(duration(Some("NaN")).is_err());
    }
}
