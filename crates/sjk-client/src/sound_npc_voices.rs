//! NPC custom sounds (`*pain25`, `*death1`, ...): `CG_CustomSound` for an entity
//! number at or above `MAX_CLIENTS` reads the NPC's own `npcClient` sets
//! (`cg_players.c:213-218`), never a player's.
//!
//! The server names each NPC sound directory once in `CS_SOUNDS` as `*$<dir>` and
//! points the NPC's `csSounds_Std`/`_Combat`/`_Extra`/`_Jedi` at it
//! (`NPC_stats.c:757-814`); `CG_HandleNPCSounds` (`cg_servercmds.c:718-850`) then
//! registers `sound/chars/<dir>/misc/<name>` for each name of that kind's table.
//! A kind whose index is zero has no sounds, and a missing file stays silent:
//! unlike a player (`CG_LoadCISounds`), an NPC has no `mp_generic` fallback.
//!
//! Sets are loaded when their `CS_SOUNDS` slot changes, one per slot, so an event
//! only copies the three it needs. The `Extra` kind (`*chase1`, `*cover1`, ...)
//! has no event in this adapter's table and is not loaded.

use super::*;

/// Every custom name the adapter plays, loaded from one NPC sound directory.
/// Which fields a voice takes from it depends on the kind that names it
/// ([`npc_voice`]).
pub(super) fn npc_custom_set(
    vfs: &VirtualFileSystem,
    directory: &str,
    intern: &mut impl FnMut(&str) -> u16,
) -> CustomSet {
    let mut sound = |name: &str| {
        let path = format!("sound/chars/{directory}/misc/{name}.wav");
        sound_exists(vfs, &path).then(|| intern(&path))
    };
    CustomSet {
        pushed: ["pushed1", "pushed2", "pushed3"].map(&mut sound),
        choke: ["choke1", "choke2", "choke3"].map(&mut sound),
        push_fail: sound("pushfail"),
        jump: sound("jump1"),
        pain: ["pain25", "pain50", "pain75", "pain100"].map(&mut sound),
        death: ["death1", "death2", "death3"].map(&mut sound),
        land: sound("land1"),
        gasp: sound("gasp"),
        taunt: sound("taunt"),
        anger: ["anger1", "anger2", "anger3"].map(&mut sound),
        victory: ["victory1", "victory2", "victory3"].map(&mut sound),
        taunt_numbered: ["taunt1", "taunt2", "taunt3"].map(&mut sound),
        deflect: ["deflect1", "deflect2", "deflect3"].map(&mut sound),
        gloat: ["gloat1", "gloat2", "gloat3"].map(&mut sound),
    }
}

/// The NPC sound directory a `CS_SOUNDS` string names: `*$<dir>`.
pub(super) fn npc_sound_directory(path: &str) -> Option<&str> {
    path.strip_prefix("*$")
        .filter(|directory| !directory.is_empty())
}

/// One NPC's voice from its `Std`, `Combat` and `Jedi` sets, as `CG_CustomSound`
/// matches the names: `cg_customSoundNames` first (death, jump, pain, choke, gasp,
/// land, taunt), then `cg_customCombatSoundNames` (anger, victory, pushed), then
/// `cg_customJediSoundNames` (taunt1-3, deflect, gloat, pushfail)
/// (`cg_players.c:45-160`, `:300-322`).
pub(super) fn npc_voice(std: CustomSet, combat: CustomSet, jedi: CustomSet) -> CustomSet {
    CustomSet {
        pushed: combat.pushed,
        choke: std.choke,
        push_fail: jedi.push_fail,
        jump: std.jump,
        pain: std.pain,
        death: std.death,
        land: std.land,
        gasp: std.gasp,
        taunt: std.taunt,
        anger: combat.anger,
        victory: combat.victory,
        taunt_numbered: jedi.taunt_numbered,
        deflect: jedi.deflect,
        gloat: jedi.gloat,
    }
}

impl LegacySoundAdapter {
    /// The set an NPC's `csSounds_*` index names; empty for zero or an unloaded slot.
    fn npc_set(&self, index: u8) -> CustomSet {
        if index == 0 {
            return CustomSet::default();
        }
        self.npc_sets
            .get(usize::from(index))
            .copied()
            .flatten()
            .unwrap_or_default()
    }

    /// The custom sounds of the entity an event voices, `CG_CustomSound`'s `ci`.
    ///
    /// An `ET_NPC` event (`CG_EntityEvent` sets `clientNum = es->number` for one,
    /// `cg_event.c:1855-1857`) and a temporary entity naming an NPC in `clientNum`
    /// (`G_EntitySound`) take that NPC's own sets; an owner at or above
    /// `MAX_CLIENTS` that is no NPC in the snapshot has none (`npcClient` is
    /// `NULL`, `cg_players.c:220-223`). Every other event keeps its player's set.
    pub(super) fn voice_of(&self, entity: &EventSubject, snapshot: &Snapshot) -> CustomSet {
        if let Some([std, combat, _, jedi]) = entity.npc_sounds {
            return npc_voice(self.npc_set(std), self.npc_set(combat), self.npc_set(jedi));
        }
        let client = usize::from(entity.client_num);
        if client < MAX_CLIENTS {
            return self.custom[client];
        }
        snapshot
            .entities
            .iter()
            .find(|state| {
                state.number() == entity.client_num
                    && state.entity_type() == crate::npc_identity::ET_NPC
            })
            .map_or_else(CustomSet::default, |npc| {
                let [std, combat, _, jedi] = npc.npc_sound_sets();
                npc_voice(self.npc_set(std), self.npc_set(combat), self.npc_set(jedi))
            })
    }
}

#[cfg(test)]
#[path = "sound_npc_voices_tests.rs"]
mod tests;
