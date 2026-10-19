//! Test packs (`docs/identity.md`, "Test packs"): PK3s the SJK team sends sealed to the
//! players it chooses. The identity service keeps each one's locked body beside
//! `identity.key` and, once the hub's list is read, hands over the packs this key may
//! open, their keys in memory only ([`follow_identity`]). Each world's game file system
//! mounts them above the game data ([`mount_on_top`]), read through a reader that
//! decrypts as the game reads, so the plain PK3 is never on disk. A pack that arrives or
//! goes mid-game takes effect at the next map load; the emote catalogue is read again
//! then, so a test pack's emotes appear.

use sjk_identity::service::TestPacks;
use sjk_identity::test_pack_cache::Opened;
use sjk_vfs::VirtualFileSystem;
use std::sync::Mutex;

struct State {
    /// The packs ready, as the service last handed them over.
    ready: Vec<Opened>,
    revision: u64,
    note: Option<String>,
    /// The ids the last world mounted, to see when the set changes.
    mounted: Vec<String>,
}

static STATE: Mutex<State> = Mutex::new(State {
    ready: Vec::new(),
    revision: 0,
    note: None,
    mounted: Vec::new(),
});

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Take the service's test packs when they changed, and log what its last check did
/// when that changes. Called twice a second.
pub(crate) fn follow_identity(packs: TestPacks) {
    let mut state = lock();
    if packs.note.is_some() && packs.note != state.note {
        if let Some(note) = &packs.note {
            crate::log::progress(format_args!("{note}"));
        }
        state.note = packs.note;
    }
    if packs.revision != state.revision {
        state.revision = packs.revision;
        for pack in &packs.packs {
            if !state.ready.iter().any(|ready| ready.id == pack.id) {
                crate::log::progress(format_args!(
                    "test pack {} ready: mounted from the next map load",
                    pack.file
                ));
            }
        }
        state.ready = packs.packs;
    }
}

/// Mount the test packs ready into a world's file system being built, above everything
/// mounted so far. A pack that does not open (its body damaged) is named in the log and
/// left out.
pub(crate) fn mount_on_top(vfs: &mut VirtualFileSystem) {
    let mut state = lock();
    let mut mounted = Vec::new();
    for pack in &state.ready {
        let opened = pack
            .reader()
            .map_err(|error| error.to_string())
            .and_then(|reader| {
                vfs.mount_pk3_reader(format!("test pack {}", pack.file), reader)
                    .map_err(|error| error.to_string())
            });
        match opened {
            Ok(_) => mounted.push(pack.id.clone()),
            Err(why) => crate::log::progress(format_args!(
                "warning: skipping test pack {}: {why}",
                pack.file
            )),
        }
    }
    if mounted != state.mounted {
        for pack in state.ready.iter().filter(|pack| mounted.contains(&pack.id)) {
            crate::log::progress(format_args!("test pack {} mounted", pack.file));
        }
        state.mounted = mounted;
        // The emotes are read from the game data once; read them again from this world's.
        crate::emotes::lock().catalogue = None;
    }
}
