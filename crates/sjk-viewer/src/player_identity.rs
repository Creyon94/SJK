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

/// The hub address a fresh install uses, so players set nothing. With
/// `cl_identity` off, or `cl_hubUrl` emptied, nothing is sent anywhere.
pub(crate) const DEFAULT_HUB_URL: &str = "https://sjk.dfox.app";
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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Tag {
    /// The hub's operator vouches for this player's key.
    pub(crate) verified: bool,
    /// The medals the SJK team gave them, for the ribbon bars.
    pub(crate) medals: crate::medals::Medals,
}

#[derive(Default)]
struct Runtime {
    service: Option<Service>,
    /// Why the key file could not be used; the feature stays off until restart.
    key_error: Option<String>,
    sent_settings: Option<Settings>,
    sent_location: Option<Location>,
    sent_name: Option<String>,
    next_sync: Option<Instant>,
}

static RUNTIME: Mutex<Runtime> = Mutex::new(Runtime {
    service: None,
    key_error: None,
    sent_settings: None,
    sent_location: None,
    sent_name: None,
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

/// Bring the service in line with the settings, the in-game `name` the player wears
/// (the `name` setting, which the hub keeps in the key's name history: the player
/// chooses nothing) and the player's place.
pub(crate) fn apply(
    config_directory: &Path,
    settings: Settings,
    name: String,
    location: Option<Location>,
) {
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
    if runtime.sent_name.as_ref() != Some(&name) {
        service.set_name(name.clone());
        runtime.sent_name = Some(name);
    }
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

/// Send a bug report through the service; false when the service has not started.
pub(crate) fn report(report: sjk_identity::BugReport) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.report(report);
    true
}

/// Send a world note through the service: its tag for [`note_image`], or `None` when
/// the service has not started.
pub(crate) fn note(note: sjk_identity::WorldNote) -> Option<u64> {
    lock().service.as_ref().map(|service| service.note(note))
}

/// The picture of the note tagged `tag`.
pub(crate) fn note_image(tag: u64, jpeg: Vec<u8>) {
    if let Some(service) = lock().service.as_ref() {
        service.note_image(tag, jpeg);
    }
}

/// The outcome of the last world note, once the service has one.
pub(crate) fn note_outcome() -> Option<sjk_identity::ReportOutcome> {
    lock()
        .service
        .as_ref()
        .and_then(|service| service.with_snapshot(|snapshot| snapshot.note.clone()))
}

/// The outcome of the last bug report, once the service has one.
pub(crate) fn report_outcome() -> Option<sjk_identity::ReportOutcome> {
    lock()
        .service
        .as_ref()
        .and_then(|service| service.with_snapshot(|snapshot| snapshot.report.clone()))
}

/// Send a report about another player through the service; false when the service
/// has not started.
pub(crate) fn player_report(report: sjk_identity::PlayerReport) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.player_report(report);
    true
}

/// The outcome of the last player report, once the service has one.
pub(crate) fn player_report_outcome() -> Option<sjk_identity::ReportOutcome> {
    lock()
        .service
        .as_ref()
        .and_then(|service| service.with_snapshot(|snapshot| snapshot.player_report.clone()))
}

/// Whether the player may report other players: only with the identity on, the hub
/// answering and their key verified by the SJK team.
pub(crate) fn report_gate() -> crate::ingame_menu::players::Gate {
    use crate::ingame_menu::players::Gate;
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return Gate::NoIdentity;
    };
    service.with_snapshot(|snapshot| match (&snapshot.status, &snapshot.me) {
        (sjk_identity::Status::Disabled | sjk_identity::Status::NoHub, _) => Gate::NoIdentity,
        (_, None) => Gate::Offline,
        (_, Some(me)) if me.verified => Gate::Open,
        (_, Some(_)) => Gate::NotVerified,
    })
}

/// What the hub knows of the player in `slot` whom the game shows as `shown`: their
/// key, hub name and verified flag, from a live claim under that name.
pub(crate) fn hub_mark(slot: u8, shown: &str) -> Option<crate::ingame_menu::players::HubMark> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        snapshot
            .badge(slot, shown)
            .map(|player| crate::ingame_menu::players::HubMark {
                key_id: player.key_id.clone(),
                name: player.name.clone(),
                verified: player.verified,
                medals: crate::medals::Medals::from_wire(&player.medals),
            })
    })
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
            medals: crate::medals::Medals::from_wire(&player.medals),
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
                medals: crate::medals::Medals::from_wire(&player.medals),
            })
    })
}

/// The player's own key id and the medals their profile lists, once the hub has
/// answered; `None` before that or with the feature off. Read twice a second at most,
/// for the new medal pop-up.
pub(crate) fn own_medals() -> Option<(String, Vec<sjk_identity::Medal>)> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        let me = snapshot.me.as_ref()?;
        if matches!(
            snapshot.status,
            sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
        ) {
            return None;
        }
        Some((snapshot.key_id.clone(), me.medals.clone()))
    })
}

/// The slots of the players the hub's operator vouches for on this server, as bits:
/// each one whose claim names the name the game shows there, and the local player's
/// own slot when its key is verified. For the nameplates' verified badge.
pub(crate) fn verified_slots(game_state: &GameState) -> u32 {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return 0;
    };
    service.with_snapshot(|snapshot| {
        let mut slots = snapshot
            .players
            .iter()
            .filter(|player| player.verified && usize::from(player.slot) < MAX_CLIENTS)
            .filter(|player| {
                shown_name(game_state, usize::from(player.slot))
                    .is_some_and(|shown| sjk_identity::names_match(&player.claimed_name, &shown))
            })
            .fold(0_u32, |bits, player| bits | 1 << player.slot);
        if snapshot.me.as_ref().is_some_and(|me| me.verified)
            && let Ok(own) = u32::try_from(game_state.client_num)
            && own < MAX_CLIENTS as u32
        {
            slots |= 1 << own;
        }
        slots
    })
}

/// Ask the hub to change the player's bio. The result shows in
/// [`Snapshot::notice`].
pub(crate) fn set_bio(bio: String) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.set_bio(bio))
        .is_some()
}

/// Give the service the counts the client keeps for its achievements (`achievements.rs`),
/// to send to the hub; false when the service has not started (the identity is off).
pub(crate) fn set_achievement_counts(counts: &std::collections::BTreeMap<String, u64>) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.set_achievement_counts(counts.clone()))
        .is_some()
}

/// The achievements the hub holds for the player, once their profile came; `None`
/// before that or with the feature off.
pub(crate) fn own_achievements() -> Option<Vec<sjk_identity::Achievement>> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        if matches!(
            snapshot.status,
            sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
        ) {
            return None;
        }
        snapshot.me.as_ref().map(|me| me.achievements.clone())
    })
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
        assert_eq!(verified_slots(&GameState::empty_local(0)), 0);
        assert_eq!(report_gate(), crate::ingame_menu::players::Gate::NoIdentity);
        assert_eq!(hub_mark(3, "Sol"), None);
        assert!(player_report_outcome().is_none());
        assert!(own_medals().is_none());
        assert!(own_achievements().is_none());
        assert!(!set_achievement_counts(&std::collections::BTreeMap::new()));
    }
}
