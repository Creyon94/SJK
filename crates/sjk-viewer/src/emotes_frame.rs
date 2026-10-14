//! The per-frame hook of emotes (`emotes.rs`): the emotes the SJK hub relayed for
//! this server start on the players they name, when the game shows the name their
//! claim was made under in that slot; emotes end when they are over or their player
//! moves. Costs two uncontended locks a frame when nothing comes.

use super::*;
use sjk_runtime::EntityId;

impl GpuState {
    /// Start the emotes received, end those that are over.
    pub(crate) fn update_emotes(&mut self) {
        let received = player_identity::take_emotes();
        if let Some(vfs) = self.vfs.as_deref() {
            let mut state = emotes::lock();
            if state.catalogue.is_none() {
                state.catalogue = Some(emotes::load(vfs));
            }
        }
        let Some(session) = self.live_session.as_ref() else {
            let mut state = emotes::lock();
            match self.demo_session.as_ref() {
                // A demo keeps the emotes started on it (a local one, a world shot's)
                // until they end or their player moves; none arrive from the hub.
                Some(demo) => {
                    let world = demo.world();
                    state.active.update(Instant::now(), |slot: u8| {
                        world
                            .entity(EntityId::new(u64::from(slot) + 1))
                            .map(|entity| entity.current().transform.translation)
                    });
                }
                None => state.active = emotes::ActiveEmotes::default(),
            }
            return;
        };
        let world = &self.live_world;
        let position = |slot: u8| {
            world
                .entity(EntityId::new(u64::from(slot) + 1))
                .map(|entity| entity.current().transform.translation)
        };
        let now = Instant::now();
        let own = u8::try_from(session.latest_snapshot().player.client_num()).ok();
        let mut state = emotes::lock();
        for emote in received {
            // The hub's id is shown in the console as it came when no catalogue entry
            // names it: only an id that could name an emote may.
            if !emotes::valid_id(&emote.emote) {
                continue;
            }
            // The hub's echo of the player's own emote, already started by `sjkemote`.
            if own == Some(emote.slot)
                && state
                    .active
                    .get(emote.slot)
                    .is_some_and(|active| active.emote == emote.emote)
            {
                continue;
            }
            let shown = player_identity::shown_name(session.game_state(), usize::from(emote.slot));
            if !emotes::trusted(&emote, shown.as_deref()) {
                continue;
            }
            let def = state
                .catalogue
                .as_ref()
                .and_then(|catalogue| catalogue.iter().find(|def| def.id == emote.emote))
                .cloned();
            let origin = position(emote.slot).unwrap_or_default();
            state
                .active
                .start(emote.slot, &emote.emote, def.as_ref(), origin, now);
            // Until the animations exist, the console says it.
            if let Some(console) = &mut self.console {
                let name = def
                    .as_ref()
                    .map_or(emote.emote.as_str(), |def| def.name.as_str());
                console.push_log(format!("* {}^7: {name}", shown.unwrap_or_default()));
            }
        }
        state.active.update(now, position);
    }
}
