//! Map-music ownership and level-clock starts, after JoF EJK #103/#146/#161.
use super::*;
pub(crate) const CVAR: &str = "cg_musicSync";

/// A server's empty CS_MUSIC is authoritative silence, not a worldspawn fallback.
pub(super) fn select_music(
    game: Option<&GameState>,
    fallback: impl FnOnce() -> Option<assets::MusicSpec>,
) -> Option<assets::MusicSpec> {
    match game {
        Some(game) => assets::game_music(game),
        None => fallback(),
    }
}

fn elapsed_ms(snapshot: &Snapshot, game: &GameState) -> u64 {
    // CS_LEVEL_START_TIME, codemp/game/bg_public.h; already decoded, no wire changes.
    let start = game
        .config_string(21)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|value| value.trim().parse::<i32>().ok())
        .unwrap_or(0);
    (i64::from(snapshot.server_time) - i64::from(start)).max(0) as u64
}

impl GameAudio {
    pub(super) fn restore_map_music(&mut self, snapshot: &Snapshot, game: &GameState) {
        if let Some((intro, repeating)) = self.map_music {
            if self.music_sync && self.dynamic.is_none() {
                self.output
                    .start_music_at(intro, repeating, elapsed_ms(snapshot, game));
            } else {
                self.output.start_music(intro, repeating);
            }
        } else {
            self.output.stop_music();
        }
    }

    pub(super) fn apply_music_action(
        &mut self,
        action: Option<LegacyMusicAction>,
        snapshot: &Snapshot,
        game: &GameState,
    ) {
        match action {
            Some(LegacyMusicAction::StartPrivateDuel) if self.duel_enabled => {
                if let Some(handle) = self.duel_music {
                    self.output.start_music(handle, handle);
                }
            }
            Some(LegacyMusicAction::RestoreMap) => self.restore_map_music(snapshot, game),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_protocol::PlayerState;
    #[test]
    fn music_selection_honors_server_silence_and_offline_worldspawn() {
        let mut game = GameState::empty_local(0);
        assert!(
            select_music(Some(&game), || panic!(
                "server silence must not use worldspawn"
            ))
            .is_none()
        );
        game.replace_config_string(2, b"music/intro music/loop".to_vec())
            .unwrap();
        let selected = select_music(Some(&game), || panic!("server owns the track")).unwrap();
        assert_eq!(selected.intro, "music/intro");
        assert_eq!(selected.repeating, "music/loop");
        assert!(select_music(None, || Some(selected)).is_some());
    }
    #[test]
    fn level_music_clock_clamps_future_start_and_preserves_long_elapsed_time() {
        let mut game = GameState::empty_local(0);
        let mut snapshot = Snapshot {
            message_sequence: 1,
            reliable_acknowledge: 0,
            server_commands: vec![],
            server_time: 10000,
            delta_from: None,
            flags: 0,
            area_mask: vec![],
            player: PlayerState::zero(),
            vehicle_player: None,
            entities: vec![],
            consumed_bits: 0,
        };
        game.replace_config_string(21, b"2000".to_vec()).unwrap();
        assert_eq!(elapsed_ms(&snapshot, &game), 8000);
        snapshot.server_time = 1000;
        assert_eq!(elapsed_ms(&snapshot, &game), 0);
        snapshot.server_time = i32::MAX;
        game.replace_config_string(21, i32::MIN.to_string().into_bytes())
            .unwrap();
        assert_eq!(elapsed_ms(&snapshot, &game), u64::from(u32::MAX));
    }
}

#[cfg(test)]
mod restoration_tests {
    use super::*;
    use sjk_protocol::PlayerState;
    #[test]
    fn ending_a_duel_on_a_musicless_map_stops_the_current_track() {
        let (output, mut source) = AudioOutput::test_output();
        let mut audio = GameAudio::with_output(output);
        // One second of constant synthetic PCM, so silence means stopped, not EOF.
        let samples = 44100_u32;
        let size = samples * 2;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + size).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&44100_u32.to_le_bytes());
        wav.extend_from_slice(&88200_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&size.to_le_bytes());
        for _ in 0..samples {
            wav.extend_from_slice(&16384_i16.to_le_bytes());
        }
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory("synthetic", [("music/fixture.wav", wav)])
            .unwrap();
        audio.duel_music = audio.register_vfs_async(&vfs, "music/fixture.wav");
        assert!(audio.duel_music.is_some());
        assert!(audio.map_music.is_none());
        let snapshot = Snapshot {
            message_sequence: 1,
            reliable_acknowledge: 0,
            server_commands: vec![],
            server_time: 1000,
            delta_from: None,
            flags: 0,
            area_mask: vec![],
            player: PlayerState::zero(),
            vehicle_player: None,
            entities: vec![],
            consumed_bits: 0,
        };
        let game = GameState::empty_local(0);
        audio.apply_music_action(Some(LegacyMusicAction::StartPrivateDuel), &snapshot, &game);
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if source.next().unwrap().abs() > 0.001 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "synthetic duel track did not start"
            );
            std::thread::yield_now();
        }
        audio.apply_music_action(Some(LegacyMusicAction::RestoreMap), &snapshot, &game);
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if source.next().unwrap().abs() < f32::EPSILON {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "duel track continued on a musicless map"
            );
            std::thread::yield_now();
        }
    }
}
