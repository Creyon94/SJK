//! The viewer's side of player identity (`docs/identity.md`): turns the
//! `cl_identity` and `cl_hubUrl` settings and the live session into the
//! [`sjk_identity`] service's inputs, and gives the scoreboard and the Identity
//! page what the service learned.
//!
//! The service runs on its own thread; this module only compares what it last
//! told it with the current settings and place twice a second, so a frame never
//! waits for the hub. With the feature off, or no hub address, it sends
//! nothing. The key file is created the first time the feature is on.

use sjk_client::{LegacyClientInfo, decode_legacy};
use sjk_identity::{HttpHub, Hub, Identity, Location, Service, Settings, Snapshot};
use sjk_protocol::GameState;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// The hub address a fresh install uses. Empty until SJK has a hub to point at:
/// the identity key is made and kept locally, and nothing is sent anywhere.
pub(crate) const DEFAULT_HUB_URL: &str = "";
/// The key file, in the settings folder beside `config.cfg`.
const KEY_FILE: &str = "identity.key";
/// How often the settings and the player's place are compared with what the
/// service was told.
const SYNC_EVERY: Duration = Duration::from_millis(500);
/// Longest the exit waits for the service to withdraw the player's claim.
const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);
/// `CS_PLAYERS`: the first player's configstring.
const CS_PLAYERS: usize = 1131;
/// Clients the legacy protocol numbers (0 to 31).
const MAX_CLIENTS: usize = 32;

/// How a player appears on the scoreboard.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Tag {
    /// The hub's operator vouches for this player's key.
    pub(crate) verified: bool,
}

#[derive(Default)]
struct Runtime {
    service: Option<Service>,
    /// Why the key file could not be used; the feature stays off until restart.
    key_error: Option<String>,
    sent_settings: Option<Settings>,
    sent_location: Option<Location>,
    next_sync: Option<Instant>,
}

static RUNTIME: Mutex<Runtime> = Mutex::new(Runtime {
    service: None,
    key_error: None,
    sent_settings: None,
    sent_location: None,
    next_sync: None,
});

fn lock() -> MutexGuard<'static, Runtime> {
    RUNTIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn make_hub(url: &str) -> Result<Box<dyn Hub>, sjk_identity::HubError> {
    let agent = format!("SJK/{}", crate::build_info::VERSION);
    HttpHub::new(url, &agent).map(|hub| Box::new(hub) as Box<dyn Hub>)
}

/// The name the game shows for `slot`, as the server published it.
pub(crate) fn shown_name(game_state: &GameState, slot: usize) -> Option<String> {
    let bytes = game_state.config_string(CS_PLAYERS + slot)?;
    let info = LegacyClientInfo::new(bytes);
    let name = info.bytes("n").or_else(|| info.bytes("name"))?;
    (!name.is_empty()).then(|| decode_legacy(name).into_owned())
}

/// Where the player is: the server they are connected to, their slot there and
/// the name the game shows for them. `None` in a local game or without a name.
pub(crate) fn location(
    server: SocketAddr,
    local: bool,
    game_state: &GameState,
) -> Option<Location> {
    let slot = usize::try_from(game_state.client_num)
        .ok()
        .filter(|slot| *slot < MAX_CLIENTS)?;
    if local {
        return None;
    }
    Some(Location {
        server,
        slot: u8::try_from(slot).ok()?,
        name: shown_name(game_state, slot)?,
    })
}

/// Whether it is time to [`apply`] the settings and the player's place again.
/// True twice a second at most, so the caller builds them only then.
pub(crate) fn due() -> bool {
    let mut runtime = lock();
    let now = Instant::now();
    if runtime.next_sync.is_some_and(|due| now < due) {
        return false;
    }
    runtime.next_sync = Some(now + SYNC_EVERY);
    true
}

/// Bring the service in line with the settings and the player's place.
pub(crate) fn apply(config_directory: &Path, settings: Settings, location: Option<Location>) {
    let mut runtime = lock();
    if runtime.service.is_none() {
        if !settings.enabled || runtime.key_error.is_some() {
            return;
        }
        match Identity::load_or_create(&config_directory.join(KEY_FILE)) {
            Ok(identity) => {
                crate::log::progress(format_args!(
                    "identity: key {} ({})",
                    identity.key_id(),
                    KEY_FILE
                ));
                runtime.service = Some(Service::start(identity, Box::new(make_hub)));
            }
            Err(error) => {
                crate::log::progress(format_args!("identity: {error}"));
                runtime.key_error = Some(error.to_string());
                return;
            }
        }
    }
    let runtime = &mut *runtime;
    let Some(service) = runtime.service.as_ref() else {
        return;
    };
    if runtime.sent_settings.as_ref() != Some(&settings) {
        service.configure(settings.clone());
        runtime.sent_settings = Some(settings);
    }
    if runtime.sent_location != location {
        match &location {
            Some(location) => service.enter(location.clone()),
            None => service.leave(),
        }
        runtime.sent_location = location;
    }
}

/// The service's state, or `None` when it has not started (the feature has never
/// been on, or the key file is unusable: see [`key_error`]).
pub(crate) fn snapshot() -> Option<Snapshot> {
    lock().service.as_ref().map(Service::snapshot)
}

/// Why the key file could not be used, if that is why there is no identity.
pub(crate) fn key_error() -> Option<String> {
    lock().key_error.clone()
}

/// Counts changes to the known players, so tags derived from them can be kept.
pub(crate) fn revision() -> u64 {
    lock().service.as_ref().map_or(0, |service| {
        service.with_snapshot(|snapshot| snapshot.revision)
    })
}

/// The tag for the player in `slot` whom the game shows as `shown`, if the hub
/// knows them under that name.
pub(crate) fn tag(slot: u8, shown: &str) -> Option<Tag> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        snapshot.badge(slot, shown).map(|player| Tag {
            verified: player.verified,
        })
    })
}

/// What the hub knows about the player in `slot` whom the game shows as `shown`.
pub(crate) fn hub_info(slot: u8, shown: &str) -> Option<crate::hud::player_card::HubInfo> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        snapshot
            .badge(slot, shown)
            .map(|player| crate::hud::player_card::HubInfo {
                name: player.name.clone(),
                verified: player.verified,
            })
    })
}

/// Ask the hub to change the player's display name and bio. The result shows in
/// [`Snapshot::notice`].
pub(crate) fn set_profile(name: String, bio: String) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.set_profile(name, bio))
        .is_some()
}

/// Ask the hub for another player's profile (their bio).
pub(crate) fn look_up(key_id: &str) {
    if let Some(service) = lock().service.as_ref() {
        service.look_up(key_id.to_owned());
    }
}

/// Withdraw the player's claim on the way out.
pub(crate) fn shutdown() {
    // Not under the lock: the wait can last as long as the hub takes to answer.
    let service = lock().service.take();
    if let Some(service) = service {
        service.shutdown(SHUTDOWN_WAIT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tag_is_none_before_the_service_starts() {
        assert_eq!(tag(3, "Sol"), None);
        assert_eq!(revision(), 0);
        assert!(snapshot().is_none());
    }
}
